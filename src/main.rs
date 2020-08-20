use async_std::prelude::*;
use std::net::SocketAddr;
use async_std::net::TcpStream;
use std::pin::Pin;
use std::collections::BTreeMap;
use std::task::Poll;
use rand::Rng;

pub mod world;
pub mod codec;
pub mod session;

use session::{Session, SessionEvent};

#[async_std::main]
async fn main() {
    let server_port = if let Ok(port) = std::env::var("PORT") { port.parse::<u16>().unwrap_or(8081) } else { 8081 };
    let inbound = async_std::net::TcpListener::bind(SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), server_port)).await.expect(&format!("Failed to bind to port {}", server_port));
    let sessions: BTreeMap<u16, Session> = BTreeMap::new();
    let mut next_session: u16 = 1;
    
    const TIMESTEP: f32 = 1.0/60.0;
    let ticker = async_std::stream::interval(std::time::Duration::from_secs_f32(TIMESTEP));
    let mut simulation = world::Simulation::new(TIMESTEP);

    let mut free_parts: BTreeMap<u16, world::parts::Part> = BTreeMap::new();
    let mut player_parts: BTreeMap<u16, world::parts::Part> = BTreeMap::new();
    let mut rand = rand::thread_rng();

    struct EventSource {
        pub inbound: async_std::net::TcpListener,
        pub sessions: BTreeMap<u16, Session>,
        pub ticker: async_std::stream::Interval,
        should_simulate: bool
    }
    enum Event {
        NewSession(TcpStream),
        SessionEvent(u16, SessionEvent),
        SessionDisconnect(u16),
        Simulate
    }
    impl Stream for EventSource {
        type Item = Event;
        fn poll_next(mut self: Pin<&mut Self>, ctx: &mut std::task::Context) -> Poll<Option<Event>> {
            if let Poll::Ready(Some(_)) = Pin::new(&mut self.ticker.next() ).poll(ctx) {
                if self.should_simulate {
                    self.should_simulate = false;
                    return Poll::Ready(Some(Event::Simulate));
                }
            }

            for (id, session) in &mut self.sessions {
                if let Poll::Ready(result) = Pin::new(&mut session.next()).poll(ctx) {
                    if let Some(event) = result { return Poll::Ready(Some(Event::SessionEvent(*id, event))); }
                    else { return Poll::Ready(Some(Event::SessionDisconnect(*id))); }
                }
            }
            
            if let Poll::Ready(Ok((socket, _addr))) = unsafe { Pin::new_unchecked(&mut self.inbound.accept()).poll(ctx) } { return Poll::Ready(Some(Event::NewSession(socket))); }
            self.should_simulate = true;
            Poll::Pending
        }
    }
    let mut event_source = EventSource { inbound, ticker, sessions, should_simulate: true };

    while let Some(event) = event_source.next().await {
        use session::SessionEvent::*;
        use Event::*;
        match event {
            NewSession(socket) => {
                let id = next_session;
                next_session += 1;
                event_source.sessions.insert(id, Session::new(socket));
            },
            Simulate => {
                for (id, session) in &mut event_source.sessions {
                    if let Session::Spawned(_, player) = session {
                        player_parts.get(id).unwrap().thrust(&mut simulation.world, &mut player.fuel, player.thrust_forwards, player.thrust_backwards, player.thrust_clockwise, player.thrust_counterclockwise);
                    }
                }
                simulation.simulate();
                let move_messages = session::PartMoveMessage::new_all(simulation.world.get_parts());
                for (id, session) in &mut event_source.sessions { session.update_world(&move_messages); }
            },
            SessionDisconnect(id) => {
                match event_source.sessions.remove(&id) {
                    Some(Session::Spawned(_, _)) => {
                        let mut nuke_messages = Vec::new();
                        fn nuke_part(part: &world::parts::Part, simulation: &mut world::Simulation, nuke_messages: &mut Vec<Vec<u8>>) {
                            simulation.world.remove_part(world::MyHandle::Part(part.body_id));
                            nuke_messages.push(codec::ToClientMsg::RemovePart{id: part.body_id}.serialize());
                            for part in &part.attachments { nuke_part(part, simulation, nuke_messages); }
                        }
                        nuke_messages.push(codec::ToClientMsg::RemovePlayer{ id }.serialize());
                        if let Some(part) = player_parts.remove(&id) {
                            nuke_part(&part, &mut simulation, &mut nuke_messages);
                            for (_session_id, session) in &mut event_source.sessions {
                                if let Session::Spawned(socket, _) = session {
                                    for msg in &nuke_messages { socket.queue_send(async_tungstenite::tungstenite::Message::Binary(msg.clone())); }
                                }
                            };
                        };
                    },
                    _ => ()
                };
            },
            
            SessionEvent(id, ReadyToSpawn) => {
                use world::MyHandle; use world::parts::Part; use codec::*; use async_tungstenite::tungstenite::Message; use session::MyWebSocket;
                use nphysics2d::math::Isometry; use nalgebra::Vector2;
                if let Session::AwaitingHandshake(mut socket) = event_source.sessions.remove(&id).unwrap() {
                    //Graduate session to being existant
                    let core = world::parts::Part::new(world::parts::PartKind::Core, &mut simulation.world, &mut simulation.colliders, &simulation.part_static);
                    let earth_position = *simulation.world.get_rigid(simulation.planets.earth.body).unwrap().position().translation;
                    let core_body = simulation.world.get_rigid_mut(MyHandle::Part(core.body_id)).unwrap();
                    use nphysics2d::object::Body;
                    core_body.apply_force(0, &nphysics2d::algebra::Force2::torque(std::f32::consts::PI), nphysics2d::algebra::ForceType::VelocityChange, true);
                    let spawn_degrees: f32 = rand.gen::<f32>() * std::f32::consts::PI * 2.0;
                    let spawn_radius = simulation.planets.earth.radius * 1.25 + 1.0;
                    core_body.set_position(Isometry::new(Vector2::new(spawn_degrees.sin() * spawn_radius + earth_position.x, spawn_degrees.cos() * spawn_radius + earth_position.y), spawn_degrees - std::f32::consts::FRAC_PI_2));

                    let add_player_msg = codec::ToClientMsg::AddPlayer { id, name: String::default() }.serialize();

                    socket.queue_send(Message::Binary(ToClientMsg::HandshakeAccepted{id, core_id: core.body_id}.serialize()));
                    //Send over celestial object locations
                    for planet in simulation.planets.celestial_objects().iter() {
                        let position = simulation.world.get_rigid(planet.body).unwrap().position().translation;
                        socket.queue_send(Message::Binary(ToClientMsg::AddCelestialObject {
                            name: planet.name.clone(), display_name: planet.name.clone(),
                            id: planet.id, radius: planet.radius, position: (position.x, position.y)
                        }.serialize()));
                    }
                    //Send over all parts
                    fn send_part(part: &Part, simulation: &crate::world::Simulation, socket: &mut MyWebSocket) {
                        let id = part.body_id;
                        let body = simulation.world.get_rigid(MyHandle::Part(id)).unwrap();
                        let position = body.position();
                        socket.queue_send(Message::Binary(ToClientMsg::AddPart{ id: id, kind: part.kind }.serialize()));
                        socket.queue_send(Message::Binary(ToClientMsg::MovePart{
                            id,
                            x: position.translation.x, y: position.translation.y,
                            rotation_n: position.rotation.re, rotation_i: position.rotation.im,
                        }.serialize()));
                        for part in &part.attachments { send_part(part, simulation, socket); }
                    }
                    for (id, part) in &free_parts { send_part(part, &mut simulation, &mut socket); };
                    send_part(&core, &simulation, &mut socket);
                    for (other_id, other_core) in &player_parts {
                        socket.queue_send(async_tungstenite::tungstenite::Message::Binary(codec::ToClientMsg::AddPlayer{ id: *other_id, name: String::default() }.serialize()));
                        send_part(other_core, &mut simulation, &mut socket);
                        if let Some(Session::Spawned(socket, _)) = event_source.sessions.get_mut(other_id) {
                            socket.queue_send(async_tungstenite::tungstenite::Message::Binary(add_player_msg.clone()));
                            send_part(&core, &mut simulation, socket);
                        }
                    }
                    
                    //Graduate to spawned player
                    player_parts.insert(id, core);
                    event_source.sessions.insert(id, Session::Spawned(socket, session::PlayerMeta::default()));
                } else { panic!() }
            },

            SessionEvent(id, ThrusterUpdate { forward, backward, clockwise, counter_clockwise }) => {
                let msg = codec::ToClientMsg::UpdatePlayerMeta {
                    id, thrust_forward: forward, thrust_backward: backward, thrust_clockwise: clockwise, thrust_counter_clockwise: counter_clockwise
                }.serialize();
                for (_other_id, session) in &mut event_source.sessions {
                    if let Session::Spawned(socket, _) = session {
                        socket.queue_send(async_tungstenite::tungstenite::Message::Binary(msg.clone()));
                    }
                }
            }
        }
    }
    
}

async fn _race_all<O>(futures: Vec<&mut (dyn Future<Output = O> + Unpin)>) -> O {
    struct Racer<'a, O> { futures: Vec<&'a mut (dyn Future<Output = O> + Unpin)> }
    impl<'a, O> Future for Racer<'a, O> {
        type Output = O;
        fn poll(mut self: Pin<&mut Self>, ctx: &mut std::task::Context) -> Poll<O> {
            for future in &mut self.futures {
                if let Poll::Ready(result) = Pin::new(future).poll(ctx) { return Poll::Ready(result); }
            }
            Poll::Pending
        }
    };
    (Racer { futures }).await
}
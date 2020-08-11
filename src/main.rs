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
        pub ticker: async_std::stream::Interval
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
            if let Poll::Ready(Some(_)) = Pin::new(&mut self.ticker.next() ).poll(ctx) { return Poll::Ready(Some(Event::Simulate)); }

            for (id, session) in &mut self.sessions {
                if let Poll::Ready(result) = Pin::new(&mut session.next()).poll(ctx) {
                    if let Some(event) = result { return Poll::Ready(Some(Event::SessionEvent(*id, event))); }
                    else { return Poll::Ready(Some(Event::SessionDisconnect(*id))); }
                }
            }
            
            if let Poll::Ready(Ok((socket, _addr))) = unsafe { Pin::new_unchecked(&mut self.inbound.accept()).poll(ctx) } { return Poll::Ready(Some(Event::NewSession(socket))); }
            Poll::Pending
        }
    }
    let mut event_source = EventSource { inbound, ticker, sessions };

    while let Some(event) = event_source.next().await {
        use session::SessionEvent::*;
        use Event::*;
        match event {
            NewSession(socket) => {
                let id = next_session;
                next_session += 1;
                event_source.sessions.insert(id, Session::new(socket));
            },
            Simulate => { simulation.simulate(); },
            SessionDisconnect(id) => { event_source.sessions.remove(&id); },
            
            SessionEvent(id, ReadyToSpawn) => {
                use world::MyHandle; use world::parts::Part; use codec::*; use async_tungstenite::tungstenite::Message; use session::MyWebSocket;
                use nphysics2d::object::Body; use nphysics2d::math::Isometry; use nalgebra::Vector2; use nalgebra::geometry::UnitComplex;
                if let Session::AwaitingHandshake(socket) = event_source.sessions.get_mut(&id).unwrap() {
                    //Graduate session to being existant
                    simulation.world.add_player(id);
                    let core = world::parts::Part::new(world::parts::PartKind::Core, &mut simulation.world, &mut simulation.colliders, &simulation.part_static);
                    let earth_position = *simulation.world.get_rigid(simulation.planets.earth.body).unwrap().position().translation;
                    let core_body = simulation.world.get_rigid_mut(MyHandle::Part(Some(id), core.body_id)).unwrap();                    
                    let spawn_degrees: f32 = rand.gen::<f32>() * std::f32::consts::PI * 2.0;
                    let spawn_radius = simulation.planets.earth.radius * 1.25;
                    core_body.set_position(Isometry::new(Vector2::new(spawn_degrees.sin() * spawn_radius + earth_position.x, spawn_degrees.cos() * spawn_radius + earth_position.y), spawn_degrees + std::f32::consts::FRAC_PI_2));

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
                    fn send_part(id: u16, part: &Part, simulation: &crate::world::Simulation, socket: &mut MyWebSocket) {
                        let body = simulation.world.get_rigid(MyHandle::Part(None, id)).unwrap();
                        let position = body.position();
                        socket.queue_send(Message::Binary(ToClientMsg::AddPart{ id: id, kind: part.kind }.serialize()));
                        socket.queue_send(Message::Binary(ToClientMsg::MovePart{
                            id,
                            x: position.translation.x, y: position.translation.y,
                            rotation_n: position.rotation.re, rotation_i: position.rotation.im,
                        }.serialize()));
                        for part in &part.attachments { send_part(part.body_id, part, simulation, socket); }
                    }
                    for (id, part) in &free_parts { send_part(*id, part, &mut simulation, socket); };
                    send_part(core.body_id, &core, &simulation, socket);
                    for (other_id, core) in &player_parts { send_part(core.body_id, core, &mut simulation, socket); }
                    
                } else { panic!() }
            },

            _ => todo!()
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
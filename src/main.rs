use async_std::prelude::*;
use std::net::SocketAddr;
use async_std::net::TcpStream;
use std::pin::Pin;
use std::collections::{BTreeMap};
use std::task::Poll;
use rand::Rng;
use world::MyHandle;
use world::parts::Part;
use async_tungstenite::tungstenite::Message; use session::MyWebSocket;
use nalgebra::Vector2; use nalgebra::geometry::{Isometry2, UnitComplex};
use ncollide2d::pipeline::object::CollisionGroups;

pub mod world;
pub mod codec;
pub mod session;

use session::{Session, SessionEvent};

pub const TICKS_PER_SECOND: u8 = 20;
pub const DEFAULT_PART_DECAY_TICKS: u16 = TICKS_PER_SECOND as u16 * 10;

#[async_std::main]
async fn main() {
    let server_port = if let Ok(port) = std::env::var("PORT") { port.parse::<u16>().unwrap_or(8081) } else { 8081 };
    let inbound = async_std::net::TcpListener::bind(SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), server_port)).await.expect(&format!("Failed to bind to port {}", server_port));
    let sessions: BTreeMap<u16, Session> = BTreeMap::new();
    let mut next_session: u16 = 1;

    const TIMESTEP: f32 = 1.0/(TICKS_PER_SECOND as f32);
    let ticker = async_std::stream::interval(std::time::Duration::from_secs_f32(TIMESTEP));
    let mut simulation = world::Simulation::new(TIMESTEP);

    let mut free_parts: BTreeMap<u16, FreePart> = BTreeMap::new();
    const MAX_EARTH_CARGOS: u8 = 20; const TICKS_PER_EARTH_CARGO_SPAWN: u8 = TICKS_PER_SECOND * 4;
    let mut earth_cargos: u8 = 0; let mut ticks_til_earth_cargo_spawn: u8 = TICKS_PER_EARTH_CARGO_SPAWN;
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
            if !self.should_simulate { self.should_simulate = true; ctx.waker().wake_by_ref(); }
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
                if earth_cargos < MAX_EARTH_CARGOS {
                    ticks_til_earth_cargo_spawn -= 1;
                    if ticks_til_earth_cargo_spawn == 0 {
                        ticks_til_earth_cargo_spawn = TICKS_PER_EARTH_CARGO_SPAWN;
                        earth_cargos += 1;
                        let earth_position = simulation.world.get_rigid(simulation.planets.earth.body).unwrap().position().translation;
                        let part = world::parts::Part::new(world::parts::PartKind::Cargo, &mut simulation.world, &mut simulation.colliders, &simulation.part_static);
                        let id = part.body_id;
                        let body = simulation.world.get_rigid_mut(MyHandle::Part(part.body_id)).unwrap();
                        let spawn_degrees: f32 = rand.gen::<f32>() * std::f32::consts::PI * 2.0;
                        let spawn_radius = simulation.planets.earth.radius * 1.25 + 1.0;
                        body.set_position(Isometry2::new(Vector2::new(spawn_degrees.sin() * spawn_radius + earth_position.x, spawn_degrees.cos() * spawn_radius + earth_position.y), spawn_degrees));
                        free_parts.insert(part.body_id, FreePart::EarthCargo(part));

                        let add_msg = codec::ToClientMsg::AddPart { id, kind: world::parts::PartKind::Cargo }.serialize();
                        let move_msg = codec::ToClientMsg::MovePart { id, x: body.position().translation.x, y: body.position().translation.y, rotation_i: body.position().rotation.im, rotation_n: body.position().rotation.re }.serialize();
                        for (_id, session) in &mut event_source.sessions {
                            if let Session::Spawned(socket, _) = session {
                                socket.queue_send(Message::Binary(add_msg.clone()));
                                socket.queue_send(Message::Binary(move_msg.clone()));
                            }
                        }
                    }
                }
                let mut random_broadcast_messages: Vec<Vec<u8>> = Vec::new();
                for (id, session) in &mut event_source.sessions {
                    if let Session::Spawned(_, player) = session {
                        if player.fuel > 0 {
                            player_parts.get(id).unwrap().thrust(&mut simulation.world, &mut player.fuel, player.thrust_forwards, player.thrust_backwards, player.thrust_clockwise, player.thrust_counterclockwise);
                            if player.fuel < 1 {
                                player.thrust_backwards = false; player.thrust_forwards = false; player.thrust_clockwise = false; player.thrust_counterclockwise = false;
                                random_broadcast_messages.push(codec::ToClientMsg::UpdatePlayerMeta {
                                   id:  *id,
                                   thrust_forward: player.thrust_forwards, thrust_backward: player.thrust_backwards, thrust_clockwise: player.thrust_clockwise, thrust_counter_clockwise: player.thrust_counterclockwise,
                                   grabed_part: player.grabbed_part.map(|(id,_,_,_)| id)
                                }.serialize());
                            }
                        }
                        if let Some((_part_id, constraint, x, y)) = player.grabbed_part {
                            let position = simulation.world.get_rigid(MyHandle::Part(player_parts.get(&id).unwrap().body_id)).unwrap().position().translation;
                            simulation.move_mouse_constraint(constraint, x + position.x, y + position.y);
                        }
                    }
                }
                simulation.simulate();
                let move_messages = session::PartMoveMessage::new_all(simulation.world.get_parts());
                for (id, session) in &mut event_source.sessions { session.update_world(&move_messages, &random_broadcast_messages); }
            },
            SessionDisconnect(id) => {
                match event_source.sessions.remove(&id) {
                    Some(Session::Spawned(_, _)) => {
                        let mut nuke_messages = Vec::new();
                        fn nuke_part(part: &world::parts::Part, simulation: &mut world::Simulation, nuke_messages: &mut Vec<Vec<u8>>) {
                            simulation.world.remove_part(world::MyHandle::Part(part.body_id));
                            nuke_messages.push(codec::ToClientMsg::RemovePart{id: part.body_id}.serialize());
                            for part in part.attachments.iter() {
                                if let Some((part, _)) = part { nuke_part(part, simulation, nuke_messages); }
                            }
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
                use codec::*; 
                if let Session::AwaitingHandshake(mut socket) = event_source.sessions.remove(&id).unwrap() {
                    //Graduate session to being existant
                    let mut core = world::parts::Part::new(world::parts::PartKind::Core, &mut simulation.world, &mut simulation.colliders, &simulation.part_static);
                    let earth_position = *simulation.world.get_rigid(simulation.planets.earth.body).unwrap().position().translation;
                    let core_body = simulation.world.get_rigid_mut(MyHandle::Part(core.body_id)).unwrap();
                    //core_body.apply_force(0, &nphysics2d::algebra::Force2::torque(std::f32::consts::PI), nphysics2d::algebra::ForceType::VelocityChange, true);
                    let spawn_degrees: f32 = rand.gen::<f32>() * std::f32::consts::PI * 2.0;
                    let spawn_radius = simulation.planets.earth.radius * 1.25 + 1.0;
                    core_body.set_position(Isometry2::new(Vector2::new(spawn_degrees.sin() * spawn_radius + earth_position.x, spawn_degrees.cos() * spawn_radius + earth_position.y), spawn_degrees));

                    let add_player_msg = codec::ToClientMsg::AddPlayer { id, name: String::default(), core_id: core.body_id }.serialize();

                    socket.queue_send(Message::Binary(ToClientMsg::HandshakeAccepted{id, core_id: core.body_id}.serialize()));
                    socket.queue_send(Message::Binary(add_player_msg.clone()));
                    //Send over celestial object locations
                    for planet in simulation.planets.celestial_objects().iter() {
                        let position = simulation.world.get_rigid(planet.body).unwrap().position().translation;
                        socket.queue_send(Message::Binary(ToClientMsg::AddCelestialObject {
                            name: planet.name.clone(), display_name: planet.name.clone(),
                            id: planet.id, radius: planet.radius, position: (position.x, position.y)
                        }.serialize()));
                    }
                    //Send over all parts
                    fn send_part(part: &Part, owning_player: &Option<u16>, simulation: &crate::world::Simulation, socket: &mut MyWebSocket) {
                        let id = part.body_id;
                        let body = simulation.world.get_rigid(MyHandle::Part(id)).unwrap();
                        let position = body.position();
                        socket.queue_send(Message::Binary(ToClientMsg::AddPart{ id: id, kind: part.kind }.serialize()));
                        socket.queue_send(Message::Binary(ToClientMsg::MovePart{
                            id,
                            x: position.translation.x, y: position.translation.y,
                            rotation_n: position.rotation.re, rotation_i: position.rotation.im,
                        }.serialize()));
                        socket.queue_send(Message::Binary(ToClientMsg::UpdatePartMeta{
                            id, owning_player: *owning_player, thrust_mode: part.thrust_mode.into()
                        }.serialize()));
                        for part in part.attachments.iter() {
                            if let Some((part, _)) = part { send_part(part, owning_player, simulation, socket); }
                        }
                    }
                    for (id, part) in &free_parts { send_part(part, &None, &mut simulation, &mut socket); };
                    send_part(&core, &Some(id), &simulation, &mut socket);
                    for (other_id, other_core) in &player_parts {
                        socket.queue_send(async_tungstenite::tungstenite::Message::Binary(codec::ToClientMsg::AddPlayer{ id: *other_id, name: String::default(), core_id: other_core.body_id }.serialize()));
                        send_part(other_core, &Some(*other_id), &mut simulation, &mut socket);
                        if let Some(Session::Spawned(socket, _)) = event_source.sessions.get_mut(other_id) {
                            socket.queue_send(async_tungstenite::tungstenite::Message::Binary(add_player_msg.clone()));
                            send_part(&core, &Some(id), &mut simulation, socket);
                        }
                    }
                    
                    //Graduate to spawned player
                    player_parts.insert(id, core);
                    let meta = session::PlayerMeta::default();
                    socket.queue_send(Message::Binary(codec::ToClientMsg::UpdateMyMeta{ max_fuel: meta.max_fuel }.serialize()));
                    event_source.sessions.insert(id, Session::Spawned(socket, meta));
                } else { panic!() }
            },

            SessionEvent(id, ThrusterUpdate) => {
                if let Some(Session::Spawned(_socket, meta)) = event_source.sessions.get(&id) {
                    let msg = codec::ToClientMsg::UpdatePlayerMeta {
                        id,
                        thrust_forward: meta.thrust_forwards, thrust_backward: meta.thrust_backwards, thrust_clockwise: meta.thrust_clockwise, thrust_counter_clockwise: meta.thrust_counterclockwise,
                        grabed_part: meta.grabbed_part.map(|(id, _, _, _)| id)
                    }.serialize();
                    for (_other_id, session) in &mut event_source.sessions {
                        if let Session::Spawned(socket, _) = session {
                            socket.queue_send(async_tungstenite::tungstenite::Message::Binary(msg.clone()));
                        }
                    }
                }
            },

            SessionEvent(id, CommitGrab{ part_id, x, y }) => {
                if let Some(Session::Spawned(socket, player_meta)) = event_source.sessions.get_mut(&id) {
                    if player_meta.grabbed_part.is_none() {
                        let core_location = simulation.world.get_rigid(MyHandle::Part(player_parts.get(&id).unwrap().body_id)).unwrap().position().translation;
                        let point = nphysics2d::math::Point::new(x + core_location.x, y + core_location.y);
                        let mut grabbed = false;
                        if let Some(free_part) = free_parts.get_mut(&part_id) {
                            if let FreePart::Decaying(part, _) | FreePart::EarthCargo(part) = &free_part {
                                player_meta.grabbed_part = Some((part_id, simulation.equip_mouse_dragging(part_id), x, y));
                                grabbed = true;
                                free_part.become_grabbed(&mut earth_cargos);
                            }
                        }
                        if grabbed {
                            let msg = codec::ToClientMsg::UpdatePlayerMeta {
                                id,
                                thrust_forward: player_meta.thrust_forwards, thrust_backward: player_meta.thrust_backwards, thrust_clockwise: player_meta.thrust_clockwise, thrust_counter_clockwise: player_meta.thrust_counterclockwise,
                                grabed_part: Some(part_id)
                            }.serialize();
                            for (_id, session) in &mut event_source.sessions {
                                if let Session::Spawned(socket, _) = session { socket.queue_send(Message::Binary(msg.clone())); }
                            }
                        };
                    }
                }
            },
            SessionEvent(id, MoveGrab{ x, y }) => {
                if let Some(Session::Spawned(_socket, player_meta)) = event_source.sessions.get_mut(&id) {
                    if let Some((part_id, constraint, _, _)) = player_meta.grabbed_part {
                        //simulation.move_mouse_constraint(constraint, x, y);
                        player_meta.grabbed_part = Some((part_id, constraint, x, y));
                    }
                }
            },
            SessionEvent(id, ReleaseGrab) => {
                if let Some(Session::Spawned(_socket, player_meta)) = event_source.sessions.get_mut(&id) {
                    if let Some((part_id, constraint, x, y)) = player_meta.grabbed_part {
                        simulation.release_constraint(constraint);
                        player_meta.grabbed_part = None;
                        let mut attachment_msg: Option<Vec<u8>> = None;
                        let core_location = simulation.world.get_rigid(MyHandle::Part(player_parts.get(&id).unwrap().body_id)).unwrap().position().clone();
                        //println!("{:?}", point);
                        let grabbed_part_body = simulation.world.get_rigid_mut(MyHandle::Part(part_id)).unwrap();
                        grabbed_part_body.set_local_inertia(free_parts.get(&part_id).unwrap().kind.inertia());
                        grabbed_part_body.set_velocity(nphysics2d::algebra::Velocity2::new(Vector2::new(0.0,0.0), 0.0));
                        fn recurse<'a>(part: &'a mut Part, target_x: f32, target_y: f32, bodies: &world::World) -> Result<(), (&'a mut Part, usize, world::parts::AttachmentPointDetails, (f32, f32))> {
                            let attachments = part.kind.attachment_locations();
                            let pos = bodies.get_rigid(MyHandle::Part(part.body_id)).unwrap().position().clone();
                            for i in 0..part.attachments.len() {
                                if part.attachments[i].is_none() {
                                    if let Some(details) = &attachments[i] {
                                        let mut rotated = rotate_vector(details.x, details.y, pos.rotation.im, pos.rotation.re);
                                        rotated.0 += pos.translation.x;
                                        rotated.1 += pos.translation.y;
                                        if (rotated.0 - target_x).abs() <= 0.4 && (rotated.1 - target_y).abs() <= 0.4 { return Err((part, i, *details, rotated)); }
                                    }
                                }
                            }
                            for subpart in part.attachments.iter_mut() {
                                if let Some((part, _)) = subpart { recurse(part, target_x, target_y, bodies)? }
                            }
                            Ok(())
                        }
                        if let Err((part, slot_id, details, teleport_to)) = recurse(player_parts.get_mut(&id).unwrap(), x + core_location.translation.x, y + core_location.translation.y, &simulation.world) {
                            let grabbed_part_body = simulation.world.get_rigid_mut(MyHandle::Part(part_id)).unwrap();
                            grabbed_part_body.set_position(Isometry2::new(Vector2::new(teleport_to.0, teleport_to.1), details.facing.part_rotation() + core_location.rotation.angle()));
                            part.attachments[slot_id] = Some((free_parts.remove(&part_id).unwrap().extract(), simulation.equip_part_constraint(part.body_id, part_id, details.x, details.y)));
                            attachment_msg = Some(codec::ToClientMsg::UpdatePartMeta { id: part_id, owning_player: Some(id), thrust_mode: 0}.serialize());
                        } else {
                            free_parts.get_mut(&part_id).unwrap().become_decaying();
                        }
                        let msg = codec::ToClientMsg::UpdatePlayerMeta {
                            id,
                            thrust_forward: player_meta.thrust_forwards, thrust_backward: player_meta.thrust_backwards, thrust_clockwise: player_meta.thrust_clockwise, thrust_counter_clockwise: player_meta.thrust_counterclockwise,
                            grabed_part: None
                        }.serialize();
                        for (_id, session) in &mut event_source.sessions {
                            if let Session::Spawned(socket, _) = session { socket.queue_send(Message::Binary(msg.clone())); }
                        }
                        if let Some(msg) = attachment_msg {
                            for (_id, session) in &mut event_source.sessions {
                                if let Session::Spawned(socket, _) = session { socket.queue_send(Message::Binary(msg.clone())); }
                            }
                        }
                    }
                }
            }
        }
    }
}

enum FreePart {
    Decaying(world::parts::Part, u16),
    EarthCargo(world::parts::Part),
    Grabbed(world::parts::Part),
    PlaceholderLol,
}
impl std::ops::Deref for FreePart {
    type Target = world::parts::Part;
    fn deref(&self) -> &world::parts::Part {
        match self {
            FreePart::Decaying(part, _) => part,
            FreePart::EarthCargo(part) => part,
            FreePart::Grabbed(part) => part,
            FreePart::PlaceholderLol => panic!("Attempted to get part from placeholder"),
        }
    }
}
impl std::ops::DerefMut for FreePart {
    fn deref_mut(&mut self) -> &mut world::parts::Part {
        match self {
            FreePart::Decaying(part, _) => part,
            FreePart::EarthCargo(part) => part,
            FreePart::Grabbed(part) => part,
            FreePart::PlaceholderLol => panic!("Attempted to get part from placeholder"),
        }
    }
}
impl FreePart {
    pub fn become_grabbed(&mut self, earth_cargo_count: &mut u8) {
        match &self {
            FreePart::EarthCargo(_) => { *earth_cargo_count -= 1; },
            _ => ()
        }
        let potato = match std::mem::replace(self, FreePart::PlaceholderLol) {
            FreePart::PlaceholderLol => panic!("Become transform on Placerholderlol"),
            FreePart::Decaying(part, _) => FreePart::Grabbed(part),
            FreePart::EarthCargo(part) => FreePart::Grabbed(part),
            FreePart::Grabbed(_) => panic!("Into FreePart::Grabbed called on Grabbed")
        };
        *self = potato;
    }
    pub fn become_decaying(&mut self) {
        let potato = match std::mem::replace(self, FreePart::PlaceholderLol) {
            FreePart::PlaceholderLol => panic!("Become transform on Placerholderlol"),
            FreePart::Decaying(part, _) | FreePart::Grabbed(part) => FreePart::Decaying(part, DEFAULT_PART_DECAY_TICKS),
            FreePart::EarthCargo(_) => panic!("EarthCargo into Decaying directly"),
        };
        *self = potato;
    }
    pub fn extract(self) -> Part {
        match self {
            FreePart::PlaceholderLol => panic!("Tried to extract placeholderlol"),
            FreePart::Decaying(part, _) => part,
            FreePart::EarthCargo(part) => part,
            FreePart::Grabbed(part) => part,
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

pub fn rotate_vector_with_angle(x: f32, y: f32, theta: f32) -> (f32, f32) { rotate_vector(x, y, theta.sin(), theta.cos()) }
pub fn rotate_vector(x: f32, y: f32, theta_sin: f32, theta_cos: f32) -> (f32, f32) {
    ((x * theta_cos) - (y * theta_sin), (x * theta_sin) + (y * theta_cos))
}
use async_std::prelude::*;
use std::net::SocketAddr;
use async_std::net::TcpStream;
use std::pin::Pin;
use std::collections::{BTreeMap, BTreeSet};
use std::task::Poll;
use rand::Rng;
use world::MyHandle;
use world::parts::{Part, AttachedPartFacing};
use async_tungstenite::tungstenite::Message; use session::MyWebSocket;
use nalgebra::Vector2; use nalgebra::geometry::{Isometry2, UnitComplex};
use ncollide2d::pipeline::object::CollisionGroups;

pub mod world;
pub mod codec;
pub mod session;

use session::{Session, SessionEvent};

pub const TICKS_PER_SECOND: u8 = 20;
pub const DEFAULT_PART_DECAY_TICKS: u16 = TICKS_PER_SECOND as u16 * 20;

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
    let mut simulation_events = Vec::new();
    struct PlayerPlanetInteractionMeta { planet_id: u16, ticks_til_cargo_transform: u8, touching_parts: BTreeSet<u16> }
    let mut player_planet_metas: BTreeMap<u16, PlayerPlanetInteractionMeta> = BTreeMap::new();
    const TICKS_PER_CARGO_UPGRADE: u8 = TICKS_PER_SECOND;

    let my_thruster_1 = world::parts::Part::new(world::parts::PartKind::Hub, &mut simulation.world, &mut simulation.colliders, &simulation.part_static);
    simulation.world.get_rigid_mut(MyHandle::Part(my_thruster_1.body_id)).unwrap().set_position(Isometry2::new(Vector2::new(0.0, 27.0), 0.0));
    free_parts.insert(my_thruster_1.body_id, FreePart::Decaying(my_thruster_1, DEFAULT_PART_DECAY_TICKS));
    let my_thruster_2 = world::parts::Part::new(world::parts::PartKind::Hub, &mut simulation.world, &mut simulation.colliders, &simulation.part_static);
    simulation.world.get_rigid_mut(MyHandle::Part(my_thruster_2.body_id)).unwrap().set_position(Isometry2::new(Vector2::new(2.0, 27.0), 0.0));
    free_parts.insert(my_thruster_2.body_id, FreePart::Decaying(my_thruster_2, DEFAULT_PART_DECAY_TICKS));
    let my_thruster_3 = world::parts::Part::new(world::parts::PartKind::Hub, &mut simulation.world, &mut simulation.colliders, &simulation.part_static);
    simulation.world.get_rigid_mut(MyHandle::Part(my_thruster_3.body_id)).unwrap().set_position(Isometry2::new(Vector2::new(4.0, 27.0), 0.0));
    free_parts.insert(my_thruster_3.body_id, FreePart::Decaying(my_thruster_3, DEFAULT_PART_DECAY_TICKS));
    let my_thruster_4 = world::parts::Part::new(world::parts::PartKind::LandingThruster, &mut simulation.world, &mut simulation.colliders, &simulation.part_static);
    simulation.world.get_rigid_mut(MyHandle::Part(my_thruster_4.body_id)).unwrap().set_position(Isometry2::new(Vector2::new(6.0, 27.0), 0.0));
    free_parts.insert(my_thruster_4.body_id, FreePart::Decaying(my_thruster_4, DEFAULT_PART_DECAY_TICKS));
    let my_thruster_5 = world::parts::Part::new(world::parts::PartKind::LandingThruster, &mut simulation.world, &mut simulation.colliders, &simulation.part_static);
    simulation.world.get_rigid_mut(MyHandle::Part(my_thruster_5.body_id)).unwrap().set_position(Isometry2::new(Vector2::new(8.0, 27.0), 0.0));
    free_parts.insert(my_thruster_5.body_id, FreePart::Decaying(my_thruster_5, DEFAULT_PART_DECAY_TICKS));
    
    let mut ticks_til_power_regen = 5u8;

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
                let mut random_broadcast_messages: Vec<Vec<u8>> = Vec::new();
                let mut to_delete: Vec<u16> = Vec::new();
                for (id, part) in free_parts.iter_mut() {
                    if let FreePart::Decaying(_, ticks) = part {
                        *ticks -= 1;
                        if *ticks < 1 { to_delete.push(*id); }
                    }
                }
                for to_delete in to_delete {
                    if let Some(FreePart::Decaying(_part, _)) = free_parts.remove(&to_delete) {
                        simulation.world.remove_part(MyHandle::Part(to_delete));
                        random_broadcast_messages.push(codec::ToClientMsg::RemovePart{ id: to_delete }.serialize());
                    }
                }
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
                        body.set_position(Isometry2::new(Vector2::new(spawn_degrees.sin() * spawn_radius + earth_position.x, spawn_degrees.cos() * spawn_radius + earth_position.y), 0.0)); // spawn_degrees));
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
                ticks_til_power_regen -= 1;
                let is_power_regen_tick;
                if ticks_til_power_regen == 0 { ticks_til_power_regen = 5; is_power_regen_tick = true; }
                else { is_power_regen_tick = false; }
                for (id, session) in &mut event_source.sessions {
                    if let Session::Spawned(socket, player) = session {
                        if is_power_regen_tick {
                            player.power += player.power_regen_per_5_ticks;
                            if player.power > player.max_power { player.power = player.max_power; };
                        };
                        if player.power > 0 {
                            player_parts.get(id).unwrap().thrust(&mut simulation.world, &mut player.power, player.thrust_forwards, player.thrust_backwards, player.thrust_clockwise, player.thrust_counterclockwise);
                            if player.power < 1 {
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
                        if let Some(meta) = player_planet_metas.get_mut(id) {
                            meta.ticks_til_cargo_transform -= 1;
                            if meta.ticks_til_cargo_transform < 1 {
                                meta.ticks_til_cargo_transform = TICKS_PER_CARGO_UPGRADE;
                                if let Some(upgrade_into) = simulation.planets.get_celestial_object(meta.planet_id).unwrap().cargo_upgrade {
                                    fn recurse<'a>(part: &'a mut Part) -> Result<(),(&'a mut Part, usize)> {
                                        let len = part.attachments.len();
                                        for i in 0..len {
                                            if let Some((subpart, _connection, _connection2)) = &part.attachments[i] {
                                                if subpart.kind == world::parts::PartKind::Cargo { return Err((part, i)); }
                                            }
                                        };
                                        for subpart in part.attachments.iter_mut() {
                                            if let Some((part, _, _)) = subpart.as_mut() { recurse(part)?; }
                                        }
                                        Ok(())
                                    }
                                    if let Err((parent_part, slot)) = recurse(player_parts.get_mut(id).unwrap()) {
                                        //simulation.release_constraint(parent_part.attachments[slot].as_ref().unwrap().1);
                                        let part = &mut parent_part.attachments[slot].as_mut().unwrap().0;
                                        part.mutate(upgrade_into, &mut simulation.world, &mut simulation.colliders, &simulation.part_static);
                                        player.max_power -= world::parts::PartKind::Cargo.power_storage();
                                        player.max_power += upgrade_into.power_storage();
                                        player.power_regen_per_5_ticks -= world::parts::PartKind::Cargo.power_regen_per_5_ticks();
                                        player.power_regen_per_5_ticks += upgrade_into.power_regen_per_5_ticks();
                                        socket.queue_send(Message::Binary(codec::ToClientMsg::UpdateMyMeta{ max_fuel: player.max_power }.serialize()));
                                        
                                        random_broadcast_messages.push(codec::ToClientMsg::RemovePart{ id: part.body_id }.serialize());
                                        random_broadcast_messages.push(codec::ToClientMsg::AddPart{ id: part.body_id, kind: part.kind, }.serialize());
                                        random_broadcast_messages.push(codec::ToClientMsg::UpdatePartMeta{ id: part.body_id, owning_player: Some(*id), thrust_mode: part.thrust_mode.into() }.serialize());
                                    }   
                                }
                            }
                        }
                    }
                }

                simulation.simulate(&mut simulation_events);
                for event in simulation_events.drain(..) {
                    use world::SimulationEvent::*;
                    match event {
                        PlayerTouchPlanet{ player, planet, part } => {
                            let player_planet_meta = if let Some(meta) = player_planet_metas.get_mut(&player) { meta }
                            else {
                                player_planet_metas.insert(player, PlayerPlanetInteractionMeta {
                                    planet_id: planet,
                                    ticks_til_cargo_transform: TICKS_PER_CARGO_UPGRADE,
                                    touching_parts: BTreeSet::new()
                                });
                                player_planet_metas.get_mut(&player).unwrap()
                            };
                            if player_planet_meta.planet_id != planet {
                                println!("REEEEEEE");
                                for part in &player_planet_meta.touching_parts { print!("{}", part); }
                                println!("");
                            }
                            player_planet_meta.touching_parts.insert(part);
                            if let Some(Session::Spawned(_socket, player_meta)) = event_source.sessions.get_mut(&player) {
                                player_meta.power = player_meta.max_power;
                            }
                        },
                        PlayerUntouchPlanet{ player, planet, part } => {
                            if let Some(meta) = player_planet_metas.get_mut(&player) {
                                if meta.touching_parts.remove(&part) {
                                    if meta.touching_parts.is_empty() {
                                        player_planet_metas.remove(&player);
                                    }
                                }
                            }
                        },
                        PartDetach{ parent_part, detached_part, player } => todo!()
                    }
                }

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
                                if let Some((part, _, _)) = part { nuke_part(part, simulation, nuke_messages); }
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
                    simulation.colliders.get_mut(core.collider).unwrap().set_user_data(Some(Box::new(PartOfPlayer(id))));
                    //core_body.apply_force(0, &nphysics2d::algebra::Force2::torque(std::f32::consts::PI), nphysics2d::algebra::ForceType::VelocityChange, true);
                    let spawn_degrees: f32 = rand.gen::<f32>() * std::f32::consts::PI * 2.0;
                    let spawn_radius = simulation.planets.earth.radius * 1.25 + 1.0;
                    core_body.set_position(Isometry2::new(Vector2::new(spawn_degrees.sin() * spawn_radius + earth_position.x, spawn_degrees.cos() * spawn_radius + earth_position.y), 0.0));

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
                            if let Some((part, _, _)) = part { send_part(part, owning_player, simulation, socket); }
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
                    socket.queue_send(Message::Binary(codec::ToClientMsg::UpdateMyMeta{ max_fuel: meta.max_power }.serialize()));
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
                        let mut random_on_grabbed_messages: Vec<Vec<u8>> = Vec::new();
                        if let Some(free_part) = free_parts.get_mut(&part_id) {
                            if let FreePart::Decaying(part, _) | FreePart::EarthCargo(part) = &free_part {
                                player_meta.grabbed_part = Some((part_id, simulation.equip_mouse_dragging(part_id), x, y));
                                grabbed = true;
                                free_part.become_grabbed(&mut earth_cargos);
                            }
                        } else {
                            fn recurse_2(part: &mut Part, target_part: u16, free_parts: &mut BTreeMap<u16, FreePart>, simulation: &mut world::Simulation, random_on_grabbed_messages: &mut Vec<Vec<u8>>) -> Result<(),(Part, u16, u16)> {
                                for slot in part.attachments.iter_mut() {
                                    if let Some((part, connection, connection2)) = slot {
                                        if part.body_id == target_part {
                                            fn recursive_detatch(part: &mut Part, free_parts: &mut BTreeMap<u16, FreePart>, simulation: &mut world::Simulation, random_on_grabbed_messages: &mut Vec<Vec<u8>>, max_power_lost: &mut u16, regen_lost: &mut u16) {
                                                for slot in part.attachments.iter_mut() {
                                                    if let Some((part, connection, connection2)) = slot {
                                                        simulation.release_constraint(*connection);
                                                        simulation.release_constraint(*connection2);
                                                        *max_power_lost += part.kind.power_storage();
                                                        *regen_lost += part.kind.power_regen_per_5_ticks();
                                                        recursive_detatch(part, free_parts, simulation, random_on_grabbed_messages, max_power_lost, regen_lost);
                                                        if let Some((part, _, _)) = std::mem::replace(slot, None) {
                                                            random_on_grabbed_messages.push(codec::ToClientMsg::UpdatePartMeta{ id: part.body_id, owning_player: None, thrust_mode: 0 }.serialize());
                                                            free_parts.insert(part.body_id, FreePart::Decaying(part, DEFAULT_PART_DECAY_TICKS));
                                                        }
                                                    }
                                                }
                                            }
                                            let mut max_power_lost: u16 = 0;
                                            let mut regen_lost: u16 = 0;
                                            recursive_detatch(part, free_parts, simulation, random_on_grabbed_messages, &mut max_power_lost, &mut regen_lost);
                                            simulation.release_constraint(*connection);
                                            simulation.release_constraint(*connection2);
                                            if let Some((part, _, _)) = std::mem::replace(slot, None) {
                                                return Err((part, max_power_lost, regen_lost));
                                            }
                                        }
                                    }
                                }
                                for slot in part.attachments.iter_mut() {
                                    if let Some((part, _, _)) = slot {
                                        recurse_2(part, target_part, free_parts, simulation, random_on_grabbed_messages)?;
                                    }
                                }
                                Ok(())
                            }
                            if let Err((part, max_power_lost, regen_lost)) = recurse_2(player_parts.get_mut(&id).unwrap(), part_id, &mut free_parts, &mut simulation, &mut random_on_grabbed_messages) {
                                player_meta.grabbed_part = Some((part_id, simulation.equip_mouse_dragging(part_id), x, y));
                                println!("{:?}", part.kind);
                                player_meta.max_power -= part.kind.power_storage();
                                player_meta.max_power -= max_power_lost;
                                if player_meta.power > player_meta.max_power { player_meta.power = player_meta.max_power };
                                player_meta.power_regen_per_5_ticks -= regen_lost;
                                socket.queue_send(Message::Binary(codec::ToClientMsg::UpdateMyMeta{ max_fuel: player_meta.max_power }.serialize()));
                                simulation.colliders.get_mut(part.collider).unwrap().set_user_data(None);
                                grabbed = true;
                                if let Some(planet_meta) = player_planet_metas.get_mut(&id) {
                                    if planet_meta.touching_parts.remove(&part.body_id) {
                                        if planet_meta.touching_parts.is_empty() {
                                            player_planet_metas.remove(&id);
                                        }
                                    }
                                }

                                free_parts.insert(part_id, FreePart::Grabbed(part));
                            }
                        }
                        if grabbed {
                            let msg = codec::ToClientMsg::UpdatePlayerMeta {
                                id,
                                thrust_forward: player_meta.thrust_forwards, thrust_backward: player_meta.thrust_backwards, thrust_clockwise: player_meta.thrust_clockwise, thrust_counter_clockwise: player_meta.thrust_counterclockwise,
                                grabed_part: Some(part_id)
                            }.serialize();
                            let msg2 = codec::ToClientMsg::UpdatePartMeta {
                                id: part_id, thrust_mode: 0, owning_player: None
                            }.serialize();
                            for (_id, session) in &mut event_source.sessions {
                                if let Session::Spawned(socket, _) = session {
                                    socket.queue_send(Message::Binary(msg.clone()));
                                    socket.queue_send(Message::Binary(msg2.clone()));
                                    for msg in &random_on_grabbed_messages { socket.queue_send(Message::Binary(msg.clone())); };
                                }
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
                if let Some(Session::Spawned(socket, player_meta)) = event_source.sessions.get_mut(&id) {
                    if let Some((part_id, constraint, x, y)) = player_meta.grabbed_part {
                        simulation.release_constraint(constraint);
                        player_meta.grabbed_part = None;
                        let mut attachment_msg: Option<Vec<u8>> = None;
                        let core_location = simulation.world.get_rigid(MyHandle::Part(player_parts.get(&id).unwrap().body_id)).unwrap().position().clone();
                        let grabbed_part_body = simulation.world.get_rigid_mut(MyHandle::Part(part_id)).unwrap();
                        grabbed_part_body.set_local_inertia(free_parts.get(&part_id).unwrap().kind.inertia());
                        grabbed_part_body.set_velocity(nphysics2d::algebra::Velocity2::new(Vector2::new(0.0,0.0), 0.0));

                        use world::parts::CompactThrustMode;
                        println!("Start");
                        fn recurse<'a>(part: &'a mut Part, target_x: f32, target_y: f32, bodies: &world::World, parent_actual_rotation: world::parts::AttachedPartFacing, x: i16, y: i16) -> Result<(), (&'a mut Part, usize, world::parts::AttachmentPointDetails, (f32, f32), CompactThrustMode, world::parts::AttachedPartFacing)> {
                            let attachments = part.kind.attachment_locations();
                            let pos = bodies.get_rigid(MyHandle::Part(part.body_id)).unwrap().position().clone();
                            println!("Iter");
                            for i in 0..part.attachments.len() {
                                if part.attachments[i].is_none() {
                                    if let Some(details) = &attachments[i] {
                                        println!("r {:?} {:?}", details, details.facing.get_actual_rotation(parent_actual_rotation));
                                        let mut rotated = rotate_vector(details.x, details.y, pos.rotation.im, pos.rotation.re);
                                        rotated.0 += pos.translation.x;
                                        rotated.1 += pos.translation.y;
                                        if (rotated.0 - target_x).abs() <= 0.4 && (rotated.1 - target_y).abs() <= 0.4 {
                                            let my_actual_facing = details.facing.get_actual_rotation(parent_actual_rotation);
                                            use world::parts::{HorizontalThrustMode, VerticalThrustMode};
                                            let hroizontal = match my_actual_facing {
                                                AttachedPartFacing::Up => if x < 0 { HorizontalThrustMode::CounterClockwise } else if x > 0 { HorizontalThrustMode::Clockwise } else { HorizontalThrustMode::None },
                                                AttachedPartFacing::Right => if y > 0 { HorizontalThrustMode::CounterClockwise } else { HorizontalThrustMode::Clockwise },
                                                AttachedPartFacing::Down => if x < 0 { HorizontalThrustMode::Clockwise } else if x > 0 { HorizontalThrustMode::CounterClockwise } else { HorizontalThrustMode::None },
                                                AttachedPartFacing::Left => if y > 0 { HorizontalThrustMode::Clockwise } else { HorizontalThrustMode::CounterClockwise },
                                            };
                                            let vertical = match my_actual_facing  {
                                                AttachedPartFacing::Up => VerticalThrustMode::Backwards,
                                                AttachedPartFacing::Down => VerticalThrustMode::Forwards,
                                                AttachedPartFacing::Left | AttachedPartFacing::Right => VerticalThrustMode::None
                                            };
                                            let thrust_mode = CompactThrustMode::new(hroizontal, vertical);
                                            return Err((part, i, *details, rotated, thrust_mode, my_actual_facing));
                                        }
                                    }
                                }
                            }
                            for (i, subpart) in part.attachments.iter_mut().enumerate() {
                                if let Some((part, _, _)) = subpart {
                                    let my_actual_rotation = attachments[i].unwrap().facing.get_actual_rotation(parent_actual_rotation);
                                    let new_x = x + match my_actual_rotation { AttachedPartFacing::Left => -1, AttachedPartFacing::Right => 1, _ => 0 };
                                    let new_y = y + match my_actual_rotation { AttachedPartFacing::Up => 1, AttachedPartFacing::Down => -1, _ => 0 };
                                    recurse(part, target_x, target_y, bodies, my_actual_rotation, new_x, new_y)?
                                }
                            }
                            Ok(())
                        }
                        if let Err((part, slot_id, details, teleport_to, thrust_mode, my_actual_facing)) = recurse(
                            player_parts.get_mut(&id).unwrap(), 
                            x + core_location.translation.x, 
                            y + core_location.translation.y, 
                            &simulation.world,
                            world::parts::AttachedPartFacing::Up,
                            0, 0
                        ) {
                            let grabbed_part_body = simulation.world.get_rigid_mut(MyHandle::Part(part_id)).unwrap();
                            println!("e {:?} {:?} {:?}", details, core_location, my_actual_facing);
                            grabbed_part_body.set_position(Isometry2::new(Vector2::new(teleport_to.0, teleport_to.1), my_actual_facing.part_rotation() + core_location.rotation.angle()));
                            let (connection1, connection2) = simulation.equip_part_constraint(part.body_id, part_id, part.kind.attachment_locations()[slot_id].unwrap());

                            let mut grabbed_part = free_parts.remove(&part_id).unwrap().extract();
                            player_meta.max_power += grabbed_part.kind.power_storage();
                            player_meta.power_regen_per_5_ticks += grabbed_part.kind.power_regen_per_5_ticks();
                            socket.queue_send(Message::Binary(codec::ToClientMsg::UpdateMyMeta{ max_fuel: player_meta.max_power }.serialize()));
                            grabbed_part.thrust_mode = thrust_mode;
                            simulation.colliders.get_mut(grabbed_part.collider).unwrap().set_user_data(Some(Box::new(PartOfPlayer(id))));
                            part.attachments[slot_id] = Some((grabbed_part, connection1, connection2));
                            attachment_msg = Some(codec::ToClientMsg::UpdatePartMeta { id: part_id, owning_player: Some(id), thrust_mode: thrust_mode.into() }.serialize());
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

pub struct PartOfPlayer (u16);
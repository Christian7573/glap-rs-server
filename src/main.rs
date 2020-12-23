#[macro_use] extern crate serde_derive;
use async_std::prelude::*;
use std::net::SocketAddr;
use futures::{FutureExt, StreamExt};
use std::pin::Pin;
use std::collections::{BTreeMap, BTreeSet};
use std::task::Poll;
use rand::Rng;
use world::MyHandle;
use world::parts::{Part, AttachedPartFacing};
use nalgebra::Vector2; use nalgebra::geometry::{Isometry2, UnitComplex};
use ncollide2d::pipeline::object::CollisionGroups;
use std::sync::Arc;
use std::any::Any;
use async_std::sync::{Sender, Receiver, channel};

pub mod world;
pub mod codec;
pub mod session;
pub mod beamout;
use codec::*;
use session::ToSerializerEvent;

pub const TICKS_PER_SECOND: u8 = 20;
pub const DEFAULT_PART_DECAY_TICKS: u16 = TICKS_PER_SECOND as u16 * 20;

#[derive(Clone)]
pub struct ApiDat { prefix: String, beamout: String, beamin: String, password: String }

/*#[derive(Clone)]
pub struct AbortKit { sender: Sender<()>, reciever: Reciever<()>, handle: futures::future::AbortHandle, registration: futures::future::AbortRegistration }
impl AbortKit {
    pub fn new() -> AbortKit {
        let (sender, reciever) = channel(4);
        let (handle, registration) = AbortHandle::new_pair();
        AbortKit {
            sender, reciever,
            handle, registration
        }
    }
    pub fn make_abortable<F: Future>(&self, future: F) -> futures::future::Abortable<F> {
        futures::future::Abortable::new(future, self.registration.clone())
    }
    pub async fn abort(&mut self) {
        self.sender.send(()).await
    }
}*/

#[async_std::main]
async fn main() {
    let server_port = if let Ok(port) = std::env::var("PORT") { port.parse::<u16>().unwrap_or(8081) } else { 8081 };
    let listener = async_std::net::TcpListener::bind(SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), server_port)).await.expect(&format!("Failed to bind to port {}", server_port));

    let api = std::env::var("API").ok().map(|prefix| ApiDat {
        prefix: prefix.clone(),
        beamout: prefix.clone() + "/user/^^^^/beamout",
        beamin: prefix.clone() + "/session/^^^^/beamin",
        password: std::env::var("API_PASSWORD").unwrap_or(String::with_capacity(0)),
    });

    let api = if let Some(api) = api {
        let ping_addr = api.prefix.clone() + "/ping";
        println!("Pinging API at ${}", ping_addr);
        let res = surf::get(ping_addr).await;
        if let Ok(mut res) = res {
            if res.status().is_success() && res.body_string().await.map(|body| body == "PONG" ).unwrap_or(false) { println!("API Ping success"); Some(api) }
            else { eprintln!("API Ping Failed"); None }
        } else { eprintln!("API Ping Failed Badly"); None }
    } else { println!("No API"); None };

    let api = api.map(|api| Arc::new(api));

    let (to_game, to_me) = channel::<session::ToGameEvent>(1024);
    let (to_serializer, to_me_serializer) = channel::<Vec<session::ToSerializerEvent>>(256);
    println!("Hello from game task");
    let _incoming_connection_acceptor = async_std::task::Builder::new()
        .name("incoming_connection_acceptor".to_string())
        .spawn(session::incoming_connection_acceptor(listener, to_game.clone(), to_serializer.clone(), api.clone()));
    let _serializer = async_std::task::Builder::new()
        .name("serializer".to_string())
        .spawn(session::serializer(to_me_serializer, to_game.clone()));

    const TIMESTEP: f32 = 1.0/(TICKS_PER_SECOND as f32);
    let ticker = async_std::stream::interval(std::time::Duration::from_secs_f32(TIMESTEP));
    let mut simulation = world::Simulation::new(TIMESTEP);

    let mut players: BTreeMap<u16, (PlayerMeta, Part)> = BTreeMap::new();
    let mut free_parts: BTreeMap<u16, FreePart> = BTreeMap::new();
    const MAX_EARTH_CARGOS: u8 = 20; const TICKS_PER_EARTH_CARGO_SPAWN: u8 = TICKS_PER_SECOND * 4;
    let mut earth_cargos: u8 = 0; let mut ticks_til_earth_cargo_spawn: u8 = TICKS_PER_EARTH_CARGO_SPAWN;
    let mut rand = rand::thread_rng();

    struct EventSource {
        pub inbound: async_std::sync::Receiver<session::ToGameEvent>,
        pub ticker: async_std::stream::Interval,
    }
    enum Event {
        InboundEvent(session::ToGameEvent),
        Simulate
    }
    impl Stream for EventSource {
        type Item = Event;
        fn poll_next(mut self: Pin<&mut Self>, ctx: &mut std::task::Context) -> Poll<Option<Event>> {
            if let Poll::Ready(Some(_)) = self.ticker.poll_next_unpin(ctx) { return Poll::Ready(Some(Event::Simulate)); }
            match self.inbound.poll_next_unpin(ctx) {
                Poll::Ready(Some(event)) => return Poll::Ready(Some(Event::InboundEvent(event))),
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => ()
            };
            Poll::Pending
        }
    }
    let mut event_source = EventSource { inbound: to_me, ticker };
    let mut simulation_events = Vec::new();
    const TICKS_PER_CARGO_UPGRADE: u8 = TICKS_PER_SECOND;

    /*let my_thruster_1 = world::parts::Part::new(world::parts::PartKind::Hub, &mut simulation.world, &mut simulation.colliders, &simulation.part_static);
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
    free_parts.insert(my_thruster_5.body_id, FreePart::Decaying(my_thruster_5, DEFAULT_PART_DECAY_TICKS));*/
    
    let mut ticks_til_power_regen = 5u8;

    while let Some(event) = event_source.next().await {
        use session::ToGameEvent::*;
        use session::ToSerializerEvent as ToSerializer;
        let mut outbound_events = Vec::new();
        match event {
            Event::Simulate => {
                let mut to_delete: Vec<u16> = Vec::new();
                for (id, part) in free_parts.iter_mut() {
                    match part {
                        FreePart::Decaying(_, ticks) => {
                            *ticks -= 1;
                            if *ticks < 1 { to_delete.push(*id); }
                        },
                        FreePart::EarthCargo(part, ticks) => {
                            *ticks -= 1;
                            if *ticks < 1 {
                                let earth_position = simulation.world.get_rigid(simulation.planets.earth.body).unwrap().position().translation;
                                let body = simulation.world.get_rigid_mut(MyHandle::Part(part.body_id)).unwrap();
                                let spawn_degrees: f32 = rand.gen::<f32>() * std::f32::consts::PI * 2.0;
                                let spawn_radius = simulation.planets.earth.radius * 1.25 + 1.0;
                                body.set_position(Isometry2::new(Vector2::new(spawn_degrees.sin() * spawn_radius + earth_position.x, spawn_degrees.cos() * spawn_radius + earth_position.y), 0.0)); // spawn_degrees));
                                use nphysics2d::object::Body;
                                body.apply_force(0, &nphysics2d::math::Force::zero(), nphysics2d::math::ForceType::Force, true);
                                *ticks = TICKS_PER_SECOND as u16 * 60;
                            }
                        },
                        FreePart::Grabbed(_) => (),
                        FreePart::PlaceholderLol => panic!(),
                    }
                }
                for to_delete in to_delete {
                    if let Some(FreePart::Decaying(_part, _)) = free_parts.remove(&to_delete) {
                        simulation.world.remove_part(MyHandle::Part(to_delete));
                        outbound_events.push(ToSerializer::Broadcast(codec::ToClientMsg::RemovePart{ id: to_delete }));
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
                        free_parts.insert(part.body_id, FreePart::EarthCargo(part, TICKS_PER_SECOND as u16 * 60));

                        outbound_events.push(ToSerializer::Broadcast(codec::ToClientMsg::AddPart { id, kind: world::parts::PartKind::Cargo }));
                        outbound_events.push(ToSerializer::Broadcast(codec::ToClientMsg::MovePart { id, x: body.position().translation.x, y: body.position().translation.y, rotation_i: body.position().rotation.im, rotation_n: body.position().rotation.re }));
                    }
                }
                ticks_til_power_regen -= 1;
                let is_power_regen_tick;
                if ticks_til_power_regen == 0 { ticks_til_power_regen = 5; is_power_regen_tick = true; }
                else { is_power_regen_tick = false; }
                for (id, (player, part)) in &mut players {
                    if is_power_regen_tick {
                        player.power += player.power_regen_per_5_ticks;
                        if player.power > player.max_power { player.power = player.max_power; };
                    };
                    if player.power > 0 {
                        part.thrust(&mut simulation.world, &mut player.power, player.thrust_forwards, player.thrust_backwards, player.thrust_clockwise, player.thrust_counterclockwise);
                        if player.power < 1 {
                            player.thrust_backwards = false; player.thrust_forwards = false; player.thrust_clockwise = false; player.thrust_counterclockwise = false;
                            outbound_events.push(ToSerializer::Broadcast(codec::ToClientMsg::UpdatePlayerMeta {
                                id:  *id,
                                thrust_forward: player.thrust_forwards, thrust_backward: player.thrust_backwards, thrust_clockwise: player.thrust_clockwise, thrust_counter_clockwise: player.thrust_counterclockwise,
                                grabed_part: player.grabbed_part.map(|(id,_,_,_)| id)
                            }));
                        }
                    }
                    if let Some((_part_id, constraint, x, y)) = player.grabbed_part {
                        let position = simulation.world.get_rigid(MyHandle::Part(part.body_id)).unwrap().position().translation;
                        simulation.move_mouse_constraint(constraint, x + position.x, y + position.y);
                    }
                    if let Some(planet_id) = player.touching_planet {
                        player.ticks_til_cargo_transform -= 1;
                        if player.ticks_til_cargo_transform < 1 {
                            player.ticks_til_cargo_transform = TICKS_PER_CARGO_UPGRADE;
                            if let Some(upgrade_into) = simulation.planets.get_celestial_object(planet_id).unwrap().cargo_upgrade {
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
                                if let Err((parent_part, slot)) = recurse(part) {
                                    //simulation.release_constraint(parent_part.attachments[slot].as_ref().unwrap().1);
                                    let part = &mut parent_part.attachments[slot].as_mut().unwrap().0;
                                    part.mutate(upgrade_into, &mut simulation.world, &mut simulation.colliders, &simulation.part_static);
                                    player.max_power -= world::parts::PartKind::Cargo.power_storage();
                                    player.max_power += upgrade_into.power_storage();
                                    player.power_regen_per_5_ticks -= world::parts::PartKind::Cargo.power_regen_per_5_ticks();
                                    player.power_regen_per_5_ticks += upgrade_into.power_regen_per_5_ticks();
                                    outbound_events.push(ToSerializer::Message(*id, codec::ToClientMsg::UpdateMyMeta{ max_power: player.max_power, can_beamout: player.can_beamout }));
                                    
                                    outbound_events.push(ToSerializer::Broadcast(codec::ToClientMsg::RemovePart{ id: part.body_id }));
                                    outbound_events.push(ToSerializer::Broadcast(codec::ToClientMsg::AddPart{ id: part.body_id, kind: part.kind, }));
                                    outbound_events.push(ToSerializer::Broadcast(codec::ToClientMsg::UpdatePartMeta{ id: part.body_id, owning_player: Some(*id), thrust_mode: part.thrust_mode.into() }));
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
                            let player_id = player;
                            if let Some((player, _part)) = players.get_mut(&player) {
                                player.touching_planet = Some(planet);
                                player.can_beamout = simulation.planets.get_celestial_object(planet).unwrap().can_beamout;
                                player.ticks_til_cargo_transform = TICKS_PER_SECOND;
                                player.parts_touching_planet.insert(part);
                                player.power = player.max_power;
                                outbound_events.push(ToSerializer::Message(player_id, codec::ToClientMsg::UpdateMyMeta{ max_power: player.max_power, can_beamout: player.can_beamout }));
                            }
                        },
                        PlayerUntouchPlanet{ player, planet, part } => {
                            let player_id = player;
                            if let Some((player, _part)) = players.get_mut(&player) {
                                if player.parts_touching_planet.remove(&part) {
                                    if player.parts_touching_planet.is_empty() { 
                                        player.touching_planet = None;
                                        player.can_beamout = false;
                                        outbound_events.push(ToSerializer::Message(player_id, codec::ToClientMsg::UpdateMyMeta{ max_power: player.max_power, can_beamout: player.can_beamout }));
                                    }
                                }
                            }
                        },
                    }
                }

                for (player, core) in players.values_mut() { 
                    let mut max_power_lost = 0;
                    let mut regen_lost = 0;
                    recursive_broken_check(core, &mut simulation, &mut free_parts, &mut outbound_events, &mut max_power_lost, &mut regen_lost);
                    player.max_power -= max_power_lost;
                    player.power_regen_per_5_ticks -= regen_lost;
                }

                outbound_events.push(ToSerializer::WorldUpdate(
                    {
                        let mut out = BTreeMap::new();
                        for (id, (player, core)) in &players {
                            let mut parts = Vec::new();
                            fn recursive_part_move(parts: &mut Vec<session::WorldUpdatePartMove>, part: &Part, simulation: &world::Simulation) {
                                let body = simulation.world.get_rigid(MyHandle::Part(part.body_id)).unwrap();
                                let position = body.position();
                                parts.push(session::WorldUpdatePartMove {
                                    id: part.body_id,
                                    x: position.translation.x, y: position.translation.y,
                                    rot_cos: position.rotation.re, rot_sin: position.rotation.im
                                });
                                for i in 0..part.attachments.len() {
                                    if let Some((part, _, _)) = &part.attachments[i] { recursive_part_move(parts, part, simulation); };
                                };
                            }
                            recursive_part_move(&mut parts, core, &simulation);
                            out.insert(*id, ((parts[0].x, parts[0].y), parts, ToClientMsg::PostSimulationTick{ your_power: player.power }));
                        }
                        out
                    },
                    free_parts.iter().map(|(id, _)| {
                        let body = simulation.world.get_rigid(MyHandle::Part(*id)).unwrap();
                        let position = body.position();
                        session::WorldUpdatePartMove {
                            id: *id,
                            x: position.translation.x, y: position.translation.y,
                            rot_cos: position.rotation.re, rot_sin: position.rotation.im
                        }
                    }).collect::<Vec<_>>()
                ));
            },


            Event::InboundEvent(PlayerQuit { id }) => {
                fn nuke_part(part: &world::parts::Part, simulation: &mut world::Simulation, out: &mut Vec<ToSerializer>) {
                    simulation.world.remove_part(world::MyHandle::Part(part.body_id));
                    out.push(ToSerializer::Broadcast(codec::ToClientMsg::RemovePart{id: part.body_id}));
                    for part in part.attachments.iter() {
                        if let Some((part, _, _)) = part { nuke_part(part, simulation, out); }
                    }
                }
                outbound_events.push(ToSerializer::Broadcast(codec::ToClientMsg::RemovePlayer{ id }));
                if let Some((player, part)) = players.remove(&id) {
                    nuke_part(&part, &mut simulation, &mut outbound_events);
                    if let Some((part_id, constraint_id, _, _)) = player.grabbed_part {
                        if let Some(part) = free_parts.get_mut(&part_id) {
                            part.become_decaying();
                            simulation.release_constraint(constraint_id);
                        }
                    }
                    outbound_events.push(ToSerializer::Broadcast(ToClientMsg::ChatMessage{ username: String::from("Server"), msg: player.name.clone() + " left the game", color: String::from("#e270ff") }));
                } else { panic!("RE Player Quit Error"); }
            },
            
            Event::InboundEvent(NewPlayer{ id, name, parts, beamout_token }) => { 
                //Graduate session to being existant
                /*let mut core = world::parts::Part::new(world::parts::PartKind::Core, &mut simulation.world, &mut simulation.colliders, &simulation.part_static);
                let core_body = simulation.world.get_rigid_mut(MyHandle::Part(core.body_id)).unwrap();
                simulation.colliders.get_mut(core.collider).unwrap().set_user_data(Some(Box::new(PartOfPlayer(id))));*/
                //core_body.apply_force(0, &nphysics2d::algebra::Force2::torque(std::f32::consts::PI), nphysics2d::algebra::ForceType::VelocityChange, true);
                let earth_position = *simulation.world.get_rigid(simulation.planets.earth.body).unwrap().position().translation;
                let earth_radius = simulation.planets.earth.radius;
                let core = beamout::RecursivePartDescription::inflate_root(&parts, &mut simulation, earth_position.x, earth_position.y, Some(earth_radius), &mut rand );
                let mut max_power = 0u32; let mut power_regen = 0u32;
                fn recursive_part_beamin(part: &Part, player_id: u16, simulation: &mut world::Simulation, max_power: &mut u32, power_regen: &mut u32) {
                    let collider = simulation.colliders.get_mut(part.collider).unwrap();
                    collider.set_user_data(Some(Box::new(PartOfPlayer(player_id))));
                    *max_power += part.kind.power_storage();
                    *power_regen += part.kind.power_regen_per_5_ticks();
                    for i in 0..part.attachments.len() {
                        if let Some((attachment, _, _)) = &part.attachments[i] { recursive_part_beamin(attachment, player_id, simulation, max_power, power_regen); }
                    };
                }
                recursive_part_beamin(&core, id, &mut simulation, &mut max_power, &mut power_regen);

                outbound_events.push(ToSerializer::Message(id, ToClientMsg::HandshakeAccepted{ id, core_id: core.body_id, can_beamout: beamout_token.is_some() }));
                outbound_events.push(ToSerializer::Broadcast(codec::ToClientMsg::AddPlayer { id, name: name.clone(), core_id: core.body_id }));

                //Send over celestial object locations
                for planet in simulation.planets.celestial_objects().iter() {
                    let position = simulation.world.get_rigid(planet.body).unwrap().position().translation;
                    outbound_events.push(ToSerializer::Message(id, ToClientMsg::AddCelestialObject {
                        name: planet.name.clone(), display_name: planet.name.clone(),
                        id: planet.id, radius: planet.radius, position: (position.x, position.y)
                    }));
                }
                //Send over all parts
                fn send_part(part: &Part, owning_player: &Option<u16>, simulation: &crate::world::Simulation, player_id: u16, out: &mut Vec<ToSerializer>) {
                    let body = simulation.world.get_rigid(MyHandle::Part(part.body_id)).unwrap();
                    let position = body.position();
                    out.push(ToSerializer::Message(player_id, ToClientMsg::AddPart{ id: part.body_id, kind: part.kind }));
                    out.push(ToSerializer::Message(player_id, ToClientMsg::MovePart{
                        id: part.body_id,
                        x: position.translation.x, y: position.translation.y,
                        rotation_n: position.rotation.re, rotation_i: position.rotation.im,
                    }));
                    out.push(ToSerializer::Message(player_id, ToClientMsg::UpdatePartMeta{
                        id: part.body_id, owning_player: *owning_player, thrust_mode: part.thrust_mode.into()
                    }));
                    for part in part.attachments.iter() {
                        if let Some((part, _, _)) = part { send_part(part, owning_player, simulation, player_id, out); }
                    }
                }
                for (_id, part) in &free_parts { send_part(part, &None, &mut simulation, id, &mut outbound_events); };
                send_part(&core, &Some(id), &simulation, id, &mut outbound_events);
                for (other_id, (other_player, other_core)) in &players {
                    outbound_events.push(ToSerializer::Message(id, codec::ToClientMsg::AddPlayer{ id: *other_id, name: other_player.name.clone(), core_id: other_core.body_id }));
                    send_part(other_core, &Some(*other_id), &mut simulation, id, &mut outbound_events);
                    send_part(&core, &Some(id), &mut simulation, *other_id, &mut outbound_events);
                }
                
                //Graduate to spawned player
                let mut meta = PlayerMeta::new(name.clone());
                meta.max_power = max_power;
                meta.power_regen_per_5_ticks = power_regen;
                meta.power = meta.max_power;
                outbound_events.push(ToSerializer::Message(id, codec::ToClientMsg::UpdateMyMeta{ max_power: meta.max_power, can_beamout: meta.can_beamout }));
                players.insert(id, (meta, core));
                outbound_events.push(ToSerializer::Broadcast(ToClientMsg::ChatMessage{ username: String::from("Server"), msg: name + " joined the game", color: String::from("#e270ff") }));
            },

            Event::InboundEvent(PlayerMessage{ id, msg }) => {
                match msg {
                    ToServerMsg::SetThrusters{ forward, backward, clockwise, counter_clockwise } => {
                        if let Some((player, _core)) = players.get_mut(&id) {
                            if player.power > 0 {
                                player.thrust_forwards = forward;
                                player.thrust_backwards = backward;
                                player.thrust_clockwise = clockwise;
                                player.thrust_counterclockwise = counter_clockwise;
                                outbound_events.push(ToSerializer::Broadcast(codec::ToClientMsg::UpdatePlayerMeta {
                                    id,
                                    thrust_forward: forward, thrust_backward: backward, thrust_clockwise: clockwise, thrust_counter_clockwise: counter_clockwise,
                                    grabed_part: player.grabbed_part.map(|(id, _, _, _)| id)
                                }));
                            };
                        }
                    },

                    ToServerMsg::CommitGrab{ grabbed_id, x, y } => {
                        if let Some((player_meta, core)) = players.get_mut(&id) {
                            if player_meta.grabbed_part.is_none() {
                                let core_location = simulation.world.get_rigid(MyHandle::Part(core.body_id)).unwrap().position().translation;
                                let point = nphysics2d::math::Point::new(x + core_location.x, y + core_location.y);
                                let mut grabbed = false;
                                if let Some(free_part) = free_parts.get_mut(&grabbed_id) {
                                    if let FreePart::Decaying(part, _) | FreePart::EarthCargo(part, _) = &free_part {
                                        player_meta.grabbed_part = Some((grabbed_id, simulation.equip_mouse_dragging(grabbed_id), x, y));
                                        grabbed = true;
                                        free_part.become_grabbed(&mut earth_cargos);
                                    }
                                } else {
                                    fn recurse_2(part: &mut Part, target_part: u16, free_parts: &mut BTreeMap<u16, FreePart>, simulation: &mut world::Simulation, out: &mut Vec<ToSerializer>) -> Result<(),(Part, u32, u32)> {
                                        for slot in part.attachments.iter_mut() {
                                            if let Some((part, connection, connection2)) = slot {
                                                if part.body_id == target_part {
                                                    let mut max_power_lost: u32 = 0;
                                                    let mut regen_lost: u32 = 0;
                                                    recursive_detatch(part, free_parts, simulation, out, &mut max_power_lost, &mut regen_lost);
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
                                                recurse_2(part, target_part, free_parts, simulation, out)?;
                                            }
                                        }
                                        Ok(())
                                    }
                                    if let Err((part, max_power_lost, regen_lost)) = recurse_2(core, grabbed_id, &mut free_parts, &mut simulation, &mut outbound_events) {
                                        player_meta.grabbed_part = Some((grabbed_id, simulation.equip_mouse_dragging(grabbed_id), x, y));
                                        player_meta.max_power -= part.kind.power_storage();
                                        player_meta.max_power -= max_power_lost;
                                        if player_meta.power > player_meta.max_power { player_meta.power = player_meta.max_power };
                                        player_meta.power_regen_per_5_ticks -= regen_lost;
                                        player_meta.power_regen_per_5_ticks -= part.kind.power_regen_per_5_ticks();
                                        outbound_events.push(ToSerializer::Message(id, codec::ToClientMsg::UpdateMyMeta{ max_power: player_meta.max_power, can_beamout: player_meta.can_beamout }));
                                        simulation.colliders.get_mut(part.collider).unwrap().set_user_data(None);
                                        grabbed = true;
                                        if player_meta.parts_touching_planet.remove(&part.body_id) {
                                            if player_meta.parts_touching_planet.is_empty() {
                                                player_meta.touching_planet = None;
                                            }
                                        }
        
                                        free_parts.insert(grabbed_id, FreePart::Grabbed(part));
                                    }
                                }
                                if grabbed {
                                    outbound_events.push(ToSerializer::Broadcast(codec::ToClientMsg::UpdatePlayerMeta {
                                        id,
                                        thrust_forward: player_meta.thrust_forwards, thrust_backward: player_meta.thrust_backwards, thrust_clockwise: player_meta.thrust_clockwise, thrust_counter_clockwise: player_meta.thrust_counterclockwise,
                                        grabed_part: Some(grabbed_id)
                                    }));
                                    outbound_events.push(ToSerializer::Broadcast(codec::ToClientMsg::UpdatePartMeta {
                                        id: grabbed_id, thrust_mode: 0, owning_player: None
                                    }));
                                };
                            }
                        }
                    },
                    ToServerMsg::MoveGrab{ x, y } => {
                        if let Some((player_meta, core)) = players.get_mut(&id) {
                            if let Some((part_id, constraint, _, _)) = player_meta.grabbed_part {
                                //simulation.move_mouse_constraint(constraint, x, y);
                                player_meta.grabbed_part = Some((part_id, constraint, x, y));
                            }
                        }
                    },
                    ToServerMsg::ReleaseGrab => {
                        if let Some((player_meta, core)) = players.get_mut(&id) {
                            if let Some((part_id, constraint, x, y)) = player_meta.grabbed_part {
                                simulation.release_constraint(constraint);
                                player_meta.grabbed_part = None;
                                let mut attachment_msg: Option<Vec<u8>> = None;
                                let core_location = simulation.world.get_rigid(MyHandle::Part(core.body_id)).unwrap().position().clone();
                                let grabbed_part_body = simulation.world.get_rigid_mut(MyHandle::Part(part_id)).unwrap();
                                grabbed_part_body.set_local_inertia(free_parts.get(&part_id).unwrap().kind.inertia());
                                grabbed_part_body.set_velocity(nphysics2d::algebra::Velocity2::new(Vector2::new(0.0,0.0), 0.0));
        
                                use world::parts::CompactThrustMode;
                                fn recurse3<'a>(part: &'a mut Part, target_x: f32, target_y: f32, bodies: &world::World, parent_actual_rotation: world::parts::AttachedPartFacing, x: i16, y: i16) -> Result<(), (&'a mut Part, usize, world::parts::AttachmentPointDetails, (f32, f32), CompactThrustMode, world::parts::AttachedPartFacing)> {
                                    let attachments = part.kind.attachment_locations();
                                    let pos = bodies.get_rigid(MyHandle::Part(part.body_id)).unwrap().position().clone();
                                    for i in 0..part.attachments.len() {
                                        if part.attachments[i].is_none() {
                                            if let Some(details) = &attachments[i] {
                                                let mut rotated = rotate_vector(details.x, details.y, pos.rotation.im, pos.rotation.re);
                                                rotated.0 += pos.translation.x;
                                                rotated.1 += pos.translation.y;
                                                if (rotated.0 - target_x).abs() <= 0.4 && (rotated.1 - target_y).abs() <= 0.4 {
                                                    let my_actual_facing = details.facing.get_actual_rotation(parent_actual_rotation);
                                                    let thrust_mode = CompactThrustMode::calculate(my_actual_facing, x, y);
                                                    return Err((part, i, *details, rotated, thrust_mode, my_actual_facing));
                                                }
                                            }
                                        }
                                    }
                                    for (i, subpart) in part.attachments.iter_mut().enumerate() {
                                        if let Some((part, _, _)) = subpart {
                                            let my_actual_rotation = attachments[i].unwrap().facing.get_actual_rotation(parent_actual_rotation);
                                            let deltas = my_actual_rotation.attachment_offset();
                                            recurse3(part, target_x, target_y, bodies, my_actual_rotation, x + deltas.0, y + deltas.1)?
                                        }
                                    }
                                    Ok(())
                                }
                                if let Err((part, slot_id, details, teleport_to, thrust_mode, my_actual_facing)) = recurse3(
                                    core, 
                                    x + core_location.translation.x, 
                                    y + core_location.translation.y, 
                                    &simulation.world,
                                    world::parts::AttachedPartFacing::Up,
                                    0, 0
                                ) {
                                    let grabbed_part_body = simulation.world.get_rigid_mut(MyHandle::Part(part_id)).unwrap();
                                    grabbed_part_body.set_position(Isometry2::new(Vector2::new(teleport_to.0, teleport_to.1), my_actual_facing.part_rotation() + core_location.rotation.angle()));
                                    let (connection1, connection2) = simulation.equip_part_constraint(part.body_id, part_id, part.kind.attachment_locations()[slot_id].unwrap());
        
                                    let mut grabbed_part = free_parts.remove(&part_id).unwrap().extract();
                                    player_meta.max_power += grabbed_part.kind.power_storage();
                                    player_meta.power_regen_per_5_ticks += grabbed_part.kind.power_regen_per_5_ticks();
                                    outbound_events.push(ToSerializer::Message(id, codec::ToClientMsg::UpdateMyMeta{ max_power: player_meta.max_power, can_beamout: player_meta.can_beamout }));
                                    grabbed_part.thrust_mode = thrust_mode;
                                    simulation.colliders.get_mut(grabbed_part.collider).unwrap().set_user_data(Some(Box::new(PartOfPlayer(id))));
                                    part.attachments[slot_id] = Some((grabbed_part, connection1, connection2));
                                    outbound_events.push(ToSerializer::Broadcast(codec::ToClientMsg::UpdatePartMeta { id: part_id, owning_player: Some(id), thrust_mode: thrust_mode.into() }));
                                } else {
                                    free_parts.get_mut(&part_id).unwrap().become_decaying();
                                }
        
                                outbound_events.push(ToSerializer::Broadcast(codec::ToClientMsg::UpdatePlayerMeta {
                                    id,
                                    thrust_forward: player_meta.thrust_forwards, thrust_backward: player_meta.thrust_backwards, thrust_clockwise: player_meta.thrust_clockwise, thrust_counter_clockwise: player_meta.thrust_counterclockwise,
                                    grabed_part: None
                                }));
                            }
                        }
                    },
                    ToServerMsg::BeamOut => {
                        if let Some((player, core)) = players.remove(&id) {
                            let beamout_layout = beamout::RecursivePartDescription::deflate(&core);
                            outbound_events.push(ToSerializer::Broadcast(codec::ToClientMsg::BeamOutAnimation { player_id: id }));
                            fn recursive_beamout_remove(part: &Part, simulation: &mut world::Simulation) {
                                for slot in 0..part.attachments.len() {
                                    if let Some((part, joint1, joint2)) = part.attachments[slot].as_ref() {
                                        simulation.release_constraint(*joint1);
                                        simulation.release_constraint(*joint2);
                                        recursive_beamout_remove(part, simulation);
                                    }
                                }
                                simulation.world.remove_part(MyHandle::Part(part.body_id));
                            }
                            recursive_beamout_remove(&core, &mut simulation);
                            beamout::spawn_beamout_request(player.beamout_token, beamout_layout, api.clone());
                            let my_to_serializer = to_serializer.clone();
                            async_std::task::spawn(async move {
                                futures_timer::Delay::new(std::time::Duration::from_millis(2500)).await;
                                my_to_serializer.send(vec![ ToSerializer::DeleteWriter(id) ]).await;
                            });
                        }
                    },
                    _ => { outbound_events.push(ToSerializer::DeleteWriter(id)); }
                }
            },
        }
        to_serializer.send(outbound_events).await;
    }
}


//TODO: parallelize
fn recursive_broken_check(part: &mut Part, simulation: &mut world::Simulation, free_parts: &mut BTreeMap<u16, FreePart>, out: &mut Vec<ToSerializerEvent>, max_power_lost: &mut u32, regen_lost: &mut u32) {
    for i in 0..part.attachments.len() {
        if let Some((attachment, joint1, joint2)) = &mut part.attachments[i] {
            if simulation.is_constraint_broken(*joint1) || simulation.is_constraint_broken(*joint2) {
                simulation.release_constraint(*joint1);
                simulation.release_constraint(*joint2);
                *max_power_lost += attachment.kind.power_storage();
                *regen_lost += attachment.kind.power_regen_per_5_ticks();
                recursive_detatch(attachment, free_parts, simulation, out, max_power_lost, regen_lost);
                if let Some((part, _, _)) = std::mem::replace(&mut part.attachments[i], None) {
                    out.push(ToSerializerEvent::Broadcast(codec::ToClientMsg::UpdatePartMeta{ id: part.body_id, owning_player: None, thrust_mode: 0 }));
                    free_parts.insert(part.body_id, FreePart::Decaying(part, DEFAULT_PART_DECAY_TICKS));
                }
            } else {
                recursive_broken_check(attachment, simulation, free_parts, out, max_power_lost, regen_lost);
            }
        }
    }
}


fn recursive_detatch(part: &mut Part, free_parts: &mut BTreeMap<u16, FreePart>, simulation: &mut world::Simulation, out: &mut Vec<ToSerializerEvent>, max_power_lost: &mut u32, regen_lost: &mut u32) {
    for slot in part.attachments.iter_mut() {
        if let Some((part, connection, connection2)) = slot {
            simulation.release_constraint(*connection);
            simulation.release_constraint(*connection2);
            *max_power_lost += part.kind.power_storage();
            *regen_lost += part.kind.power_regen_per_5_ticks();
            recursive_detatch(part, free_parts, simulation, out, max_power_lost, regen_lost);
            if let Some((part, _, _)) = std::mem::replace(slot, None) {
                out.push(ToSerializerEvent::Broadcast(codec::ToClientMsg::UpdatePartMeta{ id: part.body_id, owning_player: None, thrust_mode: 0 }));
                free_parts.insert(part.body_id, FreePart::Decaying(part, DEFAULT_PART_DECAY_TICKS));
            }
        }
    }
}

enum FreePart {
    Decaying(world::parts::Part, u16),
    EarthCargo(world::parts::Part, u16),
    Grabbed(world::parts::Part),
    PlaceholderLol,
}
impl std::ops::Deref for FreePart {
    type Target = world::parts::Part;
    fn deref(&self) -> &world::parts::Part {
        match self {
            FreePart::Decaying(part, _) => part,
            FreePart::EarthCargo(part, _) => part,
            FreePart::Grabbed(part) => part,
            FreePart::PlaceholderLol => panic!("Attempted to get part from placeholder"),
        }
    }
}
impl std::ops::DerefMut for FreePart {
    fn deref_mut(&mut self) -> &mut world::parts::Part {
        match self {
            FreePart::Decaying(part, _) => part,
            FreePart::EarthCargo(part, _) => part,
            FreePart::Grabbed(part) => part,
            FreePart::PlaceholderLol => panic!("Attempted to get part from placeholder"),
        }
    }
}
impl FreePart {
    pub fn become_grabbed(&mut self, earth_cargo_count: &mut u8) {
        match &self {
            FreePart::EarthCargo(_, _) => { *earth_cargo_count -= 1; },
            _ => ()
        }
        let potato = match std::mem::replace(self, FreePart::PlaceholderLol) {
            FreePart::PlaceholderLol => panic!("Become transform on Placerholderlol"),
            FreePart::Decaying(part, _) => FreePart::Grabbed(part),
            FreePart::EarthCargo(part, _) => FreePart::Grabbed(part),
            FreePart::Grabbed(_) => panic!("Into FreePart::Grabbed called on Grabbed")
        };
        *self = potato;
    }
    pub fn become_decaying(&mut self) {
        let potato = match std::mem::replace(self, FreePart::PlaceholderLol) {
            FreePart::PlaceholderLol => panic!("Become transform on Placerholderlol"),
            FreePart::Decaying(part, _) | FreePart::Grabbed(part) => FreePart::Decaying(part, DEFAULT_PART_DECAY_TICKS),
            FreePart::EarthCargo(_, _) => panic!("EarthCargo into Decaying directly"),
        };
        *self = potato;
    }
    pub fn extract(self) -> Part {
        match self {
            FreePart::PlaceholderLol => panic!("Tried to extract placeholderlol"),
            FreePart::Decaying(part, _) => part,
            FreePart::EarthCargo(part, _) => part,
            FreePart::Grabbed(part) => part,
        }
    }
}

pub fn rotate_vector_with_angle(x: f32, y: f32, theta: f32) -> (f32, f32) { rotate_vector(x, y, theta.sin(), theta.cos()) }
pub fn rotate_vector(x: f32, y: f32, theta_sin: f32, theta_cos: f32) -> (f32, f32) {
    ((x * theta_cos) - (y * theta_sin), (x * theta_sin) + (y * theta_cos))
}

pub struct PlayerMeta {
    pub name: String,
    pub beamout_token: Option<String>, 

    pub thrust_forwards: bool,
    pub thrust_backwards: bool,
    pub thrust_clockwise: bool,
    pub thrust_counterclockwise: bool,

    pub power: u32,
    pub max_power: u32,
    pub power_regen_per_5_ticks: u32,

    pub grabbed_part: Option<(u16, nphysics2d::joint::DefaultJointConstraintHandle, f32, f32)>,

    pub touching_planet: Option<u16>,
    ticks_til_cargo_transform: u8,
    parts_touching_planet: BTreeSet<u16>,
    can_beamout: bool,
}
impl PlayerMeta {
    fn new(name: String, beamout_token: Option<String>) -> PlayerMeta { PlayerMeta {
        name,
        beamout_token,
        thrust_backwards: false, thrust_clockwise: false, thrust_counterclockwise: false, thrust_forwards: false,
        power: 100 * crate::TICKS_PER_SECOND as u32, max_power: 100 * crate::TICKS_PER_SECOND as u32,
        power_regen_per_5_ticks: 0,
        grabbed_part: None,
        touching_planet: None,
        parts_touching_planet: BTreeSet::new(),
        ticks_til_cargo_transform: TICKS_PER_SECOND,
        can_beamout: false,
    } }
}
pub struct PartOfPlayer (u16);

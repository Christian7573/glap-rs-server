#[macro_use] extern crate serde_derive;
#[macro_use] extern crate lazy_static;
use async_std::prelude::*;
use std::net::SocketAddr;
use futures::{FutureExt, StreamExt};
use std::pin::Pin;
use std::collections::{BTreeMap, BTreeSet};
use std::task::Poll;
use rand::Rng;
use world::nphysics_types::*;
use world::parts::{Part, AttachedPartFacing};
use nalgebra::Vector2; use nalgebra::geometry::{Isometry2, UnitComplex};
use ncollide2d::pipeline::object::CollisionGroups;
use std::sync::Arc;
use std::any::Any;
use async_std::sync::{Sender, Receiver, channel};
use nphysics2d::object::Body;

pub mod world;
pub mod codec;
pub mod session;
pub mod beamout;
use codec::*;
use session::ToSerializerEvent;

use world::parts::{RecursivePartDescription, PartKind};

pub const TICKS_PER_SECOND: u8 = 20;
pub const DEFAULT_PART_DECAY_TICKS: u16 = TICKS_PER_SECOND as u16 * 20;

#[derive(Clone)]
pub struct ApiDat { prefix: String, beamout: String, beamin: String, password: String }

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

    let mut players: BTreeMap<u16, PlayerMeta> = BTreeMap::new();
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

    let mut ticks_til_power_regen = 5u8;

    while let Some(event) = event_source.next().await {
        use session::ToGameEvent::*;
        use session::ToSerializerEvent as ToSerializer;
        let mut outbound_events = Vec::new();
        match event {
            Event::Simulate => {
                let mut to_delete: Vec<u16> = Vec::new();
                for (part_handle, meta) in free_parts.iter_mut() {
                    match meta {
                        FreePart::Decaying(_part, ticks) => {
                            *ticks -= 1;
                            if *ticks < 1 { to_delete.push(*part_handle); }
                        },
                        FreePart::EarthCargo(part, ticks) => {
                            *ticks -= 1;
                            if *ticks < 1 {
                                let earth_position = simulation.world.get_rigid(simulation.planets.earth.body).unwrap().position().translation;
                                let part = simulation.world.get_part_mut(*part).expect("Invalid Earth Cargo");
                                let body = part.body_mut();
                                let spawn_degrees: f32 = rand.gen::<f32>() * std::f32::consts::PI * 2.0;
                                let spawn_radius = simulation.planets.earth.radius * 1.25 + 1.0;
                                body.set_position(Isometry2::new(Vector2::new(spawn_degrees.sin() * spawn_radius + earth_position.x, spawn_degrees.cos() * spawn_radius + earth_position.y), 0.0)); // spawn_degrees));
                                //use nphysics2d::object::Body;
                                //body.apply_force(0, &nphysics2d::math::Force::zero(), nphysics2d::math::ForceType::Force, true);
                                body.activate();
                                *ticks = TICKS_PER_SECOND as u16 * 60;
                            }
                        },
                        FreePart::Grabbed(_part) => (),
                        FreePart::PlaceholderLol => panic!(),
                    }
                }
                for to_delete in to_delete {
                    let meta = free_parts.remove(&to_delete).unwrap();
                    outbound_events.extend(simulation.delete_parts_recursive(*meta).into_iter().map(|msg| ToSerializer::Broadcast(msg)));
                }
                if earth_cargos < MAX_EARTH_CARGOS {
                    ticks_til_earth_cargo_spawn -= 1;
                    if ticks_til_earth_cargo_spawn == 0 {
                        ticks_til_earth_cargo_spawn = TICKS_PER_EARTH_CARGO_SPAWN;
                        earth_cargos += 1; 
                        let earth_position = simulation.world.get_rigid(simulation.planets.earth.body).unwrap().position().translation;
                        let spawn_degrees: f32 = rand.gen::<f32>() * std::f32::consts::PI * 2.0;
                        let spawn_radius = simulation.planets.earth.radius * 1.25 + 1.0;
                        let spawn_pos = Isometry2::new(Vector2::new(spawn_degrees.sin() * spawn_radius + earth_position.x, spawn_degrees.cos() * spawn_radius + earth_position.y), 0.0);
                        let part_handle = RecursivePartDescription::from(PartKind::Cargo).inflate(&mut (&mut simulation.world).into(), &mut simulation.colliders, &mut simulation.joints, spawn_pos);
                        let part = simulation.world.get_part(part_handle).unwrap();
                        let part_id = part.id();
                        free_parts.insert(part_id, FreePart::EarthCargo(part_handle, TICKS_PER_SECOND as u16 * 60));
                        outbound_events.push(ToSerializer::Broadcast(part.add_msg()));
                        outbound_events.push(ToSerializer::Broadcast(part.move_msg()));
                        outbound_events.push(ToSerializer::Broadcast(part.update_meta_msg()));
                    }
                }
                ticks_til_power_regen -= 1;
                let is_power_regen_tick;
                if ticks_til_power_regen == 0 { ticks_til_power_regen = 5; is_power_regen_tick = true; }
                else { is_power_regen_tick = false; }
                for (id, player) in &mut players {
                    if is_power_regen_tick {
                        player.power += player.power_regen_per_5_ticks;
                        if player.power > player.max_power { player.power = player.max_power; };
                    };
                    if player.power > 0 {
                        simulation.world.recurse_part_mut(player.core, Default::default(), &mut |mut handle: world::PartVisitHandleMut| {
                            (*handle).thrust_no_recurse(&mut player.power, player.thrust_forwards, player.thrust_backwards, player.thrust_clockwise, player.thrust_counterclockwise);
                        });
                        if player.power < 1 {
                            player.thrust_backwards = false; player.thrust_forwards = false; player.thrust_clockwise = false; player.thrust_counterclockwise = false;
                            outbound_events.push(ToSerializer::Broadcast(codec::ToClientMsg::UpdatePlayerMeta {
                                id:  *id,
                                thrust_forward: player.thrust_forwards, thrust_backward: player.thrust_backwards, thrust_clockwise: player.thrust_clockwise, thrust_counter_clockwise: player.thrust_counterclockwise,
                                grabed_part: player.grabbed_part.map(|(id,_,_,_)| id)
                            }));
                        }
                    }
                    if let Some((part_id, constraint, x, y)) = player.grabbed_part {
                        let core = simulation.world.get_part_mut(player.core).expect("Player iter invalid core part");
                        let position = core.body().position().translation;
                        simulation.move_mouse_constraint(constraint, x + position.x, y + position.y);
                    }
                    if let Some(planet_id) = player.touching_planet {
                        player.ticks_til_cargo_transform -= 1;
                        if player.ticks_til_cargo_transform < 1 {
                            player.ticks_til_cargo_transform = TICKS_PER_CARGO_UPGRADE;
                            if let Some(upgrade_into) = simulation.planets.get_celestial_object(planet_id).unwrap().cargo_upgrade {
                                let core = simulation.world.get_part(player.core).expect("Player iter invalid core part");
                                if let Some((parent_part_handle, slot)) = core.find_cargo_recursive(&simulation.world) {
                                    let parent_part_handle = parent_part_handle.unwrap_or(player.core);
                                    let parent_part = simulation.world.get_part_mut(parent_part_handle).unwrap();
                                    let old_part_handle = parent_part.detach_part_player_agnostic(slot, &mut simulation.joints).unwrap();
                                    let old_part = simulation.world.remove_part_unprotected(old_part_handle);
                                    outbound_events.push(ToSerializer::Broadcast(old_part.remove_msg()));
                                    if player.parts_touching_planet.remove(&old_part_handle) {
                                        if player.parts_touching_planet.is_empty() { 
                                            player.touching_planet = None;
                                            player.can_beamout = false;
                                        }
                                    }
                                    let new_part_handle = old_part.mutate(upgrade_into, &mut Some(player), &mut simulation.world, &mut simulation.colliders, &mut simulation.joints);
                                    let parent_part = simulation.world.get_part_mut(parent_part_handle).unwrap();
                                    parent_part.attach_part_player_agnostic(slot, new_part_handle, parent_part_handle, &mut simulation.joints);
                                    let new_part = simulation.world.get_part(new_part_handle).unwrap();
                                    outbound_events.push(ToSerializer::Message(*id, player.update_my_meta()));
                                    outbound_events.push(ToSerializer::Broadcast(new_part.add_msg()));
                                    outbound_events.push(ToSerializer::Broadcast(new_part.move_msg()));
                                    outbound_events.push(ToSerializer::Broadcast(new_part.update_meta_msg()));
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
                            if let Some(player) = players.get_mut(&player) {
                                if planet == simulation.planets.sun.id {
                                    //Kill player
                                    outbound_events.push(ToSerializer::Broadcast(codec::ToClientMsg::IncinerationAnimation{ player_id }));
                                    let my_to_serializer = to_serializer.clone();
                                    let player = players.remove(&player_id).unwrap();
                                    let deflated_ship = simulation.world.get_part(player.core).unwrap().deflate(&simulation.world);
                                    //Don't need to send deletion messages since the client will
                                    //take care of IncinerationAnimation
                                    simulation.delete_parts_recursive(player.core);
                                    async_std::task::spawn(async move {
                                        futures_timer::Delay::new(std::time::Duration::from_millis(2500)).await;
                                        my_to_serializer.send(vec![ ToSerializer::DeleteWriter(player_id) ]).await;
                                    });
                                } else {
                                    player.touching_planet = Some(planet);
                                    player.can_beamout = simulation.planets.get_celestial_object(planet).unwrap().can_beamout;
                                    player.ticks_til_cargo_transform = TICKS_PER_SECOND;
                                    player.parts_touching_planet.insert(part);
                                    player.power = player.max_power;
                                    outbound_events.push(ToSerializer::Message(player_id, codec::ToClientMsg::UpdateMyMeta{ max_power: player.max_power, can_beamout: player.can_beamout }));
                                }
                            } else if planet == simulation.planets.sun.id {
                                outbound_events.push(ToSerializer::Broadcast(simulation.world.get_part(part).unwrap().remove_msg()));
                            }
                        },
                        PlayerUntouchPlanet{ player, planet, part } => {
                            let player_id = player;
                            if let Some(player) = players.get_mut(&player) {
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

                for player in players.values_mut() { 
                    recursive_broken_detach(player.core, &mut simulation, &mut free_parts, &mut Some(player), &mut outbound_events);
                }

                outbound_events.push(ToSerializer::WorldUpdate(
                    {
                        let mut out = BTreeMap::new();
                        for (id, player) in &players {
                            let mut parts = Vec::new();
                            let part = simulation.world.get_part(player.core).unwrap();
                            let vel = part.body().velocity();
                            part.physics_update_msg(&simulation.world, &mut parts);
                            out.insert(*id, ((parts[0].x, parts[0].y), (vel.linear.x, vel.linear.y), parts, ToClientMsg::PostSimulationTick{ your_power: player.power }));
                        }
                        out
                    },
                    free_parts.iter().map(|(id, meta)| {
                        let body = simulation.world.get_rigid(**meta).unwrap();
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
                outbound_events.push(ToSerializer::Broadcast(codec::ToClientMsg::RemovePlayer{ id }));
                if let Some(player) = players.remove(&id) {
                    outbound_events.extend(simulation.delete_parts_recursive(player.core).into_iter().map(|msg| ToSerializer::Broadcast(msg)));
                    if let Some((part_id, constraint_id, _, _)) = player.grabbed_part {
                        if let Some(part) = free_parts.get_mut(&part_id) {
                            part.become_decaying();
                            simulation.release_constraint(constraint_id);
                        }
                    }
                    outbound_events.push(ToSerializer::Broadcast(ToClientMsg::ChatMessage{ username: String::from("Server"), msg: player.name.clone() + " left the game", color: String::from("#e270ff") }));
                } 
            },
            
            Event::InboundEvent(NewPlayer{ id, name, parts, beamout_token }) => { 
                let earth_position = simulation.world.get_rigid(simulation.planets.earth.body).unwrap().position().translation.vector;
                let earth_radius = simulation.planets.earth.radius;
                use rand::Rng;

                let spawn_degrees: f32 = rand.gen::<f32>() * std::f32::consts::PI * 2.0;
                let core_handle = simulation.inflate(&parts, Isometry2::new(Vector2::new(0.0,0.0), spawn_degrees - std::f32::consts::FRAC_PI_2));
                let spawn_radius: f32 = earth_radius * 1.25 + 1.0;
                let spawn_center = (Vector2::new(spawn_degrees.cos(), spawn_degrees.sin()) * spawn_radius) + earth_position;
                let mut max_extent: i32 = 1;
                simulation.world.recurse_part(core_handle, Default::default(), &mut |handle: world::PartVisitHandle| max_extent = max_extent.max(handle.details().part_rel_x.abs()).max(handle.details().part_rel_y.abs()));
                simulation.world.recurse_part_mut(core_handle, Default::default(), &mut |mut handle: world::PartVisitHandleMut| {
                    let part = &mut handle;
                    let new_pos = Isometry2::new(
                        part.body().position().translation.vector.clone() + spawn_center,
                        part.body().position().rotation.angle()
                    );
                    part.body_mut().set_position(new_pos);
                });

                let core = simulation.world.get_part_mut(core_handle).unwrap();

                outbound_events.push(ToSerializer::Message(id, ToClientMsg::HandshakeAccepted{ id, core_id: core.id(), can_beamout: beamout_token.is_some() }));
                outbound_events.push(ToSerializer::Broadcast(codec::ToClientMsg::AddPlayer { id, name: name.clone(), core_id: core.id() }));
                
                let mut player = PlayerMeta::new(id, core_handle, name.clone(), beamout_token);
                simulation.world.recurse_part_mut(core_handle, Default::default(), &mut |mut handle| {
                    let part = &mut handle;
                    part.join_to(&mut player);
                    outbound_events.push(ToSerializer::Broadcast(part.add_msg()));
                    outbound_events.push(ToSerializer::Broadcast(part.move_msg()));
                    outbound_events.push(ToSerializer::Broadcast(part.update_meta_msg()));
                });
                player.power = player.max_power;

                //Send over celestial object locations
                for planet in simulation.planets.celestial_objects().iter() {
                    let position = simulation.world.get_rigid(planet.body).unwrap().position().translation;
                    outbound_events.push(ToSerializer::Message(id, ToClientMsg::AddCelestialObject {
                        name: planet.name.clone(), display_name: planet.name.clone(),
                        id: planet.id, radius: planet.radius, position: (position.x, position.y)
                    }));
                }
                for (_id, part) in &free_parts { simulation.world.recurse_part(**part, Default::default(), &mut |handle: world::PartVisitHandle| {
                    let part = &handle;
                    outbound_events.push(ToSerializer::Message(id, part.add_msg()));
                    outbound_events.push(ToSerializer::Message(id, part.move_msg()));
                    outbound_events.push(ToSerializer::Message(id, part.update_meta_msg()));
                }); }
                for (other_id, other_player) in &players {
                    let other_core = simulation.world.get_part(other_player.core).unwrap();
                    outbound_events.push(ToSerializer::Message(id, codec::ToClientMsg::AddPlayer{ id: *other_id, name: other_player.name.clone(), core_id: other_core.id() }));
                    simulation.world.recurse_part(other_player.core, Default::default(), &mut |handle: world::PartVisitHandle| {
                        let part = &handle;
                        outbound_events.push(ToSerializer::Message(id, part.add_msg()));
                        outbound_events.push(ToSerializer::Message(id, part.move_msg()));
                        outbound_events.push(ToSerializer::Message(id, part.update_meta_msg()));
                    });
                }
                
                outbound_events.push(ToSerializer::Message(id, codec::ToClientMsg::UpdateMyMeta{ max_power: player.max_power, can_beamout: player.can_beamout }));
                players.insert(id, player);
                outbound_events.push(ToSerializer::Broadcast(ToClientMsg::ChatMessage{ username: String::from("Server"), msg: name + " joined the game", color: String::from("#e270ff") }));
            },

            Event::InboundEvent(PlayerMessage{ id, msg }) => {
                match msg {
                    ToServerMsg::SetThrusters{ forward, backward, clockwise, counter_clockwise } => {
                        if let Some(player) = players.get_mut(&id) {
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
                        if let Some(player_meta) = players.get_mut(&id) {
                            let core = simulation.world.get_part(player_meta.core).unwrap();
                            if player_meta.grabbed_part.is_none() {
                                let core_location = core.body().position().translation;
                                let point = nphysics2d::math::Point::new(x + core_location.x, y + core_location.y);
                                if let Some(free_part) = free_parts.get_mut(&grabbed_id) {
                                    if let FreePart::Decaying(part, _) | FreePart::EarthCargo(part, _) = &free_part {
                                        player_meta.grabbed_part = Some((grabbed_id, simulation.equip_mouse_dragging(*part), x, y));
                                        outbound_events.push(ToSerializer::Broadcast(simulation.world.get_part(*part).unwrap().update_meta_msg()));
                                        outbound_events.push(ToSerializer::Broadcast(player_meta.update_meta_msg()));
                                        free_part.become_grabbed(&mut earth_cargos);
                                    }
                                } else {
                                    let world = &mut simulation.world;
                                    let joints = &mut simulation.joints;
                                    if let Some(part_handle) = simulation.world.recurse_part_mut_with_return(player_meta.core, Default::default(), &mut |mut handle| {
                                        for (i, attachment) in (*handle).attachments().iter().enumerate() {
                                            if let Some(attachment) = attachment {
                                                if handle.get_part(**attachment).unwrap().id() == grabbed_id {
                                                    return Some((*handle).detach_part_player_agnostic(i, joints).unwrap())
                                                };
                                            }
                                        };
                                        None
                                    }) {
                                        simulation.world.get_part_mut(part_handle).unwrap().remove_from(player_meta);
                                        //what was I thinking here simulation.world.recurse_part_mut(part_handle, 0, 0, AttachedPartFacing::Up, AttachedPartFacing::Up, &mut |_handle, part: &mut world::parts::Part, _, _, _, _| part.join_to(player_meta));
                                        let mut parts_affected = BTreeSet::new();
                                        parts_affected.insert(part_handle);
                                        simulation.world.recursive_detach_all(part_handle, &mut Some(player_meta), &mut simulation.joints, &mut parts_affected);
                                        player_meta.grabbed_part = Some((grabbed_id, simulation.equip_mouse_dragging(part_handle), x, y));
                                        if player_meta.parts_touching_planet.remove(&part_handle) {
                                            if player_meta.parts_touching_planet.is_empty() { 
                                                player_meta.touching_planet = None;
                                                player_meta.can_beamout = false;
                                            }
                                        }
                                        //outbound_events.push(ToSerializer::Message(id, codec::ToClientMsg::UpdateMyMeta{ max_power: player_meta.max_power, can_beamout: player_meta.can_beamout }));

                                        for part_affected in parts_affected {
                                            let part = simulation.world.get_part(part_affected).unwrap();
                                            free_parts.insert(part.id(), FreePart::Decaying(part_affected, DEFAULT_PART_DECAY_TICKS));
                                            outbound_events.push(ToSerializer::Broadcast(part.update_meta_msg()));
                                        }
                                        free_parts.insert(grabbed_id, FreePart::Grabbed(part_handle));
                                        outbound_events.push(ToSerializer::Broadcast(player_meta.update_meta_msg()));
                                    };
                                }
                            }
                        }
                    },
                    ToServerMsg::MoveGrab{ x, y } => {
                        if let Some(player_meta) = players.get_mut(&id) {
                            if let Some((part_id, constraint, _, _)) = player_meta.grabbed_part {
                                //simulation.move_mouse_constraint(constraint, x, y);
                                player_meta.grabbed_part = Some((part_id, constraint, x, y));
                            }
                        }
                    },
                    ToServerMsg::ReleaseGrab => {
                        if let Some(player_meta) = players.get_mut(&id) {
                            if let Some((part_id, constraint, x, y)) = player_meta.grabbed_part {
                                simulation.release_constraint(constraint);
                                player_meta.grabbed_part = None;
                                let mut attachment_msg: Option<Vec<u8>> = None;
                                let core_location = simulation.world.get_rigid(player_meta.core).unwrap().position().clone();
                                let grabbed_part_handle = **free_parts.get(&part_id).unwrap();
                                let grabbed_part = simulation.world.get_part_mut(grabbed_part_handle).unwrap();
                                let inertia = grabbed_part.kind().inertia();
                                grabbed_part.body_mut().set_local_inertia(inertia);
                                grabbed_part.body_mut().set_velocity(nphysics2d::algebra::Velocity2::new(Vector2::new(0.0,0.0), 0.0));
        
                                use world::parts::CompactThrustMode;
                                let target_x = x + core_location.translation.x;
                                let target_y = y + core_location.translation.y; 
                                if let Some((parent_handle, attachment_slot, attachment_details, teleport_to, thrust_mode, true_facing)) = simulation.world.recurse_part_mut_with_return(
                                    player_meta.core, Default::default(),
                                    &mut |mut handle| {
                                        let parent = &mut handle;
                                        let attachments = parent.kind().attachment_locations();
                                        let pos = parent.body().position().clone();
                                        for (i, attachment) in parent.attachments().iter().enumerate() {
                                            if attachment.is_none() {
                                                if let Some(details) = &attachments[i] {
                                                    let mut rotated = rotate_vector(details.x, details.y, pos.rotation.im, pos.rotation.re);
                                                    rotated.0 += pos.translation.x;
                                                    rotated.1 += pos.translation.y;
                                                    if (rotated.0 - target_x).abs() <= 0.4 && (rotated.1 - target_y).abs() <= 0.4 {
                                                        let my_true_facing = details.facing.compute_true_facing(handle.details().true_facing);
                                                        let thrust_mode = CompactThrustMode::calculate(my_true_facing, handle.details().part_rel_x, handle.details().part_rel_y);
                                                        return Some((handle.handle(), i, *details, rotated, thrust_mode, my_true_facing));
                                                    }
                                                }
                                            }
                                        }
                                        None
                                    }
                                ) {
                                    let parent = simulation.world.get_part_mut(parent_handle).unwrap();
                                    //TODO: Check if we can use parent.body.position instead of core_location
                                    parent.attach_part_player_agnostic(attachment_slot, grabbed_part_handle, parent_handle, &mut simulation.joints);
                                    free_parts.remove(&part_id);
                                    let grabbed_part = simulation.world.get_part_mut(grabbed_part_handle).unwrap();
                                    grabbed_part.body_mut().set_position(Isometry2::new(Vector2::new(teleport_to.0, teleport_to.1), true_facing.part_rotation() + core_location.rotation.angle()));
                                    grabbed_part.join_to(player_meta);
                                    outbound_events.push(ToSerializer::Message(id, player_meta.update_my_meta()));
                                    grabbed_part.thrust_mode = thrust_mode;
                                    outbound_events.push(ToSerializer::Broadcast(grabbed_part.update_meta_msg()));
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
                        if let Some(player) = players.remove(&id) {
                            let core = simulation.world.get_part(player.core).unwrap();
                            let beamout_layout = core.deflate(&simulation.world);
                            outbound_events.push(ToSerializer::Broadcast(codec::ToClientMsg::BeamOutAnimation { player_id: id }));
                            outbound_events.push(ToSerializer::Broadcast(codec::ToClientMsg::ChatMessage { username: "Server".to_owned(), msg: format!("{} has left the game", player.name), color: "#e270ff".to_owned() }));
                            simulation.delete_parts_recursive(player.core);
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

            Event::InboundEvent(AdminCommand { id, command }) => {
                let chunks: Vec<String> = command.split_whitespace().map(|s| s.to_string()).collect();
                match chunks[0].as_str() {
                    "/teleport" => {
                        if chunks.len() == 3 {
                            if let (Ok(x), Ok(y)) = (chunks[1].parse::<f32>(), chunks[2].parse::<f32>()) {
                                let teleport_to = Vector2::new(x, y);
                                if let Some(player_meta) = players.get_mut(&id) {
                                    let core_pos = simulation.world.get_rigid(player_meta.core).unwrap().position().translation.vector;
                                    println!("Teleporting {} to: {} {}", player_meta.name, x, y);
                                    simulation.world.recurse_part_mut(player_meta.core, Default::default(), &mut |mut handle: world::PartVisitHandleMut| {
                                        let pos = Isometry2::new(
                                                (*handle).body().position().clone().translation.vector - core_pos + teleport_to,
                                                (*handle).body().position().rotation.angle()
                                        );
                                        (*handle).body_mut().set_position(pos);
                                    });
                                }
                            }
                        }
                    },

                    _ => {
                        to_serializer.send(vec! [ToSerializerEvent::Message(id, ToClientMsg::ChatMessage{ username: String::from("Server"), msg: String::from("You cannot use that command"), color: String::from("#FF0000") })]).await;
                    }
                    
                }
            }
        }
        to_serializer.send(outbound_events).await;
    }
}


fn recursive_broken_detach(root: MyHandle, simulation: &mut world::Simulation, free_parts: &mut BTreeMap<u16, FreePart>, player: &mut Option<&mut PlayerMeta>, out: &mut Vec<ToSerializerEvent> ) {
    let mut broken_parts = Vec::new();
    let world = &mut simulation.world;
    let joints = &mut simulation.joints;
    world.recurse_part(root, Default::default(), &mut |handle| {
        for (i, attachment) in (*handle).attachments().iter().enumerate() {
            if let Some(attachment) = attachment {
                if attachment.is_broken(joints) { broken_parts.push((handle.handle(), i)) };
            }
        }
    });
    let mut affected_parts = BTreeSet::new();
    for (parent, attachment_slot) in broken_parts {
        simulation.world.recursive_detach_one(parent, attachment_slot, player, &mut simulation.joints, &mut affected_parts);
    }
    for part_handle in affected_parts {
        if let Some(part) = simulation.world.get_part(part_handle) {
            out.push(ToSerializerEvent::Broadcast(part.update_meta_msg()));
            free_parts.insert(part.id(), FreePart::Decaying(part_handle, DEFAULT_PART_DECAY_TICKS));
        }
        if let Some(player) = player {
            if player.parts_touching_planet.remove(&part_handle) {
                if player.parts_touching_planet.is_empty() {
                    player.can_beamout = false;
                    player.touching_planet = None;
                }
            }
        }
    }
    if let Some(player) = player {
        out.push(ToSerializerEvent::Message(player.id, player.update_my_meta()));
    }
}

enum FreePart {
    Decaying(MyHandle, u16),
    EarthCargo(MyHandle, u16),
    Grabbed(MyHandle),
    PlaceholderLol,
}

impl FreePart {
    pub fn become_grabbed(&mut self, earth_cargo_count: &mut u8) {
        if let FreePart::EarthCargo(_, _) = self { *earth_cargo_count -= 1 };
        match self {
            FreePart::Decaying(part, _) | FreePart::EarthCargo(part, _) => { *self = FreePart::Grabbed(*part) }
            FreePart::PlaceholderLol | FreePart::Grabbed(_) => panic!("FreePart::Grabbed called on bad")
        }
    }
    pub fn become_decaying(&mut self) {
        match self {
            FreePart::Decaying(part, _) | FreePart::Grabbed(part) => { *self = FreePart::Decaying(*part, DEFAULT_PART_DECAY_TICKS) }
            FreePart::PlaceholderLol | FreePart::EarthCargo(_, _) => panic!("FreePart::Grabbed called on bad")
        }
    }
}
impl std::ops::Deref for FreePart {
    type Target = MyHandle;
    fn deref(&self) -> &MyHandle {
        match self {
            FreePart::Decaying(handle, _) | FreePart::EarthCargo(handle, _) | FreePart::Grabbed(handle) => handle,
            FreePart::PlaceholderLol => panic!("how did we get here")
        }
    }
}

pub fn rotate_vector_with_angle(x: f32, y: f32, theta: f32) -> (f32, f32) { rotate_vector(x, y, theta.sin(), theta.cos()) }
pub fn rotate_vector(x: f32, y: f32, theta_sin: f32, theta_cos: f32) -> (f32, f32) {
    ((x * theta_cos) - (y * theta_sin), (x * theta_sin) + (y * theta_cos))
}

pub struct PlayerMeta {
    pub id: u16,
    pub name: String,
    pub beamout_token: Option<String>, 

    pub core: MyHandle,
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
    parts_touching_planet: BTreeSet<MyHandle>,
    can_beamout: bool,
}
impl PlayerMeta {
    fn new(my_id: u16, core_handle: MyHandle, name: String, beamout_token: Option<String>) -> PlayerMeta { PlayerMeta {
        id: my_id,
        core: core_handle,
        name,
        beamout_token,
        thrust_backwards: false, thrust_clockwise: false, thrust_counterclockwise: false, thrust_forwards: false,
        //power: 100 * crate::TICKS_PER_SECOND as u32, max_power: 100 * crate::TICKS_PER_SECOND as u32,
        power: 0, max_power: 0,
        power_regen_per_5_ticks: 0,
        grabbed_part: None,
        touching_planet: None,
        parts_touching_planet: BTreeSet::new(),
        ticks_til_cargo_transform: TICKS_PER_SECOND,
        can_beamout: false,
    } }

    fn update_meta_msg(&self) -> ToClientMsg {
        ToClientMsg::UpdatePlayerMeta {
            id: self.id,
            grabed_part: self.grabbed_part.as_ref().map(|(id, _, _, _)| *id),
            thrust_forward: self.thrust_forwards,
            thrust_backward: self.thrust_backwards,
            thrust_clockwise: self.thrust_clockwise,
            thrust_counter_clockwise: self.thrust_counterclockwise,
        }
    }
    fn update_my_meta(&self) -> ToClientMsg {
        ToClientMsg::UpdateMyMeta {
            max_power: self.max_power,
            can_beamout: self.can_beamout,
        }
    }
}
pub struct PartOfPlayer (u16);


use std::collections::{BTreeMap, BTreeSet};
use crate::PartOfPlayer;
use generational_arena::{Arena, Index};
use crate::codec::{ ToClientMsg, PartKind };
use std::ops::{Deref, DerefMut};
use rapier2d::dynamics::{BodyStatus, CCDSolver, JointSet, RigidBody, RigidBodyBuilder, RigidBodyHandle, RigidBodySet, IntegrationParameters, Joint, JointHandle, MassProperties, BallJoint, JointParams, IslandManager};
use rapier2d::geometry::{BroadPhase, NarrowPhase, ColliderSet, IntersectionEvent, ContactEvent, ColliderHandle};
use rapier2d::pipeline::{PhysicsPipeline, ChannelEventCollector};
use rapier2d::crossbeam::channel::{Sender as CSender, Receiver as CReceiver, unbounded as c_channel};
use crate::storage7573::Storage7573;
use crate::PlayerMeta;

pub mod planets;
pub mod parts;
use parts::{Part, AttachedPartFacing, RecursivePartDescription};
use planets::AmPlanet;

pub mod typedef {
    pub type MyUnits = f32;
    pub type PartHandle = generational_arena::Index;
    //pub type MyIsometry = nphysics2d::math::Isometry<MyUnits>;
    pub type Vector = rapier2d::na::Matrix2x1<f32>;
    pub type Isometry = rapier2d::math::Isometry<f32>;
    pub type UnitComplex = rapier2d::na::UnitComplex<f32>;
    pub type Point = rapier2d::math::Point<f32>;
}
use typedef::*;

//const PART_JOINT_MAX_TORQUE: f32 = 200.0;
const PART_JOINT_MAX_TORQUE: f32 = 15.0;
const PART_JOINT_MAX_FORCE: f32 = PART_JOINT_MAX_TORQUE * 3.5;

pub struct Simulation {
    pub world: World,
    pub islands: IslandManager,
    pipeline: PhysicsPipeline,
    integration_parameters: IntegrationParameters,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    pub colliders: ColliderSet,
    pub joints: JointSet,
    ccd_solver: CCDSolver,
    steps_per_batch: u8,
    intersection_events: CReceiver<IntersectionEvent>,
    contact_events: CReceiver<ContactEvent>,
    event_collector: ChannelEventCollector,
}
pub enum SimulationEvent {
    PlayerTouchPlanet { player: u16, part: PartHandle, planet: u8, },
    PlayerUntouchPlanet { player: u16, part: PartHandle, planet: u8 },
    PartsDetached( BTreeSet<PartHandle> ),
}


pub fn make_local_point(reference: &Isometry, local: Point) -> Point {
    Point { coords: reference.transform_vector(&local.coords) } + &reference.translation.vector
}
pub fn apply_force_locally(body: &mut RigidBody, force: Vector, local: Point, wake_up: bool) {
    body.apply_force_at_point(body.position() * force, make_local_point(body.position(), local), wake_up);
}

impl Simulation {
    pub fn new(step_time: f32, steps_per_batch: u8) -> Simulation {
        /*mechanics.set_timestep(step_time);
        mechanics.integration_parameters.max_ccd_substeps = 5;*/
        let mut colliders = ColliderSet::new();
        let mut world = World::new(&mut colliders);

        let intersection_events = c_channel();
        let contact_events = c_channel();
        let event_collector = ChannelEventCollector::new(intersection_events.0, contact_events.0);
        let mut integration_parameters = IntegrationParameters::default();
        integration_parameters.dt = step_time / steps_per_batch as f32;

        let simulation = Simulation {
            pipeline: PhysicsPipeline::new(),
            integration_parameters,
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            ccd_solver: CCDSolver::new(),

            steps_per_batch,
            world,
            islands: IslandManager::new(),
            colliders,
            joints: JointSet::new(),
            event_collector,
            intersection_events: intersection_events.1,
            contact_events: contact_events.1,
        };
        simulation
    }


    pub fn simulate(&mut self, events: &mut Vec<SimulationEvent>) {
        self.world.advance_orbits();
        self.world.celestial_gravity();
        const GRAVITYNT: Vector = Vector::new(0.0, 0.0);
        for _ in 0..self.steps_per_batch {
            self.pipeline.step(
                &GRAVITYNT,
                &self.integration_parameters,
                &mut self.islands,
                &mut self.broad_phase,
                &mut self.narrow_phase,
                self.world.bodies_mut_unchecked(),
                &mut self.colliders,
                &mut self.joints,
                &mut self.ccd_solver,
                &(), &self.event_collector,
            );

            let mut to_detach = Vec::new();
            //let mut max_impulse_lin: f32 = 0.0;
            //let mut max_impulse_ang: f32 = 0.0;
            for (handle, part) in &mut self.world.parts {
                for i in 0..4 {
                    if let Some(attachment) = &part.attachments()[i] {
                        let joint = self.joints.get(attachment.connection).unwrap();
                        if let JointParams::FixedJoint(joint_params) = joint.params {
                            //max_impulse_ang = max_impulse_ang.max(joint_params.impulse.z.abs());
                            //max_impulse_lin = max_impulse_lin.max(joint_params.impulse.xy().magnitude().abs());
                            if joint_params.impulse.xy().magnitude().abs() >= PART_JOINT_MAX_FORCE || joint_params.impulse.z.abs() >= PART_JOINT_MAX_TORQUE {
                                to_detach.push((handle, i));
                                //events.push(SimulationEvent::PartDetached { detached_part: **attachment });
                                //Part::detach_part_player_agnostic_bad(part, i, &mut self.world.bodies, &mut self.islands, &mut self.joints);
                            }
                        } else {
                            panic!("Fixed Jointn't");
                        }
                    }
                }
            }
            let mut parts_affected = BTreeSet::new();
            for (parent, i) in to_detach {
                self.world.recursive_detach_one(parent, i, &mut None, &mut self.islands, &mut self.colliders, &mut self.joints, &mut parts_affected);
            }
            if !parts_affected.is_empty() { events.push(SimulationEvent::PartsDetached(parts_affected)) };
            //println!("{} {}", max_impulse_lin, max_impulse_ang);


            while let Ok(contact_event) = self.contact_events.try_recv() {
                match contact_event {
                    ContactEvent::Started(handle1, handle2) => {
                        let planet: u8;
                        let other: ColliderHandle;
                        if let Storage7573::Planet(am_planet) = self.colliders.get(handle1).unwrap().user_data.into() {
                            planet = am_planet; other = handle2;
                        } else if let Storage7573::Planet(am_planet) = self.colliders.get(handle2).unwrap().user_data.into() {
                            planet = am_planet; other = handle1;
                        } else { continue; }
                        let part_coll = self.colliders.get(other).unwrap();
                        if let Storage7573::PartOfPlayer(player_id) = self.colliders[other].user_data.into() {
                            events.push(SimulationEvent::PlayerTouchPlanet{ player: player_id, part: *self.world.parts_reverse_lookup.get(&part_coll.parent().unwrap().into_raw_parts()).unwrap(), planet });
                        }
                    },
                    ContactEvent::Stopped(handle1, handle2) => {
                        let planet: u8;
                        let other: ColliderHandle;
                        if let Storage7573::Planet(am_planet) = self.colliders.get(handle1).unwrap().user_data.into() {
                            planet = am_planet; other = handle2;
                        } else if let Storage7573::Planet(am_planet) = self.colliders.get(handle2).unwrap().user_data.into() {
                            planet = am_planet; other = handle1;
                        } else { continue; }
                        let part_coll = self.colliders.get(other).unwrap();
                        if let Storage7573::PartOfPlayer(player_id) = self.colliders[other].user_data.into() {
                            events.push(SimulationEvent::PlayerUntouchPlanet{ player: player_id, part: *self.world.parts_reverse_lookup.get(&part_coll.parent().unwrap().into_raw_parts()).unwrap(), planet });
                        }
                    }
                }
            }

            for contact_pair in self.narrow_phase.contact_pairs() {
                if !contact_pair.has_any_active_contact { continue };
                let collider1 = &self.colliders[contact_pair.collider1];
                let collider2 = &self.colliders[contact_pair.collider2];
                let storage1 = Storage7573::from(collider1.user_data);
                let storage2 = Storage7573::from(collider2.user_data);
                let non_planet;
                let planet;
                if matches!(storage1, Storage7573::Planet(_id)) && !matches!(storage2, Storage7573::Planet(_id)) {
                    non_planet = collider2;
                    planet = &self.world.planets.planets[&storage1.planet_id()];
                } else if matches!(storage2, Storage7573::Planet(_id)) && !matches!(storage1, Storage7573::Planet(_id)) {
                    non_planet = collider1;
                    planet = &self.world.planets.planets[&storage2.planet_id()];
                } else {
                    continue;
                }
                let planet_vel = planet.orbit.as_ref().map(|orbit| orbit.last_velocity()).unwrap_or((0.0, 0.0));
                let body = self.world.get_part_rigid_mut(*self.world.parts_reverse_lookup.get(&non_planet.parent().unwrap().into_raw_parts()).unwrap()).unwrap();
                body.set_linvel(Vector::new(planet_vel.0, planet_vel.1), true);
            }
        }
    }

    pub fn equip_mouse_dragging(&mut self, part: PartHandle) -> MouseDraggingStuff {
        let mut jank_rigid_body = RigidBodyBuilder::new_dynamic()
            .additional_mass(100.0)
            .build();
        
        let part_actual = self.world.get_part(part).unwrap();
        let body_handle = part_actual.body_handle();
        let body = self.world.get_part_rigid_mut(part).unwrap();
        let mut mass = *body.mass_properties();
        mass.set_mass(0.000001, false);
        body.set_mass_properties(mass, true);
        let space = body.position().translation;
        jank_rigid_body.set_translation(space.vector, true);
        let constraint = BallJoint::new(
            //Point::new(space.x, space.y),
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
        );
        let movable_handle = self.world.bodies.insert(jank_rigid_body);
        let joint_handle = self.joints.insert(self.world.bodies_mut_unchecked(), movable_handle, body_handle, constraint);
        MouseDraggingStuff { body: movable_handle, joint: joint_handle }
    }
    pub fn move_mouse_constraint(&mut self, drag_stuff: &mut MouseDraggingStuff, x: f32, y: f32) {
        if let Some(body) = self.world.bodies.get_mut(drag_stuff.body) {
            body.set_translation(Vector::new(x, y), true);
        /*if let Some(JointParams::BallJoint(mut constraint)) = self.joints.get_mut(constraint_id).map(|j: &mut Joint| j.params ) {
            constraint.local_anchor1 = Point::new(x, y);*/
        } else {
            panic!("Mousen't");
        }
    }
    pub fn release_constraint(&mut self, constraint_id: JointHandle) {
        self.joints.remove(constraint_id, &mut self.islands, self.world.bodies_mut_unchecked(), true);
    }
    pub fn release_mouse_constraint(&mut self, drag_stuff: MouseDraggingStuff) {
        self.joints.remove(drag_stuff.joint, &mut self.islands, self.world.bodies_mut_unchecked(), true);
        self.world.bodies.remove(drag_stuff.body, &mut self.islands, &mut self.colliders, &mut self.joints);
        //self.release_constraint(constraint_id);
        //TODO set mass back to normal
    }

    pub fn is_constraint_broken(&self, handle: JointHandle) -> bool {
        //self.joints.get(handle).map(|joint| joint.is_broken()).unwrap_or(true)
        false
    }

    pub fn inflate(&mut self, parts: &RecursivePartDescription, initial_location: Isometry) -> PartHandle {
        parts.inflate(&mut self.world, &mut self.colliders, &mut self.joints, initial_location)
    }
    pub fn delete_parts_recursive(&mut self, index: PartHandle) -> Vec<ToClientMsg> {
        let mut removal_msgs = Vec::new();
        self.world.delete_parts_recursive(index, &mut self.islands, &mut self.colliders, &mut self.joints, &mut removal_msgs);
        removal_msgs
    }
}

pub struct MouseDraggingStuff {
    body: RigidBodyHandle,
    joint: JointHandle,
}

type PartsReverseLookup = BTreeMap<(u32, u32), Index>;
pub struct World {
    parts: Arena<Part>,
    bodies: RigidBodySet,
    pub planets: planets::Planets,
    reference_point_body: RigidBodyHandle,
    pub(self) parts_reverse_lookup: PartsReverseLookup,
}

/*pub struct WorldAddHandle<'a>(&'a mut World); 
impl<'a> WorldAddHandle<'a> {
    pub fn add_now(&mut self, object: WorldlyObject) -> Index { self.0.storage.insert(object) }
    pub fn add_later(&mut self) -> Index { self.0.storage.insert(WorldlyObject::Uninitialized) }
    pub fn add_its_later(&mut self, index: Index, object: WorldlyObject) {
        match std::mem::replace(self.0.storage.get_mut(index).expect("add_its_later: the index doesn't exist"), object) {
            WorldlyObject::Uninitialized => {},
            _ => panic!("add_its_later: the index wasn't WorldlyObject::Uninitialized. Storage is now poisioned(?)")
        }
    }
    pub fn deconstruct(self) -> &'a mut World { self.0 }
}
impl<'a> From<&'a mut World> for WorldAddHandle<'a> {
    fn from(world: &'a mut World) -> WorldAddHandle<'a> { WorldAddHandle(world) }
}*/

impl World {
    pub fn get_part_rigid(&self, index: PartHandle) -> Option<&RigidBody> {
        self.parts.get(index).map(|part| self.bodies.get(part.body_handle())).flatten()
    }
    pub fn get_part(&self, index: PartHandle) -> Option<&Part> {
        self.parts.get(index)
    }
    pub fn get_part_rigid_mut(&mut self, index: PartHandle) -> Option<&mut RigidBody> {
        if let Some(part) = self.parts.get(index) {
            self.bodies.get_mut(part.body_handle())
        } else {
            None
        }
    }
    pub fn get_part_mut(&mut self, index: PartHandle) -> Option<&mut Part> {
        self.parts.get_mut(index)
    }
    pub fn delete_parts_recursive(&mut self, index: PartHandle, islands: &mut IslandManager, colliders: &mut ColliderSet, joints: &mut JointSet, removal_msgs: &mut Vec<ToClientMsg>) {
        self.parts.remove(index).map(|part| part.delete_recursive(self, islands, colliders, joints, removal_msgs));
    }
    pub fn bodies_unchecked(&self) -> &RigidBodySet { &self.bodies }
    pub fn bodies_mut_unchecked(&mut self) -> &mut RigidBodySet { &mut self.bodies }
    pub fn removal_equipment_unchecked(&mut self) -> (&mut RigidBodySet, &mut PartsReverseLookup) { ( &mut self.bodies, &mut self.parts_reverse_lookup ) }

    pub fn recurse_part<'a, F>(&'a self, part_handle: PartHandle, details: PartVisitDetails, func: &mut F)
    where F: FnMut(PartVisitHandle<'a>) {
        if let Some(part) = self.get_part(part_handle) {
            func(PartVisitHandle(self, part_handle, part, details));
            let attachment_dat = part.kind().attachment_locations();
            for (i, attachment) in part.attachments().iter().enumerate() {
                if let (Some(attachment), Some(attachment_dat)) = (attachment, attachment_dat[i]) {
                    let true_facing = attachment_dat.facing.compute_true_facing(details.true_facing);
                    let delta_rel_part = true_facing.delta_rel_part();
                    self.recurse_part(**attachment, PartVisitDetails {
                        part_rel_x: details.part_rel_x + delta_rel_part.0,
                        part_rel_y: details.part_rel_y + delta_rel_part.1,
                        my_facing: attachment_dat.facing,
                        true_facing
                    }, func);
                }
            }
        }
    }
    pub fn recurse_part_mut<'a, F>(&'a mut self, part_handle: PartHandle, details: PartVisitDetails, func: &mut F)
    where F: FnMut(PartVisitHandleMut<'_>) {
        if self.get_part_mut(part_handle).is_some() {
            func(PartVisitHandleMut(self, part_handle, details));
            let part = self.get_part(part_handle).unwrap();
            let attachment_dat = part.kind().attachment_locations();
            for (i, attachment) in part.attachments().iter().map(|attachment| attachment.as_ref().map(|attach| **attach)).collect::<Vec<_>>().into_iter().enumerate() {
                if let (Some(attachment), Some(attachment_dat)) = (attachment, attachment_dat[i]) {
                    let true_facing = attachment_dat.facing.compute_true_facing(details.true_facing);
                    let delta_rel_part = true_facing.delta_rel_part();
                    let details = PartVisitDetails {
                        part_rel_x: details.part_rel_x + delta_rel_part.0,
                        part_rel_y: details.part_rel_y + delta_rel_part.1,
                        my_facing: attachment_dat.facing,
                        true_facing
                    };
                    self.recurse_part_mut(attachment, details, func);
                }
            }
        }
    }
    pub fn recurse_part_with_return<'a, V, F>(&'a self, part_handle: PartHandle, details: PartVisitDetails, func: &mut F) -> Option<V>
    where F: FnMut(PartVisitHandle<'a>) -> Option<V> {
        if let Some(part) = self.get_part(part_handle) {
            let result = func(PartVisitHandle(self, part_handle, part, details));
            if result.is_some() { return result };
            let attachment_dat = part.kind().attachment_locations();
            for (i, attachment) in part.attachments().iter().enumerate() {
                if let (Some(attachment), Some(attachment_dat)) = (attachment, attachment_dat[i]) {
                    let true_facing = attachment_dat.facing.compute_true_facing(details.true_facing);
                    let delta_rel_part = true_facing.delta_rel_part();
                    if let Some(result) = self.recurse_part_with_return(**attachment, PartVisitDetails {
                        part_rel_x: details.part_rel_x + delta_rel_part.0,
                        part_rel_y: details.part_rel_y + delta_rel_part.1,
                        my_facing: attachment_dat.facing,
                        true_facing
                    }, func) {
                        return Some(result)
                    }
                }
            }
        }
        return None;
    }
    pub fn recurse_part_mut_with_return<'a, V, F>(&'a mut self, part_handle: PartHandle, details: PartVisitDetails, func: &mut F) -> Option<V>
    where F: FnMut(PartVisitHandleMut<'_>) -> Option<V> {
        if self.get_part_mut(part_handle).is_some() {
            let result = func(PartVisitHandleMut(self, part_handle, details));
            if result.is_some() { return result };
            drop(result);
            let part = self.get_part_mut(part_handle).unwrap();
            let attachment_dat = part.kind().attachment_locations();
            for (i, attachment) in part.attachments().iter().map(|attachment| attachment.as_ref().map(|attach| **attach)).collect::<Vec<_>>().into_iter().enumerate() {
                if let (Some(attachment), Some(attachment_dat)) = (attachment, attachment_dat[i]) {
                    let true_facing = attachment_dat.facing.compute_true_facing(details.true_facing);
                    let delta_rel_part = true_facing.delta_rel_part();
                    if let Some(result) = self.recurse_part_mut_with_return(attachment, PartVisitDetails {
                        part_rel_x: details.part_rel_x + delta_rel_part.0,
                        part_rel_y: details.part_rel_y + delta_rel_part.1,
                        my_facing: attachment_dat.facing,
                        true_facing
                    }, func) {
                        return Some(result)
                    }
                }
            }
        }
        return None;
    }

    pub fn recursive_thrust_player(&mut self, core_handle: PartHandle, player: &mut PlayerMeta) {
        if let Some(part) = self.parts.get(core_handle) {
            part.thrust_no_recurse(&mut player.power, player.thrust_forwards, player.thrust_backwards, player.thrust_clockwise, player.thrust_counterclockwise, &mut self.bodies);
            let attachments = [part.attachments()[0].as_ref().map(|e| **e), part.attachments()[1].as_ref().map(|e| **e), part.attachments()[2].as_ref().map(|e| **e), part.attachments()[3].as_ref().map(|e| **e)];
            for i in 0..4 {
                if let Some(attachment) = attachments[i] {
                    self.recursive_thrust_player(attachment, player);
                }
            }
        }
    }

    pub fn recursive_detach_one(&mut self, parent_handle: PartHandle, attachment_slot: usize, player: &mut Option<&mut crate::PlayerMeta>, islands: &mut IslandManager, colliders: &mut ColliderSet, joints: &mut JointSet, parts_affected: &mut BTreeSet<PartHandle>) {
        if let Some(attachment_handle) = Part::detach_part_player_agnostic(parent_handle, attachment_slot, self, islands, joints) {
            parts_affected.insert(attachment_handle);
            if let Some(player) = player {
                if let Some(attached_part) = self.get_part_mut(attachment_handle) {
                    attached_part.remove_from(*player, colliders);
                }
            }
            self.recursive_detach_all(attachment_handle, player, islands, colliders, joints, parts_affected);                                
        }
    }
    pub fn recursive_detach_all(&mut self, parent_handle: PartHandle, player: &mut Option<&mut crate::PlayerMeta>, islands: &mut IslandManager, colliders: &mut ColliderSet, joints: &mut JointSet, parts_affected: &mut BTreeSet<PartHandle>) {
        if let Some(part) = self.get_part_mut(parent_handle) {
            for i in 0..part.attachments().len() {
                self.recursive_detach_one(parent_handle, i, player, islands, colliders, joints, parts_affected);
            }
        }
    }
    pub fn remove_part_unchecked(&mut self, part_handle: PartHandle) -> Part {
        self.parts.remove(part_handle).expect("remove_part_unchecked")
    }

    fn new(colliders: &mut ColliderSet) -> World { 
        let mut bodies = RigidBodySet::new();
        let reference_point_body = RigidBodyBuilder::new(BodyStatus::Static).additional_mass(0f32).build();
        let reference_point_body = bodies.insert(reference_point_body);
        let planets = planets::Planets::new(&mut bodies, colliders);
        World {
            bodies,
            parts: Arena::new(),
            planets,
            reference_point_body,
            parts_reverse_lookup: BTreeMap::new(),
        }
    }

    fn advance_orbits(&mut self) {
        self.planets.advance_orbits(&mut self.bodies);
    }

    fn celestial_gravity(&mut self) {
        for (_part_handle, part) in self.parts.iter() {
            let part = &mut self.bodies[part.body_handle()];
            const GRAVITATION_CONSTANT: f32 = 1.0; //Lolrandom
            for body in self.planets.planets.values() {
                let distance: (f32, f32) = ((body.position.0 - part.position().translation.x),
                                            (body.position.1 - part.position().translation.y));
                let magnitude: f32 = part.mass() * body.mass
                                     / (distance.0.powf(2f32) + distance.1.powf(2f32));
                                     //* GRAVITATION_CONSTANT;
                if distance.0.abs() > distance.1.abs() {
                    part.apply_force(Vector::new(if distance.0 >= 0.0 { magnitude } else { -magnitude }, distance.1 / distance.0.abs() * magnitude), false);
                } else {
                    part.apply_force(Vector::new(distance.0 / distance.1.abs() * magnitude, if distance.1 >= 0.0 { magnitude } else { -magnitude }), false);
                }
            }
        }
    }

    fn reference_point_body(&self) -> RigidBodyHandle {
        self.reference_point_body
    }
}

#[derive(Copy, Clone)]
pub struct PartVisitDetails {
    pub part_rel_x: i32,
    pub part_rel_y: i32,
    pub my_facing: AttachedPartFacing,
    pub true_facing: AttachedPartFacing,
}
impl Default for PartVisitDetails {
    fn default() -> Self { PartVisitDetails {
        part_rel_x: 0,
        part_rel_y: 0,
        my_facing: AttachedPartFacing::Up,
        true_facing: AttachedPartFacing::Up,
    } }
}

pub struct PartVisitHandle<'a> (&'a World, PartHandle, &'a Part, PartVisitDetails);
impl<'a> PartVisitHandle<'a> {
    pub fn get_part(&self, handle: PartHandle) -> &'a Part { self.2 }
    pub fn get_rigid(&mut self, handle: PartHandle) -> &RigidBody { self.0.get_part_rigid(self.1).unwrap() }
    pub fn handle(&self) -> PartHandle { self.1 }
    pub fn details(&self) -> &PartVisitDetails { &self.3 }
}
impl<'a> Deref for PartVisitHandle<'a> {
    type Target = Part;
    fn deref(&self) -> &Part { self.2 }
}
pub struct PartVisitHandleMut<'a> (&'a mut World, PartHandle, PartVisitDetails);
impl<'a> PartVisitHandleMut<'a> {
    pub fn get_part(&self, handle: PartHandle) -> Option<&Part> { self.0.get_part(handle) }
    pub fn get_rigid(&self, handle: PartHandle) -> Option<&RigidBody> { self.0.get_part_rigid(handle) }
    pub fn get_part_mut(&mut self, handle: PartHandle) -> Option<&mut Part> { self.0.get_part_mut(handle) }
    pub fn get_rigid_mut(&mut self, handle: PartHandle) -> Option<&mut RigidBody> { self.0.get_part_rigid_mut(handle) }
    pub fn self_rigid(&self) -> &RigidBody { self.get_rigid(self.1).unwrap() }
    pub fn self_rigid_mut(&mut self) -> &mut RigidBody { self.get_rigid_mut(self.1).unwrap() }
    pub fn handle(&self) -> PartHandle { self.1 }
    pub fn details(&self) -> &PartVisitDetails { &self.2 }
    pub fn world_unchecked(&self) -> &World { &*self.0 }
    pub fn world_mut_unchecked(&mut self) -> &mut World { self.0 }
}
impl<'a> Deref for PartVisitHandleMut<'a> {
    type Target = Part;
    fn deref(&self) -> &Part { self.get_part(self.1).unwrap() }
}
impl<'a> DerefMut for PartVisitHandleMut<'a> {
    fn deref_mut(&mut self) -> &mut Part { self.get_part_mut(self.1).unwrap() }
}

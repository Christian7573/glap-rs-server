use std::collections::{BTreeMap, BTreeSet};
//use num_traits::Pow;

use crate::PartOfPlayer;
use generational_arena::{Arena, Index};
use crate::codec::ToClientMsg;
use std::ops::{Deref, DerefMut};

use rapier2d::dynamics::{BodyStatus, CCDSolver, JointSet, RigidBody, RigidBodyBuilder, RigidBodyHandle, RigidBodySet, IntegrationParameters};
use rapier2d::geometry::{BroadPhase, NarrowPhase, ColliderSet};
use rapier2d::pipeline::PhysicsPipeline;

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
}
use typedef::*;

pub struct Simulation {
    pub world: World,
    pipeline: PhysicsPipeline,
    integration_parameters: IntegrationParameters,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    pub colliders: ColliderSet,
    pub joints: JointSet,
    ccd_solver: CCDSolver,
    steps_per_batch: u8,
}
pub enum SimulationEvent {
    PlayerTouchPlanet { player: u16, part: PartHandle, planet: u8, },
    PlayerUntouchPlanet { player: u16, part: PartHandle, planet: u8 },
}


impl Simulation {
    pub fn new(step_time: f32, steps_per_batch: u8) -> Simulation {
        /*mechanics.set_timestep(step_time);
        mechanics.integration_parameters.max_ccd_substeps = 5;*/
        let mut colliders = ColliderSet::new();
        let mut world = World::new(&mut colliders);

        let simulation = Simulation {
            pipeline: PhysicsPipeline::new(),
            integration_parameters: IntegrationParameters::default(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            ccd_solver: CCDSolver::new(),

            steps_per_batch,
            world,
            colliders,
            joints: JointSet::new()
        };
        simulation
    }


    pub fn simulate(&mut self, events: &mut Vec<SimulationEvent>) {
        self.world.celestial_gravity();
        self.mechanics.step(&mut self.geometry, &mut self.world, &mut self.colliders, &mut self.joints, &mut self.persistant_forces);
        for contact_event in self.geometry.contact_events() {
            match contact_event {
                ContactEvent::Started(handle1, handle2) => {
                    let planet: u16;
                    let other: DefaultColliderHandle;
                    if let Some(am_planet) = self.colliders.get(*handle1).unwrap().user_data().map(|any| any.downcast_ref::<AmPlanet>()).flatten() {
                        planet = am_planet.id; other = *handle2;
                    } else if let Some(am_planet) = self.colliders.get(*handle2).unwrap().user_data().map(|any| any.downcast_ref::<AmPlanet>()).flatten() {
                        planet = am_planet.id; other = *handle1;
                    } else { continue; }
                    let part_coll = self.colliders.get(other).unwrap();
                    if let Some(part) = self.world.get_part(part_coll.body()) {
                        if let Some(player_id) = part.part_of_player() {
                            events.push(SimulationEvent::PlayerTouchPlanet{ player: player_id, part: part_coll.body(), planet });
                        }
                    }
                },
                ContactEvent::Stopped(handle1, handle2) => {
                    let planet: u16;
                    let other: DefaultColliderHandle;
                    if let Some(am_planet) = self.colliders.get(*handle1).unwrap().user_data().map(|any| any.downcast_ref::<AmPlanet>()).flatten() {
                        planet = am_planet.id; other = *handle2;
                    } else if let Some(am_planet) = self.colliders.get(*handle2).unwrap().user_data().map(|any| any.downcast_ref::<AmPlanet>()).flatten() {
                        planet = am_planet.id; other = *handle1;
                    } else { continue; }
                    let part_coll = self.colliders.get(other).unwrap();
                    if let Some(part) = self.world.get_part(part_coll.body()) {
                        if let Some(player_id) = part.part_of_player() {
                            events.push(SimulationEvent::PlayerUntouchPlanet{ player: player_id, part: part_coll.body(), planet });
                        }
                    }
                }
            }
        }
    }

    pub fn equip_mouse_dragging(&mut self, part: PartHandle) -> DefaultJointConstraintHandle {
        let body = self.world.get_rigid_mut(part).unwrap();
        body.set_local_inertia(Inertia2::new(0.00000001, body.augmented_mass().angular));
        let space = body.position().translation;
        let constraint = MouseConstraint::new(
            BodyPartHandle(part, 0),
            BodyPartHandle(self.world.reference_point_body, 0),
            Point::new(0.0,0.0),
            Point::new(space.x, space.y),
            1000.0
        );
        self.joints.insert(constraint)
    }
    pub fn move_mouse_constraint(&mut self, constraint_id: DefaultJointConstraintHandle, x: f32, y: f32) {
        if let Some(Some(constraint)) = self.joints.get_mut(constraint_id).map(|c: &mut dyn JointConstraint<MyUnits, PartHandle>| c.downcast_mut::<MouseConstraint<MyUnits, PartHandle>>() ) {
            constraint.set_anchor_2(Point::new(x, y));
        }
    }
    pub fn release_constraint(&mut self, constraint_id: DefaultJointConstraintHandle) {
        self.joints.remove(constraint_id);
    }

    pub fn is_constraint_broken(&self, handle: DefaultJointConstraintHandle) -> bool {
        self.joints.get(handle).map(|joint| joint.is_broken()).unwrap_or(true)
    }

    pub fn geometrical_world(&self) -> &MyGeometricalWorld { &self.geometry }

    pub fn inflate(&mut self, parts: &RecursivePartDescription, initial_location: MyIsometry) -> PartHandle {
        parts.inflate(&mut (&mut self.world).into(), &mut self.colliders, &mut self.joints, initial_location)
    }
    pub fn delete_parts_recursive(&mut self, index: PartHandle) -> Vec<ToClientMsg> {
        let mut removal_msgs = Vec::new();
        self.world.delete_parts_recursive(index, &mut self.colliders, &mut self.joints, &mut removal_msgs);
        removal_msgs
    }
}

pub struct World {
    parts: Arena<Part>,
    bodies: RigidBodySet,
    pub planets: planets::Planets,
    reference_point_body: Index,
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
        self.parts.get(index).map(|obj| obj.rigid()).flatten()
    }
    pub fn get_part(&self, index: PartHandle) -> Option<&Part> {
        self.parts.get(index)
    }
    pub fn get_part_rigid_mut(&mut self, index: PartHandle) -> Option<&mut RigidBody> {
        self.parts.get(index).map(|obj| self.bodies.get(obj.body_handle)).flatten()
    }
    pub fn get_part_mut(&mut self, index: PartHandle) -> Option<&mut Part> {
        self.storage.get_mut(index)
    }
    pub fn delete_parts_recursive(&mut self, index: PartHandle, colliders: &mut ColliderSet, joints: &mut JointSet, removal_msgs: &mut Vec<ToClientMsg>) {
        match self.parts.remove(index) {
            Some(part) => {
                self.removal_events.push_back(index);
                part.delete_recursive(self, colliders, joints, removal_msgs);
            },
            None => (),
        }
    }
    pub fn bodies_unchecked(&self) -> &RigidBodySet { &self.bodies }
    pub fn bodies_mut_unchecked(&mut self) -> &mut RigidBodySet { &mut self.bodies }

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

    pub fn recursive_detach_one(&mut self, parent_handle: PartHandle, attachment_slot: usize, player: &mut Option<&mut crate::PlayerMeta>, joints: &mut JointSet, parts_affected: &mut BTreeSet<PartHandle>) {
        if let Some(parent) = self.get_part_mut(parent_handle) {
            if let Some(attachment_handle) = parent.detach_part_player_agnostic(attachment_slot, joints) {
                parts_affected.insert(attachment_handle);
                if let Some(player) = player {
                    if let Some(attached_part) = self.get_part_mut(attachment_handle) {
                        attached_part.remove_from(*player);
                    }
                }
                self.recursive_detach_all(attachment_handle, player, joints, parts_affected);                                
            }
        }
    }
    pub fn recursive_detach_all(&mut self, parent_handle: PartHandle, player: &mut Option<&mut crate::PlayerMeta>, joints: &mut JointSet, parts_affected: &mut BTreeSet<PartHandle>) {
        if let Some(part) = self.get_part_mut(parent_handle) {
            for i in 0..part.attachments().len() {
                self.recursive_detach_one(parent_handle, i, player, joints, parts_affected);
            }
        }
    }
    pub fn remove_part_unchecked(&mut self, part_handle: PartHandle) -> Part {
        self.parts.remove(part_handle).expect("remove_part_unchecked")
    }

    pub fn iter_parts_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item=(PartHandle, &'a mut Part, &'a mut RigidBody)> + 'a> {
        Box::new(self.parts.iter_mut().map(|(handle, part)| (handle, part, self.bodies[part.body_handle])))
    }

    fn new(colliders: &mut ColliderSet) -> World { 
        let bodies = RigidBodySet::new();
        let reference_point_body = RigidBodyBuilder::new(BodyStatus::Static).mass(0f32).build();
        let reference_point_body = bodies.insert(reference_point_body);
        World {
            bodies,
            parts: Arena::new(),
            planets: planets::Planets::new(&mut bodies, colliders),
            reference_point_body,
        }
    }

    fn celestial_gravity(&mut self) {
        for (_part_handle, part) in self.parts.iter() {
            let part = &mut self.bodies[part.body_handle];
            const GRAVITATION_CONSTANT: f32 = 1.0; //Lolrandom
            for body in &self.world.planets.celestial_objects() {
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
    pub fn get_rigid(&mut self, handle: PartHandle) -> &RigidBody { self.0.get_part_rigid(self.2.body_handle).unwrap() }
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
    pub fn get_rigid_mut(&mut self, handle: PartHandle) -> Option<&mut RigidBody> { self.0.get_rigid_mut(handle) }
    pub fn handle(&self) -> PartHandle { self.1 }
    pub fn details(&self) -> &PartVisitDetails { &self.2 }
}
impl<'a> Deref for PartVisitHandleMut<'a> {
    type Target = Part;
    fn deref(&self) -> &Part { self.get_part(self.1).unwrap() }
}
impl<'a> DerefMut for PartVisitHandleMut<'a> {
    fn deref_mut(&mut self) -> &mut Part { self.get_part_mut(self.1).unwrap() }
}

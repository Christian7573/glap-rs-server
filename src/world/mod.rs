use nalgebra::Vector2;
use nphysics2d::object::{RigidBody, Body, BodyPartHandle, DefaultColliderHandle};
use std::collections::{BTreeMap, BTreeSet};
use nphysics2d::force_generator::DefaultForceGeneratorSet;
use num_traits::Pow;
use nphysics2d::algebra::{Force2, ForceType, Inertia2};
use nphysics2d::joint::{DefaultJointConstraintHandle, MouseConstraint, JointConstraint};
use nphysics2d::math::Point;
use ncollide2d::pipeline::ContactEvent;
use crate::PartOfPlayer;
use generational_arena::{Arena, Index};
use crate::codec::ToClientMsg;

pub mod planets;
pub mod parts;

pub mod nphysics_types {
    pub type MyUnits = f32;
    pub type MyHandle = Index;
    pub type MyIsometry = nphysics2d::math::Isometry<MyUnits>;
    pub type MyColliderHandle = nphysics2d::object::DefaultColliderHandle;
    pub type MyMechanicalWorld = nphysics2d::world::MechanicalWorld<MyUnits, MyHandle, MyColliderHandle>;
    pub type MyBodySet = super::World;
    pub type MyRigidBody = nphysics2d::object::RigidBody<MyUnits>;
    pub type MyGeometricalWorld = nphysics2d::world::GeometricalWorld<MyUnits, MyHandle, MyColliderHandle>;
    pub type MyColliderSet = nphysics2d::object::DefaultColliderSet<MyUnits, MyHandle>;
    pub type MyJointSet = nphysics2d::joint::DefaultJointConstraintSet<MyUnits, MyHandle>;
    pub type MyJointHandle = nphysics2d::joint::DefaultJointConstraintHandle;
    pub type MyForceSet = nphysics2d::force_generator::DefaultForceGeneratorSet<MyUnits, MyHandle>;
}
use nphysics_types::*;

pub struct Simulation {
    pub world: World,
    mechanics: MyMechanicalWorld,
    geometry: MyGeometricalWorld,
    pub colliders: MyColliderSet,
    pub joints: MyJointSet,
    persistant_forces: MyForceSet,
    pub planets: planets::Planets,
}
pub enum SimulationEvent {
    PlayerTouchPlanet { player: u16, part: u16, planet: u16, },
    PlayerUntouchPlanet { player: u16, part: u16, planet: u16 },
}


impl Simulation {
    pub fn new(step_time: f32) -> Simulation {
        let mut mechanics = MyMechanicalWorld::new(Vector2::new(0.0, 0.0));
        mechanics.set_timestep(step_time);
        let geometry: MyGeometricalWorld = MyGeometricalWorld::new();
        let mut colliders: MyColliderSet = MyColliderSet::new();
        let mut bodies = World::default();
        let planets = planets::Planets::new(&mut colliders, &mut bodies);
        let simulation = Simulation {
            mechanics, geometry, colliders, world: bodies,
            joints: MyJointSet::new(),
            persistant_forces: MyForceSet::new(),
            planets,
        };
        simulation
    }

    fn celestial_gravity(&mut self) {
        fn do_gravity_for_part(part: &mut MyRigidBody, celestial_bodies: &[planets::CelestialObject]) {
            const GRAVITATION_CONSTANT: f32 = 1.0; //Lolrandom
            for body in celestial_bodies {
                let distance: (f32, f32) = ((body.position.0 - part.position().translation.x),
                                            (body.position.1 - part.position().translation.y));
                let magnitude: f32 = part.augmented_mass().linear * body.mass
                                     / (distance.0.pow(2f32) + distance.1.pow(2f32));
                                     //* GRAVITATION_CONSTANT;
                if distance.0.abs() > distance.1.abs() {
                    part.apply_force(0, &Force2::linear(Vector2::new(if distance.0 >= 0.0 { magnitude } else { -magnitude }, distance.1 / distance.0.abs() * magnitude)), ForceType::Force, false);
                } else {
                    part.apply_force(0, &Force2::linear(Vector2::new(distance.0 / distance.1.abs() * magnitude, if distance.1 >= 0.0 { magnitude } else { -magnitude })), ForceType::Force, false);
                }
                
            }
        }
        for part in self.world.parts.values_mut() {
            do_gravity_for_part(part, &self.world.celestial_objects);
        }
    }

    pub fn simulate(&mut self, events: &mut Vec<SimulationEvent>) {
        self.celestial_gravity();
        self.mechanics.step(&mut self.geometry, &mut self.world, &mut self.colliders, &mut self.joints, &mut self.persistant_forces);
        for contact_event in self.geometry.contact_events() {
            match contact_event {
                ContactEvent::Started(handle1, handle2) => {
                    let planet: u16;
                    let other: DefaultColliderHandle;
                    if let MyHandle::CelestialObject(planet_id) = self.colliders.get(*handle1).unwrap().body() {
                        planet = planet_id; other = *handle2;
                    } else if let MyHandle::CelestialObject(planet_id) = self.colliders.get(*handle2).unwrap().body() {
                        planet = planet_id; other = *handle1;
                    } else { continue; }
                    let part_coll = self.colliders.get(other).unwrap();
                    if let MyHandle::Part(part_id) = part_coll.body() {
                        if let Some(PartOfPlayer(player_id)) = part_coll.user_data().map(|dat| dat.downcast_ref()).flatten() {
                            events.push(SimulationEvent::PlayerTouchPlanet{ player: *player_id, part: part_id, planet: planet });
                        }
                    }
                },
                ContactEvent::Stopped(handle1, handle2) => {
                    let planet: u16;
                    let other: DefaultColliderHandle;
                    if let MyHandle::CelestialObject(planet_id) = self.colliders.get(*handle1).unwrap().body() {
                        planet = planet_id; other = *handle2;
                    } else if let MyHandle::CelestialObject(planet_id) = self.colliders.get(*handle2).unwrap().body() {
                        planet = planet_id; other = *handle1;
                    } else { continue; }
                    let part_coll = self.colliders.get(other).unwrap();
                    if let MyHandle::Part(part_id) = part_coll.body() {
                        if let Some(PartOfPlayer(player_id)) = part_coll.user_data().map(|dat| dat.downcast_ref()).flatten() {
                            events.push(SimulationEvent::PlayerUntouchPlanet{ player: *player_id, part: part_id, planet: planet });
                        }
                    }
                }
            }
        }
    }

    pub fn equip_mouse_dragging(&mut self, part_id: u16) -> DefaultJointConstraintHandle {
        let body = self.world.get_rigid_mut(MyHandle::Part(part_id)).unwrap();
        body.set_local_inertia(Inertia2::new(0.00000001, body.augmented_mass().angular));
        let space = body.position().translation;
        let constraint = MouseConstraint::new(
            BodyPartHandle(MyHandle::Part(part_id), 0),
            BodyPartHandle(MyHandle::ReferencePoint, 0),
            Point::new(0.0,0.0),
            Point::new(space.x, space.y),
            1000.0
        );
        self.joints.insert(constraint)
    }
    pub fn move_mouse_constraint(&mut self, constraint_id: DefaultJointConstraintHandle, x: f32, y: f32) {
        if let Some(Some(constraint)) = self.joints.get_mut(constraint_id).map(|c: &mut dyn JointConstraint<MyUnits, MyHandle>| c.downcast_mut::<MouseConstraint<MyUnits, MyHandle>>() ) {
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

    pub fn delete_parts_recursive(&mut self, index: MyHandle) -> Vec<ToClientMsg> {
        let mut removal_msgs = Vec::new();
        self.world.delete_parts_recursive(index, &mut self.colliders, &mut self.joints, &mut removal_msgs);
        removal_msgs
    }
}

type MyStorage = Arena<WorldlyObject>;
pub struct World {
    storage: MyStorage,
    removal_events: std::collections::VecDeque<MyHandle>,
    reference_point_body: Index,
}

pub enum WorldlyObject {
    CelestialObject(MyRigidBody),
    Part(parts::Part),
    Uninitialized,
}
impl WorldlyObject {
    pub fn rigid(&self) -> Option<&MyRigidBody> {
         match self {
            WorldlyObject::Part(part) => Some(part.body()),
            WorldlyObject::CelestialObject(body) => Some(body),
            WorldlyObject::Uninitialized => None
        }
    }
    pub fn rigid_mut(&mut self) -> Option<&mut MyRigidBody> {
        match self {
            WorldlyObject::Part(part) => Some(part.body_mut()),
            WorldlyObject::CelestialObject(body) => Some(body),
            WorldlyObject::Uninitialized => None
        }
    }

}

pub struct WorldAddHandle<'a>(&'a mut MyStorage); 
impl<'a> WorldAddHandle<'a> {
    pub fn add_now(&self, object: WorldlyObject) -> Index { self.0.insert(object) }
    pub fn add_later(&self) -> Index { self.0.insert(WorldlyObject::Uninitialized) }
    pub fn add_its_later(&self, index: Index, object: WorldlyObject) {
        match std::mem::replace(self.0.get_mut(index).expect("add_its_later: the index doesn't exist"), object) {
            WorldlyObject::Uninitialized => {},
            _ => panic!("add_its_later: the index wasn't WorldlyObject::Uninitialized. Storage is now poisioned(?)")
        }
    }
}
impl<'a> From<&'a mut World> for WorldAddHandle<'a> {
    fn from(world: &'a mut World) -> WorldAddHandle<'a> { WorldAddHandle(&mut world.storage) }
}

impl World {
    pub fn get_rigid(&self, index: MyHandle) -> Option<&MyRigidBody> {
        self.storage.get(index).map(|obj| obj.rigid()).flatten()
    }
    pub fn get_part(&self, index: MyHandle) -> Option<&parts::Part> {
        self.storage.get(index).map(|obj| match obj { WorldlyObject::Part(part) => Some(part), _ => None }).flatten()
    }
    pub fn get_rigid_mut(&mut self, index: MyHandle) -> Option<&MyRigidBody> {
        self.storage.get_mut(index).map(|obj| obj.rigid()).flatten()
    }
    pub fn get_part_mut(&mut self, index: MyHandle) -> Option<&mut parts::Part> {
        self.storage.get_mut(index).map(|obj| match obj { WorldlyObject::Part(part) => Some(part), _ => None }).flatten()
    }
    pub fn delete_parts_recursive(&mut self, index: MyHandle, colliders: &mut MyColliderSet, joints: &mut MyJointSet, removal_msgs: &mut Vec<ToClientMsg>) {
        match self.storage.remove(index) {
            Some(WorldlyObject::Part(part)) => {
                self.removal_events.push(index);
                part.delete_recursive(self, colliders, joints, removal_msgs);
            },
            None => (),
            _ => panic!("Delete part called on non-part")
        }
    }
    pub fn add_celestial_object(&mut self, body: MyRigidBody) -> MyHandle { self.storage.insert(WorldlyObject::CelestialObject(body)) }
}
impl Default for World {
    fn default() -> World { 
        let mut storage = Arena::new();
        let reference_point_body = nphysics2d::object::RigidBodyDesc::new().status(nphysics2d::object::BodyStatus::Static).mass(0f32).build();
        let reference_point_body = storage.insert(WorldlyObject::CelestialObject(reference_point_body));
        World {
            storage,
            reference_point_body,
            removal_events: std::collections::VecDeque::<MyHandle>::new(),
        }
    }
}
impl nphysics2d::object::BodySet<MyUnits> for World {
    type Handle = MyHandle;
    fn get(&self, handle: Self::Handle) -> Option<&dyn nphysics2d::object::Body<MyUnits>> {
        if let Some(ptr) = self.get_rigid(handle) { Some(ptr) }
        else { None }
    }
    fn get_mut(&mut self, handle: Self::Handle) -> Option<&mut dyn nphysics2d::object::Body<MyUnits>> {
        if let Some(ptr) = self.get_rigid_mut(handle) { Some(ptr) }
        else { None }
    }
    fn contains(&self, handle: Self::Handle) -> bool {
        self.get_rigid(handle).is_some()
    }
    fn foreach(&self, f: &mut dyn FnMut(Self::Handle, &dyn nphysics2d::object::Body<MyUnits>)) {
        for (id, obj) in &self.storage {
            if let Some(body) = obj.rigid() { f(id, body) }
        }
    }
    fn foreach_mut(&mut self, f: &mut dyn FnMut(Self::Handle, &mut dyn nphysics2d::object::Body<MyUnits>)) {
        for (id, obj) in &mut self.storage {
            if let Some(body) = obj.rigid_mut() { f(id, body) }
        }
    }
    fn pop_removal_event(&mut self) -> Option<Self::Handle> {
        self.removal_events.pop_front()
    }
}

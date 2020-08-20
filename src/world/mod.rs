use nalgebra::Vector2;
use nphysics2d::object::{RigidBody, Body};
use std::collections::{BTreeMap, BTreeSet};
use nphysics2d::force_generator::DefaultForceGeneratorSet;
use num_traits::Pow;
use nphysics2d::algebra::{Force2, ForceType};

pub mod planets;
pub mod parts;

pub type MyUnits = f32;
pub type MyColliderHandle = nphysics2d::object::DefaultColliderHandle;
pub type MyMechanicalWorld = nphysics2d::world::MechanicalWorld<MyUnits, MyHandle, MyColliderHandle>;
pub type MyGeometricalWorld = nphysics2d::world::GeometricalWorld<MyUnits, MyHandle, MyColliderHandle>;
pub type MyColliderSet = nphysics2d::object::DefaultColliderSet<MyUnits, MyHandle>;
pub type MyJointSet = nphysics2d::joint::DefaultJointConstraintSet<MyUnits, MyHandle>;
pub type MyForceSet = nphysics2d::force_generator::DefaultForceGeneratorSet<MyUnits, MyHandle>;

pub struct Simulation {
    pub world: World,
    mechanics: MyMechanicalWorld,
    geometry: MyGeometricalWorld,
    pub colliders: MyColliderSet,
    joints: MyJointSet,
    persistant_forces: MyForceSet,
    pub planets: planets::Planets,
    pub part_static: parts::PartStatic
}
impl Simulation {
    pub fn new(step_time: f32) -> Simulation {
        let mut mechanics = MyMechanicalWorld::new(Vector2::new(0.0, 0.0));
        let mut geometry: MyGeometricalWorld = MyGeometricalWorld::new();
        let mut colliders: MyColliderSet = MyColliderSet::new();
        let mut bodies = World::default();
        let planets = planets::Planets::new(&mut colliders, &mut bodies);
        let mut simulation = Simulation {
            mechanics, geometry, colliders, world: bodies,
            joints: MyJointSet::new(),
            persistant_forces: MyForceSet::new(),
            planets,
            part_static: Default::default()
        };
        simulation.mechanics.set_timestep(step_time);

        //Add planets n stuff here

        simulation
    }

    fn celestial_gravity(&mut self) {
        fn do_gravity_for_part(part: &mut RigidBody<MyUnits>, celestial_bodies: &BTreeMap<u16, RigidBody<MyUnits>>) {
            const GRAVITATION_CONSTANT: f32 = 1.0; //Lolrandom
            for body in celestial_bodies.values() {
                let distance: (f32, f32) = ((body.position().translation.x - part.position().translation.x),
                                            (body.position().translation.y - part.position().translation.y));
                let magnitude: f32 = part.augmented_mass().linear * body.augmented_mass().linear 
                                     / (distance.0.pow(2f32) + distance.1.pow(2f32))
                                     * GRAVITATION_CONSTANT;
                if distance.0.abs() > distance.1.abs() {
                    part.apply_force(0, &Force2::linear(Vector2::new(if distance.0 >= 0.0 { magnitude } else { -magnitude }, distance.1 / distance.0 * magnitude)), ForceType::Force, false);
                } else {
                    part.apply_force(0, &Force2::linear(Vector2::new(distance.0 / distance.1 * magnitude, if distance.1 >= 0.0 { magnitude } else { -magnitude })), ForceType::Force, false);
                }
                
            }
        }
        // for player in self.world.player_parts.values_mut() {
        //     for part in player.values_mut() { do_gravity_for_part(part, &mut self.world.celestial_objects); }
        // }
        for part in self.world.parts.values_mut() {
            do_gravity_for_part(part, &self.world.celestial_objects);
        }
    }

    pub fn simulate(&mut self) {
        self.celestial_gravity();
        self.mechanics.step(&mut self.geometry, &mut self.world, &mut self.colliders, &mut self.joints, &mut self.persistant_forces);
    }
}

pub struct World {
    celestial_objects: BTreeMap<u16, RigidBody<MyUnits>>,
    // free_parts: BTreeMap<u16, RigidBody<MyUnits>>,
    // player_parts: BTreeMap<u16, BTreeMap<u16, RigidBody<MyUnits>>>,
    parts: BTreeMap<u16, RigidBody<MyUnits>>,
    removal_events: std::collections::VecDeque<MyHandle>,
    next_celestial_object: u16,
    next_part: u16
}
#[derive(Copy, Eq, Debug)]
pub enum MyHandle {
    CelestialObject(u16),
    Part(u16),
}
impl Clone for MyHandle {
    fn clone(&self) -> MyHandle { *self }
}
impl PartialEq for MyHandle {
    fn eq(&self, other: &Self) -> bool {
        match self {
            MyHandle::CelestialObject(id) => if let MyHandle::CelestialObject(other_id) = other { *id == *other_id } else { false },
            MyHandle::Part(id) => if let MyHandle::Part(other_id) = other { *id == *other_id } else { false }
        }
    }
}
impl std::hash::Hash for MyHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u16(match self {
            MyHandle::CelestialObject(id) => *id,
            MyHandle::Part(id) => *id
        });
    }
}

impl World {
    pub fn add_celestial_object(&mut self, body: RigidBody<MyUnits>) -> MyHandle {
        let id = self.next_celestial_object;
        self.next_celestial_object += 1;
        let handle = MyHandle::CelestialObject(id);
        self.celestial_objects.insert(id, body);
        handle
    }
    pub fn add_part(&mut self, body: RigidBody<MyUnits>) -> u16 {
        let id = self.next_part;
        self.next_part += 1;
        self.parts.insert(id, body);
        id
    }
    pub fn get_rigid(&self, handle: MyHandle) -> Option<&RigidBody<MyUnits>> {
        match handle {
            MyHandle::CelestialObject(id) => self.celestial_objects.get(&id),
            MyHandle::Part(id) => self.parts.get(&id),
        }
    }
    pub fn get_rigid_mut(&mut self, handle: MyHandle) -> Option<&mut RigidBody<MyUnits>> {
        match handle {
            MyHandle::CelestialObject(id) => self.celestial_objects.get_mut(&id),
            MyHandle::Part(id) => self.parts.get_mut(&id)
        }
    }
    pub fn get_parts(&self) -> &BTreeMap<u16, RigidBody<MyUnits>> { &self.parts }
    pub fn remove_part(&mut self, handle: MyHandle) {
        if let MyHandle::Part(id) = handle {
            if let Some(_) = self.parts.remove(&id) { self.removal_events.push_back(handle); }
        } else { panic!(); };
    }
}
impl Default for World {
    fn default() -> World { World {
        celestial_objects: BTreeMap::new(),
        parts: BTreeMap::new(),
        removal_events: std::collections::VecDeque::new(),
        next_celestial_object: 0,
        next_part: 0,
    } }
}
impl nphysics2d::object::BodySet<MyUnits> for World {
    type Handle = MyHandle;
    fn get(&self, handle: Self::Handle) -> Option<&dyn nphysics2d::object::Body<MyUnits>> {
        let ptr = match handle {
            MyHandle::CelestialObject(id) => self.celestial_objects.get(&id),
            MyHandle::Part(id) => self.parts.get(&id),
        };
        if let Some(ptr) = ptr { Some(ptr) }
        else { None }
    }
    fn get_mut(&mut self, handle: Self::Handle) -> Option<&mut dyn nphysics2d::object::Body<MyUnits>> {
        let ptr = match handle {
            MyHandle::CelestialObject(id) => self.celestial_objects.get_mut(&id),
            MyHandle::Part(id) => self.parts.get_mut(&id),
        };
        if let Some(ptr) = ptr { Some(ptr) }
        else { None }
    }
    fn contains(&self, handle: Self::Handle) -> bool {
        match handle {
            MyHandle::CelestialObject(id) => self.celestial_objects.contains_key(&id),
            MyHandle::Part(id) => self.parts.contains_key(&id),
        }
    }
    fn foreach(&self, f: &mut dyn FnMut(Self::Handle, &dyn nphysics2d::object::Body<MyUnits>)) {
        for (id, body) in &self.celestial_objects { f(MyHandle::CelestialObject(*id), body); }
        // for (player, bodies) in &self.player_parts {
        //     for (id, body) in bodies { f(MyHandle::Part(Some(*player), *id), body); }
        // }
        for (id, body) in &self.parts { f(MyHandle::Part(*id), body); }
    }
    fn foreach_mut(&mut self, f: &mut dyn FnMut(Self::Handle, &mut dyn nphysics2d::object::Body<MyUnits>)) {
        for (id, body) in &mut self.celestial_objects { f(MyHandle::CelestialObject(*id), body); }
        // for (player, bodies) in &mut self.player_parts {
        //     for (id, body) in bodies { f(MyHandle::Part(Some(*player), *id), body); }
        // }
        for (id, body) in &mut self.parts { f(MyHandle::Part(*id), body); }
    }
    fn pop_removal_event(&mut self) -> Option<Self::Handle> {
        self.removal_events.pop_front()
    }
}
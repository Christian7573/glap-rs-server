use nalgebra::Vector2;
use nphysics2d::object::{RigidBody, Body};
use std::collections::BTreeMap;
use nphysics2d::force_generator::DefaultForceGeneratorSet;
use num_traits::Pow;
use nphysics2d::algebra::{Force2, ForceType};

pub mod planets;

type MyUnits = f32;
type MyColliderHandle = nphysics2d::object::DefaultColliderHandle;
type MyMechanicalWorld = nphysics2d::world::MechanicalWorld<MyUnits, MyHandle, MyColliderHandle>;
type MyGeometricalWorld = nphysics2d::world::GeometricalWorld<MyUnits, MyHandle, MyColliderHandle>;
type MyColliderSet = nphysics2d::object::DefaultColliderSet<MyUnits, MyHandle>;
type MyJointSet = nphysics2d::joint::DefaultJointConstraintSet<MyUnits, MyHandle>;
type MyForceSet = nphysics2d::force_generator::DefaultForceGeneratorSet<MyUnits, MyHandle>;

pub struct Simulation {
    bodies: World,
    mechanics: MyMechanicalWorld,
    geometry: MyGeometricalWorld,
    colliders: MyColliderSet,
    joints: MyJointSet,
    persistant_forces: MyForceSet,
    planets: planets::Planets
}
impl Simulation {
    pub fn new(step_time: f32) -> Simulation {
        let mut mechanics = MyMechanicalWorld::new(Vector2::new(0.0, 0.0));
        let mut geometry: MyGeometricalWorld = MyGeometricalWorld::new();
        let mut colliders: MyColliderSet = MyColliderSet::new();
        let mut bodies = World::default();
        let planets = planets::Planets::new(&mut mechanics, &mut geometry, &mut colliders, &mut bodies);
        let mut simulation = Simulation {
            mechanics, geometry, colliders, bodies,
            joints: MyJointSet::new(),
            persistant_forces: MyForceSet::new(),
            planets,
        };
        simulation.mechanics.set_timestep(step_time);

        //Add planets n stuff here

        simulation
    }

    fn celestial_gravity(&mut self) {
        fn do_gravity_for_part(part: &mut RigidBody<MyUnits>, celestial_bodies: &BTreeMap<u16, RigidBody<MyUnits>>) {
            const GRAVITATION_CONSTANT: f32 = 0.00000001; //Lolrandom
            for body in celestial_bodies.values() {
                let distance: (f32, f32) = ((part.position().translation.x - body.position().translation.x).abs(),
                                            (part.position().translation.y - body.position().translation.y).abs());
                let magnitude: f32 = part.augmented_mass().linear * body.augmented_mass().linear 
                                     / (distance.0.pow(2f32) + distance.1.pow(2f32))
                                     * GRAVITATION_CONSTANT;
                if distance.0 > distance.1 {
                    part.apply_force(0, &Force2::linear(Vector2::new(magnitude, distance.1 / distance.0 * magnitude)), ForceType::Force, true);
                } else {
                    part.apply_force(0, &Force2::linear(Vector2::new(distance.0 / distance.1 * magnitude, magnitude)), ForceType::Force, true);
                }
                
            }
        }
        for player in self.bodies.player_parts.values_mut() {
            for part in player.values_mut() { do_gravity_for_part(part, &mut self.bodies.celestial_objects); }
        }
        for part in self.bodies.free_parts.values_mut() {
            do_gravity_for_part(part, &self.bodies.celestial_objects);
        }
    }

    pub fn simulate(&mut self) {
        self.celestial_gravity();
        self.mechanics.step(&mut self.geometry, &mut self.bodies, &mut self.colliders, &mut self.joints, &mut self.persistant_forces);
    }
}

pub struct World {
    celestial_objects: BTreeMap<u16, RigidBody<MyUnits>>,
    free_parts: BTreeMap<u16, RigidBody<MyUnits>>,
    player_parts: BTreeMap<u16, BTreeMap<u16, RigidBody<MyUnits>>>,
    removal_events: std::collections::VecDeque<MyHandle>,
    next_celestial_object: u16,
    next_part: u16
}
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum MyHandle {
    CelestialObject(u16),
    FreePart(u16),
    PlayerPart(u16, u16)
}

impl World {
    fn add_celestial_object(&mut self, body: RigidBody<MyUnits>) -> MyHandle {
        let id = self.next_celestial_object;
        self.next_celestial_object += 1;
        let handle = MyHandle::CelestialObject(id);
        self.celestial_objects.insert(id, body);
        handle
    }
    fn add_free_part(&mut self, body: RigidBody<MyUnits>) -> MyHandle {
        let id = self.next_part;
        self.next_part += 1;
        let handle = MyHandle::FreePart(id);
        self.free_parts.insert(id, body);
        handle
    }
}
impl Default for World {
    fn default() -> World { World {
        celestial_objects: BTreeMap::new(),
        free_parts: BTreeMap::new(),
        player_parts: BTreeMap::new(),
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
            MyHandle::PlayerPart(player, id) => self.player_parts.get(&player).map(|player| player.get(&id)).flatten(),
            MyHandle::FreePart(id) => self.celestial_objects.get(&id),
        };
        if let Some(ptr) = ptr { Some(ptr) }
        else { None }
    }
    fn get_mut(&mut self, handle: Self::Handle) -> Option<&mut dyn nphysics2d::object::Body<MyUnits>> {
        let ptr = match handle {
            MyHandle::CelestialObject(id) => self.celestial_objects.get_mut(&id),
            MyHandle::PlayerPart(player, id) => self.player_parts.get_mut(&player).map(|player| player.get_mut(&id)).flatten(),
            MyHandle::FreePart(id) => self.celestial_objects.get_mut(&id),
        };
        if let Some(ptr) = ptr { Some(ptr) }
        else { None }
    }
    fn contains(&self, handle: Self::Handle) -> bool {
        match handle {
            MyHandle::CelestialObject(id) => self.celestial_objects.contains_key(&id),
            MyHandle::PlayerPart(player, id) => self.player_parts.get(&player).map(|player| player.contains_key(&id)).unwrap_or(false),
            MyHandle::FreePart(id) => self.celestial_objects.contains_key(&id),
        }
    }
    fn foreach(&self, f: &mut dyn FnMut(Self::Handle, &dyn nphysics2d::object::Body<MyUnits>)) {
        for (id, body) in &self.celestial_objects { f(MyHandle::CelestialObject(*id), body); }
        for (player, bodies) in &self.player_parts {
            for (id, body) in bodies { f(MyHandle::PlayerPart(*player, *id), body); }
        }
        for (id, body) in &self.free_parts { f(MyHandle::FreePart(*id), body); }
    }
    fn foreach_mut(&mut self, f: &mut dyn FnMut(Self::Handle, &mut dyn nphysics2d::object::Body<MyUnits>)) {
        for (id, body) in &mut self.celestial_objects { f(MyHandle::CelestialObject(*id), body); }
        for (player, bodies) in &mut self.player_parts {
            for (id, body) in bodies { f(MyHandle::PlayerPart(*player, *id), body); }
        }
        for (id, body) in &mut self.free_parts { f(MyHandle::FreePart(*id), body); }
    }
    fn pop_removal_event(&mut self) -> Option<Self::Handle> {
        self.removal_events.pop_front()
    }
}
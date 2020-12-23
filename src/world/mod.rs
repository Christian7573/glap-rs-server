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
            part_static: Default::default()
        };
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
                    part.apply_force(0, &Force2::linear(Vector2::new(if distance.0 >= 0.0 { magnitude } else { -magnitude }, distance.1 / distance.0.abs() * magnitude)), ForceType::Force, false);
                } else {
                    part.apply_force(0, &Force2::linear(Vector2::new(distance.0 / distance.1.abs() * magnitude, if distance.1 >= 0.0 { magnitude } else { -magnitude })), ForceType::Force, false);
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

    pub fn equip_part_constraint(&mut self, parent: u16, child: u16, attachment: parts::AttachmentPointDetails) -> (DefaultJointConstraintHandle, DefaultJointConstraintHandle) {
        let offset = (attachment.perpendicular.0 * 0.2, attachment.perpendicular.1 * 0.2);
        //println!("{} {}", attachment.x + offset.0, attachment.y + offset.1);
        let mut constraint1 = nphysics2d::joint::RevoluteConstraint::new(
            BodyPartHandle(MyHandle::Part(parent), 0),
            BodyPartHandle(MyHandle::Part(child), 0),
            Point::new(attachment.x + offset.0, attachment.y + offset.1),
            Point::new(0.2, 0.0)
        );
        //println!("{} {}", attachment.x - offset.0, attachment.y - offset.1);
        let mut constraint2 = nphysics2d::joint::RevoluteConstraint::new(
            BodyPartHandle(MyHandle::Part(parent), 0),
            BodyPartHandle(MyHandle::Part(child), 0),
            Point::new(attachment.x - offset.0, attachment.y - offset.1),
            Point::new(-0.2, 0.0)
        );
        const MAX_TORQUE: f32 = 100.0;
        const MAX_FORCE: f32 = MAX_TORQUE * 3.0;
        constraint1.set_break_torque(MAX_TORQUE);
        constraint1.set_break_force(MAX_FORCE);
        constraint2.set_break_torque(MAX_TORQUE);
        constraint2.set_break_force(MAX_FORCE);
        (self.joints.insert(constraint1), self.joints.insert(constraint2))
    }
    pub fn is_constraint_broken(&self, handle: DefaultJointConstraintHandle) -> bool {
        self.joints.get(handle).map(|joint| joint.is_broken()).unwrap_or(true)
    }

    pub fn geometrical_world(&self) -> &MyGeometricalWorld { &self.geometry }
}

pub struct World {
    celestial_objects: BTreeMap<u16, RigidBody<MyUnits>>,
    // free_parts: BTreeMap<u16, RigidBody<MyUnits>>,
    // player_parts: BTreeMap<u16, BTreeMap<u16, RigidBody<MyUnits>>>,
    parts: BTreeMap<u16, RigidBody<MyUnits>>,
    removal_events: std::collections::VecDeque<MyHandle>,
    next_celestial_object: u16,
    next_part: u16,
    reference_point_body: RigidBody<MyUnits>
}
#[derive(Copy, Eq, Debug)]
pub enum MyHandle {
    CelestialObject(u16),
    Part(u16),
    ReferencePoint
}
impl Clone for MyHandle {
    fn clone(&self) -> MyHandle { *self }
}
impl PartialEq for MyHandle {
    fn eq(&self, other: &Self) -> bool {
        match self {
            MyHandle::CelestialObject(id) => if let MyHandle::CelestialObject(other_id) = other { *id == *other_id } else { false },
            MyHandle::Part(id) => if let MyHandle::Part(other_id) = other { *id == *other_id } else { false },
            MyHandle::ReferencePoint => if let MyHandle::ReferencePoint = other { true } else { false }
        }
    }
}
impl std::hash::Hash for MyHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u16(match self {
            MyHandle::CelestialObject(id) => *id,
            MyHandle::Part(id) => *id,
            MyHandle::ReferencePoint => 1
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
    fn swap_part(&mut self, part_id: u16, with: RigidBody<MyUnits>) {
        self.removal_events.push_back(MyHandle::Part(part_id));
        self.parts.insert(part_id, with).expect("Attempted swap on a non-existant part");
    }
    pub fn get_rigid(&self, handle: MyHandle) -> Option<&RigidBody<MyUnits>> {
        match handle {
            MyHandle::CelestialObject(id) => self.celestial_objects.get(&id),
            MyHandle::Part(id) => self.parts.get(&id),
            MyHandle::ReferencePoint => Some(&self.reference_point_body)
        }
    }
    pub fn get_rigid_mut(&mut self, handle: MyHandle) -> Option<&mut RigidBody<MyUnits>> {
        match handle {
            MyHandle::CelestialObject(id) => self.celestial_objects.get_mut(&id),
            MyHandle::Part(id) => self.parts.get_mut(&id),
            MyHandle::ReferencePoint => Some(&mut self.reference_point_body)
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
        reference_point_body: nphysics2d::object::RigidBodyDesc::new().status(nphysics2d::object::BodyStatus::Static).build()
    } }
}
impl nphysics2d::object::BodySet<MyUnits> for World {
    type Handle = MyHandle;
    fn get(&self, handle: Self::Handle) -> Option<&dyn nphysics2d::object::Body<MyUnits>> {
        let ptr = match handle {
            MyHandle::CelestialObject(id) => self.celestial_objects.get(&id),
            MyHandle::Part(id) => self.parts.get(&id),
            MyHandle::ReferencePoint => Some(&self.reference_point_body),
        };
        if let Some(ptr) = ptr { Some(ptr) }
        else { None }
    }
    fn get_mut(&mut self, handle: Self::Handle) -> Option<&mut dyn nphysics2d::object::Body<MyUnits>> {
        let ptr = match handle {
            MyHandle::CelestialObject(id) => self.celestial_objects.get_mut(&id),
            MyHandle::Part(id) => self.parts.get_mut(&id),
            MyHandle::ReferencePoint => Some(&mut self.reference_point_body),
        };
        if let Some(ptr) = ptr { Some(ptr) }
        else { None }
    }
    fn contains(&self, handle: Self::Handle) -> bool {
        match handle {
            MyHandle::CelestialObject(id) => self.celestial_objects.contains_key(&id),
            MyHandle::Part(id) => self.parts.contains_key(&id),
            MyHandle::ReferencePoint => true,
        }
    }
    fn foreach(&self, f: &mut dyn FnMut(Self::Handle, &dyn nphysics2d::object::Body<MyUnits>)) {
        for (id, body) in &self.celestial_objects { f(MyHandle::CelestialObject(*id), body); }
        // for (player, bodies) in &self.player_parts {
        //     for (id, body) in bodies { f(MyHandle::Part(Some(*player), *id), body); }
        // }
        for (id, body) in &self.parts { f(MyHandle::Part(*id), body); }
        f(MyHandle::ReferencePoint, &self.reference_point_body);
    }
    fn foreach_mut(&mut self, f: &mut dyn FnMut(Self::Handle, &mut dyn nphysics2d::object::Body<MyUnits>)) {
        for (id, body) in &mut self.celestial_objects { f(MyHandle::CelestialObject(*id), body); }
        // for (player, bodies) in &mut self.player_parts {
        //     for (id, body) in bodies { f(MyHandle::Part(Some(*player), *id), body); }
        // }
        for (id, body) in &mut self.parts { f(MyHandle::Part(*id), body); }
        f(MyHandle::ReferencePoint, &mut self.reference_point_body);
    }
    fn pop_removal_event(&mut self) -> Option<Self::Handle> {
        self.removal_events.pop_front()
    }
}

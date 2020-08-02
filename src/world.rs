use nalgebra::Vector2;
use nphysics2d::object::RigidBody;
use std::collections::BTreeMap;
use nphysics2d::object::DefaultColliderSet;
use nphysics2d::joint::DefaultJointConstraintSet;
use nphysics2d::force_generator::DefaultForceGeneratorSet;

type MyUnits = f32;
type MyColliderHandle = nphysics2d::object::DefaultColliderHandle;
type MyMechanicalWorld = nphysics2d::world::MechanicalWorld<MyUnits, MyHandle, MyColliderHandle>;
type MyGeometricalWorld = nphysics2d::world::GeometricalWorld<MyUnits, MyHandle, MyColliderHandle>;

pub struct Simulation {
    bodies: World,
    mechanics: MyMechanicalWorld,
    geometry: MyGeometricalWorld,
    colliders: DefaultColliderSet<MyUnits>,
    joints: DefaultJointConstraintSet<MyUnits>,
    persistant_forces: DefaultForceGeneratorSet<MyUnits>
}
impl Simulation {
    pub fn new() -> Simulation {
        let mut simulation = Simulation {
            bodies: World::default(),
            mechanics: MyMechanicalWorld::new(Vector2::new(0.0, 0.0)),
            geometry: MyGeometricalWorld::new(),
            colliders: DefaultColliderSet::new(),
            joints: DefaultJointConstraintSet::new(),
            persistant_forces: DefaultForceGeneratorSet::new()
        };

        //Add planets n stuff here

        simulation
    }
}

pub struct World {
    celestial_objects: BTreeMap<u16, RigidBody<MyUnits>>,
    free_parts: BTreeMap<u16, RigidBody<MyUnits>>,
    player_parts: BTreeMap<u16, BTreeMap<u16, RigidBody<MyUnits>>>,
    removal_events: std::collections::VecDeque<MyHandle>
}
impl Default for World {
    fn default() -> World { World {
        celestial_objects: BTreeMap::new(),
        free_parts: BTreeMap::new(),
        player_parts: BTreeMap::new(),
        removal_events: std::collections::VecDeque::new()
    } }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum MyHandle {
    CelestialObject(u16),
    FreePart(u16),
    PlayerPart(u16, u16)
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


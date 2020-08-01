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
    system_bodies: BTreeMap<u16, RigidBody<MyUnits>>,
    player_bodies: BTreeMap<u16, BTreeMap<u16, RigidBody<MyUnits>>>,
    removal_events: std::collections::VecDeque<MyHandle>
}
impl Default for World {
    fn default() -> World { World {
        system_bodies: BTreeMap::new(),
        player_bodies: BTreeMap::new(),
        removal_events: std::collections::VecDeque::new()
    } }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum MyHandle {
    SystemBody(u16),
    PlayerBody(u16, u16)
}
impl nphysics2d::object::BodySet<MyUnits> for World {
    type Handle = MyHandle;
    fn get(&self, handle: Self::Handle) -> Option<&dyn nphysics2d::object::Body<MyUnits>> {
        let ptr = match handle {
            MyHandle::SystemBody(id) => self.system_bodies.get(&id),
            MyHandle::PlayerBody(player, id) => {
                self.player_bodies.get(&player).map(|player| player.get(&id)).flatten()
            }
        };
        if let Some(ptr) = ptr { Some(ptr) }
        else { None }
    }
    fn get_mut(&mut self, handle: Self::Handle) -> Option<&mut dyn nphysics2d::object::Body<MyUnits>> {
        let ptr = match handle {
            MyHandle::SystemBody(id) => self.system_bodies.get_mut(&id),
            MyHandle::PlayerBody(player, id) => {
                self.player_bodies.get_mut(&player).map(|player| player.get_mut(&id)).flatten()
            }
        };
        if let Some(ptr) = ptr { Some(ptr) }
        else { None }
    }
    fn contains(&self, handle: Self::Handle) -> bool {
        match handle {
            MyHandle::SystemBody(id) => self.system_bodies.contains_key(&id),
            MyHandle::PlayerBody(player, id) => self.player_bodies.get(&player).map(|player| player.contains_key(&id)).unwrap_or(false)
        }
    }
    fn foreach(&self, f: &mut dyn FnMut(Self::Handle, &dyn nphysics2d::object::Body<MyUnits>)) {
        for (id, body) in &self.system_bodies {
            f(MyHandle::SystemBody(*id), body);
        }
        for (player, bodies) in &self.player_bodies {
            for (id, body) in bodies {
                f(MyHandle::PlayerBody(*player, *id), body);
            }
        }
    }
    fn foreach_mut(&mut self, f: &mut dyn FnMut(Self::Handle, &mut dyn nphysics2d::object::Body<MyUnits>)) {
        for (id, body) in &mut self.system_bodies {
            f(MyHandle::SystemBody(*id), body);
        }
        for (player, bodies) in &mut self.player_bodies {
            for (id, body) in bodies {
                f(MyHandle::PlayerBody(*player, *id), body);
            }
        }
    }
    fn pop_removal_event(&mut self) -> Option<Self::Handle> {
        self.removal_events.pop_front()
    }
}


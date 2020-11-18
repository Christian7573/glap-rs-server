use super::{MyUnits, MyHandle};
use nphysics2d::object::{RigidBody, RigidBodyDesc, BodyStatus, BodyPartHandle};
use nphysics2d::object::ColliderDesc;
use ncollide2d::shape::{Ball, ShapeHandle};
use nalgebra::Vector2;
use nphysics2d::material::{BasicMaterial, MaterialHandle};
use rand::Rng;

pub struct Planets {
    pub earth: CelestialObject,
    pub moon: CelestialObject,
    pub planet_material: MaterialHandle<MyUnits>,
    pub mars: CelestialObject,
    pub mercury: CelestialObject,
    pub jupiter: CelestialObject,
    pub pluto: CelestialObject,
    pub saturn: CelestialObject,
    pub neptune: CelestialObject,
    pub venus: CelestialObject,
    pub uranus: CelestialObject,
    pub sun: CelestialObject
}
impl Planets {
    pub fn new(colliders: &mut super::MyColliderSet, bodies: &mut super::World) -> Planets {
        const EARTH_MASS: f32 = 600.0;
        const EARTH_SIZE: f32 = 25.0;
        let earth_pos = Vector2::new(planet_location(500.0), planet_location(500.0));
        let planet_material = MaterialHandle::new(BasicMaterial::new(0.0, 1.0));
        let earth = {
            let body = RigidBodyDesc::new()
            .translation(earth_pos.clone())
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_MASS)
                .build();
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = EARTH_SIZE;
            let shape = ShapeHandle::new(Ball::new(RADIUS));
            let collider = ColliderDesc::new(shape)
                .material(planet_material.clone())
                .build(BodyPartHandle(body_handle, 0));
            colliders.insert(collider);

            let id = if let MyHandle::CelestialObject(id) = body_handle { id } else { panic!() };
            
            CelestialObject {
                name: String::from("earth"),
                display_name: String::from("Earth"),
                radius: RADIUS,
                body: body_handle,
                id,
                cargo_upgrade: None,
                can_beamout: true,
            }
        };

        let moon = {
            let body = RigidBodyDesc::new()
            .translation(Vector2::new(planet_location(100.0), planet_location(100.0)) + earth_pos.clone())
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_MASS / 35.0)
                .build();
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = EARTH_SIZE / 4.0;
            let shape = ShapeHandle::new(Ball::new(RADIUS));
            let collider = ColliderDesc::new(shape)
                .material(planet_material.clone())
                .build(BodyPartHandle(body_handle, 0));
            colliders.insert(collider);

            let id = if let MyHandle::CelestialObject(id) = body_handle { id } else { panic!() };
            
            CelestialObject {
                name: String::from("moon"),
                display_name: String::from("Moon"),
                radius: RADIUS,
                body: body_handle,
                id,
                cargo_upgrade: Some(super::parts::PartKind::LandingThruster),
                can_beamout: true,
            }
        };

        let mars = {
            let body = RigidBodyDesc::new()
                .translation(Vector2::new(1500.0,-1000.0))
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_MASS / 4.0)
                .build();
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = EARTH_SIZE / 2.0;
            let shape = ShapeHandle::new(Ball::new(RADIUS));
            let collider = ColliderDesc::new(shape)
                .material(planet_material.clone())
                .build(BodyPartHandle(body_handle, 0));
            colliders.insert(collider);

            let id = if let MyHandle::CelestialObject(id) = body_handle { id } else { panic!() };
            
            CelestialObject {
                name: String::from("mars"),
                display_name: String::from("Mars"),
                radius: RADIUS,
                body: body_handle,
                id,
                cargo_upgrade: Some(super::parts::PartKind::Hub),
                can_beamout: false,
            }
        };

        let mercury = {
            let body = RigidBodyDesc::new()
                .translation(Vector2::new(-1600.0,267.0))
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_MASS / 25.0)
                .build();
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = EARTH_SIZE * 0.38;
            let shape = ShapeHandle::new(Ball::new(RADIUS));
            let collider = ColliderDesc::new(shape)
                .material(planet_material.clone())
                .build(BodyPartHandle(body_handle, 0));
            colliders.insert(collider);

            let id = if let MyHandle::CelestialObject(id) = body_handle { id } else { panic!() };
            
            CelestialObject {
                name: String::from("mercury"),
                display_name: String::from("Mercury"),
                radius: RADIUS,
                body: body_handle,
                id,
                cargo_upgrade: Some(super::parts::PartKind::SolarPanel),
                can_beamout: false,
            }
        };

        let jupiter = {
            let body = RigidBodyDesc::new()
                .translation(Vector2::new(2500.0, 1000.0))
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_MASS * 10.0)
                .build();
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = EARTH_SIZE * 2.0;
            let shape = ShapeHandle::new(Ball::new(RADIUS));
            let collider = ColliderDesc::new(shape)
                .material(planet_material.clone())
                .build(BodyPartHandle(body_handle, 0));
            colliders.insert(collider);

            let id = if let MyHandle::CelestialObject(id) = body_handle { id } else { panic!() };

            CelestialObject {
                name: String::from("jupiter"),
                display_name: String::from("jupiter"),
                radius: RADIUS,
                body: body_handle,
                id,
                cargo_upgrade: Some(super::parts::PartKind::Hub),
                can_beamout: false,
            }
        };

        let pluto = {
            let body = RigidBodyDesc::new()
                .translation(Vector2::new(-2000.0, -2000.0))
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_SIZE / 35.0)
                .build();
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = EARTH_SIZE / 4.0;
            let shape = ShapeHandle::new(Ball::new(RADIUS));
            let collider = ColliderDesc::new(shape)
                .material(planet_material.clone())
                .build(BodyPartHandle(body_handle, 0));
            colliders.insert(collider);

            let id = if let MyHandle::CelestialObject(id) = body_handle { id } else { panic!() };

            CelestialObject {
                name: String::from("pluto"),
                display_name: String::from("pluto"),
                radius: RADIUS,
                body: body_handle,
                id,
                cargo_upgrade: Some(super::parts::PartKind::Hub),
                can_beamout: false,
            }
        };

        let saturn = {
            let body = RigidBodyDesc::new()
                .translation(Vector2::new(-100.0, -1305.0))
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_SIZE / 35.0)
                .build();
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = EARTH_SIZE / 4.0;
            let shape = ShapeHandle::new(Ball::new(RADIUS));
            let collider = ColliderDesc::new(shape)
                .material(planet_material.clone())
                .build(BodyPartHandle(body_handle, 0));
            colliders.insert(collider);

            let id = if let MyHandle::CelestialObject(id) = body_handle { id } else { panic!() };

            CelestialObject {
                name: String::from("saturn"),
                display_name: String::from("saturn"),
                radius: RADIUS,
                body: body_handle,
                id,
                cargo_upgrade: Some(super::parts::PartKind::Hub),
                can_beamout: false,
            }
        };

        let neptune = {
            let body = RigidBodyDesc::new()
                .translation(Vector2::new(-1700.0, 2000.0))
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_SIZE / 35.0)
                .build();
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = EARTH_SIZE / 4.0;
            let shape = ShapeHandle::new(Ball::new(RADIUS));
            let collider = ColliderDesc::new(shape)
                .material(planet_material.clone())
                .build(BodyPartHandle(body_handle, 0));
            colliders.insert(collider);

            let id = if let MyHandle::CelestialObject(id) = body_handle { id } else { panic!() };

            CelestialObject {
                name: String::from("neptune"),
                display_name: String::from("neptune"),
                radius: RADIUS,
                body: body_handle,
                id,
                cargo_upgrade: Some(super::parts::PartKind::Hub),
                can_beamout: false,
            }
        };

        let venus = {
            let body = RigidBodyDesc::new()
                .translation(Vector2::new(1700.0, -300.0))
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_SIZE / 35.0)
                .build();
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = EARTH_SIZE / 4.0;
            let shape = ShapeHandle::new(Ball::new(RADIUS));
            let collider = ColliderDesc::new(shape)
                .material(planet_material.clone())
                .build(BodyPartHandle(body_handle, 0));
            colliders.insert(collider);

            let id = if let MyHandle::CelestialObject(id) = body_handle { id } else { panic!() };

            CelestialObject {
                name: String::from("venus"),
                display_name: String::from("venus"),
                radius: RADIUS,
                body: body_handle,
                id,
                cargo_upgrade: Some(super::parts::PartKind::Hub),
                can_beamout: false,
            }
        };

        let uranus = {
            let body = RigidBodyDesc::new()
                .translation(Vector2::new(500.0, 1500.0))
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_SIZE / 35.0)
                .build();
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = EARTH_SIZE / 4.0;
            let shape = ShapeHandle::new(Ball::new(RADIUS));
            let collider = ColliderDesc::new(shape)
                .material(planet_material.clone())
                .build(BodyPartHandle(body_handle, 0));
            colliders.insert(collider);

            let id = if let MyHandle::CelestialObject(id) = body_handle { id } else { panic!() };

            CelestialObject {
                name: String::from("uranus"),
                display_name: String::from("uranus"),
                radius: RADIUS,
                body: body_handle,
                id,
                cargo_upgrade: Some(super::parts::PartKind::Hub),
                can_beamout: false,
            }
        };

        let sun = {
            let body = RigidBodyDesc::new()
                .translation(Vector2::new(0.0,0.0))
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_MASS * 20.0)
                .build();
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = EARTH_SIZE * 7.0;
            let shape = ShapeHandle::new(Ball::new(RADIUS));
            let collider = ColliderDesc::new(shape)
                .material(planet_material.clone())
                .build(BodyPartHandle(body_handle, 0));
            colliders.insert(collider);

            let id = if let MyHandle::CelestialObject(id) = body_handle { id } else { panic!() };
            
            CelestialObject {
                name: String::from("sun"),
                display_name: String::from("sun"),
                radius: RADIUS,
                body: body_handle,
                id,
                cargo_upgrade: None,
                can_beamout: false,
            }
        };

        Planets {
            earth, moon, planet_material, mars, mercury, jupiter, pluto, saturn, neptune, venus, uranus, sun,
        }
    }

    pub fn celestial_objects<'a>(&'a self) -> [&'a CelestialObject; 11] {
        [&self.earth, &self.moon, &self.mars, &self.mercury, &self.jupiter, &self.pluto, &self.saturn, &self.neptune, &self.venus, &self.uranus, &self.sun]
    }
    pub fn get_celestial_object<'a>(&'a self, id: u16) -> Result<&'a CelestialObject, ()> {
        if id == self.earth.id { Ok(&self.earth) }
        else if id == self.moon.id { Ok(&self.moon) }
        else if id == self.mars.id { Ok(&self.mars) }
        else if id == self.mercury.id { Ok(&self.mercury) }
        else if id == self.jupiter.id { Ok(&self.jupiter) }
        else if id == self.pluto.id { Ok(&self.pluto) }
        else if id == self.saturn.id { Ok(&self.saturn) }
        else if id == self.neptune.id { Ok(&self.neptune) }
        else if id == self.venus.id { Ok(&self.venus) }
        else if id == self.uranus.id { Ok(&self.uranus) }
        else if id == self.sun.id { Ok(&self.sun) }
        else { Err(()) }
    }
}

pub struct CelestialObject {
    pub name: String,
    pub display_name: String,
    pub radius: f32,
    pub body: MyHandle,
    pub id: u16,
    pub cargo_upgrade: Option<super::parts::PartKind>,
    pub can_beamout: bool,
}

pub fn planet_location(radius: f32) -> f32 {
    let mut rng = rand::thread_rng();
    let angle = rng.gen_range(0.0, 360.0);
    let x = (radius * f32::cos(angle * std::f32::consts::PI / 180.0));
    x
}
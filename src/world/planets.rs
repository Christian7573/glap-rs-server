use super::{MyUnits, MyHandle};
use nphysics2d::object::{RigidBody, RigidBodyDesc, BodyStatus, BodyPartHandle};
use nphysics2d::object::ColliderDesc;
use ncollide2d::shape::{Ball, ShapeHandle};
use nalgebra::Vector2;
use nphysics2d::material::{BasicMaterial, MaterialHandle};

pub struct Planets {
    pub earth: CelestialObject,
    pub moon: CelestialObject,
    pub planet_material: MaterialHandle<MyUnits>,
    pub mars: CelestialObject,
}
impl Planets {
    pub fn new(colliders: &mut super::MyColliderSet, bodies: &mut super::World) -> Planets {
        const EARTH_MASS: f32 = 650.0;
        let planet_material = MaterialHandle::new(BasicMaterial::new(0.0, 1.0));
        let earth = {
            let body = RigidBodyDesc::new()
                .translation(Vector2::new(0.0,0.0))
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_MASS)
                .build();
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = 25.0;
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
            }
        };

        let moon = {
            let body = RigidBodyDesc::new()
                .translation(Vector2::new(42.4530083832,69.2770133538))
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_MASS / 41.0)
                .build();
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = 25.0 / 4.0;
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
                cargo_upgrade: Some(super::parts::PartKind::LandingThruster)
            }
        };

        let mars = {
            let body = RigidBodyDesc::new()
                .translation(Vector2::new(0.0,-1000.0))
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_MASS / 2.0)
                .build();
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = 25.0 / 2.0;
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
            }
        };

        Planets {
            earth, moon, planet_material, mars,
        }
    }

    pub fn celestial_objects<'a>(&'a self) -> [&'a CelestialObject; 3] {
        [&self.earth, &self.moon, &self.mars]
    }
    pub fn get_celestial_object<'a>(&'a self, id: u16) -> Result<&'a CelestialObject, ()> {
        if id == self.earth.id { Ok(&self.earth) }
        else if id == self.moon.id { Ok(&self.moon) }
        else if id == self.mars.id { Ok(&self.mars) }
        else { Err(()) }
    }
}

pub struct CelestialObject {
    pub name: String,
    pub display_name: String,
    pub radius: f32,
    pub body: MyHandle,
    pub id: u16,
    pub cargo_upgrade: Option<super::parts::PartKind>
}

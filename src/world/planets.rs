use super::MyHandle;
use nphysics2d::object::{RigidBody, RigidBodyDesc, BodyStatus, BodyPartHandle};
use nphysics2d::object::ColliderDesc;
use ncollide2d::shape::{Ball, ShapeHandle};
use nalgebra::Vector2;

pub struct Planets {
    earth: CelestialObject
}
impl Planets {
    pub fn new(colliders: &mut super::MyColliderSet, bodies: &mut super::World) -> Planets {
        let earth = {
            let body = RigidBodyDesc::new()
                .translation(Vector2::new(0.0,0.0))
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(100.0)
                .build();
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = 50.0;
            let shape = ShapeHandle::new(Ball::new(RADIUS));
            let collider = ColliderDesc::new(shape)
                //.margin()
                .build(BodyPartHandle(body_handle, 0));
            colliders.insert(collider);
            
            CelestialObject {
                name: String::from("earth"),
                display_name: String::from("Earth"),
                radius: RADIUS,
                body: body_handle
            }
        };

        Planets {
            earth
        }
    }
}

pub struct CelestialObject {
    name: String,
    display_name: String,
    radius: f32,
    body: MyHandle
}
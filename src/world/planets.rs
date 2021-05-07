use super::{MyUnits, PartHandle};
use std::collections::BTreeMap;
use rapier2d::dynamics::{RigidBody, RigidBodyBuilder, BodyStatus, RigidBodyHandle, RigidBodySet};
use rapier2d::geometry::{ColliderBuilder, SharedShape, Collider, ColliderSet, ColliderHandle};
use super::typedef::*;
use crate::codec::PlanetKind;
use rand::Rng;
use super::parts::PartKind;

pub struct Planets {
    planets: BTreeMap<u8, CelestialObject>
}
impl Planets {
    pub fn new(world: &mut super::World, colliders: &mut ColliderSet) -> Planets {
        const EARTH_MASS: f32 = 600.0;
        const EARTH_SIZE: f32 = 25.0;
        let bodies = &mut world.bodies;

        let mut planets = BTreeMap::new();

        let sun_id = make_planet_id();
        let earth_id = make_planet_id();
        let sun = {
            let id = sun_id;
            let mass = EARTH_MASS * 50.0;
            let body = RigidBodyBuilder::new_static()
                .gravity_enabled(false)
                .mass(EARTH_MASS * 50.0)
				.user_data(AmPlanet {id})
                .build();
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = EARTH_SIZE * 4.7;
            let collider = ColliderBuilder::new(SharedShape::ball(RADIUS)).build();
            colliders.insert(collider, body_handle, bodies);

            CelestialObject {
                kind: PlanetKind::Sun,
                orbit: None,
                radius: RADIUS,
                mass,
                cargo_upgrade: None,
                can_beamout: false,
                body_handle,
            }
        };

        let earth = {
			let id = earth_id;
            let mass = EARTH_MASS;
            let body = RigidBodyBuilder::new_kinematic()
                .gravity_enabled(false)
                .mass(mass)
				.user_data(AmPlanet {id})
                .build();
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = EARTH_SIZE;
            let collider = ColliderBuilder::new(SharedShape::ball(RADIUS)).build();
            colliders.insert(collider, body_handle, bodies);
            
            CelestialObject {
                kind: PlanetKind::Earth,
                orbit: Some(Orbit {
                    orbit_around: sun_id,
                    radius: (1500.0, 1500.0),
                    rotation: 0,
                }),
                radius: RADIUS,
                mass,
                cargo_upgrade: None,
                can_beamout: true,
                body_handle,
            }
        };

        let moon = {
            let id = make_planet_id();
            let mass = EARTH_MASS / 35.0;
            let body = RigidBodyBuilder::new_dynamic()
                .gravity_enabled(false)
                .mass(mass)
				.user_data(AmPlanet {id})
                .build();
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = EARTH_SIZE / 4.0;
            let collider = ColliderBuilder::new(SharedShape::ball(RADIUS));
            colliders.insert(collider, body_handle, bodies);

            CelestialObject {
                kind: PlanetKind::Moon,
                orbit: Some(Orbit {
                    orbit_around: earth_id,
                    radius: (100.0, 100.0),
                    rotation: 0.0,
                }),
                radius: RADIUS,
                mass,
                cargo_upgrade: Some(super::parts::PartKind::LandingThruster),
                can_beamout: true,
                body_handle,
            }
        };

        let mars = {
			let id = make_planet_id();
            let mass = EARTH_MASS / 4.0;
            let body = RigidBodyBuilder::new_dynamic()
                .gravity_enabled(false)
                .mass(mass)
				.user_data(AmPlanet {id})
                .build();
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = EARTH_SIZE / 2.0;
            let collider = ColliderBuilder::new(SharedShape::ball(RADIUS)).build();
            colliders.insert(collider, body_handle, bodies);
            
            CelestialObject {
                kind: PlanetKind::Mars,
                orbit: Some(Orbit {
                    orbit_around: sun_id,
                    radius: (2000.0, 2000.0),
                    rotation: 0.0,
                }),
                radius: RADIUS,
                cargo_upgrade: Some(super::parts::PartKind::Hub),
                can_beamout: false,
                mass,
                body_handle,
            }
        };

        let mercury = {
            let body = RigidBodyDesc::new()
                .translation(planet_location(500.0))
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_MASS / 15.0)
                .build();
            let position = (body.position().translation.x, body.position().translation.y);
            let mass = body.augmented_mass().linear;
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = EARTH_SIZE * 0.38;
            let shape = ShapeHandle::new(Ball::new(RADIUS));
			let id = make_planet_id();
            let collider = ColliderDesc::new(shape)
                .material(planet_material.clone())
				.user_data(AmPlanet {id})
                .build(BodyPartHandle(body_handle, 0));
            colliders.insert(collider);

            
            
            CelestialObject {
                name: String::from("mercury"),
                display_name: String::from("Mercury"),
                radius: RADIUS,
                body: body_handle,
                id,
                cargo_upgrade: Some(super::parts::PartKind::SolarPanel),
                can_beamout: false,
                position,
                mass,
            }
        };

        let jupiter = {
            let body = RigidBodyDesc::new()
                .translation(planet_location(3500.0))
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_MASS * 10.0)
                .build();
            let position = (body.position().translation.x, body.position().translation.y);
            let mass = body.augmented_mass().linear;
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = EARTH_SIZE * 2.0;
            let shape = ShapeHandle::new(Ball::new(RADIUS));
			let id = make_planet_id();
            let collider = ColliderDesc::new(shape)
                .material(planet_material.clone())
				.user_data(AmPlanet {id})
                .build(BodyPartHandle(body_handle, 0));
            colliders.insert(collider);

            

            CelestialObject {
                name: String::from("jupiter"),
                display_name: String::from("jupiter"),
                radius: RADIUS,
                body: body_handle,
                id,
                cargo_upgrade: Some(super::parts::PartKind::Thruster),
                can_beamout: false,
                position,
                mass,
            }
        };

        let pluto = {
            let body = RigidBodyDesc::new()
                .translation(planet_location(6000.0))
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_MASS / 10.0)
                .build();
            let position = (body.position().translation.x, body.position().translation.y);
            let mass = body.augmented_mass().linear;
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = EARTH_SIZE / 4.0;
            let shape = ShapeHandle::new(Ball::new(RADIUS));
			let id = make_planet_id();
            let collider = ColliderDesc::new(shape)
                .material(planet_material.clone())
				.user_data(AmPlanet {id})
                .build(BodyPartHandle(body_handle, 0));
            colliders.insert(collider);

            

            CelestialObject {
                name: String::from("pluto"),
                display_name: String::from("pluto"),
                radius: RADIUS,
                body: body_handle,
                id,
                cargo_upgrade: Some(PartKind::LandingWheel),
                can_beamout: false,
                position,
                mass,
            }
        };

        let saturn = {
            let body = RigidBodyDesc::new()
                .translation(planet_location(4000.0))
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_MASS * 10.0)
                .build();
            let position = (body.position().translation.x, body.position().translation.y);
            let mass = body.augmented_mass().linear;
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = EARTH_SIZE * 2.0;
            let shape = ShapeHandle::new(Ball::new(RADIUS));
			let id = make_planet_id();
            let collider = ColliderDesc::new(shape)
                .material(planet_material.clone())
				.user_data(AmPlanet {id})
                .build(BodyPartHandle(body_handle, 0));
            colliders.insert(collider);

            

            CelestialObject {
                name: String::from("saturn"),
                display_name: String::from("saturn"),
                radius: RADIUS,
                body: body_handle,
                id,
                cargo_upgrade: Some(PartKind::SuperThruster),
                can_beamout: false,
                position,
                mass,
            }
        };

        let neptune = {
            let body = RigidBodyDesc::new()
                .translation(planet_location(5500.0))
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_MASS * 4.0)
                .build();
            let position = (body.position().translation.x, body.position().translation.y);
            let mass = body.augmented_mass().linear;
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = EARTH_SIZE * 1.5;
            let shape = ShapeHandle::new(Ball::new(RADIUS));
			let id = make_planet_id();
            let collider = ColliderDesc::new(shape)
                .material(planet_material.clone())
				.user_data(AmPlanet {id})
                .build(BodyPartHandle(body_handle, 0));
            colliders.insert(collider);

            

            CelestialObject {
                name: String::from("neptune"),
                display_name: String::from("neptune"),
                radius: RADIUS,
                body: body_handle,
                id,
                cargo_upgrade: Some(PartKind::HubThruster),
                can_beamout: false,
                position,
                mass,
            }
        };

        let venus = {
            let body = RigidBodyDesc::new()
                .translation(planet_location(1000.0))
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_MASS * 1.3)
                .build();
            let position = (body.position().translation.x, body.position().translation.y);
            let mass = body.augmented_mass().linear;
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = EARTH_SIZE;
            let shape = ShapeHandle::new(Ball::new(RADIUS));
			let id = make_planet_id();
            let collider = ColliderDesc::new(shape)
                .material(planet_material.clone())
				.user_data(AmPlanet {id})
                .build(BodyPartHandle(body_handle, 0));
            colliders.insert(collider);

            

            CelestialObject {
                name: String::from("venus"),
                display_name: String::from("venus"),
                radius: RADIUS,
                body: body_handle,
                id,
                cargo_upgrade: Some(PartKind::EcoThruster),
                can_beamout: false,
                position,
                mass,
            }
        };

        let uranus = {
            let body = RigidBodyDesc::new()
                .translation(planet_location(4800.0))
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_MASS * 4.0)
                .build();
            let position = (body.position().translation.x, body.position().translation.y);
            let mass = body.augmented_mass().linear;
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = EARTH_SIZE * 2.0;
            let shape = ShapeHandle::new(Ball::new(RADIUS));
			let id = make_planet_id();
            let collider = ColliderDesc::new(shape)
                .material(planet_material.clone())
				.user_data(AmPlanet {id})
                .build(BodyPartHandle(body_handle, 0));
            colliders.insert(collider);

            

            CelestialObject {
                name: String::from("uranus"),
                display_name: String::from("uranus"),
                radius: RADIUS,
                body: body_handle,
                id,
                cargo_upgrade: Some(PartKind::PowerHub),
                can_beamout: false,
                position,
                mass,
            }
        };


        let trade = {
            let body = RigidBodyDesc::new()
                .translation(Vector2::new(earth_pos.x / earth_pos.magnitude() * -2500.0, earth_pos.y / earth_pos.magnitude() * -2500.0))
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_MASS)
                .build();
            let position = (body.position().translation.x, body.position().translation.y);
            let mass = body.augmented_mass().linear;
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = EARTH_SIZE * 0.75;
            let shape = ShapeHandle::new(Ball::new(RADIUS));
			let id = make_planet_id();
            let collider = ColliderDesc::new(shape)
                .material(planet_material.clone())
				.user_data(AmPlanet {id})
                .build(BodyPartHandle(body_handle, 0));
            colliders.insert(collider);

            

            CelestialObject {
                name: String::from("trade"),
                display_name: String::from("Trade Planet"),
                radius: RADIUS,
                body: body_handle,
                id,
                cargo_upgrade: None,
                can_beamout: true,
                position,
                mass,
            }
        };

        Planets {
            earth, moon, planet_material, mars, mercury, jupiter, /* pluto, */ saturn, neptune, venus, uranus, sun, /* trade, */
        }
    }

    pub fn celestial_objects<'a>(&'a self) -> [&'a CelestialObject; 10] {
        [&self.earth, &self.moon, &self.mars, &self.mercury, &self.jupiter, /* &self.pluto, */ &self.saturn, &self.neptune, &self.venus, &self.uranus, &self.sun, /* &self.trade */]
    }
    pub fn get_celestial_object<'a>(&'a self, id: u16) -> Result<&'a CelestialObject, ()> {
        if id == self.earth.id { Ok(&self.earth) }
        else if id == self.moon.id { Ok(&self.moon) }
        else if id == self.mars.id { Ok(&self.mars) }
        else if id == self.mercury.id { Ok(&self.mercury) }
        else if id == self.jupiter.id { Ok(&self.jupiter) }
        //else if id == self.pluto.id { Ok(&self.pluto) }
        else if id == self.saturn.id { Ok(&self.saturn) }
        else if id == self.neptune.id { Ok(&self.neptune) }
        else if id == self.venus.id { Ok(&self.venus) }
        else if id == self.uranus.id { Ok(&self.uranus) }
        else if id == self.sun.id { Ok(&self.sun) }
        //else if id == self.trade.id { Ok(&self.trade) }
        else { Err(()) }
    }
}

static mut NEXT_PLANET_ID: u16 = 1;
fn make_planet_id() -> u16 {
    unsafe { let id = NEXT_PLANET_ID; NEXT_PLANET_ID += 1; id }
}

pub struct CelestialObject {
    pub kind: PlanetKind,
    pub orbit: Option<Orbit>,
    pub radius: f32,
    pub mass: f32,
    pub cargo_upgrade: Option<super::parts::PartKind>,
    pub can_beamout: bool,
    pub body_handle: RigidBodyHandle,
}
pub struct Orbit {
    orbit_around: u8,
    radius: (f32, f32),
    rotation: f32,
}

#[derive(Copy, Clone)]
pub struct AmPlanet {
    pub id: u16
}
//use nphysics2d::utils::UserData;
/*impl UserData for AmPlanet {
    fn clone_boxed(&self) -> Box<dyn UserData> { Box::new(*self) }
    fn to_any(&self) -> Box<dyn std::any::Any + Send + Sync> { Box::new(*self) }
    fn as_any(&self) -> &dyn std::any::Any { self }    
}*/

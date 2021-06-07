use super::{MyUnits, PartHandle};
use std::collections::BTreeMap;
use rapier2d::dynamics::{RigidBody, RigidBodyBuilder, BodyStatus, RigidBodyHandle, RigidBodySet};
use rapier2d::geometry::{ColliderBuilder, SharedShape, Collider, ColliderSet, ColliderHandle};
use super::typedef::*;
use crate::codec::PlanetKind;
use crate::storage7573::Storage7573;
use rand::Rng;
use super::parts::PartKind;

use Storage7573::Planet;

pub struct Planets {
    pub planets: BTreeMap<u8, CelestialObject>,
    pub earth_id: u8,
    pub trade_id: u8,
    pub sun_id: u8,
}
impl Planets {
    pub fn new(bodies: &mut RigidBodySet, colliders: &mut ColliderSet) -> Planets {
        const EARTH_MASS: f32 = 600.0;
        const EARTH_SIZE: f32 = 25.0;

        let mut planets = BTreeMap::new();

        let sun_id = make_planet_id();
        let earth_id = make_planet_id();
        let sun = {
            let id = sun_id;
            let mass = EARTH_MASS * 50.0;
            let body = RigidBodyBuilder::new_static()
                .additional_mass(EARTH_MASS * 50.0)
				.user_data(Planet(id).into())
                .build();
            let body_handle = bodies.insert(body);
            const RADIUS: f32 = EARTH_SIZE * 4.7;
            let collider = ColliderBuilder::new(SharedShape::ball(RADIUS)).build();
            colliders.insert(collider, body_handle, bodies);

            planets.insert(id, CelestialObject {
                kind: PlanetKind::Sun,
                orbit: None,
                radius: RADIUS,
                mass,
                cargo_upgrade: None,
                can_beamout: false,
                body_handle,
                position: (0.0, 0.0),
            });
        };

        let earth = {
			let id = earth_id;
            let mass = EARTH_MASS;
            let body = RigidBodyBuilder::new_kinematic()
                .additional_mass(mass)
				.user_data(Planet(id).into())
                .build();
            let body_handle = bodies.insert(body);
            const RADIUS: f32 = EARTH_SIZE;
            let collider = ColliderBuilder::new(SharedShape::ball(RADIUS)).build();
            colliders.insert(collider, body_handle, bodies);
            
            planets.insert(id, CelestialObject {
                kind: PlanetKind::Earth,
                orbit: Some(Orbit {
                    orbit_around: sun_id,
                    radius: (1500.0, 1500.0),
                    rotation: 0.0,
                    total_ticks: 3600,
                    ticks_ellapsed: 0,
                    last_next_position: (0.0, 0.0),
                }),
                radius: RADIUS,
                mass,
                cargo_upgrade: None,
                can_beamout: true,
                body_handle,
                position: (0.0, 0.0),
            });
        };

        let moon = {
            let id = make_planet_id();
            let mass = EARTH_MASS / 35.0;
            let body = RigidBodyBuilder::new_dynamic()
                .additional_mass(mass)
				.user_data(Planet(id).into())
                .build();
            let body_handle = bodies.insert(body);
            const RADIUS: f32 = EARTH_SIZE / 4.0;
            let collider = ColliderBuilder::new(SharedShape::ball(RADIUS)).build();
            colliders.insert(collider, body_handle, bodies);

            planets.insert(id, CelestialObject {
                kind: PlanetKind::Moon,
                orbit: Some(Orbit {
                    orbit_around: earth_id,
                    radius: (100.0, 100.0),
                    rotation: 0.0,
                    total_ticks: 600,
                    ticks_ellapsed: 0,
                    last_next_position: (0.0, 0.0),
                }),
                radius: RADIUS,
                mass,
                cargo_upgrade: Some(super::parts::PartKind::LandingThruster),
                can_beamout: true,
                body_handle,
                position: (0.0, 0.0),
            });
        };

        let mars = {
			let id = make_planet_id();
            let mass = EARTH_MASS / 4.0;
            let body = RigidBodyBuilder::new_dynamic()
                .additional_mass(mass)
				.user_data(Planet(id).into())
                .build();
            let body_handle = bodies.insert(body);
            const RADIUS: f32 = EARTH_SIZE / 2.0;
            let collider = ColliderBuilder::new(SharedShape::ball(RADIUS)).build();
            colliders.insert(collider, body_handle, bodies);
            
            planets.insert(id, CelestialObject {
                kind: PlanetKind::Mars,
                orbit: Some(Orbit {
                    orbit_around: sun_id,
                    radius: (2000.0, 2000.0),
                    rotation: 0.0,
                    total_ticks: 4800,
                    ticks_ellapsed: 0,
                    last_next_position: (0.0, 0.0),
                }),
                radius: RADIUS,
                cargo_upgrade: Some(super::parts::PartKind::Hub),
                can_beamout: false,
                mass,
                body_handle,
                position: (0.0, 0.0),
            });
        };

        let mercury = {
			let id = make_planet_id();
            let mass = EARTH_MASS / 15.0;
            let body = RigidBodyBuilder::new_kinematic()
                .additional_mass(mass)
				.user_data(Planet(id).into())
                .build();
            let body_handle = bodies.insert(body);
            const RADIUS: f32 = EARTH_SIZE * 0.38;
            let collider = ColliderBuilder::new(SharedShape::ball(RADIUS)).build();
            colliders.insert(collider, body_handle, bodies);
            
            planets.insert(id, CelestialObject {
                kind: PlanetKind::Mercury,
                orbit: Some(Orbit {
                    orbit_around: sun_id,
                    radius: (500.0, 500.0),
                    rotation: 0.0,
                    total_ticks: 1200,
                    ticks_ellapsed: 0,
                    last_next_position: (0.0, 0.0),
                }),
                radius: RADIUS,
                mass,
                cargo_upgrade: Some(super::parts::PartKind::SolarPanel),
                can_beamout: false,
                body_handle,
                position: (0.0, 0.0),
            });
        };

        let jupiter = {
			let id = make_planet_id();
            let mass = EARTH_MASS * 10.0;
            let body = RigidBodyBuilder::new_kinematic()
                .additional_mass(mass)
				.user_data(Planet(id).into())
                .build();
            let body_handle = bodies.insert(body);
            const RADIUS: f32 = EARTH_SIZE * 2.0;
            let collider = ColliderBuilder::new(SharedShape::ball(RADIUS)).build();
            colliders.insert(collider, body_handle, bodies);

            planets.insert(id, CelestialObject {
                kind: PlanetKind::Jupiter,
                orbit: Some(Orbit {
                    orbit_around: sun_id,
                    radius: (3500.0, 3500.0),
                    rotation: 0.0,
                    total_ticks: 8400,
                    ticks_ellapsed: 0,
                    last_next_position: (0.0, 0.0),
                }),
                radius: RADIUS,
                mass,
                cargo_upgrade: Some(super::parts::PartKind::Thruster),
                can_beamout: false,
                body_handle,
                position: (0.0, 0.0),
            });
        };

        let pluto = {
			let id = make_planet_id();
            let mass = EARTH_MASS / 10.0;
            let body = RigidBodyBuilder::new_kinematic()
                .additional_mass(mass)
                .user_data(Planet(id).into())
                .build();
            let body_handle = bodies.insert(body);
            const RADIUS: f32 = EARTH_SIZE / 4.0;
            let collider = ColliderBuilder::new(SharedShape::ball(RADIUS)).build();
            colliders.insert(collider, body_handle, bodies);

            planets.insert(id, CelestialObject {
                kind: PlanetKind::Pluto,
                orbit: Some(Orbit {
                    orbit_around: sun_id,
                    radius: (8000.0, 6000.0),
                    rotation: std::f32::consts::PI / 5.0,
                    total_ticks: 3394,
                    ticks_ellapsed: 0,
                    last_next_position: (0.0, 0.0),
                }),
                radius: RADIUS,
                mass,
                cargo_upgrade: Some(PartKind::LandingWheel),
                can_beamout: false,
                body_handle,
                position: (0.0, 0.0),
            });
        };

        let saturn = {
			let id = make_planet_id();
            let mass = EARTH_MASS * 10.0;
            let body = RigidBodyBuilder::new_kinematic()
                .additional_mass(mass)
				.user_data(Planet(id).into())
                .build();
            let position = (body.position().translation.x, body.position().translation.y);
            let body_handle = bodies.insert(body);
            const RADIUS: f32 = EARTH_SIZE * 2.0;
            let collider = ColliderBuilder::new(SharedShape::ball(RADIUS)).build();
            colliders.insert(collider, body_handle, bodies);

            planets.insert(id, CelestialObject {
                kind: PlanetKind::Saturn,
                orbit: Some(Orbit {
                    orbit_around: sun_id,
                    radius: (4000.0, 4000.0),
                    rotation: 0.0,
                    total_ticks: 9600,
                    ticks_ellapsed: 0,
                    last_next_position: (0.0, 0.0),
                }),
                radius: RADIUS,
                mass,
                cargo_upgrade: Some(PartKind::SuperThruster),
                can_beamout: false,
                body_handle,
                position: (0.0, 0.0),
            });
        };

        let neptune = {
			let id = make_planet_id();
            let mass = EARTH_MASS * 4.0;
            let body = RigidBodyBuilder::new_kinematic()
                .additional_mass(mass)
				.user_data(Planet(id).into())
                .build();
            let body_handle = bodies.insert(body);
            const RADIUS: f32 = EARTH_SIZE * 1.5;
            let collider = ColliderBuilder::new(SharedShape::ball(RADIUS)).build();
            colliders.insert(collider, body_handle, bodies);

            planets.insert(id, CelestialObject {
                kind: PlanetKind::Neptune,
                orbit: Some(Orbit {
                    orbit_around: sun_id,
                    radius: (5500.0, 5500.0),
                    rotation: 0.0,
                    total_ticks: 13200,
                    ticks_ellapsed: 0,
                    last_next_position: (0.0, 0.0),
                }),
                radius: RADIUS,
                mass,
                cargo_upgrade: Some(PartKind::HubThruster),
                can_beamout: false,
                body_handle,
                position: (0.0, 0.0),
            });
        };

        let venus = {
			let id = make_planet_id();
            let mass = EARTH_MASS * 1.3;
            let body = RigidBodyBuilder::new_kinematic()
                .additional_mass(EARTH_MASS * 1.3)
				.user_data(Planet(id).into())
                .build();
            let body_handle = bodies.insert(body);
            const RADIUS: f32 = EARTH_SIZE;
            let collider = ColliderBuilder::new(SharedShape::ball(RADIUS)).build();
            colliders.insert(collider, body_handle, bodies);

            planets.insert(id, CelestialObject {
                kind: PlanetKind::Venus,
                orbit: Some(Orbit {
                    orbit_around: sun_id,
                    radius: (1000.0, 1000.0),
                    rotation: 0.0,
                    total_ticks: 2400,
                    ticks_ellapsed: 0,
                    last_next_position: (0.0, 0.0),
                }),
                radius: RADIUS,
                mass,
                cargo_upgrade: Some(PartKind::EcoThruster),
                can_beamout: false,
                body_handle,
                position: (0.0, 0.0),
            });
        };

        let uranus = {
			let id = make_planet_id();
            let mass = EARTH_MASS * 4.0;
            let body = RigidBodyBuilder::new_kinematic()
                .additional_mass(mass)
				.user_data(Planet(id).into())
                .build();
            let body_handle = bodies.insert(body);
            const RADIUS: f32 = EARTH_SIZE * 2.0;
            let collider = ColliderBuilder::new(SharedShape::ball(RADIUS)).build();
            colliders.insert(collider, body_handle, bodies);

            planets.insert(id, CelestialObject {
                kind: PlanetKind::Uranus,
                orbit: Some(Orbit {
                    orbit_around: sun_id,
                    radius: (4800.0, 4800.0),
                    rotation: 0.0,
                    total_ticks: 11520,
                    ticks_ellapsed: 0,
                    last_next_position: (0.0, 0.0),
                }),
                radius: RADIUS,
                mass,
                cargo_upgrade: Some(PartKind::PowerHub),
                can_beamout: false,
                body_handle,
                position: (0.0, 0.0),
            });
        };


        let trade_id = make_planet_id();
        let trade = {
			let id = trade_id;
            let mass = EARTH_MASS;
            let body = RigidBodyBuilder::new_kinematic()
                .additional_mass(mass)
				.user_data(Planet(id).into())
                .build();
            let body_handle = bodies.insert(body);
            const RADIUS: f32 = EARTH_SIZE * 0.75;
            let collider = ColliderBuilder::new(SharedShape::ball(RADIUS)).build();
            colliders.insert(collider, body_handle, bodies);

            planets.insert(id, CelestialObject {
                kind: PlanetKind::Trade,
                orbit: Some(Orbit {
                    orbit_around: sun_id,
                    radius: (1500.0, 1500.0),
                    rotation: 0.0,
                    total_ticks: 3600,
                    ticks_ellapsed: 1800,
                    last_next_position: (0.0, 0.0),
                }),
                radius: RADIUS,
                mass,
                cargo_upgrade: None,
                can_beamout: true,
                body_handle,
                position: (0.0, 0.0),
            });
        };

        /*Planets {
            earth, moon, planet_material, mars, mercury, jupiter, /* pluto, */ saturn, neptune, venus, uranus, sun, /* trade, */
        }*/
        Planets { planets, earth_id, trade_id, sun_id }
    }

    /*pub fn celestial_objects<'a>(&'a self) -> [&'a CelestialObject; 10] {
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
    }*/
}

static mut NEXT_PLANET_ID: u8 = 1;
fn make_planet_id() -> u8 {
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
    pub position: (f32, f32),
}

pub struct Orbit {
    orbit_around: u8,
    radius: (f32, f32),
    rotation: f32,
    total_ticks: u32,
    ticks_ellapsed: u32,
    last_next_position: (f32, f32),
}
const TICKS_PER_SECOND: f32 = crate::TICKS_PER_SECOND as f32;
impl Orbit {
    pub fn calculate_position_vel(&mut self, planets: &Planets) -> ((f32, f32), (f32, f32)) {
        let ticks_ellapsed = self.ticks_ellapsed as f32;
        let total_ticks = self.total_ticks as f32;
        let radians = ticks_ellapsed / total_ticks * 2.0 * std::f32::consts::PI;
        let mut pos = (self.radius.0 * radians.cos(), self.radius.1 * radians.sin());
        if self.rotation != 0.0 { Self::my_rotate_point(&mut pos, self.rotation) };
        let parent_planet = &planets.planets[&self.orbit_around];
        let pos = (pos.0 + parent_planet.position.0, pos.1 + parent_planet.position.1);

        let ticks_ellapsed = (ticks_ellapsed + 1.0); // % total_ticks;
        let radians = ticks_ellapsed / total_ticks * 2.0 * std::f32::consts::PI;
        let mut next_pos = (self.radius.0 * radians.cos(), self.radius.1 * radians.sin());
        if self.rotation != 0.0 { Self::my_rotate_point(&mut next_pos, self.rotation) };
        let parent_next_pos = if let Some(orbit) = &parent_planet.orbit { orbit.last_next_position } else { parent_planet.position };
        let next_pos = (next_pos.0 + parent_next_pos.0, next_pos.1 + parent_next_pos.1);

        let vel = ((next_pos.0 - pos.0) * TICKS_PER_SECOND, (next_pos.1 - pos.1) * TICKS_PER_SECOND);
        (pos, vel)
    }
    pub fn advance(&mut self, planets: &Planets) -> ((f32, f32), (f32, f32)) {
        self.ticks_ellapsed += 1;
        if self.ticks_ellapsed >= self.total_ticks { self.ticks_ellapsed = 0; }
        self.calculate_position_vel(planets)
    }

    fn my_rotate_point(point: &mut (f32, f32), radians: f32) {
        *point = crate::rotate_vector_with_angle(point.0, point.1, radians)        
    }
}

#[derive(Copy, Clone)]
pub struct AmPlanet {
    pub id: u8
}
//use nphysics2d::utils::UserData;
/*impl UserData for AmPlanet {
    fn clone_boxed(&self) -> Box<dyn UserData> { Box::new(*self) }
    fn to_any(&self) -> Box<dyn std::any::Any + Send + Sync> { Box::new(*self) }
    fn as_any(&self) -> &dyn std::any::Any { self }    
}*/

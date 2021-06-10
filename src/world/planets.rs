use super::{MyUnits, PartHandle};
use std::collections::BTreeMap;
use rapier2d::dynamics::{RigidBody, RigidBodyBuilder, BodyStatus, RigidBodyHandle, RigidBodySet};
use rapier2d::geometry::{ColliderBuilder, SharedShape, Collider, ColliderSet, ColliderHandle};
use super::typedef::*;
use crate::codec::{ PlanetKind, ToClientMsg };
use crate::storage7573::Storage7573;
use rand::Rng;
use super::parts::PartKind;

use Storage7573::Planet;

pub struct Planets {
    pub planets: BTreeMap<u8, CelestialObject>,
    pub earth_id: u8,
    pub trade_id: u8,
    pub sun_id: u8,
    pub planet_ids: Vec<u8>,
}
impl Planets {
    pub fn new(bodies: &mut RigidBodySet, colliders: &mut ColliderSet) -> Planets {
        const EARTH_MASS: f32 = 600.0;
        const EARTH_SIZE: f32 = 25.0;
        let mut rand = rand::thread_rng();

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

        let earth_orbit_duration = 3600 * 20 * 3;
        let earth_ticks_ellapsed = rand.gen_range(0..earth_orbit_duration);
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
                    total_ticks: earth_orbit_duration,
                    ticks_ellapsed: earth_ticks_ellapsed,
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

            let total_ticks = 600 * 20 * 3;
            planets.insert(id, CelestialObject {
                kind: PlanetKind::Moon,
                orbit: Some(Orbit {
                    orbit_around: earth_id,
                    radius: (100.0, 100.0),
                    rotation: 0.0,
                    total_ticks,
                    ticks_ellapsed: rand.gen_range(0..total_ticks),
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
            
            let total_ticks = 4800 * 20 * 3;
            planets.insert(id, CelestialObject {
                kind: PlanetKind::Mars,
                orbit: Some(Orbit {
                    orbit_around: sun_id,
                    radius: (2000.0, 2000.0),
                    rotation: 0.0,
                    total_ticks,
                    ticks_ellapsed: rand.gen_range(0..total_ticks),
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
            
            let total_ticks = 1200 * 20 * 3;
            planets.insert(id, CelestialObject {
                kind: PlanetKind::Mercury,
                orbit: Some(Orbit {
                    orbit_around: sun_id,
                    radius: (500.0, 500.0),
                    rotation: 0.0,
                    total_ticks,
                    ticks_ellapsed: rand.gen_range(0..total_ticks),
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

            let total_ticks = 8400 * 20 * 3;
            planets.insert(id, CelestialObject {
                kind: PlanetKind::Jupiter,
                orbit: Some(Orbit {
                    orbit_around: sun_id,
                    radius: (3500.0, 3500.0),
                    rotation: 0.0,
                    total_ticks,
                    ticks_ellapsed: rand.gen_range(0..total_ticks),
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

            let total_ticks = 3394 * 20 * 3;
            planets.insert(id, CelestialObject {
                kind: PlanetKind::Pluto,
                orbit: Some(Orbit {
                    orbit_around: sun_id,
                    radius: (8000.0, 6000.0),
                    rotation: std::f32::consts::PI / 5.0,
                    total_ticks,
                    ticks_ellapsed: rand.gen_range(0..total_ticks),
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

            let total_ticks = 9600 * 20 * 3;
            planets.insert(id, CelestialObject {
                kind: PlanetKind::Saturn,
                orbit: Some(Orbit {
                    orbit_around: sun_id,
                    radius: (4000.0, 4000.0),
                    rotation: 0.0,
                    total_ticks,
                    ticks_ellapsed: rand.gen_range(0..total_ticks),
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

            let total_ticks = 13200 * 20 * 3;
            planets.insert(id, CelestialObject {
                kind: PlanetKind::Neptune,
                orbit: Some(Orbit {
                    orbit_around: sun_id,
                    radius: (5500.0, 5500.0),
                    rotation: 0.0,
                    total_ticks,
                    ticks_ellapsed: rand.gen_range(0..total_ticks),
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

            let total_ticks = 2400 * 20 * 3;
            planets.insert(id, CelestialObject {
                kind: PlanetKind::Venus,
                orbit: Some(Orbit {
                    orbit_around: sun_id,
                    radius: (1000.0, 1000.0),
                    rotation: 0.0,
                    total_ticks,
                    ticks_ellapsed: rand.gen_range(0..total_ticks),
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

            let total_ticks = 11520 * 20 * 3;
            planets.insert(id, CelestialObject {
                kind: PlanetKind::Uranus,
                orbit: Some(Orbit {
                    orbit_around: sun_id,
                    radius: (4800.0, 4800.0),
                    rotation: 0.0,
                    total_ticks,
                    ticks_ellapsed: rand.gen_range(0..total_ticks),
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
                    total_ticks: earth_orbit_duration,
                    ticks_ellapsed: (earth_ticks_ellapsed + (earth_orbit_duration / 2)) % earth_orbit_duration,
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
        let planet_ids = planets.keys().cloned().collect();
        Planets { planets, earth_id, trade_id, sun_id, planet_ids }
    }

    pub fn advance_orbits(&mut self, bodies: &mut RigidBodySet) {
        for id in &self.planet_ids {
            if let Some(orbit) = &self.planets[&id].orbit {
                let parent_planet = &self.planets[&orbit.orbit_around];
                let parent_pos = parent_planet.position;
                let parent_next_pos = if let Some(orbit) = &parent_planet.orbit { orbit.last_next_position } else { parent_pos };
                let planet = self.planets.get_mut(id).unwrap();
                //let (pos, vel) = planet.orbit.as_mut().unwrap().advance(parent_pos, parent_next_pos);
                let pos = planet.orbit.as_mut().unwrap().advance(parent_pos);
                planet.position = pos;
                let body = &mut bodies[planet.body_handle];
                //body.set_position(Isometry::new(Vector::new(pos.0, pos.1), 0.0), true);
                //body.set_linvel(Vector::new(vel.0, vel.1), true);
                body.set_next_kinematic_position(Isometry::new(Vector::new(pos.0, pos.1), 0.0));
            }
        }
    }
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
    //pub fn calculate_position_vel(&mut self, parent_pos: (f32, f32), parent_next_pos: (f32, f32)) -> ((f32, f32), (f32, f32)) {
    pub fn calculate_position_vel(&mut self, parent_pos: (f32, f32)) -> (f32, f32) {
        let ticks_ellapsed = self.ticks_ellapsed as f32;
        let total_ticks = self.total_ticks as f32;
        let radians = ticks_ellapsed / total_ticks * 2.0 * std::f32::consts::PI;
        let mut pos = (self.radius.0 * radians.cos(), self.radius.1 * radians.sin());
        if self.rotation != 0.0 { Self::my_rotate_point(&mut pos, self.rotation) };
        let pos = (pos.0 + parent_pos.0, pos.1 + parent_pos.1);
        pos

        /*let ticks_ellapsed = (ticks_ellapsed + 1.0); // % total_ticks;
        let radians = ticks_ellapsed / total_ticks * 2.0 * std::f32::consts::PI;
        let mut next_pos = (self.radius.0 * radians.cos(), self.radius.1 * radians.sin());
        if self.rotation != 0.0 { Self::my_rotate_point(&mut next_pos, self.rotation) };
        let next_pos = (next_pos.0 + parent_next_pos.0, next_pos.1 + parent_next_pos.1);*/

        /*let vel = ((next_pos.0 - pos.0) * TICKS_PER_SECOND, (next_pos.1 - pos.1) * TICKS_PER_SECOND);
        (pos, vel)*/
    }

    //pub fn advance(&mut self, parent_pos: (f32, f32), parent_next_pos: (f32, f32)) -> ((f32, f32), (f32, f32)) {
    pub fn advance(&mut self, parent_pos: (f32, f32)) -> (f32, f32) {
        self.ticks_ellapsed += 1;
        if self.ticks_ellapsed >= self.total_ticks { self.ticks_ellapsed = 0; }
        self.calculate_position_vel(parent_pos)
    }

    fn my_rotate_point(point: &mut (f32, f32), radians: f32) {
        *point = crate::rotate_vector_with_angle(point.0, point.1, radians)        
    }

    pub fn init_messages(&self, my_id: u8) -> (ToClientMsg, ToClientMsg) {
        (
            ToClientMsg::InitCelestialOrbit {
                id: my_id,
                orbit_around_body: self.orbit_around,
                orbit_radius: self.radius,
                orbit_rotation: self.rotation,
                orbit_total_ticks: self.total_ticks,
            },
            ToClientMsg::UpdateCelestialOrbit {
                id: my_id,
                orbit_ticks_ellapsed: self.ticks_ellapsed,
            }
        )
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

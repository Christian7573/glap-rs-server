use super::{MyUnits, MyHandle};
use nphysics2d::object::{RigidBody, RigidBodyDesc, BodyStatus, BodyPartHandle};
use nphysics2d::object::ColliderDesc;
use ncollide2d::shape::{Ball, ShapeHandle};
use nalgebra::Vector2;
use nphysics2d::material::{BasicMaterial, MaterialHandle};
use rand::Rng;
use super::parts::PartKind;

pub struct Planets {
    pub earth: CelestialObject,
    pub moon: CelestialObject,
    pub planet_material: MaterialHandle<MyUnits>,
    pub mars: CelestialObject,
    pub mercury: CelestialObject,
    pub jupiter: CelestialObject,
    //pub pluto: CelestialObject,
    pub saturn: CelestialObject,
    pub neptune: CelestialObject,
    pub venus: CelestialObject,
    pub uranus: CelestialObject,
    pub sun: CelestialObject,
    //pub trade: CelestialObject,
}
impl Planets {
    pub fn new(colliders: &mut super::MyColliderSet, bodies: &mut super::World) -> Planets {
        const EARTH_MASS: f32 = 600.0;
        const EARTH_SIZE: f32 = 25.0;
        let earth_pos = planet_location(1500.0);
        let planet_material = MaterialHandle::new(BasicMaterial::new(0.0, 1.0));
        let earth = {
            let body = RigidBodyDesc::new()
            .translation(earth_pos.clone())
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_MASS)
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
                name: String::from("earth"),
                display_name: String::from("Earth"),
                radius: RADIUS,
                body: body_handle,
                id,
                cargo_upgrade: None,
                can_beamout: true,
                position: (earth_pos.x, earth_pos.y),
                mass,
            }
        };

        let moon = {
            let body = RigidBodyDesc::new()
            .translation(planet_location(100.0) + earth_pos.clone())
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_MASS / 35.0)
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
                name: String::from("moon"),
                display_name: String::from("Moon"),
                radius: RADIUS,
                body: body_handle,
                id,
                cargo_upgrade: Some(super::parts::PartKind::LandingThruster),
                can_beamout: true,
                position,
                mass,
            }
        };

        let mars = {
            let body = RigidBodyDesc::new()
                .translation(planet_location(2000.0))
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_MASS / 4.0)
                .build();
            let position = (body.position().translation.x, body.position().translation.y);
            let mass = body.augmented_mass().linear;
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = EARTH_SIZE / 2.0;
            let shape = ShapeHandle::new(Ball::new(RADIUS));
			let id = make_planet_id();
            let collider = ColliderDesc::new(shape)
                .material(planet_material.clone())
				.user_data(AmPlanet {id})
                .build(BodyPartHandle(body_handle, 0));
            colliders.insert(collider);

            
            
            CelestialObject {
                name: String::from("mars"),
                display_name: String::from("Mars"),
                radius: RADIUS,
                body: body_handle,
                id,
                cargo_upgrade: Some(super::parts::PartKind::Hub),
                can_beamout: false,
                position,
                mass,
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

        /*let pluto = {
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
        };*/

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

        let sun = {
            let body = RigidBodyDesc::new()
                .translation(Vector2::new(0.0,0.0))
                .gravity_enabled(false)
                .status(BodyStatus::Static)
                .mass(EARTH_MASS * 50.0)
                .build();
            let position = (body.position().translation.x, body.position().translation.y);
            let mass = body.augmented_mass().linear;
            let body_handle = bodies.add_celestial_object(body);
            const RADIUS: f32 = EARTH_SIZE * 4.7;
            let shape = ShapeHandle::new(Ball::new(RADIUS));
			let id = make_planet_id();
            let collider = ColliderDesc::new(shape)
                .material(planet_material.clone())
				.user_data(AmPlanet {id})
                .build(BodyPartHandle(body_handle, 0));
            colliders.insert(collider);

            
            
            CelestialObject {
                name: String::from("sun"),
                display_name: String::from("sun"),
                radius: RADIUS,
                body: body_handle,
                id,
                cargo_upgrade: None,
                can_beamout: false,
                position,
                mass,
            }
        };

        /*let trade = {
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
        };*/

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
    pub name: String,
    pub display_name: String,
    pub radius: f32,
    pub body: MyHandle,
    pub id: u16,
    pub cargo_upgrade: Option<super::parts::PartKind>,
    pub can_beamout: bool,
    pub position: (f32, f32),
    pub mass: f32,
}

pub fn planet_location(radius: f32) -> nalgebra::Matrix<f32, nalgebra::U2, nalgebra::U1, nalgebra::ArrayStorage<f32, nalgebra::U2, nalgebra::U1>> {
    let mut rng = rand::thread_rng();
    let angle: f32 = rng.gen::<f32>() * std::f32::consts::PI * 2.0;
    let pos = Vector2::new(f32::cos(angle) * radius, f32::sin(angle) * radius);
    pos
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

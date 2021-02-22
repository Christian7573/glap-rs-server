use nphysics2d::object::{RigidBody, Body, RigidBodyDesc, Collider, ColliderDesc, BodyPartHandle, BodyStatus, DefaultColliderHandle};
use nphysics2d::algebra::{Force2, ForceType, Inertia2};
use nphysics2d::math::Isometry;
use nalgebra::{Vector2, Point2};
use ncollide2d::shape::{Cuboid, ShapeHandle};
use super::{MyUnits, MyHandle};
use num_traits::identities::{Zero, One};
use ncollide2d::pipeline::object::CollisionGroups;
use nphysics2d::joint::DefaultJointConstraintHandle;

pub struct PartStatic {
    unit_cuboid: ShapeHandle<MyUnits>,
    cargo_cuboid: ShapeHandle<MyUnits>,
    solar_panel_cuboid: ShapeHandle<MyUnits>,
    attachment_collider_cuboid: ShapeHandle<MyUnits>,
    super_thruster_cuboid: ShapeHandle<MyUnits>,
}
impl Default for PartStatic {
    fn default() -> PartStatic { PartStatic {
        unit_cuboid: ShapeHandle::new(Cuboid::new(Vector2::new(0.5, 0.5))),
        cargo_cuboid: ShapeHandle::new(Cuboid::new(Vector2::new(0.38, 0.5))),
        solar_panel_cuboid: ShapeHandle::new(Cuboid::new(Vector2::new(0.31, 0.5))),
        attachment_collider_cuboid: ShapeHandle::new(Cuboid::new(Vector2::new(1.0, 1.0))),
        super_thruster_cuboid: ShapeHandle::new(Cuboid::new(Vector2::new(0.38, 0.44))),
    } }
}

pub const ATTACHMENT_COLLIDER_COLLISION_GROUP: [usize; 1] = [5];

pub struct Part {
    pub kind: PartKind,
    pub attachments: Box<[Option<(Part, DefaultJointConstraintHandle, DefaultJointConstraintHandle)>; 4]>,
    pub body_id: u16,
    pub thrust_mode: CompactThrustMode,
    pub collider: DefaultColliderHandle
}
impl Part {
    pub fn new(kind: PartKind, bodies: &mut super::World, colliders: &mut super::MyColliderSet, part_static: &PartStatic) -> Part {
        let (body_desc, collider_desc) = kind.physics_components(part_static);
        let body_id = bodies.add_part(body_desc.build());
        let collider = colliders.insert(collider_desc.build(BodyPartHandle(MyHandle::Part(body_id), 0)));
        Part {
            kind, body_id,
            attachments: Box::new([None, None, None, None]),
            thrust_mode: CompactThrustMode::default(),
            collider
        }
    }
    pub fn mutate(&mut self, new_kind: PartKind, bodies: &mut super::World, colliders: &mut super::MyColliderSet, part_static: &PartStatic) {
        for attachment in self.attachments.iter() { if attachment.is_some() { panic!("Mutated part with attachments"); } };
        let mut prev_collider = colliders.remove(self.collider).unwrap();        
        self.kind = new_kind;
        let (body_desc, collider_desc) = new_kind.physics_components(part_static);
        let mut body = body_desc.build();
        let old_body = bodies.get_rigid(MyHandle::Part(self.body_id)).unwrap();
        body.set_position(old_body.position().clone());
        body.set_velocity(old_body.velocity().clone());
        bodies.swap_part(self.body_id, body);
        let mut collider = collider_desc.build(BodyPartHandle(MyHandle::Part(self.body_id), 0));
        collider.set_user_data(prev_collider.take_user_data());
        self.collider = colliders.insert(collider);
    }
    pub fn thrust(&self, bodies: &mut super::World, fuel: &mut u32, forward: bool, backward: bool, clockwise: bool, counter_clockwise: bool) {
        match self.kind {
            PartKind::Core => {
                if *fuel > 0 {
                    let body = bodies.get_rigid_mut(MyHandle::Part(self.body_id)).unwrap();
                    let mut subtract_fuel = false;
                    if forward || counter_clockwise { subtract_fuel = true; body.apply_local_force_at_local_point(0, &Vector2::new(0.0,1.0), &Point2::new(-0.5,-0.5), ForceType::Force, true); }
                    if forward || clockwise { subtract_fuel = true; body.apply_local_force_at_local_point(0, &Vector2::new(0.0,1.0), &Point2::new(0.5,-0.5), ForceType::Force, true); }
                    if backward || clockwise { subtract_fuel = true; body.apply_local_force_at_local_point(0, &Vector2::new(0.0,-1.0), &Point2::new(-0.5,0.5), ForceType::Force, true); }
                    if backward || counter_clockwise { subtract_fuel = true; body.apply_local_force_at_local_point(0, &Vector2::new(0.0,-1.0), &Point2::new(0.5,0.5), ForceType::Force, true); }
                    if subtract_fuel { *fuel -= 1; };
                }
            },
            _ => {
                if let Some(ThrustDetails{ fuel_cost, force }) = self.kind.thrust() {
                    let should_fire = match self.thrust_mode.get_horizontal() {
                        HorizontalThrustMode::Clockwise => clockwise,
                        HorizontalThrustMode::CounterClockwise => counter_clockwise,
                        HorizontalThrustMode::None => false
                    } || match self.thrust_mode.get_vertical() {
                        VerticalThrustMode::Forwards => forward,
                        VerticalThrustMode::Backwards => backward,
                        VerticalThrustMode::None => false,
                    };
                    if *fuel >= fuel_cost && should_fire  {
                        *fuel -= fuel_cost;
                        bodies.get_rigid_mut(MyHandle::Part(self.body_id)).unwrap().apply_local_force(0, &force, ForceType::Force, true)
                    }
                }
            }
        }
        for attachment in self.attachments.iter() {
            if let Some((part, _, _)) = attachment.as_ref() {
                part.thrust(bodies, fuel, forward, backward, clockwise, counter_clockwise);
            }
        }
    }
}

pub use crate::codec::PartKind;
impl PartKind {
    pub fn physics_components(&self, part_static: &PartStatic) -> (RigidBodyDesc<MyUnits>, ColliderDesc<MyUnits>) {
        match self {
            _ => {
                (
                    RigidBodyDesc::new().status(BodyStatus::Dynamic).local_inertia(self.inertia()),
                    ColliderDesc::new( match self {
                        PartKind::Core | PartKind::Hub | PartKind::PowerHub | PartKind::HubThruster => part_static.unit_cuboid.clone(),
                        PartKind::Cargo | PartKind::LandingThruster | PartKind::Thruster => part_static.cargo_cuboid.clone(),
                        PartKind::SolarPanel | PartKind::EcoThruster | PartKind::LandingWheel => part_static.solar_panel_cuboid.clone(), 
                        PartKind::SuperThruster => part_static.super_thruster_cuboid.clone(),
                    } )
                    .translation( match self {
                        PartKind::Core => Vector2::zero(),
                        PartKind::Thruster | PartKind::SuperThruster => Vector2::new(0.0, 0.44),
                        _ => Vector2::new(0.0, 0.5)
                    } ) 
                )
            }
        }
    }
    fn thrust(&self) -> Option<ThrustDetails> {
        match self {
            PartKind::Core => panic!("PartKind thrust called on core"),
            PartKind::Hub => None,
            PartKind::LandingThruster => Some(ThrustDetails{ fuel_cost: 2, force: Force2::linear_at_point(Vector2::new(0.0, -5.0), &Point2::new(0.0, 1.0)) }),
            PartKind::Cargo | PartKind::SolarPanel => None,
            PartKind::Thruster => Some(ThrustDetails{ fuel_cost: 4, force: Force2::linear_at_point(Vector2::new(0.0, -9.0), &Point2::new(0.0, 1.0)) }),
            PartKind::SuperThruster => Some(ThrustDetails { fuel_cost: 7, force: Force2::linear_at_point(Vector2::new(0.0, -13.5), &Point2::new(0.0, 1.0)) }),
            PartKind::HubThruster => Some(ThrustDetails { fuel_cost: 4, force: Force2::linear_at_point(Vector2::new(0.0, -6.0), &Point2::new(0.0, 1.0)) }),
            PartKind::EcoThruster => Some(ThrustDetails { fuel_cost: 1, force: Force2::linear_at_point(Vector2::new(0.0, -3.5), &Point2::new(0.0, 1.0)) }),
            PartKind::PowerHub | PartKind::LandingWheel => None,
        }
    }
    pub fn inertia(&self) -> Inertia2<MyUnits> {
        match self {
            PartKind::Core => Inertia2::new(1.0,1.0),
            PartKind::Cargo => Inertia2::new(0.5, 0.5),
            PartKind::LandingThruster => Inertia2::new(1.5, 1.5),
            PartKind::Hub => Inertia2::new(0.75, 0.75),
            PartKind::SolarPanel => Inertia2::new(0.4, 0.4),
            PartKind::Thruster => Inertia2::new(1.6, 1.6),
            PartKind::SuperThruster => Inertia2::new(1.8, 1.8),
            PartKind::HubThruster => Inertia2::new(1.6, 1.6),
            PartKind::EcoThruster => Inertia2::new(1.35, 1.35),
            PartKind::PowerHub => Inertia2::new(1.1, 1.1),
            PartKind::LandingWheel => Inertia2::new(0.75, 0.75),
        }
    }
    pub fn attachment_locations(&self) -> [Option<AttachmentPointDetails>; 4] {
        match self {
            PartKind::Core => [
                Some(AttachmentPointDetails{ x: 0.0, y: 0.6, facing: AttachedPartFacing::Up, perpendicular: (1.0, 0.0) }),
                Some(AttachmentPointDetails{ x: -0.6, y: 0.0, facing: AttachedPartFacing::Right, perpendicular: (0.0, 1.0) }),
                Some(AttachmentPointDetails{ x: 0.0, y: -0.6, facing: AttachedPartFacing::Down, perpendicular: (-1.0, 0.0) }),
                Some(AttachmentPointDetails{ x: 0.6, y: 0.0, facing: AttachedPartFacing::Left, perpendicular: (0.0, -1.0) }),
            ],
            PartKind::Hub | PartKind::PowerHub => [
                None,
                Some(AttachmentPointDetails{ x: 0.6, y: 0.5, facing: AttachedPartFacing::Left, perpendicular: (0.0, -1.0) }),
                Some(AttachmentPointDetails{ x: 0.0, y: 1.1, facing: AttachedPartFacing::Up, perpendicular: (1.0, 0.0) }),
                Some(AttachmentPointDetails{ x: -0.6, y: 0.5, facing: AttachedPartFacing::Right, perpendicular: (0.0, 1.0) }),
            ],
            PartKind::Cargo | PartKind::LandingThruster | PartKind::SolarPanel | PartKind::Thruster | PartKind::SuperThruster | PartKind::EcoThruster | PartKind::LandingWheel => [ None, None, None, None ],
            PartKind::HubThruster => [
                None,
                Some(AttachmentPointDetails{ x: 0.6, y: 0.5, facing: AttachedPartFacing::Left, perpendicular: (0.0, -1.0) }),
                None, //Some(AttachmentPointDetails{ x: 0.0, y: 1.1, facing: AttachedPartFacing::Up, perpendicular: (1.0, 0.0) }),
                Some(AttachmentPointDetails{ x: -0.6, y: 0.5, facing: AttachedPartFacing::Right, perpendicular: (0.0, 1.0) }),
            ],
        }
    }
    pub fn power_storage(&self) -> u32 {
        const CORE_MAX_POWER: u32 = 100 * crate::TICKS_PER_SECOND as u32;
        match self {
            PartKind::Core => CORE_MAX_POWER,
            PartKind::Cargo => CORE_MAX_POWER / 10,
            PartKind::LandingThruster | PartKind::HubThruster => CORE_MAX_POWER / 5,
            PartKind::Hub => CORE_MAX_POWER / 3,
            PartKind::SolarPanel => 0,
            PartKind::Thruster | PartKind::SuperThruster => CORE_MAX_POWER / 4,
            PartKind::EcoThruster => 0,
            PartKind::PowerHub => CORE_MAX_POWER / 6 * 2,
            PartKind::LandingWheel => 0,
        }
    }
    pub fn power_regen_per_5_ticks(&self) -> u32 {
        match self {
            PartKind::SolarPanel => 2,
            _ => 0,
        }
    }
    // pub fn get_attachable_positions(&self) -> [(Isometry<super::MyUnits>, )] {
        
    // }
}

#[derive(Copy, Clone, Debug)]
pub struct AttachmentPointDetails {
    pub x: f32,
    pub y: f32,
    pub perpendicular: (f32,f32),
    pub facing: AttachedPartFacing
}
#[derive(Copy, Clone, Debug)]
pub enum AttachedPartFacing { Up, Right, Down, Left }
impl AttachedPartFacing {
    pub fn part_rotation(&self) -> f32 {
        match self {
            AttachedPartFacing::Up => 0.0,
            AttachedPartFacing::Right => std::f32::consts::FRAC_PI_2,
            AttachedPartFacing::Down => std::f32::consts::PI,
            AttachedPartFacing::Left => -std::f32::consts::FRAC_PI_2,
        }
    }
    pub fn get_actual_rotation(&self, parent_actual_rotation: AttachedPartFacing) -> AttachedPartFacing {
        let parent_actual_rotation: u8 = parent_actual_rotation.into();
        let my_rotation: u8 = (*self).into();
        let num: u8 = parent_actual_rotation + my_rotation;
        if num > 3 { (num - 4).into() } else { num.into() }
    }
    pub fn attachment_offset(&self) -> (i16,i16) {
        let new_x = match self { AttachedPartFacing::Left => -1, AttachedPartFacing::Right => 1, _ => 0 };
        let new_y = match self { AttachedPartFacing::Up => 1, AttachedPartFacing::Down => -1, _ => 0 };
        (new_x, new_y)
    }
}
impl Into<u8> for AttachedPartFacing {
    fn into(self) -> u8 { match self {
        AttachedPartFacing::Up => 0,
        AttachedPartFacing::Right => 1,
        AttachedPartFacing::Down => 2,
        AttachedPartFacing::Left => 3
    } }
}
impl From<u8> for AttachedPartFacing {
    fn from(other: u8) -> Self { match other {
        0 => AttachedPartFacing::Up,
        1 => AttachedPartFacing::Right,
        2 => AttachedPartFacing::Down,
        3 => AttachedPartFacing::Left,
        _ => panic!()
    } }
}

#[derive(Copy, Clone, Debug)]
pub enum VerticalThrustMode { Forwards, Backwards, None }
impl Into<u8> for VerticalThrustMode {
    fn into(self) -> u8 { match self { 
        VerticalThrustMode::Backwards => 0b00000000,
        VerticalThrustMode::Forwards => 0b00000100,
        VerticalThrustMode::None => 0b00001000,
    } }
}
impl From<u8> for VerticalThrustMode {
    fn from(val: u8) -> Self { match val & 0b00001100 {
        0b00000000 => VerticalThrustMode::Backwards,
        0b00000100 => VerticalThrustMode::Forwards,
        0b00001000 => VerticalThrustMode::None,
        _ => panic!()
    } }
}
#[derive(Copy, Clone, Debug)]
pub enum HorizontalThrustMode { Clockwise, CounterClockwise, None }
impl Into<u8> for HorizontalThrustMode {
    fn into(self) -> u8 { match self {
        HorizontalThrustMode::CounterClockwise => 0b00000000,
        HorizontalThrustMode::Clockwise => 0b00000001,
        HorizontalThrustMode::None => 0b00000010,
    } }
}
impl From<u8> for HorizontalThrustMode {
    fn from(val: u8) -> Self { match val & 0b00000011 {
        0b00000000 => HorizontalThrustMode::CounterClockwise,
        0b00000001 => HorizontalThrustMode::Clockwise,
        0b00000010 => HorizontalThrustMode::None,
        _ => panic!()
    } }
}
#[derive(Copy, Clone, Debug)]
pub struct CompactThrustMode( u8 );
impl CompactThrustMode {
    pub fn new(horizontal: HorizontalThrustMode, vertical: VerticalThrustMode) -> CompactThrustMode {
        let horizontal: u8 = horizontal.into();
        let vertical: u8 = vertical.into();
        CompactThrustMode (horizontal | vertical)
    }
    pub fn get_horizontal(&self) -> HorizontalThrustMode { HorizontalThrustMode::from(self.0) }
    pub fn get_vertical(&self) -> VerticalThrustMode { VerticalThrustMode::from(self.0) }
    pub fn get(&self) -> (HorizontalThrustMode, VerticalThrustMode) { (self.get_horizontal(), self.get_vertical()) }
    pub fn set_horizontal(&mut self, horizontal: HorizontalThrustMode) { std::mem::replace::<CompactThrustMode>(self, CompactThrustMode::new(horizontal, self.get_vertical())); }
    pub fn set_vertical(&mut self, vertical: VerticalThrustMode) { std::mem::replace::<CompactThrustMode>(self, CompactThrustMode::new(self.get_horizontal(), vertical)); }
    pub fn set(&mut self, horizontal: HorizontalThrustMode, vertical: VerticalThrustMode) { std::mem::replace::<CompactThrustMode>(self, CompactThrustMode::new(horizontal, vertical)); }

    pub fn calculate(my_actual_facing: AttachedPartFacing, x: i16, y: i16) -> CompactThrustMode {
        let hroizontal = match my_actual_facing {
            AttachedPartFacing::Up => if x < 0 { HorizontalThrustMode::CounterClockwise } else if x > 0 { HorizontalThrustMode::Clockwise } else { HorizontalThrustMode::None },
            AttachedPartFacing::Right => if y > 0 { HorizontalThrustMode::CounterClockwise } else { HorizontalThrustMode::Clockwise },
            AttachedPartFacing::Down => if x < 0 { HorizontalThrustMode::Clockwise } else if x > 0 { HorizontalThrustMode::CounterClockwise } else { HorizontalThrustMode::None },
            AttachedPartFacing::Left => if y > 0 { HorizontalThrustMode::Clockwise } else { HorizontalThrustMode::CounterClockwise },
        };
        let vertical = match my_actual_facing  {
            AttachedPartFacing::Up => VerticalThrustMode::Backwards,
            AttachedPartFacing::Down => VerticalThrustMode::Forwards,
            AttachedPartFacing::Left | AttachedPartFacing::Right => VerticalThrustMode::None
        };
        CompactThrustMode::new(hroizontal, vertical)
    }
}
impl From<u8> for CompactThrustMode {
    fn from(byte: u8) -> CompactThrustMode { CompactThrustMode( byte ) }
}
impl Into<u8> for CompactThrustMode {
    fn into(self) -> u8 { self.0 }
}
impl Default for CompactThrustMode {
    fn default() -> Self { 0.into() }
}
// enum FireDirection {
    
// }

struct ThrustDetails { fuel_cost: u32, force: Force2<MyUnits> }

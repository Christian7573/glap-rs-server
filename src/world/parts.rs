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
    attachment_collider_cuboid: ShapeHandle<MyUnits>,
}
impl Default for PartStatic {
    fn default() -> PartStatic { PartStatic {
        unit_cuboid: ShapeHandle::new(Cuboid::new(Vector2::new(0.5, 0.5))),
        cargo_cuboid: ShapeHandle::new(Cuboid::new(Vector2::new(0.2714, 0.3563))),
        attachment_collider_cuboid: ShapeHandle::new(Cuboid::new(Vector2::new(1.0, 1.0)))
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
        colliders.remove(self.collider);
        self.kind = new_kind;
        let (body_desc, collider_desc) = new_kind.physics_components(part_static);
        let mut body = body_desc.build();
        let old_body = bodies.get_rigid(MyHandle::Part(self.body_id)).unwrap();
        body.set_position(old_body.position().clone());
        body.set_velocity(old_body.velocity().clone());
        bodies.swap_part(self.body_id, body);
        self.collider = colliders.insert(collider_desc.build(BodyPartHandle(MyHandle::Part(self.body_id), 0)));
    }
    pub fn thrust(&self, bodies: &mut super::World, fuel: &mut u16, forward: bool, backward: bool, clockwise: bool, counter_clockwise: bool) {
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
                        HorizontalThrustMode::Either => clockwise | counter_clockwise
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
            PartKind::Core | PartKind::Hub | PartKind::Cargo | PartKind::LandingThruster => {
                (
                    RigidBodyDesc::new().status(BodyStatus::Dynamic).local_inertia(self.inertia()),
                    ColliderDesc::new(part_static.unit_cuboid.clone())
                        .translation(if let PartKind::Core = self { Vector2::zero() } else { Vector2::new(0.0, 0.5) })
                )
            }
        }
    }
    fn thrust(&self) -> Option<ThrustDetails> {
        match self {
            PartKind::Core => panic!("PartKind thrust called on core"),
            PartKind::Hub => None,
            PartKind::LandingThruster => Some(ThrustDetails{ fuel_cost: 3, force: Force2::linear_at_point(Vector2::new(0.0, -5.0), &Point2::new(0.0, 1.0)) }),
            PartKind::Cargo => None
        }
    }
    pub fn inertia(&self) -> Inertia2<MyUnits> {
        match self {
            PartKind::Core | PartKind::Hub => Inertia2::new(1.0,1.0),
            PartKind::Cargo => Inertia2::new(0.5, 2.0),
            PartKind::LandingThruster => Inertia2::new(1.5, 1.5),
            _ => todo!()
        }
    }
    pub fn attachment_locations(&self) -> [Option<AttachmentPointDetails>; 4] {
        match self {
            PartKind::Core => [
                Some(AttachmentPointDetails{ x: 0.0, y: 0.6, facing: AttachedPartFacing::Up, perpendicular: (1.0, 0.0) }),
                Some(AttachmentPointDetails{ x: 0.6, y: 0.0, facing: AttachedPartFacing::Right, perpendicular: (0.0, 1.0) }),
                Some(AttachmentPointDetails{ x: 0.0, y: -0.6, facing: AttachedPartFacing::Down, perpendicular: (-1.0, 0.0) }),
                Some(AttachmentPointDetails{ x: -0.6, y: 0.0, facing: AttachedPartFacing::Left, perpendicular: (0.0, -1.0) }),
            ],
            PartKind::Hub => [
                Some(AttachmentPointDetails{ x: 0.0, y: 0.6, facing: AttachedPartFacing::Up, perpendicular: (1.0, 0.0) }),
                Some(AttachmentPointDetails{ x: 0.6, y: 0.0, facing: AttachedPartFacing::Right, perpendicular: (0.0, 1.0) }),
                None,
                Some(AttachmentPointDetails{ x: -0.6, y: 0.0, facing: AttachedPartFacing::Left, perpendicular: (0.0, -1.0) }),
            ],
            PartKind::Cargo | PartKind::LandingThruster => [ None, None, None, None ]
        }
    }
    // pub fn get_attachable_positions(&self) -> [(Isometry<super::MyUnits>, )] {
        
    // }
}

#[derive(Copy, Clone)]
pub struct AttachmentPointDetails {
    pub x: f32,
    pub y: f32,
    pub perpendicular: (f32,f32),
    pub facing: AttachedPartFacing
}
#[derive(Copy, Clone)]
pub enum AttachedPartFacing { Up, Right, Down, Left }
impl AttachedPartFacing {
    pub fn part_rotation(&self) -> f32 {
        match self {
            AttachedPartFacing::Up => 0.0,
            AttachedPartFacing::Right => -std::f32::consts::FRAC_PI_2,
            AttachedPartFacing::Down => std::f32::consts::PI,
            AttachedPartFacing::Left => std::f32::consts::PI,
        }
    }
    pub fn get_actual_rotation(&self, parent_actual_rotation: AttachedPartFacing) -> AttachedPartFacing {
        let parent_actual_rotation: u8 = parent_actual_rotation.into();
        let my_rotation: u8 = (*self).into();
        let num: u8 = parent_actual_rotation + my_rotation;
        if num > 3 { (num - 4).into() } else { num.into() }
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
pub enum HorizontalThrustMode { Clockwise, CounterClockwise, Either }
impl Into<u8> for HorizontalThrustMode {
    fn into(self) -> u8 { match self {
        HorizontalThrustMode::CounterClockwise => 0b00000000,
        HorizontalThrustMode::Clockwise => 0b00000001,
        HorizontalThrustMode::Either => 0b00000010,
    } }
}
impl From<u8> for HorizontalThrustMode {
    fn from(val: u8) -> Self { match val & 0b00000011 {
        0b00000000 => HorizontalThrustMode::CounterClockwise,
        0b00000001 => HorizontalThrustMode::Clockwise,
        0b00000010 => HorizontalThrustMode::Either,
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

struct ThrustDetails { fuel_cost: u16, force: Force2<MyUnits> }
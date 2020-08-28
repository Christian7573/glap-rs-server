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
    pub attachments: Box<[Option<(Part, DefaultJointConstraintHandle)>; 4]>,
    pub body_id: u16,
    pub thrust_mode: CompactThrustMode,
    pub attachment_colliders: Option<[Option<DefaultColliderHandle>; 4]>,

}
impl Part {
    pub fn new(kind: PartKind, bodies: &mut super::World, colliders: &mut super::MyColliderSet, part_static: &PartStatic) -> Part {
        let body_id = kind.initialize(bodies, colliders, part_static);
        Part {
            kind, body_id,
            attachments: Box::new([None, None, None, None]),
            thrust_mode: CompactThrustMode::default(),
            attachment_colliders: None
        }
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
                        VerticalThrustMode::Backwards => backward
                    };
                    if *fuel >= fuel_cost && should_fire  {
                        *fuel -= fuel_cost;
                        bodies.get_rigid_mut(MyHandle::Part(self.body_id)).unwrap().apply_force(0, &force, ForceType::Force, true)
                    }
                }
            }
        }
    }

    pub fn enable_attachment_colliders(&mut self, part_static: &PartStatic, collider_set: &mut super::MyColliderSet) {
        if self.attachment_colliders.is_none() {
            //let collision_groups = CollisionGroups::empty().with_membership(&ATTACHMENT_COLLIDER_COLLISION_GROUP).with_whitelist(groups)
            match self.kind {
                PartKind::Core => {
                    let colliders = [
                        Some(collider_set.insert(ColliderDesc::new(part_static.attachment_collider_cuboid.clone())
                            .position(Isometry::new(Vector2::new(0.0f32,0.5f32), 0f32))
                            .collision_groups(CollisionGroups::empty().with_membership(&ATTACHMENT_COLLIDER_COLLISION_GROUP))
                            //.sensor(true)
                            .build(BodyPartHandle(MyHandle::Part(self.body_id), 0)))),
                        Some(collider_set.insert(ColliderDesc::new(part_static.attachment_collider_cuboid.clone())
                            .position(Isometry::new(Vector2::new(0.5f32,0f32), -std::f32::consts::FRAC_PI_2))
                            //.sensor(true)
                            .collision_groups(CollisionGroups::empty().with_membership(&ATTACHMENT_COLLIDER_COLLISION_GROUP))
                            .sensor(true)
                            .build(BodyPartHandle(MyHandle::Part(self.body_id), 0)))),
                        Some(collider_set.insert(ColliderDesc::new(part_static.attachment_collider_cuboid.clone())
                            .position(Isometry::new(Vector2::new(0.0f32,-0.5f32), -std::f32::consts::PI))
                            .collision_groups(CollisionGroups::empty().with_membership(&ATTACHMENT_COLLIDER_COLLISION_GROUP))
                            //.sensor(true)
                            .build(BodyPartHandle(MyHandle::Part(self.body_id), 0)))),
                        Some(collider_set.insert(ColliderDesc::new(part_static.attachment_collider_cuboid.clone())
                            .position(Isometry::new(Vector2::new(-0.5f32,0f32), -std::f32::consts::PI - std::f32::consts::FRAC_PI_2))
                            .collision_groups(CollisionGroups::empty().with_membership(&ATTACHMENT_COLLIDER_COLLISION_GROUP))
                            //.sensor(true)
                            .build(BodyPartHandle(MyHandle::Part(self.body_id), 0))))
                    ];
                    self.attachment_colliders = Some(colliders);
                },
                _ => todo!()
            }
        } else { println!("Eanble attachment colliders called twice"); }
    }
    pub fn disable_attachment_colliders() {
        
    }
}

pub use crate::codec::PartKind;
impl PartKind {
    pub fn initialize(&self, bodies: &mut super::World, colliders: &mut super::MyColliderSet, part_static: &PartStatic) -> u16 {
        match self {
            PartKind::Core | PartKind::Hub => {
                let body = RigidBodyDesc::new().status(BodyStatus::Dynamic).local_inertia(self.inertia()).build();
                let id = bodies.add_part(body);
                let translation = if let PartKind::Hub = self { Vector2::new(0.0, 0.5) } else { Vector2::zero() };
                let collider = ColliderDesc::new(part_static.unit_cuboid.clone())
                    .translation(translation)
                    .build(BodyPartHandle (MyHandle::Part(id), 0));
                colliders.insert(collider);
                id
            },
            PartKind::Cargo => {
                let body = RigidBodyDesc::new().status(BodyStatus::Dynamic).local_inertia(self.inertia()).build();
                let id = bodies.add_part(body);
                let collider = ColliderDesc::new(part_static.cargo_cuboid.clone())
                    .translation(Vector2::new(0.0, 0.3563))
                    .build(BodyPartHandle(MyHandle::Part(id), 0));
                colliders.insert(collider);
                id
            },
            PartKind::LandingThruster => todo!()
        }
    }
    fn thrust(&self) -> Option<ThrustDetails> {
        match self {
            PartKind::Core => panic!("PartKind thrust called on core"),
            PartKind::Hub => None,
            PartKind::LandingThruster => Some(ThrustDetails{ fuel_cost: 3, force: Force2::linear_at_point(Vector2::new(0.0, 5.0), &Point2::new(0.0, 0.8)) }),
            PartKind::Cargo => None
        }
    }
    pub fn inertia(&self) -> Inertia2<MyUnits> {
        match self {
            PartKind::Core | PartKind::Hub => Inertia2::new(1.0,1.0),
            PartKind::Cargo => Inertia2::new(2.0, 2.0),
            _ => todo!()
        }
    }
    
    // pub fn get_attachable_positions(&self) -> [(Isometry<super::MyUnits>, )] {
        
    // }
}

#[derive(Copy, Clone, Debug)]
pub enum VerticalThrustMode { Forwards, Backwards }
#[derive(Copy, Clone, Debug)]
pub enum HorizontalThrustMode { Clockwise, CounterClockwise, Either }
#[derive(Copy, Clone, Debug)]
pub struct CompactThrustMode( u8 );
impl CompactThrustMode {
    pub fn new(horizontal: HorizontalThrustMode, vertical: VerticalThrustMode) -> CompactThrustMode {
        let horizontal: u8 = match horizontal {
            HorizontalThrustMode::Clockwise => 1,
            HorizontalThrustMode::CounterClockwise => 0,
            HorizontalThrustMode::Either => 2
        };
        let vertical: u8 = if let VerticalThrustMode::Forwards = vertical { 4 } else { 0 };
        CompactThrustMode (horizontal | vertical)
    }
    pub fn get_horizontal(&self) -> HorizontalThrustMode { 
        match self.0 & 0b00000011 {
            1 => HorizontalThrustMode::Clockwise,
            0 => HorizontalThrustMode::CounterClockwise,
            2 => HorizontalThrustMode::Either,
            _ => panic!()
        }
    }
    pub fn get_vertical(&self) -> VerticalThrustMode { if self.0 & 0b00001100 > 0 { VerticalThrustMode::Forwards } else { VerticalThrustMode::Backwards } }
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
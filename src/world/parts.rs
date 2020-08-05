use tinyvec::ArrayVec;
use nphysics2d::object::{RigidBody, Body, RigidBodyDesc, Collider, ColliderDesc, BodyPartHandle};
use nphysics2d::algebra::{Force2, ForceType};
use nphysics2d::math::Isometry;
use nalgebra::{Vector2, Point2};
use ncollide2d::shape::{Cuboid, ShapeHandle};
use super::{MyUnits, MyHandle};
use num_traits::identities::{Zero, One};

pub struct PartStatic {
    unit_cuboid: ShapeHandle<MyUnits>,
}
impl Default for PartStatic {
    fn default() -> PartStatic { PartStatic {
        unit_cuboid: ShapeHandle::new(Cuboid::new(Vector2::new(0.5, 0.5)))
    } }
}

pub struct Part {
    pub kind: PartKind,
    pub attachments: Vec<Part>,
    pub body: MyHandle,
}
impl Part {
    fn new(kind: PartKind, bodies: &mut super::World, colliders: &mut super::MyColliderSet, part_static: &PartStatic) -> Part {
        let body = kind.initialize(bodies, colliders, part_static);
        Part {
            kind, body,
            attachments: Vec::with_capacity(5),
        }
    }
}

pub enum PartKind {
    Core,
    Cargo,
    LandingThruster,
    Hub
}
impl PartKind {
    pub fn initialize(&self, bodies: &mut super::World, colliders: &mut super::MyColliderSet, part_static: &PartStatic) -> MyHandle {
        match self {
            PartKind::Core | PartKind::Hub => {
                let body = RigidBodyDesc::new().mass(1.0).build();
                let handle = bodies.add_part(body, None);
                let translation = if let PartKind::Hub = self { Vector2::new(0.0, 0.5) } else { Vector2::zero() };
                let collider = ColliderDesc::new(part_static.unit_cuboid.clone())
                    .translation(translation)
                    .build(BodyPartHandle (handle, 0));
                colliders.insert(collider);
                handle
            },
            PartKind::Cargo | PartKind::LandingThruster => todo!()
        }
    }
    // pub fn thrust(&self, body: &mut RigidBody<super::MyUnits>) {
    //     match self {
    //         PartKind::Core => (), //This one is fired elsewhere
    //         PartKind::Cargo | PartKind::Hub => (),
    //         PartKind::LandingThruster => { body.apply_force_at_local_point(0, &Vector2::new(0.0, 5.0), &Point2::new(0.0,0.5), ForceType::Force, true); }
    //     };
    // }
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
        match self.0 & 3 {
            1 => HorizontalThrustMode::Clockwise,
            0 => HorizontalThrustMode::CounterClockwise,
            2 => HorizontalThrustMode::Either,
            _ => panic!()
        }
    }
    pub fn get_vertical(&self) -> VerticalThrustMode { if self.0 & 4 > 0 { VerticalThrustMode::Forwards } else { VerticalThrustMode::Backwards } }
    pub fn get(&self) -> (HorizontalThrustMode, VerticalThrustMode) { (self.get_horizontal(), self.get_vertical()) }
    pub fn set_horizontal(&mut self, horizontal: HorizontalThrustMode) { std::mem::replace::<CompactThrustMode>(self, CompactThrustMode::new(horizontal, self.get_vertical())); }
    pub fn set_vertical(&mut self, vertical: VerticalThrustMode) { std::mem::replace::<CompactThrustMode>(self, CompactThrustMode::new(self.get_horizontal(), vertical)); }
    pub fn set(&mut self, horizontal: HorizontalThrustMode, vertical: VerticalThrustMode) { std::mem::replace::<CompactThrustMode>(self, CompactThrustMode::new(horizontal, vertical)); }
}
// enum FireDirection {
    
// }
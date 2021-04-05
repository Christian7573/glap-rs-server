use nphysics2d::object::{RigidBody, Body, RigidBodyDesc, Collider, ColliderDesc, BodyPartHandle, BodyStatus, DefaultColliderHandle};
use nphysics2d::algebra::{Force2, ForceType, Inertia2};
use nalgebra::{Vector2, Point2};
use ncollide2d::shape::{Cuboid, ShapeHandle};
use super::{MyUnits, MyHandle};
use num_traits::identities::{Zero, One};
use ncollide2d::pipeline::object::CollisionGroups;
use nphysics2d::joint::DefaultJointConstraintHandle;
use super::nphysics_types::*;
use crate::PlayerMeta;
use crate::codec::ToClientMsg;
use super::{WorldAddHandle, World, WorldlyObject};
use crate::session::WorldUpdatePartMove;
use std::sync::atomic::{AtomicU16, Ordering as AtomicOrdering};

lazy_static! {
    static ref UNIT_CUBOID: ShapeHandle<MyUnits> = ShapeHandle::new(Cuboid::new(Vector2::new(0.5, 0.5)));
    static ref CARGO_CUBOID: ShapeHandle<MyUnits> = ShapeHandle::new(Cuboid::new(Vector2::new(0.38, 0.5)));
    static ref SOLAR_PANEL_CUBOID: ShapeHandle<MyUnits> = ShapeHandle::new(Cuboid::new(Vector2::new(0.31, 0.5)));
    static ref ATTACHMENT_COLLIDER_CUBOID: ShapeHandle<MyUnits> = ShapeHandle::new(Cuboid::new(Vector2::new(1.0, 1.0)));
    static ref SUPER_THRUSTER_CUBOID: ShapeHandle<MyUnits> = ShapeHandle::new(Cuboid::new(Vector2::new(0.38, 0.44)));
}
static mut NEXT_PART_ID: AtomicU16 = AtomicU16::new(0);

pub const ATTACHMENT_COLLIDER_COLLISION_GROUP: [usize; 1] = [5];

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct RecursivePartDescription {
    pub kind: PartKind,
    pub attachments: Vec<Option<RecursivePartDescription>>,
}
pub struct Part {
    id: u16,
    body: MyRigidBody,
    collider: DefaultColliderHandle,
    kind: PartKind,
    attachments: [Option<PartAttachment>; 4],
    pub thrust_mode: CompactThrustMode,
    part_of_player: Option<u16>,
}
pub struct PartAttachment {
    part: MyHandle,
    connections: (DefaultJointConstraintHandle, DefaultJointConstraintHandle),
    //connections: DefaultJointConstraintHandle,
}

impl RecursivePartDescription {
    pub fn inflate(&self, bodies: &mut WorldAddHandle, colliders: &mut MyColliderSet, joints: &mut MyJointSet, initial_location: MyIsometry) -> MyHandle {
        self.inflate_component(bodies, colliders, joints, initial_location, AttachedPartFacing::Up, 0, 0, None)        
    }
    pub fn inflate_component(&self, bodies: &mut WorldAddHandle, colliders: &mut MyColliderSet, joints: &mut MyJointSet, initial_location: MyIsometry, true_facing: AttachedPartFacing, rel_part_x: i32, rel_part_y: i32, id: Option<u16>) -> MyHandle {
        let (body_desc, collider_desc) = self.kind.physics_components();
        let mut body = body_desc.build();
        body.set_position(initial_location.clone());
        let body_handle = bodies.add_later();
        let collider = colliders.insert(collider_desc.build(BodyPartHandle(body_handle, 0)));
        let mut attachments: [Option<PartAttachment>; 4] = [None, None, None, None];
        for i in 0..4 {
            attachments[i] = self.attachments.get(i).map(|o| o.as_ref()).flatten().map(|recursive_part| {
                if let Some(attachment) = self.kind.attachment_locations()[i] {
                    let attachment_location = PartAttachment::calculate_attachment_position(self.kind, &initial_location, i).unwrap();
                    let attachment_true_facing = attachment.facing.compute_true_facing(true_facing);
                    let (d_part_x, d_part_y) = attachment_true_facing.delta_rel_part();
                    let attachment_part_x = rel_part_x + d_part_x;
                    let attachment_part_y = rel_part_y + d_part_y;
                    let part = recursive_part.inflate_component(bodies, colliders, joints, attachment_location, attachment_true_facing, attachment_part_x, attachment_part_y, None);
                    Some(PartAttachment::inflate(part, self.kind, body_handle, i, joints))
                } else { None }
            }).flatten();
        };
        let my_part_id = if let Some(id) = id { id } else { unsafe { NEXT_PART_ID.fetch_add(1, AtomicOrdering::AcqRel) } };
        let part = Part {
            id: my_part_id,
            body,
            collider,
            kind: self.kind,
            attachments,
            thrust_mode: CompactThrustMode::calculate(true_facing, rel_part_x, rel_part_y),
            part_of_player: None,
        };
        bodies.add_its_later(body_handle, WorldlyObject::Part(part));
        body_handle
    }
}
impl From<PartKind> for RecursivePartDescription {
    fn from(kind: PartKind) -> RecursivePartDescription {
        RecursivePartDescription { kind, attachments: Vec::with_capacity(0) }        
    }
}

impl Part {
    pub fn join_to(&mut self, player: &mut PlayerMeta) {
        player.max_power += self.kind.power_storage();
        player.power_regen_per_5_ticks += self.kind.power_regen_per_5_ticks();
        self.part_of_player = Some(player.id);
    }
    pub fn remove_from(&mut self, player: &mut PlayerMeta) {
        player.max_power -= self.kind.power_storage();
        player.power_regen_per_5_ticks -= self.kind.power_regen_per_5_ticks();
        player.power = player.power.min(player.max_power);
        self.part_of_player = None;
    }
    pub fn part_of_player(&self) -> Option<u16> { self.part_of_player }
    pub fn mutate(mut self, mutate_into: PartKind, player: &mut Option<&mut PlayerMeta>, bodies: &mut MyBodySet, colliders: &mut MyColliderSet, joints: &mut MyJointSet) -> MyHandle {
        if let Some(player) = player { self.remove_from(player); }
        let mut old_attachments = self.attachments;
        let mut raw_attachments: [Option<MyHandle>; 4] = [None, None, None, None];
        for i in 0..4 {
            if let Some(attachment) = std::mem::replace(&mut old_attachments[i], None) {
                raw_attachments[i] = Some(attachment.deflate(joints));
            }
        };
        let position = self.body.position().clone();
        colliders.remove(self.collider);
        let mut add_handle = WorldAddHandle::from(bodies);
        let part_index = RecursivePartDescription::from(mutate_into).inflate_component(&mut add_handle, colliders, joints, position, AttachedPartFacing::Up, 0, 0, Some(self.id));
        let bodies = add_handle.deconstruct();
        let part = bodies.get_part_mut(part_index).unwrap();
        for i in 0..4 {
            if let Some(attachment) = &raw_attachments[i] {
                part.attach_part_player_agnostic(i, *attachment, part_index, joints);
            }
        }
        part.thrust_mode = self.thrust_mode;
        if let Some(player) = player { part.join_to(player) };
        part_index
    }
    pub fn deflate(&self, world: &MyBodySet) -> RecursivePartDescription {
        RecursivePartDescription {
            kind: self.kind,
            attachments: self.attachments[..].iter().map(|attachment| attachment.as_ref().map(|attachment| world.get_part(**attachment).unwrap().deflate(world))).collect()
        }
    }

    pub fn attach_part_player_agnostic(&mut self, attachment_slot: usize, part_handle: MyHandle, my_handle: MyHandle, joints: &mut MyJointSet) {
        //if self.kind.attachment_locations()[attachment_slot].is_none() { panic!("Can't attach to that slot") };
        if self.attachments[attachment_slot].is_some() { panic!("Already attached there"); }
        self.attachments[attachment_slot] = Some(PartAttachment::inflate(part_handle, self.kind, my_handle, attachment_slot, joints));
    }
    pub fn detach_part_player_agnostic(&mut self, attachment_slot: usize, joints: &mut MyJointSet) -> Option<MyHandle> {
        if let Some(part_attachment) = std::mem::replace(&mut self.attachments[attachment_slot], None) {
            Some(part_attachment.deflate(joints))
        } else { None }
    }

    pub fn thrust_no_recurse(&mut self, fuel: &mut u32, forward: bool, backward: bool, clockwise: bool, counter_clockwise: bool) {
        match self.kind {
            PartKind::Core => {
                if *fuel > 0 {
                    let body = &mut self.body;
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
                        self.body.apply_local_force(0, &force, ForceType::Force, true)
                    }
                }
            }
        }
    }

    pub fn find_cargo_recursive(&self, bodies: &MyBodySet) -> Option<(Option<MyHandle>, usize)> {
        for (i, attachment) in self.attachments.iter().enumerate() {
            if let Some(attachment) = attachment {
                let part = bodies.get_part(**attachment).expect("find_cargo_recursive: attached to body that didn't exist");
                if part.kind == PartKind::Cargo { return Some((None, i)) }
                else {
                    match part.find_cargo_recursive(bodies) {
                        Some((Some(parent_handle), attachment_slot)) => return Some((Some(parent_handle), attachment_slot)),
                        Some((None, attachment_slot)) => return Some((Some(**attachment), attachment_slot)),
                        None => ()
                    }
                }
            }
        }
        None
    }

    pub fn delete_recursive(mut self, bodies: &mut MyBodySet, colliders: &mut MyColliderSet, joints: &mut MyJointSet, removal_msgs: &mut Vec<ToClientMsg>) {
        colliders.remove(self.collider);
        removal_msgs.push(self.remove_msg());
        for attachment in self.attachments.iter_mut() {
            if let Some(attachment) = std::mem::replace(attachment, None) {
                let attachment = attachment.deflate(joints);
                bodies.delete_parts_recursive(attachment, colliders, joints, removal_msgs);
            }
        }
    }

    pub fn id(&self) -> u16 { self.id }
    pub fn kind(&self) -> PartKind { self.kind }
    pub fn body(&self) -> &MyRigidBody { &self.body }
    pub fn body_mut(&mut self) -> &mut MyRigidBody { &mut self.body }

    pub fn inflation_msgs(&self) -> [ToClientMsg; 3] {
        [ self.add_msg(), self.move_msg(), self.update_meta_msg() ]
    }
    pub fn add_msg(&self) -> ToClientMsg { ToClientMsg::AddPart { id: self.id, kind: self.kind } }
    pub fn move_msg(&self) -> ToClientMsg { ToClientMsg::MovePart {
        id: self.id, x: self.body.position().translation.x, y: self.body.position().translation.y,
        rotation_n: self.body.position().rotation.re, rotation_i: self.body.position().rotation.im
    } }
    pub fn update_meta_msg(&self) -> ToClientMsg { ToClientMsg::UpdatePartMeta { id: self.id, owning_player: self.part_of_player, thrust_mode: self.thrust_mode.into() } }
    pub fn remove_msg(&self) -> ToClientMsg { ToClientMsg::RemovePart { id: self.id } }

    pub fn physics_update_msg(&self, bodies: &MyBodySet, out: &mut Vec<WorldUpdatePartMove>) {
        let position = self.body.position();
        out.push(WorldUpdatePartMove {
            id: self.id,
            x: position.translation.x, y: position.translation.y,
            rot_cos: position.rotation.re, rot_sin: position.rotation.im
        });
        for attachment in &self.attachments {
            if let Some(attachment) = attachment {
                bodies.get_part(**attachment).unwrap().physics_update_msg(bodies, out);
            }
        }
    }

    pub fn attachments(&self) -> &[Option<PartAttachment>; 4] { &self.attachments }
}

impl PartAttachment {
    pub fn calculate_attachment_position(parent: PartKind, parent_location: &MyIsometry, attachment_slot: usize) -> Option<MyIsometry> {
        if let Some(attachment) = parent.attachment_locations()[attachment_slot] {
            Some(MyIsometry::new(parent_location.transform_point(&Point2::new(attachment.x, attachment.y)).coords, attachment.facing.part_rotation() + parent_location.rotation.angle()))
        } else {
            eprintln!("calculate_attachment_position: PartKind {:?} doesn't have attachment slot {}", parent, attachment_slot);
            None
        }
    }

    pub fn inflate(part: MyHandle, parent: PartKind, parent_body_handle: MyHandle, attachment_slot: usize, joints: &mut MyJointSet) -> PartAttachment {
        use nphysics2d::math::Point;
        let attachment = parent.attachment_locations()[attachment_slot].expect("PartAttachment tried to inflate on invalid slot");
        const HALF_CONNECTION_WIDTH: f32 = 0.5;
        let offset = (attachment.perpendicular.0 * HALF_CONNECTION_WIDTH, attachment.perpendicular.1 * HALF_CONNECTION_WIDTH);
        let constraint1 = nphysics2d::joint::FixedConstraint::new(
            BodyPartHandle(parent_body_handle.clone(), 0),
            BodyPartHandle(part, 0),
            Point::new(attachment.x + offset.0, attachment.y + offset.1),
            nalgebra::UnitComplex::new(0f32),
            Point::new(HALF_CONNECTION_WIDTH, 0f32),
            nalgebra::UnitComplex::new(-attachment.facing.part_rotation()),
        );
        let constraint2 = nphysics2d::joint::FixedConstraint::new(
            BodyPartHandle(parent_body_handle.clone(), 0),
            BodyPartHandle(part, 0),
            Point::new(attachment.x - offset.0, attachment.y - offset.1),
            nalgebra::UnitComplex::new(0f32),
            Point::new(-HALF_CONNECTION_WIDTH, 0f32),
            nalgebra::UnitComplex::new(-attachment.facing.part_rotation()),
        );
        PartAttachment {
            part,
            connections: (joints.insert(constraint1), joints.insert(constraint2))
            //connections: joints.insert(constraint),
        }
    }

    pub fn deflate(self, joints: &mut MyJointSet) -> MyHandle {
        joints.remove(self.connections.0);
        joints.remove(self.connections.1);
        self.part
    }

    pub fn is_broken(&self, joints: &MyJointSet) -> bool {
        return false;
        /*joints.get(self.connections.0).map(|joint| joint.is_broken()).unwrap_or(true)
        || joints.get(self.connections.1).map(|joint| joint.is_broken()).unwrap_or(true)*/
    }
}

impl std::ops::Deref for PartAttachment {
    type Target = MyHandle;
    fn deref(&self) -> &MyHandle { &self.part }
}

pub use crate::codec::PartKind;
impl PartKind {
    pub fn physics_components(&self) -> (RigidBodyDesc<MyUnits>, ColliderDesc<MyUnits>) {
        match self {
            _ => {
                (
                    RigidBodyDesc::new().status(BodyStatus::Dynamic).local_inertia(self.inertia()),
                    ColliderDesc::new( match self {
                        PartKind::Core | PartKind::Hub | PartKind::PowerHub | PartKind::HubThruster => UNIT_CUBOID.clone(),
                        PartKind::Cargo | PartKind::LandingThruster | PartKind::Thruster => CARGO_CUBOID.clone(),
                        PartKind::SolarPanel | PartKind::EcoThruster | PartKind::LandingWheel => SOLAR_PANEL_CUBOID.clone(), 
                        PartKind::SuperThruster => SUPER_THRUSTER_CUBOID.clone(),
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
            PartKind::EcoThruster => Some(ThrustDetails { fuel_cost: 1, force: Force2::linear_at_point(Vector2::new(0.0, -5.5), &Point2::new(0.0, 1.0)) }),
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
            PartKind::Cargo => 0, //CORE_MAX_POWER / 10,
            PartKind::LandingThruster | PartKind::HubThruster => CORE_MAX_POWER / 5,
            PartKind::Hub => CORE_MAX_POWER / 3,
            PartKind::SolarPanel => 0,
            PartKind::Thruster | PartKind::SuperThruster => CORE_MAX_POWER / 4,
            PartKind::EcoThruster => CORE_MAX_POWER / 6,
            PartKind::PowerHub => CORE_MAX_POWER / 3 * 2,
            PartKind::LandingWheel => 0,
        }
    }
    pub fn power_regen_per_5_ticks(&self) -> u32 {
        match self {
            PartKind::SolarPanel => 2,
            _ => 0,
        }
    }

    pub fn can_beamout(&self) -> bool {
        match self {
            PartKind::Cargo => false,
            _ => true
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
    pub fn compute_true_facing(&self, parent_true_facing: AttachedPartFacing) -> AttachedPartFacing {
        let parent_actual_rotation: u8 = parent_true_facing.into();
        let my_rotation: u8 = (*self).into();
        let num: u8 = parent_actual_rotation + my_rotation;
        if num > 3 { (num - 4).into() } else { num.into() }
    }
    pub fn delta_rel_part(&self) -> (i32,i32) {
        let new_x = match self { AttachedPartFacing::Left => -3, AttachedPartFacing::Right => 3, _ => 0 };
        let new_y = match self { AttachedPartFacing::Up => 3, AttachedPartFacing::Down => -3, _ => 0 };
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

    pub fn calculate(part_true_facing: AttachedPartFacing, rel_part_x: i32, rel_part_y: i32) -> CompactThrustMode {
        let x = rel_part_x; let y = rel_part_y;
        let hroizontal = match part_true_facing {
            AttachedPartFacing::Up => if x < 0 { HorizontalThrustMode::CounterClockwise } else if x > 0 { HorizontalThrustMode::Clockwise } else { HorizontalThrustMode::None },
            AttachedPartFacing::Right => if y > 0 { HorizontalThrustMode::CounterClockwise } else { HorizontalThrustMode::Clockwise },
            AttachedPartFacing::Down => if x < 0 { HorizontalThrustMode::Clockwise } else if x > 0 { HorizontalThrustMode::CounterClockwise } else { HorizontalThrustMode::None },
            AttachedPartFacing::Left => if y > 0 { HorizontalThrustMode::Clockwise } else { HorizontalThrustMode::CounterClockwise },
        };
        let vertical = match part_true_facing  {
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

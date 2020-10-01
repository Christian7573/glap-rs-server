use surf::Client;
use crate::world::{MyHandle, World};
use crate::world::parts::{Part, PartKind};
use serde::ser::{Serialize, Serializer};
use serde::de::{Deserialize, Deserializer, Error};
use crate::ApiDat;
use crate::world::Simulation;
use crate::world::parts::AttachedPartFacing;
use nphysics2d::object::{RigidBody, BodyPart, Body};
use nphysics2d::math::{Isometry, Vector};
use crate::rotate_vector_with_angle;
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub struct RecursivePartDescription {
    kind: PartKind,
    attachments: Vec<Option<RecursivePartDescription>>,
}
impl RecursivePartDescription {
    pub fn deflate(part: &Part, ) -> RecursivePartDescription {
        Self::deflate_recursive(part)
    }
    fn deflate_recursive(part: &Part) -> RecursivePartDescription {
        let mut attachments = Vec::with_capacity(part.attachments.len());
        for slot in 0..part.attachments.len() {
            attachments.push(part.attachments[slot].as_ref().map(|(part, _, _)| {
                if part.kind != PartKind::Cargo { Some(Self::deflate_recursive(part)) }
                else { None }
            }).flatten());
        };
        RecursivePartDescription {
            kind: part.kind,
            attachments
        }
    }
    pub fn inflate_root(&self, simulation: &mut Simulation, cx: f32, cy: f32) -> Part {
        Self::inflate_recursive(self, simulation, cx, cy, 0.0, AttachedPartFacing::Up)
    }
    fn inflate_recursive(&self, simulation: &mut Simulation, x: f32, y: f32, rot: f32, actual_facing: AttachedPartFacing) -> Part {
        let mut part = Part::new(self.kind, &mut simulation.world, &mut simulation.colliders, &simulation.part_static);
        simulation.world.get_rigid_mut(MyHandle::Part(part.body_id)).unwrap().set_position(Isometry::new(Vector::new(x, y), rot));
        
        for i in 0..part.attachments.len() {
            if let Some(Some(attachment)) = self.attachments.get(i) {
                if let Some(details) = self.kind.attachment_locations()[i] {
                    let rotated_attach_point = rotate_vector_with_angle(details.x, details.y, rot);
                    let new_actual_facing = details.facing.get_actual_rotation(actual_facing);
                    let new_part = Self::inflate_recursive(
                        attachment,
                        simulation,
                        x + rotated_attach_point.0,
                        y + rotated_attach_point.1,
                        rot + details.facing.part_rotation(),
                        new_actual_facing,
                    );
                    let (joint1, joint2) = simulation.equip_part_constraint(part.body_id, new_part.body_id, details);
                    part.attachments[i] = Some((new_part, joint1, joint2));
                }
            }
        };

        part
    }
}

impl Serialize for PartKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.val_of().serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for PartKind {
    fn deserialize<D: Deserializer<'de>>(deserilizer: D) -> Result<Self, D::Error> {
        let dat = [ u8::deserialize(deserilizer)? ];
        Self::deserialize(&dat, &mut 0).map_err(|_| D::Error::custom("Failed to deserialize PartKind"))
    }
}

pub fn spawn_beamout_request(session_id: Option<String>, beamout_layout: RecursivePartDescription, api: Option<Arc<ApiDat>>) {
    if let Some(api) = &api {
        if let Some(session_id) = session_id {
            let uri = api.beamout.clone() + "?session=" + &session_id;
            async_std::task::spawn(async {
                let beamout_layout = beamout_layout;
                match surf::post(uri).body(serde_json::to_string(&beamout_layout).unwrap()).await {
                    Ok(res) if !res.status().is_success() => { eprintln!("Beamout post does not indicate success"); },
                    Err(err) => { eprintln!("Beamout post failed\n{}", err); },
                    _ => {}
                };
            });
        }
    } 
}

pub async fn beamin_request(session_id: Option<String>, api: Option<Arc<ApiDat>>) -> Option<RecursivePartDescription> {
    let api = api.as_ref()?;
    let session_id = session_id?;
    let uri = api.beamin.clone() + "?session=" + &session_id;
    let mut response = surf::get(uri).await.ok()?;
    if response.status().is_success() {
        let body_json = response.body_json().await.ok()?;
        serde_json::from_value::<RecursivePartDescription>(body_json).ok()
    } else { None }
}

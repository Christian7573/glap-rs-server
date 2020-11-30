use surf::Client;
use crate::world::{MyHandle, World};
use crate::world::parts::{Part, PartKind, CompactThrustMode};
use serde::ser::{Serialize, Serializer};
use serde::de::{Deserialize, Deserializer, Error};
use crate::ApiDat;
use crate::world::Simulation;
use crate::world::parts::AttachedPartFacing;
use nphysics2d::object::{RigidBody, BodyPart, Body};
use nphysics2d::math::{Isometry, Vector};
use crate::rotate_vector_with_angle;
use std::sync::Arc;
use futures::FutureExt;

#[derive(Serialize, Deserialize)]
pub struct RecursivePartDescription {
    pub kind: PartKind,
    pub attachments: Vec<Option<RecursivePartDescription>>,
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
    pub fn inflate_root(&self, simulation: &mut Simulation, cx: f32, cy: f32, planet_radius: Option<f32>, rand: &mut rand::rngs::ThreadRng) -> Part {
        let mut margins = 0f32;
        let part = Self::inflate_recursive(self, simulation, cx, cy, 0.0, AttachedPartFacing::Up, cx, cy, &mut margins, 0, 0);
        if let Some(radius) = planet_radius {
            use rand::Rng;
            let spawn_degrees: f32 = rand.gen::<f32>() * std::f32::consts::PI * 2.0;
            let spawn_radius = radius * 1.25 + 1.0 + margins;
            let spawn_center = (spawn_degrees.cos() * spawn_radius + cx, spawn_degrees.sin() * spawn_radius + cy);
            let core_position = simulation.world.get_rigid(MyHandle::Part(part.body_id)).unwrap().position();
            let core_position = (core_position.translation.x, core_position.translation.y);

            fn recursive_part_move(part: &Part, core_position: (f32, f32), spawn_center: (f32, f32), spawn_degrees: f32, simulation: &mut Simulation) {
                let body = simulation.world.get_rigid_mut(MyHandle::Part(part.body_id)).unwrap();
                let pos = body.position().translation;
                let vec_from_core = (pos.x - core_position.0, pos.y - core_position.1);
                let rotated_vec = rotate_vector_with_angle(vec_from_core.0, vec_from_core.1, spawn_degrees);
                let new_pos = (rotated_vec.0 + spawn_center.0, rotated_vec.1 + spawn_center.1);
                let new_rotation = body.position().rotation.angle() + spawn_degrees;
                body.set_position(Isometry::new(Vector::new(new_pos.0, new_pos.1), new_rotation));
                for i in 0..part.attachments.len() {
                    if let Some((attachment, _, _)) = &part.attachments[i] { recursive_part_move(attachment, core_position, spawn_center, spawn_degrees, simulation); }
                }
            }
            recursive_part_move(&part, core_position, spawn_center, spawn_radius, simulation);
        };
        part
    }
    fn inflate_recursive(&self, simulation: &mut Simulation, x: f32, y: f32, rot: f32, actual_facing: AttachedPartFacing, cx: f32, cy: f32, margins: &mut f32, attach_x: i16, attach_y: i16) -> Part {
        let mut part = Part::new(self.kind, &mut simulation.world, &mut simulation.colliders, &simulation.part_static);
        simulation.world.get_rigid_mut(MyHandle::Part(part.body_id)).unwrap().set_position(Isometry::new(Vector::new(x, y), rot));
        *margins = margins.max((x-cx).abs()).max((y-cy).abs());
        part.thrust_mode = CompactThrustMode::calculate(actual_facing, attach_x, attach_y);
        
        for i in 0..part.attachments.len() {
            if let Some(Some(attachment)) = self.attachments.get(i) {
                if let Some(details) = self.kind.attachment_locations()[i] {
                    let rotated_attach_point = rotate_vector_with_angle(details.x, details.y, rot);
                    let new_actual_facing = details.facing.get_actual_rotation(actual_facing);
                    let deltas = new_actual_facing.attachment_offset();
                    let new_part = Self::inflate_recursive(
                        attachment,
                        simulation,
                        x + rotated_attach_point.0,
                        y + rotated_attach_point.1,
                        rot + details.facing.part_rotation(),
                        new_actual_facing,
                        cx, cy, margins,
                        attach_x + deltas.0, attach_y + deltas.1,
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
        let dat = u8::deserialize(deserilizer)?;
        match Self::deserialize(&mut futures::stream::once(futures::future::ready(dat))).now_or_never() {
            Some(Ok(kind)) => Ok(kind),
            _ => Err(D::Error::custom("Failed to deserialize PartKind"))
        }
    }
}

/*macro_rules! FormatWithString {
    ($format_input:ident) => { {
        
    } }
}*/

pub fn spawn_beamout_request(beamout_token: Option<String>, beamout_layout: RecursivePartDescription, api: Option<Arc<ApiDat>>) {
    if let Some(api) = &api {
        if let Some(beamout_token) = beamout_token {
            let uri = api.beamout.replacen("^^^^", &beamout_token, 1);
            let password = api.password.clone();
            async_std::task::spawn(async {
                let beamout_layout = beamout_layout;
                match surf::post(uri).header("password", password).body(serde_json::to_string(&beamout_layout).unwrap()).await {
                    Ok(res) if !res.status().is_success() => { eprintln!("Beamout post does not indicate success {}", res.status()); },
                    Err(err) => { eprintln!("Beamout post failed\n{}", err); },
                    _ => {}
                };
            });
        } else { println!("Session didn't have beamout token"); }
    } 
}

#[derive(Serialize, Deserialize)]
pub struct BeaminResponse {
    pub is_admin: bool,
    pub beamout_token: String,
    pub layout: Option<RecursivePartDescription>
}

pub async fn beamin_request(session: Option<String>, api: Option<Arc<ApiDat>>) -> Option<BeaminResponse> {
    let api = api.as_ref()?;
    let session = session?;
    let uri = api.beamin.replacen("^^^^", &session, 1);
    let password = api.password.clone();
    let mut response = surf::get(uri).header("password", password).await.ok()?;
    if response.status().is_success() {
        let body_json = response.body_json().await.ok()?;
        serde_json::from_value::<BeaminResponse>(body_json).ok()
    } else { eprintln!("Beamin bad {}", response.status()); None }
}

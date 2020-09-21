use surf::Client;
use crate::world::{MyHandle, World};
use crate::world::parts::{Part, PartKind};
use serde::ser::{Serialize, Serializer};
use serde::de::{Deserialize, Deserializer, Error};

#[derive(Serialize, Deserialize)]
pub struct RecursivePartDescription {
    kind: PartKind,
    dx: f32, dy: f32,
    drot: f32,
    attachments: Vec<Option<RecursivePartDescription>>,
}
impl RecursivePartDescription {
    pub fn deflate(part: &Part, world: &World) -> RecursivePartDescription {
        let pos = world.get_rigid(MyHandle::Part(part.body_id)).unwrap().position();
        Self::deflate_recursive(part, world, pos.translation.x, pos.translation.y, pos.rotation.angle())
    }
    fn deflate_recursive(part: &Part, world: &World, ox: f32, oy: f32, orot: f32) -> RecursivePartDescription {
        let mut attachments = Vec::with_capacity(part.attachments.len());
        for slot in 0..part.attachments.len() {
            attachments.push(part.attachments[slot].as_ref().map(|(part, _, _)| {
                if part.kind != PartKind::Cargo { Some(Self::deflate_recursive(part, world, ox, oy, orot)) }
                else { None }
            }).flatten());
        };
        let pos = world.get_rigid(MyHandle::Part(part.body_id)).unwrap().position();
        RecursivePartDescription {
            kind: part.kind,
            dx: pos.translation.x - ox,
            dy: pos.translation.y - oy,
            drot: pos.rotation.angle() - orot,
            attachments
        }
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


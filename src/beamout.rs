use crate::world::parts::{RecursivePartDescription, PartKind};
use serde::ser::{Serialize, Serializer};
use serde::de::{Deserialize, Deserializer, Error};
use crate::ApiDat;
use std::sync::Arc;
use futures::FutureExt;
use async_std::task::JoinHandle;


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

pub fn spawn_beamout_request(beamout_token: String, mut beamout_layout: RecursivePartDescription, api: Arc<ApiDat>) -> JoinHandle<()> {
    fn recurse_can_beamout(part: &mut RecursivePartDescription) {
        for attachment in &mut part.attachments {
            if let Some(part) = attachment {
                if !part.kind.can_beamout() { *attachment = None }
                else { recurse_can_beamout(part) }
            }
        }
    }
    recurse_can_beamout(&mut beamout_layout);

    let uri = api.beamout.replacen("^^^^", &beamout_token, 1);
    let password = api.password.clone();
    async_std::task::spawn(async move {
        let beamout_layout = beamout_layout;
        match surf::post(uri).header("password", password).body(serde_json::to_string(&beamout_layout).unwrap()).await {
            Ok(res) if !res.status().is_success() => { eprintln!("Beamout post for {} does not indicate success {}", beamout_token, res.status()); },
            Err(err) => { eprintln!("Beamout post failed for {}\n{}", beamout_token, err); },
            _ => {}
        };
    })
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct BeaminResponse {
    pub is_admin: bool,
    pub beamout_token: String,
    pub layout: Option<RecursivePartDescription>
}

pub async fn beamin_request(session: String, api: Arc<ApiDat>) -> Result<BeaminResponse, Box<dyn std::error::Error>> {
    let api = api.as_ref();
    let session = session;
    let uri = api.beamin.replacen("^^^^", &session, 1);
    let password = api.password.clone();
    let mut response = surf::get(uri).header("password", password).await?;
    if response.status().is_success() {
        let body_json = response.body_json().await?;
        serde_json::from_value::<BeaminResponse>(body_json).map_err(|err| Box::new(err) as Box<dyn std::error::Error>)
    } else { eprintln!("Beamin bad for {}\n{}", session, response.status()); Err(Box::new(serde_json::Error::custom("Invalid response"))) }
}

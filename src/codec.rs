pub use {rmp_serde::to_vec as serialize, rmp_serde::from_slice as deserialize};
use serde::{Serialize, Deserialize};
use async_tungstenite::tungstenite::Message;

#[derive(Serialize, Debug)]
pub enum ToClientMsg {
    HandshakeAccepted,
    AddCelestialObject { name: String, display_name: String, position: (f32, f32), radius: f32 }
}

#[derive(Deserialize, Debug)]
pub enum FromClientMsg {
    Handshake { client: String, session: Option<String> }
}

pub fn accept_handshake() -> Message { Message::Binary(serialize( &ToClientMsg::HandshakeAccepted ).unwrap()) }
pub fn add_celestial_object(name: String, display_name: String, position: (f32, f32), radius: f32) -> Message {
    Message::Binary(serialize( &ToClientMsg::AddCelestialObject{name, display_name, position, radius} ).unwrap())
}
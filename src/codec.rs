pub use {rmp_serde::to_vec as serialize, rmp_serde::from_slice as deserialize};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Debug)]
pub enum ToClientMsg {
    
}

#[derive(Deserialize, Debug)]
pub enum FromClientMsg {
    Handshake(String, Option<String>)
}
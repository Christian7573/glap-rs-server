use std::collections::BTreeMap;
use std::pin::Pin;
use std::task::{Poll, Context};
use async_std::prelude::*;
use futures::select_biased;
use async_std::net::{TcpStream, TcpListener};
use async_std::sync::{Sender, Receiver, channel};
use futures::{Sink, SinkExt, Stream, StreamExt, FutureExt};
use nphysics2d::object::{Body, BodySet, RigidBody};
use super::world::nphysics_types::{MyHandle, MyUnits};
use super::world::parts::{Part, PartKind};
use std::ops::{Deref, DerefMut};
use crate::beamout::{BeaminResponse, beamin_request, spawn_beamout_request};
use crate::world::parts::RecursivePartDescription;
use crate::ApiDat;
use std::sync::Arc;
use std::time::Duration;

use crate::codec::*;

pub mod websocket;
use websocket::*;

pub enum ToGameEvent {
    NewPlayer { id: u16, name: String, parts: RecursivePartDescription, beamout_token: Option<String> },
    PlayerMessage { id: u16, msg: ToServerMsg },
    PlayerQuit { id: u16 },
    AdminCommand { id: u16, command: String }
}
pub enum ToSerializerEvent {
    Message (u16, ToClientMsg),
    MulticastMessage (Vec<u16>, ToClientMsg),
    Broadcast (ToClientMsg),
    WorldUpdate (BTreeMap<u16, ((f32,f32), (f32, f32), Vec<WorldUpdatePartMove>, ToClientMsg)>, Vec<WorldUpdatePartMove>),

    NewWriter (u16, Sender<Vec<OutboundWsMessage>>),
    RequestUpdate (u16),
    SendPong (u16),
    //BeamoutWriter (u16, RecursivePartDescription),
    DeleteWriter (u16),
}

pub struct WorldUpdatePartMove {
    pub id: u16,
    pub x: f32,
    pub y: f32,
    pub rot_sin: f32,
    pub rot_cos: f32,
}
pub struct WorldUpdatePlayerUpdate { pub id: u16, pub core_x: f32, pub core_y: f32, pub parts: Vec<WorldUpdatePartMove> }

/*enum Event {
    NewSocket { socket: TcpStream },
    PotentialSessionMessage { id: u16, msg: Vec<u8> },
    PotentialSessionBeamin { id: u16, parts: Option<RecursivePartDescription>, beamout_token: Option<String> },
    PotentialSessionDisconnect { id: u16 },
    SessionMessage { id: u16, msg: Vec<u8> },
    SessionDisconnect { id: u16 },
    OutboundEvent(Vec<OutboundEvent>)
}*/

pub enum GuarenteeOnePoll {
    Yesnt, Yes
}
impl Future for GuarenteeOnePoll {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<()> {
        match *self {
            GuarenteeOnePoll::Yesnt => { *self = GuarenteeOnePoll::Yes; ctx.waker().wake_by_ref(); Poll::Pending },
            GuarenteeOnePoll::Yes => Poll::Ready(())
        }
    }
}

pub async fn incoming_connection_acceptor(listener: TcpListener, to_game: Sender<ToGameEvent>, to_serializer: Sender<Vec<ToSerializerEvent>>, api: Option<Arc<ApiDat>>) {
    println!("Hello from incomming connection acceptor");
    let mut next_client_id: u16 = 1;
    while let Ok((socket, addr)) = listener.accept().await {
        let client_id = next_client_id;
        next_client_id += 1;

        let to_game = to_game.clone();
        let to_serializer = to_serializer.clone();
        let api = api.clone();

        async_std::task::Builder::new()
            .name(format!("inbound_{:?}", addr).to_string())
            .spawn(socket_reader(client_id, socket, addr, to_game, to_serializer, api)).expect("Failed to launch inbound");
    }
    panic!("Incoming connections closed");
}

async fn socket_reader(id: u16, socket: TcpStream, addr: async_std::net::SocketAddr, to_game: Sender<ToGameEvent>, to_serializer: Sender<Vec<ToSerializerEvent>>, api: Option<Arc<ApiDat>>) -> Result<(),()> {
    println!("New socket from {:?}", addr);
    let (mut socket_in, mut socket_out) = accept_websocket(socket).await?;
    println!("Accepted websocket");
    let mut first_msg = loop {
        match read_ws_message(&mut socket_in).await {
            Ok(WsEvent::Ping) => { socket_out.queue_send(pong_message().0); },
            Ok(WsEvent::Message(msg)) => break Ok(msg),
            Ok(WsEvent::Pong) | Err(_) => break Err(()),
        }
    }?;
    let first_msg = ToServerMsg::deserialize(&mut first_msg).await?;
    let (session, name) = if let ToServerMsg::Handshake{ session, client, name} = first_msg { (session, name) }
    else { return Err(()) };
    let name = {
        let tmp_name = name.trim();
        if tmp_name.is_empty() { "Unnamed".to_owned() }
        else { tmp_name.to_owned() }
    };


    let beamin_data = beamin_request(session.clone(), api.clone()).await;

    let layout: Option<RecursivePartDescription>;
    let mut is_admin: bool;
    let beamout_token: Option<String>;
    if let Some(beamin_data) = beamin_data {
        layout = beamin_data.layout;
        is_admin = beamin_data.is_admin;
        beamout_token = Some(beamin_data.beamout_token);
    } else {
        layout = None;
        is_admin = false;
        beamout_token = None;
    }

    let layout = layout.unwrap_or( RecursivePartDescription { kind: PartKind::Core, attachments: Vec::new() } );                                   
    to_game.send(ToGameEvent::NewPlayer { id, name: name.clone(), parts: layout, beamout_token }).await;
    let (to_writer, from_serializer) = channel::<Vec<OutboundWsMessage>>(50);
    async_std::task::Builder::new()
        .name(format!("outbound_${}", id))
        .spawn(socket_writer(id, socket_out, from_serializer)).expect("Failed to launch outbound");
    to_serializer.send(vec! [ToSerializerEvent::NewWriter(id, to_writer)]).await;

    loop {
        match read_ws_message(&mut socket_in).await {
            Ok(WsEvent::Message(mut msg)) => {
                let msg = ToServerMsg::deserialize(&mut msg).await;
                match msg {
                    Ok(ToServerMsg::SendChatMessage { msg }) => {
                        if msg.chars().nth(0).unwrap() == '/' {
                            let chunks: Vec<String> = msg.split_whitespace().map(|s| s.to_string()).collect();
                            match chunks[0].as_str() {
                                "/shrug" => {
                                    to_serializer.send(vec! [ToSerializerEvent::Broadcast(ToClientMsg::ChatMessage{ username: name.clone(), msg: String::from("¯\\_(ツ)_/¯"), color: String::from("#dd55ff") })]).await;
                                },
                                
                                _ => {
                                    if is_admin {
                                        to_game.send(ToGameEvent::AdminCommand { id, command: msg.clone() }).await;
                                    } else {
                                        to_serializer.send(vec! [ToSerializerEvent::Message(id, ToClientMsg::ChatMessage{ username: String::from("Server"), msg: String::from("You cannot use that command"), color: String::from("#FF0000") })]).await;
                                    }
                                }
                            }
                        } else {
                            to_serializer.send(vec! [ToSerializerEvent::Broadcast(ToClientMsg::ChatMessage{ username: name.clone(), msg, color: String::from("#dd55ff") })]).await;
                        }
                    },
                    Ok(ToServerMsg::RequestUpdate) => { to_serializer.send(vec! [ToSerializerEvent::RequestUpdate(id)]).await; },
                    Ok(msg) => { to_game.send(ToGameEvent::PlayerMessage { id, msg }).await; },
                    Err(_) => break,
                };
            },
            Ok(WsEvent::Ping) => { println!("Ponged"); to_serializer.send(vec! [ToSerializerEvent::SendPong(id)]).await; },
            Ok(WsEvent::Pong) | Err(_) => break,
        };
    };

    to_serializer.send(vec! [ToSerializerEvent::DeleteWriter(id)]).await;
    Ok(())
}

pub async fn serializer(mut to_me: Receiver<Vec<ToSerializerEvent>>, to_game: Sender<ToGameEvent>) {
    println!("Hello from serializer task");
    let mut writers: BTreeMap<u16, (Sender<Vec<OutboundWsMessage>>, Vec<OutboundWsMessage>, bool)> = BTreeMap::new();
    while let Some(events) = to_me.next().await {
        for event in events {
            match event {
                ToSerializerEvent::NewWriter(id, to_writer) => {
                    writers.insert(id, (to_writer, Vec::new(), false));
                },
                ToSerializerEvent::DeleteWriter(id) => {
                    if let Some((writer, mut queue, _request_update)) = writers.remove(&id) {
                        queue.push(websocket::close_message());
                        writer.send(queue).await;
                        to_game.send(ToGameEvent::PlayerQuit { id }).await;
                    }
                },
                ToSerializerEvent::RequestUpdate(id) => {
                    if let Some((_to_writer, _queue, request_update)) = writers.get_mut(&id) {
                        *request_update = true;
                    }
                },
                ToSerializerEvent::SendPong(id) => {
                    if let Some((to_writer, _queue, _request_update)) = writers.get_mut(&id) {
                        to_writer.send(vec! [pong_message()]).await;
                    }
                },

                ToSerializerEvent::Message(id, msg) => {
                    if let Some((_writer, queue, _request_update)) = writers.get_mut(&id) {
                        let mut out = Vec::new();
                        msg.serialize(&mut out);
                        let out = (&out).into();
                        queue.push(out);
                    }
                },
                ToSerializerEvent::MulticastMessage(ids, msg) => {
                    let mut out = Vec::new();
                    msg.serialize(&mut out);
                    let out = OutboundWsMessage::from(&out);
                    for id in ids {
                        if let Some((_writer, queue, _request_update)) = writers.get_mut(&id) {
                            queue.push(out.clone());
                        }
                    }
                },
                ToSerializerEvent::Broadcast(msg) => {
                    let mut out = Vec::new();
                    msg.serialize(&mut out);
                    let out = OutboundWsMessage::from(&out);
                    for (_writer, queue, _request_update) in writers.values_mut() {
                        queue.push(out.clone());
                    }
                },
                ToSerializerEvent::WorldUpdate(players, free_parts) => {
                    for (id, ((x, y), (vel_x, vel_y), parts, post_simulation)) in &players {
                        let mut msg = Vec::new();
                        ToClientMsg::MessagePack { count: parts.len() as u16 + 1 }.serialize(&mut msg);
                        ToClientMsg::UpdatePlayerVelocity { id: *id, vel_x: *vel_x, vel_y: *vel_y }.serialize(&mut msg);
                        for part in parts {
                            ToClientMsg::MovePart {
                                id: part.id, x: part.x, y: part.y,
                                rotation_n: part.rot_cos, rotation_i: part.rot_sin,
                            }.serialize(&mut msg);
                        };
                        let msg = OutboundWsMessage::from(&msg);
                        for (id, (_to_writer, queue, request_update)) in &mut writers {
                            if *request_update {
                                if let Some(((player_x, player_y), (_vel_x, _vel_y), _parts, _post_simulation)) = players.get(&id) {
                                    if (player_x - x).abs() <= 200.0 && (player_y - y).abs() <= 200.0 {
                                        queue.push(msg.clone());
                                    }
                                }
                            }
                        };
                        if let Some((_to_writer, queue, request_update)) = writers.get_mut(id) {
                            if *request_update {
                                let mut msg = Vec::new();
                                post_simulation.serialize(&mut msg);
                                queue.push(OutboundWsMessage::from(&msg));
                            }
                        };
                    };
                    let mut msg = Vec::new();
                    ToClientMsg::MessagePack { count: free_parts.len() as u16 }.serialize(&mut msg);
                    for part in free_parts {
                        ToClientMsg::MovePart {
                            id: part.id, x: part.x, y: part.y,
                            rotation_n: part.rot_cos, rotation_i: part.rot_sin,
                        }.serialize(&mut msg);
                    };
                    let msg = OutboundWsMessage::from(&msg);
                    for (_to_writer, queue, request_update) in writers.values_mut() {
                        if *request_update {
                            queue.push(msg.clone());
                        }
                        *request_update = false;
                    };
                },
            }
        }
        for (to_writer, queue, _needs_update) in writers.values_mut() {
            to_writer.send(std::mem::replace(queue, Vec::new())).await;
        }
    };
}

async fn socket_writer(_id: u16, mut out: TcpWriter, mut from_serializer: Receiver<Vec<OutboundWsMessage>>) {
    loop {
        select_biased! {
            messages = from_serializer.next().fuse() => {
                if let Some(messages) = messages {
                    for msg in messages { out.queue_send(msg.0.clone()); };
                } else {
                    break;
                };
            },
            writing = (&mut out).fuse() => {
                if writing.is_err() { break; }
            }
        };
        if let Some(messages) = from_serializer.next().await {
            for msg in messages {  out.queue_send(msg.0.clone()); };
        } else {
            break;
        }
    }
    out.await; //Flush
}

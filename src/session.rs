use std::pin::Pin;
use std::task::{Poll, Context};
use async_std::prelude::*;
use async_std::net::TcpStream;
use async_tungstenite::WebSocketStream;
use async_tungstenite::tungstenite::{Error as WsError, Message};
use futures::{Sink, SinkExt, Stream, StreamExt};
use nphysics2d::object::{Body, BodySet, RigidBody};
use super::world::{MyHandle, MyUnits};
use super::world::parts::{Part, PartKind};
use std::collections::BTreeMap;

use crate::codec::*;

pub struct PartMoveMessage {
    msg: Vec<u8>,
    x: f32, y: f32
}
impl PartMoveMessage {
    pub fn new(id: u16, body: &RigidBody<MyUnits>) -> PartMoveMessage {
        PartMoveMessage {
            x: body.position().translation.x, y: body.position().translation.y,
            msg: ToClientMsg::MovePart{ 
                id,
                x: body.position().translation.x, y: body.position().translation.y,
                rotation_n: body.position().rotation.re, rotation_i: body.position().rotation.im
            }.serialize()
        }
    }
    pub fn new_all(parts: &BTreeMap<u16, RigidBody<MyUnits>>) -> Vec<PartMoveMessage> {
        parts.iter().map(|(id, body)| Self::new(*id, body)).collect()
    }
}

pub enum Session {
    AcceptingWebSocket(Pin<Box<dyn Future<Output = Result<WebSocketStream<TcpStream>, async_tungstenite::tungstenite::Error>>>>),
    AwaitingHandshake(MyWebSocket),
    Spawned(MyWebSocket, PlayerMeta)
}
pub enum SessionEvent {
    ReadyToSpawn,
    ThrusterUpdate{ forward: bool, backward: bool, clockwise: bool, counter_clockwise: bool }
}
#[derive(Clone)]
pub struct PlayerMeta {
    pub thrust_forwards: bool,
    pub thrust_backwards: bool,
    pub thrust_clockwise: bool,
    pub thrust_counterclockwise: bool,
    pub fuel: u16,
    pub max_fuel: u16
}
impl Default for PlayerMeta {
    fn default() -> PlayerMeta { PlayerMeta {
        thrust_backwards: false, thrust_clockwise: false, thrust_counterclockwise: false, thrust_forwards: false,
        fuel: 100 * crate::TICKS_PER_SECOND as u16, max_fuel: 100 * crate::TICKS_PER_SECOND as u16
    } }
}

impl Session {
    pub fn new(socket: TcpStream) -> Session {
        let future = async_tungstenite::accept_async(socket);
        let pinbox;
        unsafe { pinbox = Pin::new_unchecked(Box::new(future)); }
        Session::AcceptingWebSocket(pinbox)
    }
    pub fn update_world(&mut self, move_messages: &Vec<PartMoveMessage>, random_broadcast_messages: &Vec<Vec<u8>>) {
        if let Session::Spawned(socket, myself) = self {
            for msg in move_messages {
                //Do some check here in the future, don't need to send messages about things that are really far away
                socket.queue_send(Message::Binary(msg.msg.clone()));
            }
            socket.queue_send(Message::Binary(ToClientMsg::PostSimulationTick{ your_fuel: myself.fuel }.serialize()));
            for msg in random_broadcast_messages { socket.queue_send(Message::Binary(msg.clone())); };
            //println!("{}", myself.fuel);
        }
    }
}
impl Stream for Session {
    type Item = SessionEvent;
    fn poll_next(
        mut self: Pin<&mut Self>,
        ctx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match &mut *self {

            Session::AcceptingWebSocket(future) => {
                if let Poll::Ready(result) = future.as_mut().poll(ctx) {
                    if let Ok(stream) = result {
                        let mut socket: MyWebSocket = stream.into();
                        println!("Accepted websocket");
                        std::mem::replace(self.get_mut(), Session::AwaitingHandshake(socket));
                        Poll::Pending
                    } else {
                        Poll::Ready(None)
                    }
                } else { Poll::Pending }
            },

            Session::AwaitingHandshake(stream) => {
                if let Poll::Ready(result) = stream.poll_next_unpin(ctx) {
                    match result {
                        Some(Message::Binary(dat)) => {
                            match ToServerMsg::deserialize(dat.as_slice(), &mut 0) {
                                Ok(ToServerMsg::Handshake{ client, session }) => {
                                    Poll::Ready(Some(SessionEvent::ReadyToSpawn))
                                },
                                _ => Poll::Ready(None)
                            }
                        },
                        Some(Message::Ping(_)) => Poll::Pending,
                        _ => { Poll::Ready(None) }
                    }
                } else { Poll::Pending }
            },

            Session::Spawned(socket, player) => {
                if let Poll::Ready(result) = socket.poll_next_unpin(ctx) {
                    match result {
                        Some(Message::Binary(dat)) => {
                            match ToServerMsg::deserialize(dat.as_slice(), &mut 0) {
                                Ok(ToServerMsg::SetThrusters { forward, backward, clockwise, counter_clockwise }) => {
                                    if player.fuel > 0 {
                                        player.thrust_forwards = forward;
                                        player.thrust_backwards = backward;
                                        player.thrust_clockwise = clockwise;
                                        player.thrust_counterclockwise = counter_clockwise;
                                        Poll::Ready(Some(SessionEvent::ThrusterUpdate { forward, backward, clockwise, counter_clockwise }))
                                    } else { Poll::Pending }
                                },
                                Err(_) => { todo!() },
                                Ok(ToServerMsg::Handshake { client, session }) => { todo!() }
                            }
                        },
                        Some(Message::Ping(_)) => Poll::Pending,
                        _ => Poll::Ready(None)
                    }
                } else { Poll::Pending }
            }
        }
    }
    
}
pub struct MyWebSocket {
    socket: WebSocketStream<TcpStream>
}
impl Stream for MyWebSocket {
    type Item = Message;
    fn poll_next(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Option<Message>> {
        //Flush
        Pin::new(&mut self.socket).poll_flush(ctx);
        //Read
        if let Poll::Ready(result) = Pin::new(&mut self.socket).poll_next(ctx) {
            if let Some(Ok(message)) = result { Poll::Ready(Some(message)) }
            else { Poll::Ready(None) }
        } else { Poll::Pending }
    }
}
impl From<WebSocketStream<TcpStream>> for MyWebSocket {
    fn from(socket: WebSocketStream<TcpStream>) -> MyWebSocket { MyWebSocket { socket } }
}
impl MyWebSocket {
    pub fn queue_send(&mut self, message: Message) {
        match self.socket.start_send_unpin(message) {
            Err(WsError::SendQueueFull(msg)) => panic!("Send queue full. Implement own queue and start using poll_ready. Msg: {}", msg),
            _ => ()
        };
    }
}
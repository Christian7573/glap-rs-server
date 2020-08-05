use std::pin::Pin;
use std::task::{Poll, Context};
use async_std::prelude::*;
use async_std::net::TcpStream;
use async_tungstenite::WebSocketStream;
use async_tungstenite::tungstenite::{Error as WsError, Message};
use futures::{Sink, SinkExt, Stream, StreamExt};
use nphysics2d::object::{Body, BodySet, RigidBody};
use super::world::MyHandle;
use super::world::parts::{Part, PartKind};

use crate::codec::*;

pub enum Session {
    AcceptingWebSocket(Pin<Box<dyn Future<Output = Result<WebSocketStream<TcpStream>, async_tungstenite::tungstenite::Error>>>>),
    AwaitingHandshake(MyWebSocket),
    Spawned(MyWebSocket, SpawnedPlayer)
}
pub enum SessionEvent {
    ReadyToSpawn
}
pub struct SpawnedPlayer {
    pub core: crate::world::parts::Part,
    pub thrust_forwards: bool,
    pub thrust_backwards: bool,
    pub thrust_clockwise: bool,
    pub thrust_counterclockwise: bool,
    pub fuel: u16,
    pub max_fuel: u16
}

impl Session {
    pub fn new(socket: TcpStream) -> Session {
        let future = async_tungstenite::accept_async(socket);
        let pinbox;
        unsafe { pinbox = Pin::new_unchecked(Box::new(future)); }
        Session::AcceptingWebSocket(pinbox)
    }

    pub fn spawn(&mut self, simulation: &crate::world::Simulation, core: super::world::parts::Part) {
        if let Session::AwaitingHandshake(socket) = self {
                let id = if let MyHandle::Part(Some(id), _) = core.body { id } else { panic!(); };
                socket.queue_send(Message::Binary(ToClientMsg::HandshakeAccepted{id}.serialize()));
                //Send over celestial object locations
                for planet in simulation.planets.celestial_objects().iter() {
                    //socket.queue_send(Message::Binary(ToClientMsg::))
                }
        } else { panic!() }
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
                                ToServerMsg::Handshake{ client, session } => {
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
                            
                        },
                        Some(Message::Ping(_)) => (),
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
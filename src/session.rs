use std::pin::Pin;
use std::task::{Poll, Context};
use async_std::prelude::*;
use async_std::net::TcpStream;
use async_tungstenite::WebSocketStream;
use async_tungstenite::tungstenite::{Error as WsError, Message};
use futures::{Sink, SinkExt, Stream, StreamExt};
use nphysics2d::object::{Body, BodySet, RigidBody};

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

    pub fn spawn(&mut self, simulation: &crate::world::Simulation) {
        if let Session::AwaitingHandshake(socket) = self {
                //Send over celestial object locations
                for planet in simulation.planets.celestial_objects().iter() {
                    let mut builder = flatbuffers::FlatBufferBuilder::new();
                    let name = builder.create_string(&planet.name);
                    let display_name = builder.create_string(&planet.name);
                    let id = match planet.body {
                        crate::world::MyHandle::CelestialObject(id) => id,
                        _ => panic!()
                    };
                    let translation = simulation.bodies.get_rigid(planet.body).unwrap().position().translation;
                    let add_obj = to_client::AddCelestialObject::create(&mut builder, &to_client::AddCelestialObjectArgs {
                        name: Some(name), display_name: Some(display_name), radius: planet.radius, id,
                        position: Some(&Vector2::new(translation.x, translation.y))
                    });
                    let msg = to_client::Msg::create(&mut builder, &to_client::MsgArgs {
                        msg: Some(add_obj.as_union_value()), msg_type: to_client::ToClientMsg::AddCelestialObject
                    });
                    builder.finish(msg, None);
                    socket.queue_send(Message::Binary(builder.finished_data().to_vec()));
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
                            let buf = flatbuffers::get_root::<to_server::Msg>(&dat[..]);
                            match buf.msg_type() {
                                to_server::ToServerMsg::Handshake => {
                                    let mut builder = flatbuffers::FlatBufferBuilder::new();
                                    let handshake_accepted = to_client::HandshakeAccepted::create(&mut builder, &to_client::HandshakeAcceptedArgs {});
                                    let msg = to_client::Msg::create(&mut builder, &to_client::MsgArgs { msg: Some(handshake_accepted.as_union_value()), msg_type: to_client::ToClientMsg::HandshakeAccepted });
                                    builder.finish(msg, None);
                                    stream.queue_send(Message::Binary(builder.finished_data().to_vec()));
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
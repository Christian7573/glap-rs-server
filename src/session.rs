use std::pin::Pin;
use std::task::{Poll, Context};
use async_std::prelude::*;
use async_std::net::{TcpStream, TcpListener};
use async_tungstenite::WebSocketStream;
use async_tungstenite::tungstenite::{Error as WsError, Message};
use futures::{Sink, SinkExt, Stream, StreamExt};
use nphysics2d::object::{Body, BodySet, RigidBody};
use super::world::{MyHandle, MyUnits};
use super::world::parts::{Part, PartKind};
use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};

use crate::codec::*;

pub enum InboundEvent {
    NewPlayer { id: u16, name: String },
    PlayerMessage { id: u16, msg: ToServerMsg },
    PlayerQuit { id: u16 }
}
pub enum OutboundEvent {
    Message (u16, ToClientMsg),
    Broadcast (ToClientMsg),
    WorldUpdate (Vec<WorldUpdatePartMove>),
    SessionBad(u16)
}
pub struct WorldUpdatePartMove { id: u16, x: f32, y: f32, rot_sin: f32, rot_cos: f32 }

enum Event {
    NewSocket { socket: TcpStream },
    PotentialSessionMessage { id: u16, message: Vec<u8> },
    PotentialSessionDisconnect { id: u16 },
    SessionMessage { id: u16, message: Vec<u8> },
    SessionDisconnect { id: u16 },
    OutboundEvent(OutboundEvent)
}

pub async fn sessiond(listener: TcpListener, inbound: async_std::sync::Sender<InboundEvent>, outbound: async_std::sync::Receiver<Vec<OutboundEvent>>) {
    struct Pulser {
        listener: TcpListener,
        potential_sessions: MaybeFairPoller7573<PotentialSession>,
        sessions: MaybeFairPoller7573<Session>,
        inbound: async_std::sync::Sender<InboundEvent>,
        outbound: async_std::sync::Receiver<InboundEvent>,
    }
}

struct MaybeFairPoller7573<T: Stream + Unpin> ( BTreeMap<u16, T>, usize );
impl<T> Stream for MaybeFairPoller7573<T> where T: Stream + Unpin {
    type Item = T::Item;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let skip = self.1;
        for (i, thing) in self.0.values_mut().enumerate().skip(skip) {
            let poll = thing.poll_next_unpin(cx);
            if poll.is_ready() { self.1 = i; return poll; }
        }
        for (i, thing) in self.0.values_mut().enumerate() {
            let poll = thing.poll_next_unpin(cx);
            if poll.is_ready() { self.1 = i; return poll; }
        }
        Poll::Pending
    }
}

// struct FairPoller7573<T: Stream + Unpin> {
//     stuff: Vec<T>,
//     indexi: BTreeMap<u16, usize>,
//     last_poll_index: usize,
//     next_id: u16
// }
// impl<T> FairPoller7573<T> where T: Stream + Unpin {
//     pub fn new() -> Self {
//         FairPoller7573 {
//             stuff: Vec::new(),
//             indexi: BTreeMap::new(),
//             last_poll_index: 0,
//             next_id: 0,
//         }
//     }
//     pub fn get(&self, id: u16) -> Option<&T> {
//         unsafe { self.indexi.get(&id).map(|index| self.stuff.get_unchecked(*index) ) }
//     }
//     pub fn get_mut(&mut self, id: u16) -> Option<&mut T> {
//         unsafe { self.indexi.get(&id).map(|index| self.stuff.get_unchecked_mut(*index) ) }
//     }
//     pub fn insert(&mut self, thing: T) -> u16 {
//         let my_id = self.next_id;
//         self.next_id += 1;
//         self.indexi.insert(my_id, self.stuff.len());
//         self.stuff.push(thing);
//         my_id
//     }
//     pub fn remove(&mut self, id: u16) -> Option<T> {
//         if let Some(index) = self.indexi.remove(&id) {
//             let thing = self.stuff.remove(index);
//             for other_index in self.indexi.values_mut().skip(index) {
//                 *other_index -= 1;
//             };
//             Some(thing)
//         } else { None }
//     }
// }
// impl<T> Stream for FairPoller7573<T> where T: Stream + Unpin {
//     type Item = T::Item;
//     fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        
//     }
// }

pub enum PotentialSession {
    AcceptingWebSocket(Pin<Box<dyn Future<Output = Result<WebSocketStream<TcpStream>, async_tungstenite::tungstenite::Error>>>>),
    AwaitingHandshake(MyWebSocket)
}
impl PotentialSession {
    pub fn new(socket: TcpStream) -> PotentialSession {
        let future = async_tungstenite::accept_async(socket);
        let pinbox;
        unsafe { pinbox = Pin::new_unchecked(Box::new(future)); }
        PotentialSession::AcceptingWebSocket(pinbox)
    }
}
impl Stream for PotentialSession {
    type Item = Vec<u8>;
    fn poll_next(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Option<Vec<u8>>> {
        match self.deref_mut() {
            PotentialSession::AcceptingWebSocket(future) => {
                if let Poll::Ready(result) = future.as_mut().poll(ctx) {
                    if let Ok(stream) = result {
                        let socket: MyWebSocket = stream.into();
                        println!("Accepted websocket");
                        std::mem::replace(self.get_mut(), PotentialSession::AwaitingHandshake(socket));
                        Poll::Pending
                    } else {
                        Poll::Ready(None)
                    }
                } else { Poll::Pending }
            },
            PotentialSession::AwaitingHandshake(stream) => {
                if let Poll::Ready(result) = stream.poll_next_unpin(ctx) {
                    match result {
                        Some(Message::Binary(dat)) => Poll::Ready(Some(dat)),
                        Some(Message::Ping(_)) => Poll::Pending,
                        _ => { Poll::Ready(None) }
                    }
                } else { Poll::Pending }
            },
        }
    }
}

pub struct Session { socket: MyWebSocket }
pub enum SessionEvent {
    ReadyToSpawn,
    ThrusterUpdate,
    CommitGrab { part_id: u16, x: f32, y: f32 },
    MoveGrab { x: f32, y: f32 },
    ReleaseGrab
}

impl Stream for Session {
    type Item = Vec<u8>;
    fn poll_next(
        mut self: Pin<&mut Self>,
        ctx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        if let Poll::Ready(result) = self.socket.poll_next_unpin(ctx) {
            match result {
                Some(Message::Binary(dat)) => Poll::Ready(Some(dat)),
                Some(Message::Ping(_)) => Poll::Pending,
                _ => Poll::Ready(None)
            }
        } else { Poll::Pending }
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
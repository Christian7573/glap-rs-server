use std::pin::Pin;
use std::task::{Poll, Context};
use async_std::prelude::*;
use async_std::net::{TcpStream, TcpListener};
use async_tungstenite::WebSocketStream;
use async_tungstenite::tungstenite::{Error as WsError, Message};
use futures::{Sink, SinkExt, Stream, StreamExt, FutureExt};
use nphysics2d::object::{Body, BodySet, RigidBody};
use super::world::{MyHandle, MyUnits};
use super::world::parts::{Part, PartKind};
use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};
use crate::beamout::{RecursivePartDescription, beamin_request, spawn_beamout_request};
use crate::ApiDat;
use std::sync::Arc;

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
    SessionBad(u16),
    BeamOutPlayer(u16, RecursivePartDescription),
}
pub struct WorldUpdatePartMove { pub id: u16, pub x: f32, pub y: f32, pub rot_sin: f32, pub rot_cos: f32 }

enum Event {
    NewSocket { socket: TcpStream },
    PotentialSessionMessage { id: u16, msg: Vec<u8> },
    PotentialSessionBeamin { id: u16, parts: Option<RecursivePartDescription> },
    PotentialSessionDisconnect { id: u16 },
    SessionMessage { id: u16, msg: Vec<u8> },
    SessionDisconnect { id: u16 },
    OutboundEvent(Vec<OutboundEvent>)
}

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

pub enum SessionDInit { InitPloz (TcpListener, async_std::sync::Sender<InboundEvent>, async_std::sync::Receiver<Vec<OutboundEvent>>, Option<Arc<ApiDat>>), IntermediateState, Inited(Pin<Box<dyn Future<Output = ()>>>) }
unsafe impl Send for SessionDInit {}
impl Future for SessionDInit {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.deref_mut() {
            SessionDInit::Inited(sessiond) => sessiond.as_mut().poll(cx),
            SessionDInit::InitPloz(listener, inbound, outbound, api) => {
                if let SessionDInit::InitPloz(listener, inbound, outbound, api) = std::mem::replace(self.deref_mut(), SessionDInit::IntermediateState) {
                    *self = SessionDInit::Inited(sessiond(listener, inbound, outbound, api).boxed_local());
                    cx.waker().wake_by_ref();
                    Poll::Pending
                } else { panic!(); }
            },
            SessionDInit::IntermediateState => panic!()
        }
    }
}

async fn sessiond(listener: TcpListener, inbound: async_std::sync::Sender<InboundEvent>, outbound: async_std::sync::Receiver<Vec<OutboundEvent>>, api: Option<Arc<ApiDat>>) {
    struct Pulser {
        listener: TcpListener,
        potential_sessions: MaybeFairPoller7573<PotentialSession>,
        sessions: MaybeFairPoller7573<Session>,
        outbound: async_std::sync::Receiver<Vec<OutboundEvent>>,
    }
    impl Stream for Pulser {
        type Item = Event;
        fn poll_next(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Option<Event>> {
            match self.outbound.poll_next_unpin(ctx) {
                Poll::Ready(Some(event)) => return Poll::Ready(Some(Event::OutboundEvent(event))),
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => (),
            };
            match self.sessions.poll_next_unpin(ctx) {
                Poll::Ready(Some((id, Some(msg)))) => return Poll::Ready(Some(Event::SessionMessage{ id, msg })),
                Poll::Ready(Some((id, None))) => return Poll::Ready(Some(Event::SessionDisconnect{ id })),
                Poll::Pending => (),
                Poll::Ready(None) => panic!("I wrote this, how did we get here?")
            };
            match self.potential_sessions.poll_next_unpin(ctx) {
                Poll::Ready(Some((id, Some(PotentialSessionEvent::Msg(msg))))) => return Poll::Ready(Some(Event::PotentialSessionMessage{ id, msg })),
                Poll::Ready(Some((id, Some(PotentialSessionEvent::Beamin(beamin))))) => return Poll::Ready(Some(Event::PotentialSessionBeamin{ id, parts: beamin })),
                Poll::Ready(Some((id, None))) => return Poll::Ready(Some(Event::PotentialSessionDisconnect{ id })),
                Poll::Pending => (),
                Poll::Ready(None) => panic!("I wrote this, how did we get here?")
            };
            unsafe {
                match Pin::new_unchecked(&mut self.listener.accept()).poll(ctx) {
                    Poll::Ready(Ok((socket, _addr))) => return Poll::Ready(Some(Event::NewSocket{ socket })),
                    Poll::Ready(Err(err)) => panic!("IO Error or something idk\n{:?}", err),
                    Poll::Pending => (),
                };
            }
            Poll::Pending
        }
    }

    let mut pulser = Pulser {
        potential_sessions: MaybeFairPoller7573( BTreeMap::new(), 0 ),
        sessions: MaybeFairPoller7573( BTreeMap::new(), 0 ),
        listener, outbound,
    };
    let mut next_session_id: u16 = 0;
    let mut serialization_vec: Vec<u8> = Vec::with_capacity(2048);
    while let Some(event) = pulser.next().await {
        match event {
            Event::OutboundEvent(events) => {
                for event in events {
                    match event {
                        OutboundEvent::Message(id, msg) => {
                            if let Some(session) = pulser.sessions.0.get_mut(&id) {
                                msg.serialize(&mut serialization_vec);
                                session.socket.queue_send(Message::Binary(serialization_vec.clone()));
                                serialization_vec.clear();
                            }
                        },
                        OutboundEvent::Broadcast(msg) => {
                            msg.serialize(&mut serialization_vec);
                            for session in pulser.sessions.0.values_mut() {
                                session.socket.queue_send(Message::Binary(serialization_vec.clone()));
                            }
                            serialization_vec.clear();
                        },
                        OutboundEvent::SessionBad(id) => {
                            pulser.sessions.0.remove(&id);
                            inbound.send(InboundEvent::PlayerQuit{ id }).await;
                        },
                        OutboundEvent::WorldUpdate(part_movements) => {
                            for part_moved in part_movements {
                                ToClientMsg::MovePart{
                                    id: part_moved.id,
                                    x: part_moved.x, y: part_moved.y,
                                    rotation_i: part_moved.rot_sin,
                                    rotation_n: part_moved.rot_cos,
                                }.serialize(&mut serialization_vec);
                                for session in pulser.sessions.0.values_mut() {
                                    session.socket.queue_send(Message::Binary(serialization_vec.clone()));
                                }
                                serialization_vec.clear();
                            }
                        },
                        OutboundEvent::BeamOutPlayer(id, beamout_layout) => {
                            if let Some(session) = pulser.sessions.0.remove(&id) {
                                let session_id = session.session_id.clone();
                                async_std::task::spawn(async {
                                    let mut socket = session.socket.socket;
                                    socket.flush().await; 
                                });

                                spawn_beamout_request(session_id, beamout_layout, api.clone());
                            } 
                        },
                    };
                };
            }

            Event::NewSocket{ socket } => {
                let my_id = next_session_id;
                next_session_id += 1;
                pulser.potential_sessions.0.insert(my_id, PotentialSession::AcceptingWebSocket(async_tungstenite::accept_async(socket).boxed_local()));
            },
            Event::PotentialSessionMessage{ id, msg } => {
                if let Ok(msg) = ToServerMsg::deserialize(msg.as_ref(), &mut 0) {
                    match msg {
                        ToServerMsg::Handshake{ session, client, name } => {
                            let beamin = beamin_request(session.clone(), api.clone()).boxed();
                            if let Some(PotentialSession::AwaitingHandshake(socket)) = pulser.potential_sessions.0.remove(&id) {
                                pulser.potential_sessions.0.insert(id, PotentialSession::AwaitingBeamin(socket, beamin, session, name));
                            }
                        },
                        _ => { pulser.potential_sessions.0.remove(&id); }
                    }
                } else { pulser.potential_sessions.0.remove(&id); }
            },
            Event::PotentialSessionBeamin{ id, parts } => {
                if let Some(PotentialSession::AwaitingBeamin(socket, _, session, name)) = pulser.potential_sessions.0.remove(&id) {
                    pulser.sessions.0.insert(id, Session{ socket, session_id: session });
                    inbound.send(InboundEvent::NewPlayer{ id, name }).await;
                }
            },
            Event::PotentialSessionDisconnect{ id } => { pulser.potential_sessions.0.remove(&id); },
            Event::SessionMessage{ id, msg } => {
                if let Ok(msg) = ToServerMsg::deserialize(msg.as_ref(), &mut 0) { inbound.send(InboundEvent::PlayerMessage{ id, msg }).await; }
                else {
                    pulser.sessions.0.remove(&id);
                    inbound.send(InboundEvent::PlayerQuit{ id }).await;
                }
            },
            Event::SessionDisconnect{ id } => {
                pulser.sessions.0.remove(&id);
                inbound.send(InboundEvent::PlayerQuit{ id }).await;
            }
        }
    };

    println!("Sessiond stopped");
}

struct MaybeFairPoller7573<T: Stream + Unpin> ( BTreeMap<u16, T>, usize );
impl<T> Stream for MaybeFairPoller7573<T> where T: Stream + Unpin {
    type Item = (u16, Option<T::Item>);
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let skip = self.1;
        for (i, (id, thing)) in self.0.iter_mut().enumerate().skip(skip) {
            if let Poll::Ready(poll) = thing.poll_next_unpin(cx) {
                let id = *id;
                self.1 = i;
                return Poll::Ready(Some((id, poll)));
            }
        }
        for (i, (id, thing)) in self.0.iter_mut().enumerate() {
            if let Poll::Ready(poll) = thing.poll_next_unpin(cx) {
                let id = *id;
                self.1 = i;
                return Poll::Ready(Some((id, poll)));
            }
        }
        Poll::Pending
    }
}

type BeaminRequestType = Pin<Box<dyn Future<Output = Option<RecursivePartDescription>>>>;
enum PotentialSession {
    AcceptingWebSocket(Pin<Box<dyn Future<Output = Result<WebSocketStream<TcpStream>, async_tungstenite::tungstenite::Error>>>>),
    AwaitingHandshake(MyWebSocket),
    AwaitingBeamin(MyWebSocket, BeaminRequestType, Option<String>, String),
}
impl PotentialSession {
    pub fn new(socket: TcpStream) -> PotentialSession {
        let future = async_tungstenite::accept_async(socket);
        let pinbox;
        unsafe { pinbox = Pin::new_unchecked(Box::new(future)); }
        PotentialSession::AcceptingWebSocket(pinbox)
    }
}
enum PotentialSessionEvent {
    Msg(Vec<u8>),
    Beamin(Option<RecursivePartDescription>),
}
impl Stream for PotentialSession {
    type Item = PotentialSessionEvent;
    fn poll_next(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Option<PotentialSessionEvent>> {
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
            PotentialSession::AwaitingHandshake(stream) => stream.poll_next_unpin(ctx).map(|dat| dat.map(|dat| PotentialSessionEvent::Msg(dat))),
            PotentialSession::AwaitingBeamin(stream, beamin_request, _session, _name) => {
                if let Poll::Ready(dat) = stream.poll_next_unpin(ctx) {
                    Poll::Ready(dat.map(|dat| PotentialSessionEvent::Msg(dat)))
                } else if let Poll::Ready(beamin) = beamin_request.as_mut().poll(ctx) {
                    Poll::Ready(Some(PotentialSessionEvent::Beamin(beamin)))
                } else { Poll::Pending }
            },
        }
    }
}

pub struct Session { socket: MyWebSocket, session_id: Option<String> }
impl Stream for Session {
    type Item = Vec<u8>;
    fn poll_next(
        mut self: Pin<&mut Self>,
        ctx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.socket.poll_next_unpin(ctx)
    }
}
pub struct MyWebSocket {
    socket: WebSocketStream<TcpStream>
}
impl Stream for MyWebSocket {
    type Item = Vec<u8>;
    fn poll_next(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Option<Vec<u8>>> {
        //Flush
        Pin::new(&mut self.socket).poll_flush(ctx);
        //Read
        match self.socket.poll_next_unpin(ctx) {
            Poll::Ready(Some(Ok(Message::Binary(dat)))) => Poll::Ready(Some(dat)),
            Poll::Ready(Some(Ok(Message::Ping(_)))) => Poll::Pending,
            Poll::Pending => Poll::Pending,
            _ => Poll::Ready(None)
        }
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

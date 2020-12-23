use std::collections::BTreeMap;
use std::pin::Pin;
use std::task::{Poll, Context};
use async_std::prelude::*;
use futures::select_biased;
use async_std::net::{TcpStream, TcpListener};
use async_std::sync::{Sender, Receiver, channel};
use futures::{Sink, SinkExt, Stream, StreamExt, FutureExt};
use nphysics2d::object::{Body, BodySet, RigidBody};
use super::world::{MyHandle, MyUnits};
use super::world::parts::{Part, PartKind};
use std::ops::{Deref, DerefMut};
use crate::beamout::{RecursivePartDescription, BeaminResponse, beamin_request, spawn_beamout_request};
use crate::ApiDat;
use std::sync::Arc;

use crate::codec::*;

pub mod websocket;
use websocket::*;

pub enum ToGameEvent {
    NewPlayer { id: u16, name: String, parts: RecursivePartDescription },
    PlayerMessage { id: u16, msg: ToServerMsg },
    PlayerQuit { id: u16 },
    AdminCommand { id: u16, command: String }
}
pub enum ToSerializerEvent {
    Message (u16, ToClientMsg),
    MulticastMessage (Vec<u16>, ToClientMsg),
    Broadcast (ToClientMsg),
    WorldUpdate (BTreeMap<u16, ((f32,f32), Vec<WorldUpdatePartMove>, ToClientMsg)>, Vec<WorldUpdatePartMove>),

    NewWriter (u16, Sender<Vec<OutboundWsMessage>>),
    RequestUpdate (u16),
    SendPong (u16),
    //BeamoutWriter (u16, RecursivePartDescription),
    DeleteWriter (u16),
}

pub struct WorldUpdatePlayerUpdate { pub id: u16, pub core_x: f32, pub core_y: f32, pub parts: Vec<WorldUpdatePartMove> }
pub struct WorldUpdatePartMove { pub id: u16, pub x: f32, pub y: f32, pub rot_sin: f32, pub rot_cos: f32 }

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

//pub enum SessionDInit { InitPloz (TcpListener, async_std::sync::Sender<InboundEvent>, async_std::sync::Receiver<Vec<OutboundEvent>>, Option<Arc<ApiDat>>), IntermediateState, Inited(Pin<Box<dyn Future<Output = ()>>>) }
//unsafe impl Send for SessionDInit {}
/*impl Future for SessionDInit {
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
}*/

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
    is_admin = true; //remove this
    let layout = layout.unwrap_or( RecursivePartDescription { kind: PartKind::Core, attachments: Vec::new() } );                                                                                                                                                        

    to_game.send(ToGameEvent::NewPlayer { id, name: name.clone(), parts: layout }).await;
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
                    writers.insert(id, (to_writer, Vec::new(), true));
                },
                ToSerializerEvent::DeleteWriter(id) => {
                    writers.remove(&id);
                    to_game.send(ToGameEvent::PlayerQuit { id }).await;
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
                    for (id, ((x, y), parts, post_simulation)) in &players {
                        let mut msg = Vec::new();
                        ToClientMsg::MessagePack { count: parts.len() as u16 }.serialize(&mut msg);
                        for part in parts {
                            ToClientMsg::MovePart {
                                id: part.id, x: part.x, y: part.y,
                                rotation_n: part.rot_cos, rotation_i: part.rot_sin,
                            }.serialize(&mut msg);
                        };
                        let msg = OutboundWsMessage::from(&msg);
                        for (id, (_to_writer, queue, request_update)) in &mut writers {
                            if *request_update {
                                if let Some(((player_x, player_y), _parts, _post_simulation)) = players.get(&id) {
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

/*async fn sessiond_old(listener: TcpListener, inbound: async_std::sync::Sender<InboundEvent>, outbound: async_std::sync::Receiver<Vec<OutboundEvent>>, api: Option<Arc<ApiDat>>) {
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
                Poll::Ready(Some((id, Some(PotentialSessionEvent::Beamin(Some(beamin)))))) => return Poll::Ready(Some(Event::PotentialSessionBeamin{ id, parts: beamin.layout, beamout_token: Some(beamin.beamout_token) })),
                Poll::Ready(Some((id, Some(PotentialSessionEvent::Beamin(None))))) => return Poll::Ready(Some(Event::PotentialSessionBeamin{ id, parts: None, beamout_token: None })),
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
                        OutboundEvent::WorldUpdate(player_movements, free_part_movements) => {
                            for player in &player_movements {
                                if let Some(session) = pulser.sessions.0.get_mut(&player.id) {
                                    session.player_x = player.core_x;
                                    session.player_y = player.core_y;
                                }
                            }
                            for player in &player_movements {
                                ToClientMsg::MessagePack { count: player.parts.len() as u16 }.serialize(&mut serialization_vec);
                                for part in &player.parts {
                                    ToClientMsg::MovePart{
                                        id: part.id,
                                        x: part.x, y: part.y,
                                        rotation_i: part.rot_sin,
                                        rotation_n: part.rot_cos,
                                    }.serialize(&mut serialization_vec);
                                }
                            }
                            /*for part_moved in part_movements {
                                for session in pulser.sessions.0.values_mut() {
                                    session.socket.queue_send(Message::Binary(serialization_vec.clone()));
                                }
                                serialization_vec.clear();
                            }*/
                        },
                        OutboundEvent::BeamOutPlayer(id, beamout_layout) => {
                            if let Some(session) = pulser.sessions.0.remove(&id) {
                                let beamout_token = session.beamout_token.clone();
                                async_std::task::spawn(async {
                                    let mut socket = session.socket.socket;
                                    socket.flush().await; 
                                });

                                spawn_beamout_request(beamout_token, beamout_layout, api.clone());
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
            Event::PotentialSessionBeamin{ id, parts, beamout_token } => {
                if let Some(PotentialSession::AwaitingBeamin(socket, _beamin_future, _session, mut name)) = pulser.potential_sessions.0.remove(&id) {
                    name = String::from(name.trim());
                    if name.is_empty() { name = String::from("Unnamed") };
                    pulser.sessions.0.insert(id, Session{ socket, beamout_token, name: name.clone(), player_x: 0.0, player_y: 0.0 });
                    inbound.send(InboundEvent::NewPlayer{ id, name, parts: parts.unwrap_or(RecursivePartDescription { kind: PartKind::Core, attachments: Vec::new() })}).await;
                }
            },
            Event::PotentialSessionDisconnect{ id } => { pulser.potential_sessions.0.remove(&id); },
            Event::SessionMessage{ id, msg } => {
                match ToServerMsg::deserialize(msg.as_ref(), &mut 0) {
                    Ok(ToServerMsg::SendChatMessage{ msg }) => {
                        ToClientMsg::ChatMessage{ username: pulser.sessions.0.get(&id).unwrap().name.clone(), msg, color: String::from("#dd55ff") }.serialize(&mut serialization_vec);
                        for (_id, session) in &mut pulser.sessions.0 {
                            session.socket.queue_send(Message::Binary(serialization_vec.clone()));
                        };
                        serialization_vec.clear();
                    },
                    Ok(msg) => { inbound.send(InboundEvent::PlayerMessage{ id, msg }).await; }
                    Err(_) => {
                        pulser.sessions.0.remove(&id);
                        inbound.send(InboundEvent::PlayerQuit{ id }).await;
                    }
                };
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

type BeaminRequestType = Pin<Box<dyn Future<Output = Option<BeaminResponse>>>>;
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
    Beamin(Option<BeaminResponse>),
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

pub struct Session { socket: MyWebSocket, player_x: f32, player_y: f32, beamout_token: Option<String>, name: String }
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
}*/

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

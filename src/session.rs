use std::pin::Pin;
use std::task::{Poll, Context};
use async_std::prelude::*;
use async_std::net::TcpStream;
use async_tungstenite::WebSocketStream;
use async_tungstenite::tungstenite::{Error as WsError, Message};
use futures::{Sink, SinkExt, Stream, StreamExt};

use crate::codec::*;

pub enum Session {
    AcceptingWebSocket(Pin<Box<dyn Future<Output = Result<WebSocketStream<TcpStream>, async_tungstenite::tungstenite::Error>>>>),
    Disconnected,
    AwaitingHandshake(MyWebSocket)
}
pub enum SessionEvent {
    ReadyToSpawn
}

impl Session {
    pub fn new(socket: TcpStream) -> Session {
        let future = async_tungstenite::accept_async(socket);
        let pinbox;
        unsafe { pinbox = Pin::new_unchecked(Box::new(future)); }
        Session::AcceptingWebSocket(pinbox)
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
                        //socket.queue_send(Message::Binary(serialize(&ToClientMsg::TestHandshake(String::from("glap.rs-0.1.0"), None)).unwrap()));
                        println!("Accepted websocket");
                        std::mem::replace(self.get_mut(), Session::AwaitingHandshake(socket));
                        Poll::Pending
                    } else {
                        std::mem::replace(self.get_mut(), Session::Disconnected);
                        Poll::Ready(None)
                    }
                } else { Poll::Pending }
            },

            Session::Disconnected => Poll::Ready(None),

            Session::AwaitingHandshake(stream) => {
                if let Poll::Ready(result) = stream.poll_next_unpin(ctx) {
                    match result {
                        Some(Message::Binary(dat)) => {
                            println!("Message recieved");
                            match deserialize::<FromClientMsg>(dat.as_slice()) {
                                Ok(FromClientMsg::Handshake{ client, session }) => {
                                    println!("Made it here");
                                    stream.queue_send(accept_handshake());
                                    Poll::Ready(Some(SessionEvent::ReadyToSpawn))
                                    //Event loop will call back once it has prepared a physics body and whatnot for us
                                },
                                Err(error) => {
                                    println!("{}", error);
                                    std::mem::replace(self.get_mut(), Session::Disconnected);
                                    Poll::Ready(None)
                                },
                                Ok(_) => {
                                    println!("Msg bad");
                                    std::mem::replace(self.get_mut(), Session::Disconnected);
                                    Poll::Ready(None)
                                }
                            }
                            // if let Ok(FromClientMsg::Handshake{ client, session }) = deserialize::<FromClientMsg>(dat.as_slice()) {
                            //     println!("Made it here");
                            //     stream.queue_send(accept_handshake());
                            //     Poll::Ready(Some(SessionEvent::ReadyToSpawn))
                            //     //Event loop will call back once it has prepared a physics body and whatnot for us
                            // } else {
                                // println!("Msg bad");
                                // std::mem::replace(self.get_mut(), Session::Disconnected);
                                // Poll::Ready(None)
                            // }
                        },
                        Some(Message::Ping(_)) => Poll::Pending,
                        _ => {
                            println!("Commit die");
                            std::mem::replace(self.get_mut(), Session::Disconnected);
                            Poll::Ready(None)
                        }
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
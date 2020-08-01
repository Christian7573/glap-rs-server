use async_std::prelude::*;
use std::net::SocketAddr;
use futures::select;
use async_std::net::TcpStream;
use std::pin::Pin;
use std::collections::BTreeMap;
use std::task::Poll;
use futures::future::FutureExt;
use async_tungstenite::WebSocketStream;

pub mod world;

#[async_std::main]
async fn main() {
    let server_port = if let Ok(port) = std::env::var("PORT") { port.parse::<u16>().unwrap_or(8080) } else { 8080 };
    let inbound = async_std::net::TcpListener::bind(SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), server_port)).await.expect(&format!("Failed to bind to port {}", server_port));
    let mut sessions: BTreeMap<u16, Session> = BTreeMap::new();
    let mut next_session: u16 = 1;
    
    let mut ticker = async_std::stream::interval(std::time::Duration::from_secs_f32(1.0/60.0));
    let mut simulation = world::Simulation::new();

    loop {
        //let session_futures = sessions.iter_mut().map(|session| session.1.get_socket)
        select! {
            socket = inbound.accept().fuse() => {
                if let Ok((socket, addr)) = socket {
                    let my_id = next_session;
                    next_session += 1;
                    sessions.insert(my_id, Session::new(socket));
                };
            },

            _ = ticker.next().fuse() => {
                //Simulate the world
            }
        };
    };
}

async fn race_all<O>(futures: Vec<&mut (dyn Future<Output = O> + Unpin)>) -> O {
    struct Racer<'a, O> { futures: Vec<&'a mut (dyn Future<Output = O> + Unpin)> }
    impl<'a, O> Future for Racer<'a, O> {
        type Output = O;
        fn poll(mut self: Pin<&mut Self>, ctx: &mut std::task::Context) -> Poll<O> {
            for future in &mut self.futures {
                if let Poll::Ready(result) = Pin::new(future).poll(ctx) { return Poll::Ready(result); }
            }
            Poll::Pending
        }
    };
    (Racer { futures }).await
}

struct Session {
    socket: WebSocketStream<TcpStream>
}
impl Session {
    pub fn new(socket: TcpStream) -> Session {
        Session {
            socket: WebSocketStream::from(socket)
        }
    }
}
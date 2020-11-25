use async_std::net::TcpStream;
use std::collections::VecDeque;
use futures::{Future, AsyncRead, AsyncWrite, Stream, Sink};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

pub struct TcpStreamWrapper {
    socket: TcpStream,
    input_buf: [u8; 64],
    input_slice: Option<(usize, usize)>,
    output: VecDeque<Arc<Vec<u8>>>,
    output_index: Option<usize>,
}

impl From<TcpStream> for TcpStreamWrapper {
    fn from(socket: TcpStream) -> TcpStreamWrapper {
        TcpStreamWrapper {
            socket,
            input_buf: [0; 64],
            input_slice: None,
            output: VecDeque::new(),
            output_index: None
        }
    }
}

impl Stream for TcpStreamWrapper {
    type Item = u8;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        //Reading
        //Data is waiting to surface to the user
        if let Some((input_index, input_end)) = self.input_slice {
            let byte = self.input_buf[input_index];
            input_index += 1;
            //Finished with this data or not
            if input_index >= input_end { self.input_slice = None; }
            else { self.input_slice = Some((input_index, input_end)); };
            //Wake incase there's more work to do
            cx.waker().wake_by_ref();
            Poll::Ready(Some(byte))
        } else {
            match Pin::new(&mut self.socket).poll_read(cx, &mut self.input_buf) {
                //Length of 0 indicates end of stream
                Poll::Ready(Ok(0)) | Poll::Ready(Err(_)) => return Poll::Ready(None),
                Poll::Ready(Ok(bytes_read)) => {
                    //Return first byte right away
                    let byte = self.input_buf[0];
                    self.input_slice = Some((1, bytes_read));
                    //Wake incase there's more work to do
                    cx.waker().wake_by_ref();
                    Poll::Ready(Some(byte))
                },
                Poll::Pending => Poll::Pending,
            }
        }
    }
}

impl TcpStreamWrapper {
    fn queue_output(&mut self, dat: Arc<Vec<u8>>) {
        self.output.push_back(dat);
    }
    
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(),()>> {
        //Writing
        if !self.output.is_empty() {
            //Position where the past write finished
            let mut output_index = if let Some(output_index) = self.output_index { output_index } else { 0 };
            match Pin::new(&mut self.socket).poll_write(cx, &self.output.get(0).unwrap()[output_index..]) {
                //Bytes were successfully written
                Poll::Ready(Ok(bytes_written)) => {
                    output_index += bytes_written;
                    if output_index >= self.output.get(0).unwrap().len() {
                        //This message is finished
                        self.output.pop_front();
                        //If there are more messages, reset output_index to 0
                        if self.output.len() > 0 { self.output_index = Some(0); }
                        else { self.output_index = None; }
                    } else {
                        self.output_index = Some(output_index);
                    }
                    //Wake incase there is more work to do
                    cx.waker().wake_by_ref();
                    Poll::Ready(Ok(()))
                },
                Poll::Ready(Err(_)) => Poll::Ready(Err(())),
                Poll::Pending => Poll::Pending
            }
        } else {
            Poll::Ready(Ok(()))
        }
    }
}

struct TcpStreamWrapperFlushFuture<'a> ( &'a mut TcpStreamWrapper );
impl<'a> Future for TcpStreamWrapperFlushFuture<'a> {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        match Pin::new(&mut self.0).poll_write(cx) {
            Poll::Ready(Ok(())) => {
                if !self.0.output.is_empty() { Poll::Ready(()) }
                else { Poll::Pending }
            },
            Poll::Ready(Err(())) => Poll::Ready(()),
            Poll::Pending => Poll::Pending
    }
}


pub struct PotentialWebsocket {
    socket: Option<TcpStream>,
    state: PotentialWebsocketState,
    encryption_thing: Option<[u8; 16]>,
    wants_upgrade: bool,
    wants_websocket: bool,
    wants_websocket_13: bool,
}

enum PotentialWebsocketState {
    ReadingHttpMethod { chars_read: u8 },
    ReadingHttpUri,
    ReadingHttpVersion,

    ReadingHttpHeaderName { name: String },
    SkippingHttpHeader,
    ReadingHttpUpgradeHeader { value: String },
    ReadingHttpConnectionHeader { value: String },
    ReadingWsEncryptionHeader { value: String },
    ReadingWsVersion { value: String },

    //FlushingData { data: Vec<u8>, next: Box<PotentialWebsocketState> },
    FlushingDataThenAccept { data: Vec<u8> },
    Temp
}

impl PotentialWebsocket {
    pub fn new(stream: TcpStream) -> Self {
        PotentialWebsocket {
            socket: Some(stream),
            state: PotentialWebsocketState::ReadingHttpMethod { chars_read: 0 },
            encryption_thing: None,
            wants_upgrade: false,
            wants_websocket: false,
            wants_websocket_13: false,
        }
    }

    fn advance(&mut self, next: u8) -> Result<bool, ()> {
        use PotentialWebsocketState::*;
        match &mut self.state {
            ReadingHttpMethod { chars_read } => {
                const METHOD: &'static str = "GET ";
                if next == METHOD.bytes().nth(*chars_read as usize).unwrap() {
                    *chars_read += 1;
                    if *chars_read == METHOD.len() as u8 { self.state = ReadingHttpUri }
                    Ok(false)
                } else {
                    Err(())
                }
            },

            ReadingHttpUri => {
                if next as char == ' ' { self.state = ReadingHttpVersion }
                Ok(false)
            },

            ReadingHttpVersion => {
                if next as char == '\n' { self.state = ReadingHttpHeaderName { name: String::default() } }
                Ok(false)
            },

            ReadingHttpHeaderName { name } => {
                if next as char == ':' {
                    let header = name.trim();
                    if header == "Upgrade" { self.state = ReadingHttpUpgradeHeader { value: String::default() } }
                    else if header == "Sec-WebSocket-Key" { self.state = ReadingWsEncryptionHeader { value: String::default() } }
                    else if header == "Connection" { self.state = ReadingHttpConnectionHeader { value: String::default() } }
                    else if header == "Sec-Websocket-Version" { self.state = ReadingWsVersion { value: String::default() } }
                    else { self.state = SkippingHttpHeader };
                    Ok(false)
                } else if name.len() > 20 {
                    Err(())
                } else {
                    name.push(next as char);
                    if name == "\r\n" {
                        //Request over
                        if self.wants_upgrade && self.wants_websocket && self.wants_websocket_13 && self.encryption_thing.is_some() {
                            use sha::utils::{Digest, DigestExt};
                            let encryption_response = base64::encode(&self.encryption_thing.unwrap()).to_string() + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
                            let encryption_response = base64::encode(&sha::sha1::Sha1::default().digest(encryption_response.as_bytes()).to_bytes());
                            let response = format!(
                                "HTTP/1.1 101 Upgrade\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-Websocket-Accept: {}\r\n\r\n",
                                encryption_response
                            );
                            self.state = PotentialWebsocketState::FlushingDataThenAccept { data: response.bytes().collect() };
                        } else {
                            return Err(())
                        }
                    };
                    Ok(false)
                }
            },

            SkippingHttpHeader => {
                if next as char == '\n' { self.state = ReadingHttpHeaderName { name: String::default() } };
                Ok(false)
            },

            ReadingWsEncryptionHeader { value } |
            ReadingHttpUpgradeHeader { value } |
            ReadingHttpConnectionHeader { value } |
            ReadingWsVersion { value } => {
                if next as char == '\n' {
                    let header = value.trim().to_string();
                    match &self.state {
                        ReadingHttpUpgradeHeader { value } => {
                            if header.contains("websocket") { self.wants_websocket = true; }
                            else { return Err(()) }
                        },
                        ReadingWsEncryptionHeader { value } => {
                            if let Ok(result) = base64::decode(header) {
                                if result.len() == 16 {
                                    let mut out = [0u8; 16];
                                    for i in 0..16 { out[i] = result[i] };
                                    self.encryption_thing = Some(out);
                                } else { return Err(()) }
                            } else { return Err(()) }
                        },
                        ReadingHttpConnectionHeader { value } => {
                            if header.contains("Upgrade") { self.wants_upgrade = true; }
                            else { return Err(()) }
                        },
                        ReadingWsVersion { value } => {
                            if header == "13" { self.wants_websocket_13 = true; }
                            else { return Err(()) }
                        },
                        _ => panic!()
                    };
                    self.state = ReadingHttpHeaderName { name: String::default() };
                    Ok(false)
                } else if value.len() > 40 {
                    Err(())
                } else {
                    value.push(next as char);
                    Ok(false)
                }
            },

            //FlushingData { data, next } => panic!("FlushingData state in advance method"),
            FlushingDataThenAccept { data } => panic!("FlushingDataThenAccept state in advance method"),
            Temp => panic!("Temp state in advance method"),
        }
    }

    fn upgrade(&mut self) -> Websocket {
        /*let mut buf = [0u8; 128];
        for i in 0..initial_dat.len() {
            buf[i] = initial_dat[i];
        };*/
        Websocket::new(std::mem::replace(&mut self.socket, None).unwrap())
    }
}

impl Future for PotentialWebsocket {
    type Output = Result<Websocket, ()>;
    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        //if matches!(&self.state, PotentialWebsocketState::FlushingData { data, next }) ||
        if matches!(&self.state, PotentialWebsocketState::FlushingDataThenAccept { data } ) {
            let myself = self.deref_mut();
            let data = match &mut myself.state {
                //PotentialWebsocketState::FlushingData { data, next } => data,
                PotentialWebsocketState::FlushingDataThenAccept { data } => data,
                _ => panic!()
            };
            match Pin::new(myself.socket.as_mut().expect("Empty PotentialWebsocket")).poll_write(ctx, data.as_ref()) {
                Poll::Ready(Ok(bytes_written)) => {
                    drop(data.splice(0..bytes_written, Vec::with_capacity(0)));
                    if data.len() < 1 {
                        ctx.waker().wake_by_ref();
                        /*if let PotentialWebsocketState::FlushingData { data, next } = &myself.state {
                            if let PotentialWebsocketState::FlushingData { data, next } = std::mem::replace(&mut myself.state, PotentialWebsocketState::Temp) {
                                myself.state = *next;
                            } else {
                                panic!();
                            }
                        } else {*/

                        //Ready
                        return Poll::Ready(Ok(self.upgrade()));
                    };
                    Poll::Pending
                },
                Poll::Ready(Err(_)) => Poll::Ready(Err(())),
                Poll::Pending => Poll::Pending
            }
        } else {
            let mut buf = [0u8; 512];
            match Pin::new(self.socket.as_mut().expect("Empty PotentialWebsocket")).poll_read(ctx, &mut buf) {
                Poll::Ready(Ok(bytes_read)) => {
                    for i in 0..bytes_read {
                        match self.advance(buf[i]) {
                            //Ok(true) => return Poll::Ready(Ok(self.upgrade(&buf[i+1..bytes_read]))),
                            Ok(_) => {
                                //if matches!(&self.state, PotentialWebsocketState::FlushingData { data, next }) ||
                                if matches!(&self.state, PotentialWebsocketState::FlushingDataThenAccept { data } ) {
                                    ctx.waker().wake_by_ref();
                                }
                            },
                            Err(_) => return Poll::Ready(Err(()))
                        };
                    };
                    Poll::Pending
                },
                Poll::Ready(Err(_)) => Poll::Ready(Err(())),
                Poll::Pending => Poll::Pending
            }
        }
    }
}

pub struct Websocket {
    socket: TcpStream,
    buf_out: Vec<Arc<Vec<u8>>>,
    incoming: WebsocketIncoming,
}

pub enum WebsocketOpCode {
    Continuation,
    Text,
    Binary,
    Close,
    Ping,
    Pong,
}

pub struct WebsocketIncoming {
    buf: [u8; 128],
    buf_len: usize,
    i: usize,
    state: WebsocketIncomingState,

    op_code: WebsocketOpCode,
    is_final_frame: bool,
    data_is_masked: bool,
    payload_length: u64,
}

pub enum WebsocketIncomingState {
    WaitingForPacket,
    
    ReadingPayloadLength,
    ReadingPayloadLengthMultiple { i: u8, bytes: [u8; 8] },
    ReadingMaskKey { i: u8, bytes: [u8; 4] },
    ReadingPayloadData { bytes_remaining: u64 },
}

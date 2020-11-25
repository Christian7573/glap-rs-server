use async_std::net::TcpStream;
use std::collections::VecDeque;
use futures::{Future, AsyncRead, AsyncWrite, Stream, Sink, StreamExt};
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
        //Writing
        self.async_write(cx);

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
    pub fn queue_send(&mut self, dat: Arc<Vec<u8>>) {
        self.output.push_back(dat);
    }
    
    fn async_write(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(),()>> {
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

    pub fn flush(&mut self) -> TcpStreamWrapperFlushFuture {
        TcpStreamWrapperFlushFuture ( self )
    }

    pub fn unwarp(self) -> TcpStream { self.socket }
}

pub struct TcpStreamWrapperFlushFuture<'a> ( &'a mut TcpStreamWrapper );
impl<'a> Future for TcpStreamWrapperFlushFuture<'a> {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        match Pin::new(self.0).async_write(cx) {
            Poll::Ready(Ok(())) => {
                if !self.0.output.is_empty() { Poll::Ready(()) }
                else { Poll::Pending }
            },
            Poll::Ready(Err(())) => Poll::Ready(()),
            Poll::Pending => Poll::Pending
        }
    }
}

fn assert_or_error(dat: bool) -> Result<(),()> { if dat { Ok(()) } else { Err(()) } }
pub async fn open_websocket(mut socket: TcpStreamWrapper) -> Result<Websocket, ()> {
    //Status line
    assert_or_error(socket.next().await.ok_or(())? as char == 'G')?;
    assert_or_error(socket.next().await.ok_or(())? as char == 'E')?;
    assert_or_error(socket.next().await.ok_or(())? as char == 'T')?;
    assert_or_error(socket.next().await.ok_or(())? as char == ' ')?;
    //Rest of status line is irrelevant
    while socket.next().await.ok_or(())? as char != '\n' {};

    let mut wants_upgrade = false;
    let mut wants_websocket = false;
    let mut wants_websocket_13 = false;
    let mut security_thing: Option<[u8; 16]> = None;
    let mut header_name = String::new();
    'header_loop: loop {
        header_name.clear();
        const HEADERS_FINISHED: &'static str = "\r\n";
        const CONNECTION: &'static str = "connection: ";
        const UPGRADE: &'static str = "upgrade: ";
        const VERSION: &'static str = "sec-websocket-version: ";
        const SECURITY: &'static str = "sec-websocket-key: ";
        loop {
            header_name.push((socket.next().await.ok_or(())? as char).to_lowercase().next().ok_or(())?);
            if HEADERS_FINISHED.find(&header_name) == Some(0)
            || CONNECTION.find(&header_name) == Some(0)
            || UPGRADE.find(&header_name) == Some(0)
            || VERSION.find(&header_name) == Some(0)
            || SECURITY.find(&header_name) == Some(0) {
                if header_name == HEADERS_FINISHED {
                    break 'header_loop;
                }
                if header_name == CONNECTION
                || header_name == UPGRADE
                || header_name == VERSION
                || header_name == SECURITY {
                    let mut value = String::with_capacity(40);
                    let lowercase = header_name != SECURITY;
                    loop {
                        let mut dat = socket.next().await.ok_or(())? as char;
                        if dat == '\n' { break }
                        else if value.len() >= 40 { return Err(()) }
                        else {
                            if lowercase { dat = dat.to_lowercase().next().ok_or(())? };
                            value.push(dat);
                        }
                    }
                    if header_name == CONNECTION {
                        for token in value.trim().split(", ") { if token == "upgrade" { wants_upgrade = true; break; }; };
                        if !wants_upgrade { return Err(()) };
                    } else if header_name == UPGRADE {
                        if value.trim() != "websocket" { return Err(()) };
                        wants_websocket = true;
                    } else if header_name == VERSION {
                        if value.trim() != "13" { return Err(()) };
                        wants_websocket_13 = true;
                    } else if header_name == SECURITY {
                        if let Ok(result) = base64::decode(value) {
                            if result.len() == 16 {
                                let mut out = [0u8; 16];
                                for i in 0..16 { out[i] = result[i] };
                                security_thing = Some(out);
                            } else { return Err(()) }
                        } else { return Err(()) }
                    }
                    break;
                }
            }
        }
    }
    if !wants_upgrade || !wants_websocket || !wants_websocket_13 || security_thing.is_none() { return Err(()) };

    use sha::utils::{Digest, DigestExt};
    let encryption_response = base64::encode(security_thing.unwrap()).to_string() + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let encryption_response = base64::encode(&sha::sha1::Sha1::default().digest(encryption_response.as_bytes()).to_bytes());
    let response = format!(
        "HTTP/1.1 101 Upgrade\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-Websocket-Accept: {}\r\n\r\n",
        encryption_response
    ).as_bytes().to_vec();
    socket.queue_send(Arc::new(response));
    socket.flush().await;
    todo!("We made it!");
}

pub struct Websocket {
    socket: TcpStreamWrapper,
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

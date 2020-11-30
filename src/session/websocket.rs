use async_std::net::TcpStream;
use std::collections::VecDeque;
use futures::{Future, AsyncRead, AsyncWrite, Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

pub fn wrap_tcp_stream(socket: TcpStream) -> (TcpReader, TcpWriter) {
    (
        TcpReader {
            socket: socket.clone(),
            input_buf: [0u8; 64],
            input_slice: None
        },
        TcpWriter {
            socket: socket.clone(),
            output: VecDeque::new(),
            output_index: None
        }
    )
}

pub struct TcpReader {
    socket: TcpStream,
    input_buf: [u8; 64],
    input_slice: Option<(usize, usize)>,
}
impl Stream for TcpReader {
    type Item = u8;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        //Reading
        //Data is waiting to surface to the user
        if let Some((mut input_index, input_end)) = self.input_slice {
            let byte = self.input_buf[input_index];
            input_index += 1;
            //Finished with this data or not
            if input_index >= input_end { self.input_slice = None; }
            else { self.input_slice = Some((input_index, input_end)); };
            //Wake incase there's more work to do
            cx.waker().wake_by_ref();
            Poll::Ready(Some(byte))
        } else {
            let myself = self.deref_mut();
            match Pin::new(&mut myself.socket).poll_read(cx, &mut myself.input_buf) {
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

pub struct TcpWriter {
    socket: TcpStream,
    output: VecDeque<Arc<Vec<u8>>>,
    output_index: Option<usize>,
}
impl TcpWriter {
    pub fn queue_send(&mut self, dat: Arc<Vec<u8>>) {
        self.output.push_back(dat);
    }
}
impl Future for TcpWriter {
    type Output = Result<(),()>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(),()>> {
        //Writing
        if !self.output.is_empty() {
            //Position where the past write finished
            let mut output_index = if let Some(output_index) = self.output_index { output_index } else { 0 };
            let myself = self.deref_mut();
            match Pin::new(&mut myself.socket).poll_write(cx, &myself. output.get(0).unwrap()[output_index..]) {
                //Bytes were successfully written
                Poll::Ready(Ok(bytes_written)) => {
                    output_index += bytes_written;
                    if output_index >= self.output.get(0).unwrap().len() {
                        //This message is finished
                        self.output.pop_front();
                        //If there are more messages, reset output_index to 0
                        if !self.output.is_empty() { self.output_index = Some(0); }
                        else { self.output_index = None;  }
                    } else {
                        self.output_index = Some(output_index);
                    }
                    //Wake incase there is more work to do
                    cx.waker().wake_by_ref();
                    Poll::Pending //Only return Poll::Ready when there is no data left to write
                },
                Poll::Ready(Err(_)) => Poll::Ready(Err(())),
                Poll::Pending => Poll::Pending
            }
        } else {
            Poll::Ready(Ok(()))
        }
    }
}

fn assert_or_error(dat: bool) -> Result<(),()> { if dat { Ok(()) } else { Err(()) } }
pub async fn accept_websocket(socket: TcpStream) -> Result<(TcpReader, TcpWriter), ()> {
    let (mut socket, mut socket_out) = wrap_tcp_stream(socket);
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
                        if let Ok(result) = base64::decode(value.trim()) {
                            if result.len() == 16 {
                                let mut out = [0u8; 16];
                                for i in 0..16 { out[i] = result[i] };
                                security_thing = Some(out);
                            } else { break 'header_loop; }
                        } else { break 'header_loop; }
                    }
                    break;
                }
            } else {
                loop {
                    if socket.next().await.ok_or(())? as char == '\n' { break };
                };
                break;
            }
        }
    }
    
    if !wants_upgrade || !wants_websocket || !wants_websocket_13 || security_thing.is_none() { 
        socket_out.queue_send(Arc::new("HTTP/1.1 400 Bad Request\r\n\r\n".as_bytes().to_vec()));
        socket_out.await;
        return Err(())
    };

    use sha::utils::{Digest, DigestExt};
    let encryption_response = base64::encode(security_thing.unwrap()).to_string() + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let encryption_response = base64::encode(&sha::sha1::Sha1::default().digest(encryption_response.as_bytes()).to_bytes());
    let response = format!(
        "HTTP/1.1 101 Upgrade\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-Websocket-Accept: {}\r\n\r\n",
        encryption_response
    ).as_bytes().to_vec();
    socket_out.queue_send(Arc::new(response));
    (&mut socket_out).await?;
    Ok((socket, socket_out))
}

const OP_CONTINUE: u8 = 0;
const OP_TEXT: u8 = 1;
const OP_BINARY: u8 = 2;
const OP_CLOSE: u8 = 8;
const OP_PING: u8 = 9;
const OP_PONG: u8 = 10;

pub enum WsEvent<'a> {
    Message(WsMessageStream<'a>),
    Ping,
    Pong
}

pub type WsMessageStream<'a> = WsByteMaskMap<futures::stream::Take<&'a mut TcpReader>>;
pub struct WsByteMaskMap<S: Stream<Item=u8>> {
    mask: [u8; 4],
    i: usize,
    stream: S,
}
impl<S: Stream<Item=u8> + Unpin> Stream for WsByteMaskMap<S> {
    type Item = u8;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        match Pin::new(&mut self.stream).poll_next(cx) {
            Poll::Ready(Some(byte)) => {
                let byte = byte ^ self.mask[self.i % 4];
                self.i += 1;
                Poll::Ready(Some(byte))
            },
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending
        }
    }
}
impl<S: Stream<Item=u8>> WsByteMaskMap<S> {
    fn new(stream: S, mask: [u8; 4]) -> WsByteMaskMap<S> {
        WsByteMaskMap {
            mask,
            i: 0,
            stream
        }
    }
}

pub async fn read_ws_message<'a>/*<F: Future<Output=()>, C: Fn(u8) -> F>*/(socket: &'a mut TcpReader,) -> Result<WsEvent<'a>, ()> {
    use byte::BytesExt;
    //let mut is_first_frame = true;
    //loop {
        let first_byte = socket.next().await.ok_or(())?;
        let is_final_frame = first_byte & 0b10000000 > 0;
        if !is_final_frame { return Err(()) };
        let op_code = first_byte & 0b00001111;

        /*if (op_code == OP_CONTINUE && is_first_frame)
        || (op_code != OP_CONTINUE && !is_first_frame)
        { return Err(()) };*/

        let second_byte = socket.next().await.ok_or(())?;
        let is_masked = second_byte & 0b10000000 > 0;
        if !is_masked { return Err(()) };
        let payload_len = second_byte & 0b01111111;
        let payload_len = match payload_len {
            126 => {
                [
                    socket.next().await.ok_or(())?,
                    socket.next().await.ok_or(())?
                ].read_with::<u16>(&mut 0, byte::ctx::BE).or(Err(()))? as usize
            },
            127 => {
                [
                    socket.next().await.ok_or(())?,
                    socket.next().await.ok_or(())?,
                    socket.next().await.ok_or(())?,
                    socket.next().await.ok_or(())?,
                    socket.next().await.ok_or(())?,
                    socket.next().await.ok_or(())?,
                    socket.next().await.ok_or(())?,
                    socket.next().await.ok_or(())?,
                ].read_with::<u64>(&mut 0, byte::ctx::BE).or(Err(()))? as usize
            },
            _ => payload_len as usize,
        };

        let mask = [
            socket.next().await.ok_or(())?,
            socket.next().await.ok_or(())?,
            socket.next().await.ok_or(())?,
            socket.next().await.ok_or(())?,
        ]; //.read_with::<u32>(&mut 0, BE).or(Err(()))?;

        match op_code {
            OP_BINARY => {
                Ok(WsEvent::Message(WsByteMaskMap::new(socket.take(payload_len), mask)))
                //Ok(WsEvent::Message(socket.take(payload_len).enumerate().map(|(i, byte)| byte ^ mask[i % 4])))
                /*for i in 0..payload_len {
                    execute(socket.next().await.ok_or(())? ^ mask[i % 4]).await;
                }*/
            },
            OP_PING => return Ok(WsEvent::Ping),
            OP_PONG => return Ok(WsEvent::Pong),
            _ => return Err(()),
        }

        //if is_final_frame { break };
        //is_first_frame = false;
    //}
}

#[derive(Clone)]
pub struct OutboundWsMessage ( pub Arc<Vec<u8>> );
impl From<&Vec<u8>> for OutboundWsMessage {
    fn from(dat: &Vec<u8>) -> OutboundWsMessage {
        use byte::BytesExt;
        use byte::ctx::BE;
        let mut out = Vec::new();
        let mut bytes_read = 0;
        let mut is_first_frame = true;
        while bytes_read < dat.len() {
            let remaining = dat.len() - bytes_read;
            out.push(
               if remaining > (2usize.pow(63)) - 1 { 0b00000000 } else { 0b10000000 } //FINISHED bit
             | if is_first_frame { OP_BINARY } else { OP_CONTINUE } //OP Code
            );

            let payload_size = remaining.min(2usize.pow(63) - 1);
            if payload_size >= 2usize.pow(16) {
                out.push(127);
                let i = out.len();
                out.push(0);
                out.push(0);
                out.push(0);
                out.push(0);
                out.push(0);
                out.push(0);
                out.push(0);
                out.push(0);
                (&mut out[i..i+8]).write_with::<u64>(&mut 0, payload_size as u64, BE).unwrap();
            } else if payload_size > 125 {
                out.push(126);
                let i = out.len();
                out.push(0);
                out.push(0);
                (&mut out[i..i+2]).write_with::<u16>(&mut 0, payload_size as u16, BE).unwrap();
            } else {
                out.push(payload_size as u8);
            }

            out.extend_from_slice(&dat[bytes_read..bytes_read+payload_size]);
            bytes_read += payload_size;
            is_first_frame = false;
        }
        OutboundWsMessage ( Arc::new(out) )
    }
}

pub fn pong_message() -> OutboundWsMessage {
    OutboundWsMessage ( Arc::new(vec! [
        0b10001010,
        0b00000000,
    ]) )
}

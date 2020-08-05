use byte::{BytesExt, BE};

fn type_string_serialize(out: &mut Vec<u8>, string: &str) {
    if string.len() > 255 { out.push(0); }
    else { out.push(string.len() as u8); for cha in string.chars() { out.push(cha as u8); } }
}
fn type_string_deserialize(buf: &[u8], index: &mut usize) -> String {
    let size = buf[*index];
    *index += 1;
    let mut string = String::with_capacity(size as usize);
    let mut my_index = *index;
    *index += size as usize;
    while my_index < *index { string.push(buf[my_index] as char); my_index += 1; }
    string
}

fn type_float_serialize(out: &mut Vec<u8>, float: &f32) {
    let index = out.len();
    out.push(0); out.push(0); out.push(0); out.push(0);
    out.write_with::<f32>(&mut index, *float, BE);
}
fn type_float_deserialize(buf: &[u8], index: &mut usize) -> f32 {
    buf.read_with(index, BE).unwrap()
}

fn type_u16_serialize(out: &mut Vec<u8>, float: &u16) {
    let index = out.len();
    out.push(0); out.push(0);
    out.write_with::<u16>(&mut index, *float, byte::BE);
}
fn type_u16_deserialize(buf: &[u8], index: &mut usize) -> u16 {
    buf.read_with(index, byte::BE).unwrap()
}

pub enum ToServerMsg {
	Handshake { client: String, session: Option<String>, },
}
impl ToServerMsg {
	pub fn serialize(&self) -> Vec<u8> {
		let mut out: Vec<u8> = Vec::new();
		match self {
			Self::Handshake { client, session} => {
				out.push(0);
				type_string_serialize(&mut out, client);
				if let Some(tmp) = session {out.push(1); type_string_serialize(&mut out, tmp);} else {out.push(0);}
			},
		};
		out
	}
	pub fn deserialize(buf: &[u8], index: &mut usize) -> Self {
		let i = *index;
		*index += 1;
		match buf[i] {
			0 => {
				let client; let session;
				client = type_string_deserialize(&buf, &mut index);
				session = {if buf[*index] > 0 {*index += 1; let tmp; tmp = type_string_deserialize(&buf, &mut index); Some(tmp)} else {*index += 1; None}};
				ToServerMsg::Handshake { client, session}
			},
		}
	}
}

pub enum ToClientMsg {
	HandshakeAccepted { id: u16, },
}
impl ToClientMsg {
	pub fn serialize(&self) -> Vec<u8> {
		let mut out: Vec<u8> = Vec::new();
		match self {
			Self::HandshakeAccepted { id} => {
				out.push(0);
				type_u16_serialize(&mut out, id);
			},
		};
		out
	}
	pub fn deserialize(buf: &[u8], index: &mut usize) -> Self {
		let i = *index;
		*index += 1;
		match buf[i] {
			0 => {
				let id;
				id = type_u16_deserialize(&buf, &mut index);
				ToClientMsg::HandshakeAccepted { id}
			},
		}
	}
}


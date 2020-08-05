use byte::{BytesExt, BE};

fn type_string_serialize(out: &mut Vec<u8>, string: &str) {
    if string.len() > 255 { out.push(0); }
    else { out.push(string.len() as u8); for cha in string.chars() { out.push(cha as u8); } }
}
fn type_string_deserialize(buf: &[u8], index: &mut usize) -> Result<String, ()> {
    let size = *buf.get(*index).ok_or(())?;
    *index += 1;
    let mut string = String::with_capacity(size as usize);
    let mut my_index = *index;
    *index += size as usize;
    if buf.len() >= *index {
        while my_index < *index { string.push(buf[my_index] as char); my_index += 1; }
        Ok(string)
    } else { Err(()) }
}

fn type_float_serialize(out: &mut Vec<u8>, float: &f32) {
    let index = out.len();
    out.push(0); out.push(0); out.push(0); out.push(0);
    out.write_with::<f32>(&mut index, *float, BE);
}
fn type_float_deserialize(buf: &[u8], index: &mut usize) -> Result<f32, ()> {
    buf.read_with(index, BE).or(Err(()))
}

fn type_u16_serialize(out: &mut Vec<u8>, float: &u16) {
    let index = out.len();
    out.push(0); out.push(0);
    out.write_with::<u16>(&mut index, *float, byte::BE);
}
fn type_u16_deserialize(buf: &[u8], index: &mut usize) -> Result<u16, ()> {
    buf.read_with(index, byte::BE).or(Err(()))
}

fn type_float_pair_serialize(out: &mut Vec<u8>, pair: &(f32, f32)) {
    type_float_serialize(out, &pair.0);
    type_float_serialize(out, &pair.1);
}
fn type_float_pair_deserialize(buf: &[u8], index: &mut usize) -> Result<(f32,f32),()> {
    Ok((type_float_deserialize(buf, index)?, type_float_deserialize(buf, index)?))
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
	pub fn deserialize(buf: &[u8], index: &mut usize) -> Result<Self,()> {
		let i = *index;
		*index += 1;
		match buf[i] {
			0 => {
				let client; let session;
				client = type_string_deserialize(&buf, index)?;
				session = {if buf[*index] > 0 {*index += 1; let tmp; tmp = type_string_deserialize(&buf, index)?; Some(tmp)} else {*index += 1; None}};
				Ok(ToServerMsg::Handshake { client, session})
			},
			_ => Err(())
		}
	}
}

pub enum ToClientMsg {
	HandshakeAccepted { id: u16, },
	AddCelestialObject { name: String, display_name: String, radius: f32, id: u16, position: (f32,f32), },
}
impl ToClientMsg {
	pub fn serialize(&self) -> Vec<u8> {
		let mut out: Vec<u8> = Vec::new();
		match self {
			Self::HandshakeAccepted { id} => {
				out.push(0);
				type_u16_serialize(&mut out, id);
			},
			Self::AddCelestialObject { name, display_name, radius, id, position} => {
				out.push(1);
				type_string_serialize(&mut out, name);
				type_string_serialize(&mut out, display_name);
				type_float_serialize(&mut out, radius);
				type_u16_serialize(&mut out, id);
				type_float_pair_serialize(&mut out, position);
			},
		};
		out
	}
	pub fn deserialize(buf: &[u8], index: &mut usize) -> Result<Self,()> {
		let i = *index;
		*index += 1;
		match buf[i] {
			0 => {
				let id;
				id = type_u16_deserialize(&buf, index)?;
				Ok(ToClientMsg::HandshakeAccepted { id})
			},
			1 => {
				let name; let display_name; let radius; let id; let position;
				name = type_string_deserialize(&buf, index)?;
				display_name = type_string_deserialize(&buf, index)?;
				radius = type_float_deserialize(&buf, index)?;
				id = type_u16_deserialize(&buf, index)?;
				position = type_float_pair_deserialize(&buf, index)?;
				Ok(ToClientMsg::AddCelestialObject { name, display_name, radius, id, position})
			},
			_ => Err(())
		}
	}
}


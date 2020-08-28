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
    let mut index = out.len();
    out.push(0); out.push(0); out.push(0); out.push(0);
    out.write_with::<f32>(&mut index, *float, BE);
}
fn type_float_deserialize(buf: &[u8], index: &mut usize) -> Result<f32, ()> {
    buf.read_with(index, BE).or(Err(()))
}

fn type_u16_serialize(out: &mut Vec<u8>, ushort: &u16) {
    let mut index = out.len();
    out.push(0); out.push(0);
    out.write_with::<u16>(&mut index, *ushort, byte::BE);
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

fn type_u8_serialize(out: &mut Vec<u8>, ubyte: &u8) { out.push(*ubyte); }
fn type_u8_deserialize(buf: &[u8], index: &mut usize) -> Result<u8, ()> {
    let i = *index;
    *index += 1;
    buf.get(i).map(|val| *val).ok_or(())
}

fn type_bool_serialize(out: &mut Vec<u8>, boolean: &bool) { out.push(if *boolean { 1 } else { 0 }); }
fn type_bool_deserialize(buf: &[u8], index: &mut usize) -> Result<bool, ()> {
    let i = *index;
    *index += 1;
    buf.get(i).map(|val| *val > 0).ok_or(())
}

#[derive(Copy, Clone)] pub enum PartKind {
	Core, Cargo, LandingThruster, Hub
}
impl PartKind {
	pub fn serialize(&self, buf: &mut Vec<u8>) {
		buf.push(match self {
			Self::Core => 0, Self::Cargo => 1, Self::LandingThruster => 2, Self::Hub => 3
		});
	}
	pub fn deserialize(buf: &[u8], index: &mut usize) -> Result<Self, ()> {
		let me = buf[*index]; *index += 1;
		match me {
			0 => Ok(Self::Core), 1 => Ok(Self::Cargo), 2 => Ok(Self::LandingThruster), 3 => Ok(Self::Hub),
			_ => Err(())
		}
	}
}

pub enum ToServerMsg {
	Handshake { client: String, session: Option<String>, },
	SetThrusters { forward: bool, backward: bool, clockwise: bool, counter_clockwise: bool, },
	CommitGrab { grabbed_id: u16, x: f32, y: f32, },
	MoveGrab { x: f32, y: f32, },
	ReleaseGrab,
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
			Self::SetThrusters { forward, backward, clockwise, counter_clockwise} => {
				out.push(1);
				type_bool_serialize(&mut out, forward);
				type_bool_serialize(&mut out, backward);
				type_bool_serialize(&mut out, clockwise);
				type_bool_serialize(&mut out, counter_clockwise);
			},
			Self::CommitGrab { grabbed_id, x, y} => {
				out.push(2);
				type_u16_serialize(&mut out, grabbed_id);
				type_float_serialize(&mut out, x);
				type_float_serialize(&mut out, y);
			},
			Self::MoveGrab { x, y} => {
				out.push(3);
				type_float_serialize(&mut out, x);
				type_float_serialize(&mut out, y);
			},
			Self::ReleaseGrab { } => {
				out.push(4);
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
			1 => {
				let forward; let backward; let clockwise; let counter_clockwise;
				forward = type_bool_deserialize(&buf, index)?;
				backward = type_bool_deserialize(&buf, index)?;
				clockwise = type_bool_deserialize(&buf, index)?;
				counter_clockwise = type_bool_deserialize(&buf, index)?;
				Ok(ToServerMsg::SetThrusters { forward, backward, clockwise, counter_clockwise})
			},
			2 => {
				let grabbed_id; let x; let y;
				grabbed_id = type_u16_deserialize(&buf, index)?;
				x = type_float_deserialize(&buf, index)?;
				y = type_float_deserialize(&buf, index)?;
				Ok(ToServerMsg::CommitGrab { grabbed_id, x, y})
			},
			3 => {
				let x; let y;
				x = type_float_deserialize(&buf, index)?;
				y = type_float_deserialize(&buf, index)?;
				Ok(ToServerMsg::MoveGrab { x, y})
			},
			4 => {
				
				Ok(ToServerMsg::ReleaseGrab { })
			},
			_ => Err(())
		}
	}
}

pub enum ToClientMsg {
	HandshakeAccepted { id: u16, core_id: u16, },
	AddCelestialObject { name: String, display_name: String, radius: f32, id: u16, position: (f32,f32), },
	AddPart { id: u16, kind: PartKind, },
	MovePart { id: u16, x: f32, y: f32, rotation_n: f32, rotation_i: f32, },
	UpdatePartMeta { id: u16, owning_player: Option<u16>, thrust_mode: u8, },
	RemovePart { id: u16, },
	AddPlayer { id: u16, core_id: u16, name: String, },
	UpdatePlayerMeta { id: u16, thrust_forward: bool, thrust_backward: bool, thrust_clockwise: bool, thrust_counter_clockwise: bool, grabed_part: Option<u16>, },
	RemovePlayer { id: u16, },
	PostSimulationTick { your_fuel: u16, },
	UpdateMyMeta { max_fuel: u16, },
}
impl ToClientMsg {
	pub fn serialize(&self) -> Vec<u8> {
		let mut out: Vec<u8> = Vec::new();
		match self {
			Self::HandshakeAccepted { id, core_id} => {
				out.push(0);
				type_u16_serialize(&mut out, id);
				type_u16_serialize(&mut out, core_id);
			},
			Self::AddCelestialObject { name, display_name, radius, id, position} => {
				out.push(1);
				type_string_serialize(&mut out, name);
				type_string_serialize(&mut out, display_name);
				type_float_serialize(&mut out, radius);
				type_u16_serialize(&mut out, id);
				type_float_pair_serialize(&mut out, position);
			},
			Self::AddPart { id, kind} => {
				out.push(2);
				type_u16_serialize(&mut out, id);
				kind.serialize(&mut out);
			},
			Self::MovePart { id, x, y, rotation_n, rotation_i} => {
				out.push(3);
				type_u16_serialize(&mut out, id);
				type_float_serialize(&mut out, x);
				type_float_serialize(&mut out, y);
				type_float_serialize(&mut out, rotation_n);
				type_float_serialize(&mut out, rotation_i);
			},
			Self::UpdatePartMeta { id, owning_player, thrust_mode} => {
				out.push(4);
				type_u16_serialize(&mut out, id);
				if let Some(tmp) = owning_player {out.push(1); type_u16_serialize(&mut out, tmp);} else {out.push(0);}
				type_u8_serialize(&mut out, thrust_mode);
			},
			Self::RemovePart { id} => {
				out.push(5);
				type_u16_serialize(&mut out, id);
			},
			Self::AddPlayer { id, core_id, name} => {
				out.push(6);
				type_u16_serialize(&mut out, id);
				type_u16_serialize(&mut out, core_id);
				type_string_serialize(&mut out, name);
			},
			Self::UpdatePlayerMeta { id, thrust_forward, thrust_backward, thrust_clockwise, thrust_counter_clockwise, grabed_part} => {
				out.push(7);
				type_u16_serialize(&mut out, id);
				type_bool_serialize(&mut out, thrust_forward);
				type_bool_serialize(&mut out, thrust_backward);
				type_bool_serialize(&mut out, thrust_clockwise);
				type_bool_serialize(&mut out, thrust_counter_clockwise);
				if let Some(tmp) = grabed_part {out.push(1); type_u16_serialize(&mut out, tmp);} else {out.push(0);}
			},
			Self::RemovePlayer { id} => {
				out.push(8);
				type_u16_serialize(&mut out, id);
			},
			Self::PostSimulationTick { your_fuel} => {
				out.push(9);
				type_u16_serialize(&mut out, your_fuel);
			},
			Self::UpdateMyMeta { max_fuel} => {
				out.push(10);
				type_u16_serialize(&mut out, max_fuel);
			},
		};
		out
	}
	pub fn deserialize(buf: &[u8], index: &mut usize) -> Result<Self,()> {
		let i = *index;
		*index += 1;
		match buf[i] {
			0 => {
				let id; let core_id;
				id = type_u16_deserialize(&buf, index)?;
				core_id = type_u16_deserialize(&buf, index)?;
				Ok(ToClientMsg::HandshakeAccepted { id, core_id})
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
			2 => {
				let id; let kind;
				id = type_u16_deserialize(&buf, index)?;
				kind = PartKind::deserialize(&buf, index)?;
				Ok(ToClientMsg::AddPart { id, kind})
			},
			3 => {
				let id; let x; let y; let rotation_n; let rotation_i;
				id = type_u16_deserialize(&buf, index)?;
				x = type_float_deserialize(&buf, index)?;
				y = type_float_deserialize(&buf, index)?;
				rotation_n = type_float_deserialize(&buf, index)?;
				rotation_i = type_float_deserialize(&buf, index)?;
				Ok(ToClientMsg::MovePart { id, x, y, rotation_n, rotation_i})
			},
			4 => {
				let id; let owning_player; let thrust_mode;
				id = type_u16_deserialize(&buf, index)?;
				owning_player = {if buf[*index] > 0 {*index += 1; let tmp; tmp = type_u16_deserialize(&buf, index)?; Some(tmp)} else {*index += 1; None}};
				thrust_mode = type_u8_deserialize(&buf, index)?;
				Ok(ToClientMsg::UpdatePartMeta { id, owning_player, thrust_mode})
			},
			5 => {
				let id;
				id = type_u16_deserialize(&buf, index)?;
				Ok(ToClientMsg::RemovePart { id})
			},
			6 => {
				let id; let core_id; let name;
				id = type_u16_deserialize(&buf, index)?;
				core_id = type_u16_deserialize(&buf, index)?;
				name = type_string_deserialize(&buf, index)?;
				Ok(ToClientMsg::AddPlayer { id, core_id, name})
			},
			7 => {
				let id; let thrust_forward; let thrust_backward; let thrust_clockwise; let thrust_counter_clockwise; let grabed_part;
				id = type_u16_deserialize(&buf, index)?;
				thrust_forward = type_bool_deserialize(&buf, index)?;
				thrust_backward = type_bool_deserialize(&buf, index)?;
				thrust_clockwise = type_bool_deserialize(&buf, index)?;
				thrust_counter_clockwise = type_bool_deserialize(&buf, index)?;
				grabed_part = {if buf[*index] > 0 {*index += 1; let tmp; tmp = type_u16_deserialize(&buf, index)?; Some(tmp)} else {*index += 1; None}};
				Ok(ToClientMsg::UpdatePlayerMeta { id, thrust_forward, thrust_backward, thrust_clockwise, thrust_counter_clockwise, grabed_part})
			},
			8 => {
				let id;
				id = type_u16_deserialize(&buf, index)?;
				Ok(ToClientMsg::RemovePlayer { id})
			},
			9 => {
				let your_fuel;
				your_fuel = type_u16_deserialize(&buf, index)?;
				Ok(ToClientMsg::PostSimulationTick { your_fuel})
			},
			10 => {
				let max_fuel;
				max_fuel = type_u16_deserialize(&buf, index)?;
				Ok(ToClientMsg::UpdateMyMeta { max_fuel})
			},
			_ => Err(())
		}
	}
}


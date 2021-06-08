use byte::{BytesExt, BE};
use futures::{Stream, StreamExt};

fn type_string_serialize(out: &mut Vec<u8>, string: &str) {
    if string.len() > 255 { out.push(0); }
    else { out.push(string.len() as u8); for cha in string.chars() { out.push(cha as u8); } }
}
async fn type_string_deserialize<S: Stream<Item=u8>+Unpin>(stream: &mut S) -> Result<String,()> {
    let size = stream.next().await.ok_or(())?;
    let mut string = String::with_capacity(size as usize);
    for _ in 0..size { string.push(stream.next().await.ok_or(())? as char); }
    Ok(string)
}

fn type_float_serialize(out: &mut Vec<u8>, float: &f32) {
    let mut index = out.len();
    out.push(0); out.push(0); out.push(0); out.push(0);
    out.write_with::<f32>(&mut index, *float, BE);
}
async fn type_float_deserialize<S: Stream<Item=u8>+Unpin>(stream: &mut S) -> Result<f32, ()> {
    let buf = [
        stream.next().await.ok_or(())?,
        stream.next().await.ok_or(())?,
        stream.next().await.ok_or(())?,
        stream.next().await.ok_or(())?,
    ];
    buf.read_with(&mut 0, byte::BE).or(Err(()))
}

fn type_u16_serialize(out: &mut Vec<u8>, ushort: &u16) {
    let mut index = out.len();
    out.push(0); out.push(0);
    out.write_with::<u16>(&mut index, *ushort, byte::BE);
}
async fn type_u16_deserialize<S: Stream<Item=u8>+Unpin>(stream: &mut S) -> Result<u16, ()> {
    let buf = [
        stream.next().await.ok_or(())?,
        stream.next().await.ok_or(())?,
    ];
    buf.read_with(&mut 0, byte::BE).or(Err(()))
}

fn type_u32_serialize(out: &mut Vec<u8>, uint: &u32) {
    let mut index = out.len();
    out.push(0); out.push(0); out.push(0); out.push(0);
    out.write_with::<u32>(&mut index, *uint, byte::BE);
}
async fn type_u32_deserialize<S: Stream<Item=u8>+Unpin>(stream: &mut S) -> Result<u32, ()> {
    let buf = [
        stream.next().await.ok_or(())?,
        stream.next().await.ok_or(())?,
        stream.next().await.ok_or(())?,
        stream.next().await.ok_or(())?,
    ];
    buf.read_with(&mut 0, byte::BE).or(Err(()))
}

fn type_float_pair_serialize(out: &mut Vec<u8>, pair: &(f32, f32)) {
    type_float_serialize(out, &pair.0);
    type_float_serialize(out, &pair.1);
}
async fn type_float_pair_deserialize<S: Stream<Item=u8>+Unpin>(stream: &mut S) -> Result<(f32, f32), ()> {
    Ok((type_float_deserialize(stream).await?, type_float_deserialize(stream).await?))
}

fn type_u8_serialize(out: &mut Vec<u8>, ubyte: &u8) { out.push(*ubyte); }
async fn type_u8_deserialize<S: Stream<Item=u8>+Unpin>(stream: &mut S) -> Result<u8,()> {
    stream.next().await.ok_or(())
}

fn type_bool_serialize(out: &mut Vec<u8>, boolean: &bool) { out.push(if *boolean { 1 } else { 0 }); }
async fn type_bool_deserialize<S: Stream<Item=u8>+Unpin>(stream: &mut S) -> Result<bool,()> {
    stream.next().await.map(|val| val > 0).ok_or(())
}


#[derive(Copy, Clone, Eq, PartialEq, Debug)] pub enum PartKind {
	Core, Cargo, LandingThruster, Hub, SolarPanel, EcoThruster, Thruster, SuperThruster, PowerHub, HubThruster, LandingWheel
}
impl PartKind {
	pub fn val_of(&self) -> u8 { match self {
			Self::Core => 0, Self::Cargo => 1, Self::LandingThruster => 2, Self::Hub => 3, Self::SolarPanel => 4, Self::EcoThruster => 5, Self::Thruster => 6, Self::SuperThruster => 7, Self::PowerHub => 8, Self::HubThruster => 9, Self::LandingWheel => 10
		} }
	pub fn serialize(&self, buf: &mut Vec<u8>) {
		buf.push(self.val_of());
	}
	pub async fn deserialize<S: Stream<Item=u8>+Unpin>(stream: &mut S) -> Result<Self, ()> {
		let me = stream.next().await.ok_or(())?;
		match me {
			0 => Ok(Self::Core), 1 => Ok(Self::Cargo), 2 => Ok(Self::LandingThruster), 3 => Ok(Self::Hub), 4 => Ok(Self::SolarPanel), 5 => Ok(Self::EcoThruster), 6 => Ok(Self::Thruster), 7 => Ok(Self::SuperThruster), 8 => Ok(Self::PowerHub), 9 => Ok(Self::HubThruster), 10 => Ok(Self::LandingWheel),
			_ => Err(())
		}
	}
}
#[derive(Copy, Clone, Eq, PartialEq, Debug)] pub enum PlanetKind {
	Earth, Venus, Mars, Moon, Sun, Mercury, Neptune, Uranus, Jupiter, Saturn, Pluto, Trade
}
impl PlanetKind {
	pub fn val_of(&self) -> u8 { match self {
			Self::Earth => 0, Self::Venus => 1, Self::Mars => 2, Self::Moon => 3, Self::Sun => 4, Self::Mercury => 5, Self::Neptune => 6, Self::Uranus => 7, Self::Jupiter => 8, Self::Saturn => 9, Self::Pluto => 10, Self::Trade => 11
		} }
	pub fn serialize(&self, buf: &mut Vec<u8>) {
		buf.push(self.val_of());
	}
	pub async fn deserialize<S: Stream<Item=u8>+Unpin>(stream: &mut S) -> Result<Self, ()> {
		let me = stream.next().await.ok_or(())?;
		match me {
			0 => Ok(Self::Earth), 1 => Ok(Self::Venus), 2 => Ok(Self::Mars), 3 => Ok(Self::Moon), 4 => Ok(Self::Sun), 5 => Ok(Self::Mercury), 6 => Ok(Self::Neptune), 7 => Ok(Self::Uranus), 8 => Ok(Self::Jupiter), 9 => Ok(Self::Saturn), 10 => Ok(Self::Pluto), 11 => Ok(Self::Trade),
			_ => Err(())
		}
	}
}

pub enum ToServerMsg {
	Handshake { client: String, session: Option<String>, name: String, },
	SetThrusters { forward: bool, backward: bool, clockwise: bool, counter_clockwise: bool, },
	CommitGrab { grabbed_id: u16, x: f32, y: f32, },
	MoveGrab { x: f32, y: f32, },
	ReleaseGrab,
	BeamOut,
	SendChatMessage { msg: String, },
	RequestUpdate,
}
impl ToServerMsg {
	pub fn serialize(&self, out: &mut Vec<u8>) {
		match self {
			Self::Handshake { client, session, name} => {
				out.push(0);
				type_string_serialize(out, client);
				if let Some(tmp) = session {out.push(1); type_string_serialize(out, tmp);} else {out.push(0);}
				type_string_serialize(out, name);
			},
			Self::SetThrusters { forward, backward, clockwise, counter_clockwise} => {
				out.push(1);
				type_bool_serialize(out, forward);
				type_bool_serialize(out, backward);
				type_bool_serialize(out, clockwise);
				type_bool_serialize(out, counter_clockwise);
			},
			Self::CommitGrab { grabbed_id, x, y} => {
				out.push(2);
				type_u16_serialize(out, grabbed_id);
				type_float_serialize(out, x);
				type_float_serialize(out, y);
			},
			Self::MoveGrab { x, y} => {
				out.push(3);
				type_float_serialize(out, x);
				type_float_serialize(out, y);
			},
			Self::ReleaseGrab { } => {
				out.push(4);
			},
			Self::BeamOut { } => {
				out.push(5);
			},
			Self::SendChatMessage { msg} => {
				out.push(6);
				type_string_serialize(out, msg);
			},
			Self::RequestUpdate { } => {
				out.push(7);
			},
		};
	}
	pub async fn deserialize<S: Stream<Item=u8>+Unpin>(stream: &mut S) -> Result<Self, ()> {
		match stream.next().await.ok_or(())? {
			0 => {
				let client; let session; let name;
				client = type_string_deserialize(stream).await?;
				session = {if stream.next().await.ok_or(())? > 0 { let tmp; tmp = type_string_deserialize(stream).await?; Some(tmp)} else { None }};
				name = type_string_deserialize(stream).await?;
				Ok(ToServerMsg::Handshake { client, session, name})
			},
			1 => {
				let forward; let backward; let clockwise; let counter_clockwise;
				forward = type_bool_deserialize(stream).await?;
				backward = type_bool_deserialize(stream).await?;
				clockwise = type_bool_deserialize(stream).await?;
				counter_clockwise = type_bool_deserialize(stream).await?;
				Ok(ToServerMsg::SetThrusters { forward, backward, clockwise, counter_clockwise})
			},
			2 => {
				let grabbed_id; let x; let y;
				grabbed_id = type_u16_deserialize(stream).await?;
				x = type_float_deserialize(stream).await?;
				y = type_float_deserialize(stream).await?;
				Ok(ToServerMsg::CommitGrab { grabbed_id, x, y})
			},
			3 => {
				let x; let y;
				x = type_float_deserialize(stream).await?;
				y = type_float_deserialize(stream).await?;
				Ok(ToServerMsg::MoveGrab { x, y})
			},
			4 => {
				
				Ok(ToServerMsg::ReleaseGrab { })
			},
			5 => {
				
				Ok(ToServerMsg::BeamOut { })
			},
			6 => {
				let msg;
				msg = type_string_deserialize(stream).await?;
				Ok(ToServerMsg::SendChatMessage { msg})
			},
			7 => {
				
				Ok(ToServerMsg::RequestUpdate { })
			},
			_ => Err(())
		}
	}
}

pub enum ToClientMsg {
	MessagePack { count: u16, },
	HandshakeAccepted { id: u16, core_id: u16, can_beamout: bool, },
	AddCelestialObject { id: u8, kind: PlanetKind, radius: f32, position: (f32,f32), },
	InitCelestialOrbit { id: u8, orbit_around_body: u8, orbit_radius: (f32,f32), orbit_rotation: f32, orbit_total_ticks: u32, },
	UpdateCelestialOrbit { id: u8, orbit_ticks_ellapsed: u32, },
	AddPart { id: u16, kind: PartKind, },
	MovePart { id: u16, x: f32, y: f32, rotation_n: f32, rotation_i: f32, },
	UpdatePartMeta { id: u16, owning_player: Option<u16>, thrust_mode: u8, },
	RemovePart { id: u16, },
	AddPlayer { id: u16, core_id: u16, name: String, },
	UpdatePlayerMeta { id: u16, thrust_forward: bool, thrust_backward: bool, thrust_clockwise: bool, thrust_counter_clockwise: bool, grabed_part: Option<u16>, },
	UpdatePlayerVelocity { id: u16, vel_x: f32, vel_y: f32, },
	RemovePlayer { id: u16, },
	PostSimulationTick { your_power: u32, },
	UpdateMyMeta { max_power: u32, can_beamout: bool, },
	BeamOutAnimation { player_id: u16, },
	IncinerationAnimation { player_id: u16, },
	ChatMessage { username: String, msg: String, color: String, },
}
impl ToClientMsg {
	pub fn serialize(&self, out: &mut Vec<u8>) {
		match self {
			Self::MessagePack { count} => {
				out.push(0);
				type_u16_serialize(out, count);
			},
			Self::HandshakeAccepted { id, core_id, can_beamout} => {
				out.push(1);
				type_u16_serialize(out, id);
				type_u16_serialize(out, core_id);
				type_bool_serialize(out, can_beamout);
			},
			Self::AddCelestialObject { id, kind, radius, position} => {
				out.push(2);
				type_u8_serialize(out, id);
				kind.serialize(out);
				type_float_serialize(out, radius);
				type_float_pair_serialize(out, position);
			},
			Self::InitCelestialOrbit { id, orbit_around_body, orbit_radius, orbit_rotation, orbit_total_ticks} => {
				out.push(3);
				type_u8_serialize(out, id);
				type_u8_serialize(out, orbit_around_body);
				type_float_pair_serialize(out, orbit_radius);
				type_float_serialize(out, orbit_rotation);
				type_u32_serialize(out, orbit_total_ticks);
			},
			Self::UpdateCelestialOrbit { id, orbit_ticks_ellapsed} => {
				out.push(4);
				type_u8_serialize(out, id);
				type_u32_serialize(out, orbit_ticks_ellapsed);
			},
			Self::AddPart { id, kind} => {
				out.push(5);
				type_u16_serialize(out, id);
				kind.serialize(out);
			},
			Self::MovePart { id, x, y, rotation_n, rotation_i} => {
				out.push(6);
				type_u16_serialize(out, id);
				type_float_serialize(out, x);
				type_float_serialize(out, y);
				type_float_serialize(out, rotation_n);
				type_float_serialize(out, rotation_i);
			},
			Self::UpdatePartMeta { id, owning_player, thrust_mode} => {
				out.push(7);
				type_u16_serialize(out, id);
				if let Some(tmp) = owning_player {out.push(1); type_u16_serialize(out, tmp);} else {out.push(0);}
				type_u8_serialize(out, thrust_mode);
			},
			Self::RemovePart { id} => {
				out.push(8);
				type_u16_serialize(out, id);
			},
			Self::AddPlayer { id, core_id, name} => {
				out.push(9);
				type_u16_serialize(out, id);
				type_u16_serialize(out, core_id);
				type_string_serialize(out, name);
			},
			Self::UpdatePlayerMeta { id, thrust_forward, thrust_backward, thrust_clockwise, thrust_counter_clockwise, grabed_part} => {
				out.push(10);
				type_u16_serialize(out, id);
				type_bool_serialize(out, thrust_forward);
				type_bool_serialize(out, thrust_backward);
				type_bool_serialize(out, thrust_clockwise);
				type_bool_serialize(out, thrust_counter_clockwise);
				if let Some(tmp) = grabed_part {out.push(1); type_u16_serialize(out, tmp);} else {out.push(0);}
			},
			Self::UpdatePlayerVelocity { id, vel_x, vel_y} => {
				out.push(11);
				type_u16_serialize(out, id);
				type_float_serialize(out, vel_x);
				type_float_serialize(out, vel_y);
			},
			Self::RemovePlayer { id} => {
				out.push(12);
				type_u16_serialize(out, id);
			},
			Self::PostSimulationTick { your_power} => {
				out.push(13);
				type_u32_serialize(out, your_power);
			},
			Self::UpdateMyMeta { max_power, can_beamout} => {
				out.push(14);
				type_u32_serialize(out, max_power);
				type_bool_serialize(out, can_beamout);
			},
			Self::BeamOutAnimation { player_id} => {
				out.push(15);
				type_u16_serialize(out, player_id);
			},
			Self::IncinerationAnimation { player_id} => {
				out.push(16);
				type_u16_serialize(out, player_id);
			},
			Self::ChatMessage { username, msg, color} => {
				out.push(17);
				type_string_serialize(out, username);
				type_string_serialize(out, msg);
				type_string_serialize(out, color);
			},
		};
	}
	pub async fn deserialize<S: Stream<Item=u8>+Unpin>(stream: &mut S) -> Result<Self, ()> {
		match stream.next().await.ok_or(())? {
			0 => {
				let count;
				count = type_u16_deserialize(stream).await?;
				Ok(ToClientMsg::MessagePack { count})
			},
			1 => {
				let id; let core_id; let can_beamout;
				id = type_u16_deserialize(stream).await?;
				core_id = type_u16_deserialize(stream).await?;
				can_beamout = type_bool_deserialize(stream).await?;
				Ok(ToClientMsg::HandshakeAccepted { id, core_id, can_beamout})
			},
			2 => {
				let id; let kind; let radius; let position;
				id = type_u8_deserialize(stream).await?;
				kind = PlanetKind::deserialize(stream).await?;
				radius = type_float_deserialize(stream).await?;
				position = type_float_pair_deserialize(stream).await?;
				Ok(ToClientMsg::AddCelestialObject { id, kind, radius, position})
			},
			3 => {
				let id; let orbit_around_body; let orbit_radius; let orbit_rotation; let orbit_total_ticks;
				id = type_u8_deserialize(stream).await?;
				orbit_around_body = type_u8_deserialize(stream).await?;
				orbit_radius = type_float_pair_deserialize(stream).await?;
				orbit_rotation = type_float_deserialize(stream).await?;
				orbit_total_ticks = type_u32_deserialize(stream).await?;
				Ok(ToClientMsg::InitCelestialOrbit { id, orbit_around_body, orbit_radius, orbit_rotation, orbit_total_ticks})
			},
			4 => {
				let id; let orbit_ticks_ellapsed;
				id = type_u8_deserialize(stream).await?;
				orbit_ticks_ellapsed = type_u32_deserialize(stream).await?;
				Ok(ToClientMsg::UpdateCelestialOrbit { id, orbit_ticks_ellapsed})
			},
			5 => {
				let id; let kind;
				id = type_u16_deserialize(stream).await?;
				kind = PartKind::deserialize(stream).await?;
				Ok(ToClientMsg::AddPart { id, kind})
			},
			6 => {
				let id; let x; let y; let rotation_n; let rotation_i;
				id = type_u16_deserialize(stream).await?;
				x = type_float_deserialize(stream).await?;
				y = type_float_deserialize(stream).await?;
				rotation_n = type_float_deserialize(stream).await?;
				rotation_i = type_float_deserialize(stream).await?;
				Ok(ToClientMsg::MovePart { id, x, y, rotation_n, rotation_i})
			},
			7 => {
				let id; let owning_player; let thrust_mode;
				id = type_u16_deserialize(stream).await?;
				owning_player = {if stream.next().await.ok_or(())? > 0 { let tmp; tmp = type_u16_deserialize(stream).await?; Some(tmp)} else { None }};
				thrust_mode = type_u8_deserialize(stream).await?;
				Ok(ToClientMsg::UpdatePartMeta { id, owning_player, thrust_mode})
			},
			8 => {
				let id;
				id = type_u16_deserialize(stream).await?;
				Ok(ToClientMsg::RemovePart { id})
			},
			9 => {
				let id; let core_id; let name;
				id = type_u16_deserialize(stream).await?;
				core_id = type_u16_deserialize(stream).await?;
				name = type_string_deserialize(stream).await?;
				Ok(ToClientMsg::AddPlayer { id, core_id, name})
			},
			10 => {
				let id; let thrust_forward; let thrust_backward; let thrust_clockwise; let thrust_counter_clockwise; let grabed_part;
				id = type_u16_deserialize(stream).await?;
				thrust_forward = type_bool_deserialize(stream).await?;
				thrust_backward = type_bool_deserialize(stream).await?;
				thrust_clockwise = type_bool_deserialize(stream).await?;
				thrust_counter_clockwise = type_bool_deserialize(stream).await?;
				grabed_part = {if stream.next().await.ok_or(())? > 0 { let tmp; tmp = type_u16_deserialize(stream).await?; Some(tmp)} else { None }};
				Ok(ToClientMsg::UpdatePlayerMeta { id, thrust_forward, thrust_backward, thrust_clockwise, thrust_counter_clockwise, grabed_part})
			},
			11 => {
				let id; let vel_x; let vel_y;
				id = type_u16_deserialize(stream).await?;
				vel_x = type_float_deserialize(stream).await?;
				vel_y = type_float_deserialize(stream).await?;
				Ok(ToClientMsg::UpdatePlayerVelocity { id, vel_x, vel_y})
			},
			12 => {
				let id;
				id = type_u16_deserialize(stream).await?;
				Ok(ToClientMsg::RemovePlayer { id})
			},
			13 => {
				let your_power;
				your_power = type_u32_deserialize(stream).await?;
				Ok(ToClientMsg::PostSimulationTick { your_power})
			},
			14 => {
				let max_power; let can_beamout;
				max_power = type_u32_deserialize(stream).await?;
				can_beamout = type_bool_deserialize(stream).await?;
				Ok(ToClientMsg::UpdateMyMeta { max_power, can_beamout})
			},
			15 => {
				let player_id;
				player_id = type_u16_deserialize(stream).await?;
				Ok(ToClientMsg::BeamOutAnimation { player_id})
			},
			16 => {
				let player_id;
				player_id = type_u16_deserialize(stream).await?;
				Ok(ToClientMsg::IncinerationAnimation { player_id})
			},
			17 => {
				let username; let msg; let color;
				username = type_string_deserialize(stream).await?;
				msg = type_string_deserialize(stream).await?;
				color = type_string_deserialize(stream).await?;
				Ok(ToClientMsg::ChatMessage { username, msg, color})
			},
			_ => Err(())
		}
	}
}


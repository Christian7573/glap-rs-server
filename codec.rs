

enum ToServerMsg {
	%s { client: <bound method BasicType.rust_signature of <__main__.BasicType object at 0x7fe02b2d55e0>>, session: <bound method OptionType.rust_signature of <__main__.OptionType object at 0x7fe02b2750d0>>, },
}
impl ToServerMsg {
	pub fn serialize(&self) -> Vec<u8> {
		let mut out: Vec<u8> = Vec::new();
		match self {
			Self::Handshake { client, session} => {
				out.push(0);
				type_string_serialize(&mut out, self.client);
				if let Some(tmp) = self.session {out.push(1); type_string_serialize(&mut out, tmp);} else {out.push(0);}
			},		};
		out
	}
	pub fn deserialize(buf: &[u8], index: &mut usize) -> Self {
		let i = *index;
		index += 1;
		match buf[i] {
			0 => {
				let client; let session;
				client = type_string_deserialize(&buf, &mut index);
				session = {if buf[index] > 0 {index += 1; let tmp; tmp = type_string_deserialize(&buf, &mut index); Some(tmp)} else {index += 1; None}};
				Handshake { client, session}
			},
		};
		out
	}
}

enum ToClientMsg {
	%s { id: <bound method BasicType.rust_signature of <__main__.BasicType object at 0x7fe02b2d5640>>, },
}
impl ToClientMsg {
	pub fn serialize(&self) -> Vec<u8> {
		let mut out: Vec<u8> = Vec::new();
		match self {
			Self::HandshakeAccepted { id} => {
				out.push(0);
				type_u16_serialize(&mut out, self.id);
			},		};
		out
	}
	pub fn deserialize(buf: &[u8], index: &mut usize) -> Self {
		let i = *index;
		index += 1;
		match buf[i] {
			0 => {
				let id;
				id = type_u16_deserialize(&buf, &mut index);
				HandshakeAccepted { id}
			},
		};
		out
	}
}


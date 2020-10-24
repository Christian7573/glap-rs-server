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

fn type_u32_serialize(out: &mut Vec<u8>, uint: &u32) {
    let mut index = out.len();
    out.push(0); out.push(0); out.push(0); out.push(0);
    out.write_with::<u32>(&mut index, *uint, byte::BE);
}
fn type_u32_deserialize(buf: &[u8], index: &mut usize) -> Result<u32, ()> {
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

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

fn type_float_pair_serialize(out: &mut Vec<u8>, pair: &(f32, f32)) {
    type_float_serialize(out, &pair.0);
    type_float_serialize(out, &pair.1);
}
fn type_float_pair_deserialize(buf: &[u8], index: &mut usize) -> (f32, f32) {
    (type_float_deserialize(buf, index), type_float_deserialize(buf, index))
}
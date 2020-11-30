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

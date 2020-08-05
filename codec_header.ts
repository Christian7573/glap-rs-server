export class Box<T> {
    v: T;
    constructor(v: T) { this.v = v; }
}

function type_string_serialize(out: number[], string: string) {
    if (string.length > 255) { out.push(0); }
    else {
        out.push(string.length);
        for (let i = 0; i < string.length; i++) out.push(string.charCodeAt(i));
    }
}
function type_string_deserialize(buf: Uint8Array, index: Box<number>): string {
    let out = "";
    let i = index.v + 1;
    index.v += buf[index.v += 1];
    while (i < index.v) out += String.fromCharCode(buf[i += 1]);
    return out; 
}

function type_float_serialize(out: number[], float: number) {
    const arr = new Float32Array([float]);
    const view = new Uint8Array(arr.buffer);
    out.push(view[0], view[1], view[2], view[3]);
}
function type_float_deserialize(buf: Uint8Array, index: Box<number>): number {
    const view = new Float32Array(buf.buffer, index.v, 1);
    return view[0];
}

function type_ushort_serialize(out: number[], ushort: number) {
    const arr = new Uint16Array([ushort]);
    const view = new Uint8Array(arr.buffer);
    out.push(view[0], view[1]);
}
function type_ushort_deserialize(buf: Uint8Array, index: Box<number>): number {
    const view = new Uint16Array(buf.buffer, index.v, 1);
    return view[0];
}

function type_float_pair_serialize(out: number[], pair: [number, number]) {
    type_float_serialize(out, pair[0])
    type_float_serialize(out, pair[1]);
}
function type_float_pair_deserialize(buf: Uint8Array, index: Box<number>): [number, number] {
    return [type_float_deserialize(buf, index), type_float_deserialize(buf, index)];
}
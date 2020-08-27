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
    const size = buf[index.v];
    index.v++;
    let i = index.v;	
    index.v += size;
    while (i < index.v) out += String.fromCharCode(buf[i++]);
    return out; 
}

function type_float_serialize(out: number[], float: number) {
    const arr = new Float32Array([float]);
    const view = new Uint8Array(arr.buffer);
    out.push(view[3], view[2], view[1], view[0]);
}
function type_float_deserialize(buf: Uint8Array, index: Box<number>): number {
    const arr = new Uint8Array([buf[index.v+3], buf[index.v+2], buf[index.v+1], buf[index.v]]);
    const view = new Float32Array(arr.buffer);
    index.v += 4;
    return view[0];
}

function type_ushort_serialize(out: number[], ushort: number) {
    const arr = new Uint16Array([ushort]);
    const view = new Uint8Array(arr.buffer);
    out.push(view[1], view[0]);
}
function type_ushort_deserialize(buf: Uint8Array, index: Box<number>): number {
    const arr = new Uint8Array([buf[index.v+1], buf[index.v]]);
    const view = new Uint16Array(arr.buffer);
    index.v += 2;
    return view[0];
}

function type_float_pair_serialize(out: number[], pair: [number, number]) {
    type_float_serialize(out, pair[0])
    type_float_serialize(out, pair[1]);
}
function type_float_pair_deserialize(buf: Uint8Array, index: Box<number>): [number, number] {
    return [type_float_deserialize(buf, index), type_float_deserialize(buf, index)];
}

function type_ubyte_serialize(out: number[], ubyte: number) { out.push(ubyte); }
function type_ubyte_deserialize(buf: Uint8Array, index: Box<number>): number { return buf[index.v++]; }

function type_boolean_serialize(out: number[], bool: boolean) { out.push(bool ? 1 : 0); }
function type_boolean_deserialize(buf: Uint8Array, index: Box<number>): boolean { return buf[index.v++] > 0; }
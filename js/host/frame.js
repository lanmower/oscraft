// Local frame codec. The js/wire lane replaces this file; keep the exports stable.
// Frame layout (little-endian):
//   order: [kind=1][u32 unit][i32 x][i32 y]
//   state: [kind=2][u32 tick][u32 units][u8 hashLen][hash utf8]

export const KIND_ORDER = 1;
export const KIND_STATE = 2;

export function encodeOrder({ unit, x, y }) {
  const b = Buffer.alloc(13);
  b.writeUInt8(KIND_ORDER, 0);
  b.writeUInt32LE(unit >>> 0, 1);
  b.writeInt32LE(x | 0, 5);
  b.writeInt32LE(y | 0, 9);
  return new Uint8Array(b);
}

export function encodeState({ tick, units, hash }) {
  const h = Buffer.from(String(hash), 'utf8').subarray(0, 255);
  const b = Buffer.alloc(10 + h.length);
  b.writeUInt8(KIND_STATE, 0);
  b.writeUInt32LE(tick >>> 0, 1);
  b.writeUInt32LE(units >>> 0, 5);
  b.writeUInt8(h.length, 9);
  h.copy(b, 10);
  return new Uint8Array(b);
}

export function decodeFrame(data) {
  const b = Buffer.from(data.buffer, data.byteOffset, data.byteLength);
  if (b.length < 1) return null;
  if (b[0] === KIND_ORDER && b.length === 13) {
    return { kind: 'order', unit: b.readUInt32LE(1), x: b.readInt32LE(5), y: b.readInt32LE(9) };
  }
  if (b[0] === KIND_STATE && b.length >= 10) {
    const n = b.readUInt8(9);
    return { kind: 'state', tick: b.readUInt32LE(1), units: b.readUInt32LE(5), hash: b.toString('utf8', 10, 10 + n) };
  }
  return null;
}

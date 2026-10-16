// Local binary frame codec for the net client. The wire lane (js/wire/) replaces this file.
// Little-endian throughout.
//   order frame (client -> host):  u8 type=1, u32 unitId, f32 x, f32 z                 (13 bytes)
//   state frame (host -> clients): u8 type=2, u32 tick, u16 count,
//                                  count x (u32 unitId, f32 x, f32 z, u8 player)       (7 + 13n bytes)

export const ORDER_MOVE = 1;
export const STATE = 2;
const ORDER_BYTES = 13;
const HEADER_BYTES = 7;
const UNIT_BYTES = 13;

export function encodeOrderMove(unitId, x, z) {
  const buf = new ArrayBuffer(ORDER_BYTES);
  const view = new DataView(buf);
  view.setUint8(0, ORDER_MOVE);
  view.setUint32(1, unitId, true);
  view.setFloat32(5, x, true);
  view.setFloat32(9, z, true);
  return new Uint8Array(buf);
}

export function decodeState(bytes) {
  if (bytes.byteLength < HEADER_BYTES) return null;
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  if (view.getUint8(0) !== STATE) return null;
  const tick = view.getUint32(1, true);
  const count = view.getUint16(5, true);
  if (bytes.byteLength < HEADER_BYTES + count * UNIT_BYTES) return null;
  const units = new Array(count);
  for (let i = 0; i < count; i++) {
    const o = HEADER_BYTES + i * UNIT_BYTES;
    units[i] = {
      id: view.getUint32(o, true),
      x: view.getFloat32(o + 4, true),
      z: view.getFloat32(o + 8, true),
      player: view.getUint8(o + 12),
    };
  }
  return { tick, units };
}

export function encodeState(tick, units) {
  const buf = new ArrayBuffer(HEADER_BYTES + units.length * UNIT_BYTES);
  const view = new DataView(buf);
  view.setUint8(0, STATE);
  view.setUint32(1, tick, true);
  view.setUint16(5, units.length, true);
  units.forEach((u, i) => {
    const o = HEADER_BYTES + i * UNIT_BYTES;
    view.setUint32(o, u.id, true);
    view.setFloat32(o + 4, u.x, true);
    view.setFloat32(o + 8, u.z, true);
    view.setUint8(o + 12, u.player);
  });
  return new Uint8Array(buf);
}

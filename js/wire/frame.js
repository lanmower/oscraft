export const WIRE_VERSION = 1;

export const FRAME = Object.freeze({ ORDER: 1, STATE: 2, HELLO: 3, ERROR: 4, DELTA: 5 });

export const MAX_UNITS = 1024;
export const MAX_ROOM_BYTES = 64;
export const MAX_MESSAGE_BYTES = 256;

const U32 = 0xffffffff;
const I32_MIN = -0x80000000;
const I32_MAX = 0x7fffffff;
const U16 = 0xffff;
const U8 = 0xff;

export class WireError extends Error {
  constructor(code, detail) {
    super(`${code}: ${detail}`);
    this.name = 'WireError';
    this.code = code;
  }
}

const encoder = new TextEncoder();
const decoder = new TextDecoder('utf-8', { fatal: true });

function check(value, min, max, field) {
  if (!Number.isInteger(value) || value < min || value > max) {
    throw new WireError('RANGE', `${field}=${value} outside [${min},${max}]`);
  }
  return value;
}

class Writer {
  constructor(size) {
    this.bytes = new Uint8Array(size);
    this.view = new DataView(this.bytes.buffer);
    this.at = 0;
  }
  u8(v, f) { check(v, 0, U8, f); this.view.setUint8(this.at, v); this.at += 1; }
  u16(v, f) { check(v, 0, U16, f); this.view.setUint16(this.at, v, true); this.at += 2; }
  u32(v, f) { check(v, 0, U32, f); this.view.setUint32(this.at, v, true); this.at += 4; }
  i32(v, f) { check(v, I32_MIN, I32_MAX, f); this.view.setInt32(this.at, v, true); this.at += 4; }
  raw(b) { this.bytes.set(b, this.at); this.at += b.length; }
}

class Reader {
  constructor(bytes) {
    this.bytes = bytes;
    this.view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    this.at = 0;
  }
  need(n) {
    if (this.at + n > this.bytes.length) {
      throw new WireError('TRUNCATED', `need ${n} bytes at ${this.at}, frame has ${this.bytes.length}`);
    }
  }
  u8() { this.need(1); const v = this.view.getUint8(this.at); this.at += 1; return v; }
  u16() { this.need(2); const v = this.view.getUint16(this.at, true); this.at += 2; return v; }
  u32() { this.need(4); const v = this.view.getUint32(this.at, true); this.at += 4; return v; }
  i32() { this.need(4); const v = this.view.getInt32(this.at, true); this.at += 4; return v; }
  take(n) { this.need(n); const b = this.bytes.subarray(this.at, this.at + n); this.at += n; return b; }
  end() {
    if (this.at !== this.bytes.length) {
      throw new WireError('LENGTH_MISMATCH', `${this.bytes.length - this.at} trailing bytes`);
    }
  }
}

function utf8(text, maxBytes, field) {
  const b = encoder.encode(String(text));
  if (b.length > maxBytes) throw new WireError('RANGE', `${field} is ${b.length} bytes, max ${maxBytes}`);
  return b;
}

const CODECS = {
  [FRAME.ORDER]: {
    size: () => 2 + 4 + 4 + 4,
    write(w, f) { w.u32(f.unitId, 'unitId'); w.i32(f.x, 'x'); w.i32(f.y, 'y'); },
    read(r) { return { unitId: r.u32(), x: r.i32(), y: r.i32() }; },
  },
  [FRAME.STATE]: {
    size: (f) => 2 + 4 + 4 + 2 + f.units.length * (4 + 2 + 1 + 4 + 4 + 4),
    write(w, f) {
      w.u32(f.tick, 'tick');
      w.u32(f.hash, 'hash');
      w.u16(f.units.length, 'count');
      for (const u of f.units) {
        w.u32(u.id, 'id');
        w.u16(u.kind, 'kind');
        w.u8(u.player, 'player');
        w.i32(u.x, 'x');
        w.i32(u.y, 'y');
        w.i32(u.hp, 'hp');
      }
    },
    read(r) {
      const tick = r.u32();
      const hash = r.u32();
      const count = r.u16();
      if (count > MAX_UNITS) throw new WireError('COUNT_OUT_OF_RANGE', `count ${count} > ${MAX_UNITS}`);
      const units = [];
      for (let i = 0; i < count; i++) {
        units.push({ id: r.u32(), kind: r.u16(), player: r.u8(), x: r.i32(), y: r.i32(), hp: r.i32() });
      }
      return { tick, hash, units };
    },
  },
  // A delta carries the units whose record changed since the previous frame and the ids that left.
  [FRAME.DELTA]: {
    size: (f) => 2 + 4 + 4 + 2 + f.units.length * (4 + 2 + 1 + 4 + 4 + 4) + 2 + f.removed.length * 4,
    write(w, f) {
      w.u32(f.tick, 'tick');
      w.u32(f.hash, 'hash');
      w.u16(f.units.length, 'count');
      for (const u of f.units) {
        w.u32(u.id, 'id');
        w.u16(u.kind, 'kind');
        w.u8(u.player, 'player');
        w.i32(u.x, 'x');
        w.i32(u.y, 'y');
        w.i32(u.hp, 'hp');
      }
      w.u16(f.removed.length, 'removedCount');
      for (const id of f.removed) w.u32(id, 'removed');
    },
    read(r) {
      const tick = r.u32();
      const hash = r.u32();
      const count = r.u16();
      if (count > MAX_UNITS) throw new WireError('COUNT_OUT_OF_RANGE', `count ${count} > ${MAX_UNITS}`);
      const units = [];
      for (let i = 0; i < count; i++) {
        units.push({ id: r.u32(), kind: r.u16(), player: r.u8(), x: r.i32(), y: r.i32(), hp: r.i32() });
      }
      const removedCount = r.u16();
      if (removedCount > MAX_UNITS) throw new WireError('COUNT_OUT_OF_RANGE', `removed ${removedCount} > ${MAX_UNITS}`);
      const removed = [];
      for (let i = 0; i < removedCount; i++) removed.push(r.u32());
      return { tick, hash, units, removed };
    },
  },
  [FRAME.HELLO]: {
    size: (f) => 2 + 1 + utf8(f.room, MAX_ROOM_BYTES, 'room').length,
    write(w, f) {
      const b = utf8(f.room, MAX_ROOM_BYTES, 'room');
      w.u8(b.length, 'roomLength');
      w.raw(b);
    },
    read(r) {
      const len = r.u8();
      if (len > MAX_ROOM_BYTES) throw new WireError('COUNT_OUT_OF_RANGE', `room ${len} > ${MAX_ROOM_BYTES}`);
      return { room: decoder.decode(r.take(len)) };
    },
  },
  [FRAME.ERROR]: {
    size: (f) => 2 + 2 + 2 + utf8(f.message, MAX_MESSAGE_BYTES, 'message').length,
    write(w, f) {
      const b = utf8(f.message, MAX_MESSAGE_BYTES, 'message');
      w.u16(f.code, 'code');
      w.u16(b.length, 'messageLength');
      w.raw(b);
    },
    read(r) {
      const code = r.u16();
      const len = r.u16();
      if (len > MAX_MESSAGE_BYTES) throw new WireError('COUNT_OUT_OF_RANGE', `message ${len} > ${MAX_MESSAGE_BYTES}`);
      return { code, message: decoder.decode(r.take(len)) };
    },
  },
};

const TYPE_NAMES = Object.freeze(Object.fromEntries(Object.entries(FRAME).map(([k, v]) => [v, k.toLowerCase()])));

export function frameTypeName(type) {
  return TYPE_NAMES[type];
}

export function encodeFrame(frame) {
  const codec = CODECS[frame && frame.type];
  if (!codec) throw new WireError('UNKNOWN_TYPE', `type=${frame && frame.type}`);
  const w = new Writer(codec.size(frame));
  w.u8(frame.type, 'type');
  w.u8(WIRE_VERSION, 'version');
  codec.write(w, frame);
  return w.bytes;
}

export function decodeFrame(input) {
  const bytes = input instanceof Uint8Array ? input : new Uint8Array(input);
  const r = new Reader(bytes);
  const type = r.u8();
  const version = r.u8();
  if (version !== WIRE_VERSION) {
    throw new WireError('UNSUPPORTED_VERSION', `version=${version}, this decoder speaks ${WIRE_VERSION}`);
  }
  const codec = CODECS[type];
  if (!codec) throw new WireError('UNKNOWN_TYPE', `type=${type}`);
  const body = codec.read(r);
  r.end();
  return { type, version, ...body };
}

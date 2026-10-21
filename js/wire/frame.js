export const WIRE_VERSION = 1;

export const FRAME = Object.freeze({ ORDER: 1, STATE: 2, HELLO: 3, ERROR: 4, DELTA: 5, CAST: 6, ATTACK: 7, ROSTER: 8, JOIN: 9, START: 10, HOLD: 11, USE_ITEM: 12, ATTACK_MOVE: 13, SWAP: 14, CLASS_OVERRIDE: 15, HOLD_OVERRIDE: 16, SWAP_NOT_READY: 17, SWAP_READY: 18, SWAP_CANCEL: 19, PACK_FILE: 20, PACK_WANT: 21, ACTION: 22, UI: 23 });

// The most units a STATE or DELTA may carry, and the most ids a DELTA may remove. decodeFrame refuses a larger count with
// COUNT_OUT_OF_RANGE; encodeFrame checks only the u16 count field (65535), so it can build a frame that peers refuse.
export const MAX_UNITS = 4096;
export const MAX_SWAP_NAME_BYTES = 64;
export const MAX_ROOM_BYTES = 64;
export const MAX_MESSAGE_BYTES = 256;
// A player's input for the pack's on_action (its name and JSON body), and a UI event the host's pack emitted (JSON body).
export const MAX_ACTION_NAME_BYTES = 64;
export const MAX_ACTION_BYTES = 4096;
export const MAX_UI_BYTES = 8192;
export const MAX_PLAYERS = 10;
export const TEAM_SIZE = 5;
export const MAX_OWNER_BYTES = 64;
export const MAX_PICK_BYTES = 32;
export const SPECTATOR = 255;
// Pack transfer: a file name is at most MAX_PACK_PATH_BYTES, one inline chunk at most MAX_PACK_CHUNK_BYTES.
export const MAX_PACK_PATH_BYTES = 256;
export const MAX_PACK_CHUNK_BYTES = 16384;
export const PACK_FLAG_REF = 1;
export const PACK_FLAG_DELETE = 2;

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

// Decodes a string field; bad UTF-8 is a WireError like every other malformed field, not a TypeError.
function text(bytes, field) {
  try {
    return decoder.decode(bytes);
  } catch {
    throw new WireError('BAD_UTF8', `${field} is not valid UTF-8`);
  }
}

const CODECS = {
  [FRAME.ORDER]: {
    size: () => 2 + 4 + 4 + 4,
    write(w, f) { w.u32(f.unitId, 'unitId'); w.i32(f.x, 'x'); w.i32(f.y, 'y'); },
    read(r) { return { unitId: r.u32(), x: r.i32(), y: r.i32() }; },
  },
  // A cast names the caster's unit, the ability slot, the target unit (-1 for a point) and the point it aims at.
  [FRAME.CAST]: {
    size: () => 2 + 4 + 1 + 4 + 4 + 4,
    write(w, f) { w.u32(f.unitId, 'unitId'); w.u8(f.slot, 'slot'); w.i32(f.target, 'target'); w.i32(f.x, 'x'); w.i32(f.y, 'y'); },
    read(r) { return { unitId: r.u32(), slot: r.u8(), target: r.i32(), x: r.i32(), y: r.i32() }; },
  },
  [FRAME.ATTACK]: {
    size: () => 2 + 4 + 4,
    write(w, f) { w.u32(f.unitId, 'unitId'); w.i32(f.targetId, 'targetId'); },
    read(r) { return { unitId: r.u32(), targetId: r.i32() }; },
  },
  // An attack-move order: the unit walks to (x, y), strikes enemies in reach on the way, and resumes after each kill.
  [FRAME.ATTACK_MOVE]: {
    size: () => 2 + 4 + 4 + 4,
    write(w, f) { w.u32(f.unitId, 'unitId'); w.i32(f.x, 'x'); w.i32(f.y, 'y'); },
    read(r) { return { unitId: r.u32(), x: r.i32(), y: r.i32() }; },
  },
  // A hold order: the unit stops where it stands and attacks only what is in reach, until a move or attack replaces it.
  [FRAME.HOLD]: {
    size: () => 2 + 4,
    write(w, f) { w.u32(f.unitId, 'unitId'); },
    read(r) { return { unitId: r.u32() }; },
  },
  // A data reload's swap tick: the tick at which every peer swaps to the new data (see the room's DATA_SWAP_LEAD_TICKS).
  // The hash is the 32-bit FNV-1a of the new image bytes, so a peer can check the image it fetched is the host's.
  // The file name is a plain module name of at most MAX_SWAP_NAME_BYTES bytes; a longer one is refused on both sides.
  [FRAME.SWAP]: {
    size: (f) => 2 + 4 + 4 + 1 + encoder.encode(f.file || '').length,
    write(w, f) {
      const name = encoder.encode(f.file || '');
      if (name.length > MAX_SWAP_NAME_BYTES) throw new WireError('RANGE', `swap file name is ${name.length} bytes, max ${MAX_SWAP_NAME_BYTES}`);
      w.u32(f.tick, 'tick');
      w.u32(f.hash, 'hash');
      w.u8(name.length, 'file length');
      w.raw(name);
    },
    read(r) {
      const tick = r.u32();
      const hash = r.u32();
      const len = r.u8();
      if (len > MAX_SWAP_NAME_BYTES) throw new WireError('COUNT_OUT_OF_RANGE', `swap file name ${len} > ${MAX_SWAP_NAME_BYTES}`);
      return { tick, hash, file: text(r.take(len), 'swap file name') };
    },
  },
  // A peer's report that it did not have the swap image by the swap tick, so it did not apply the swap there. The host
  // answers by deferring the swap (a SWAP frame with a later tick). The tick and hash name the swap it refers to.
  [FRAME.SWAP_NOT_READY]: {
    size: () => 2 + 4 + 4,
    write(w, f) { w.u32(f.tick, 'tick'); w.u32(f.hash, 'hash'); },
    read(r) { return { tick: r.u32(), hash: r.u32() }; },
  },
  // A peer's report that it holds the verified swap image for this tick and hash; the host commits the swap only once
  // every connected peer has sent one. Sent to the host, and sent again for each re-announce of the swap.
  [FRAME.SWAP_READY]: {
    size: () => 2 + 4 + 4,
    write(w, f) { w.u32(f.tick, 'tick'); w.u32(f.hash, 'hash'); },
    read(r) { return { tick: r.u32(), hash: r.u32() }; },
  },
  // The host's cancel of a swap it could not commit: every peer drops the pending swap with this tick and hash.
  [FRAME.SWAP_CANCEL]: {
    size: () => 2 + 4 + 4,
    write(w, f) { w.u32(f.tick, 'tick'); w.u32(f.hash, 'hash'); },
    read(r) { return { tick: r.u32(), hash: r.u32() }; },
  },
  // A debug override of a unit's target class: 0..3, or 255 to clear it back to the Rust rule.
  [FRAME.CLASS_OVERRIDE]: {
    size: () => 2 + 4 + 1,
    write(w, f) { w.u32(f.unitId, 'unitId'); w.u8(f.class, 'class'); },
    read(r) { return { unitId: r.u32(), class: r.u8() }; },
  },
  // A debug override of a unit's attack-move hold: 0 never holds, 1 always holds, 255 clears it back to the Rust rule.
  [FRAME.HOLD_OVERRIDE]: {
    size: () => 2 + 4 + 1,
    write(w, f) { w.u32(f.unitId, 'unitId'); w.u8(f.hold, 'hold'); },
    read(r) { return { unitId: r.u32(), hold: r.u8() }; },
  },
  // An item use: slot is the inventory index, target the unit id (-1 for none), x and y the point it aims at.
  [FRAME.USE_ITEM]: {
    size: () => 2 + 4 + 1 + 4 + 4 + 4,
    write(w, f) { w.u32(f.unitId, 'unitId'); w.u8(f.slot, 'slot'); w.i32(f.target, 'target'); w.i32(f.x, 'x'); w.i32(f.y, 'y'); },
    read(r) { return { unitId: r.u32(), slot: r.u8(), target: r.i32(), x: r.i32(), y: r.i32() }; },
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
  // The host tells one peer which player it is (SPECTATOR when no seat is free) and sends the per-player data of every seat.
  [FRAME.ROSTER]: {
    size: (f) => 2 + 1 + 1 + 1 + f.players.length * (1 + 1 + 1 + 4 + 2 + 2),
    write(w, f) {
      if (f.players.length > MAX_PLAYERS) throw new WireError('RANGE', `players ${f.players.length} > ${MAX_PLAYERS}`);
      w.u8(f.you, 'you');
      w.u8(f.team, 'team');
      w.u8(f.players.length, 'playerCount');
      for (const p of f.players) {
        w.u8(p.slot, 'slot');
        w.u8(p.team, 'playerTeam');
        w.u8(p.connected, 'connected');
        w.i32(p.gold, 'gold');
        w.u16(p.kills, 'kills');
        w.u16(p.deaths, 'deaths');
      }
    },
    read(r) {
      const you = r.u8();
      const team = r.u8();
      const count = r.u8();
      if (count > MAX_PLAYERS) throw new WireError('COUNT_OUT_OF_RANGE', `players ${count} > ${MAX_PLAYERS}`);
      const players = [];
      for (let i = 0; i < count; i++) {
        players.push({ slot: r.u8(), team: r.u8(), connected: r.u8(), gold: r.i32(), kills: r.u16(), deaths: r.u16() });
      }
      return { you, team, players };
    },
  },
  // A peer asks for the current state: lastTick 0 on a fresh join, the last applied tick on a reconnect.
  [FRAME.JOIN]: {
    size: () => 2 + 4,
    write(w, f) { w.u32(f.lastTick, 'lastTick'); },
    read(r) { return { lastTick: r.u32() }; },
  },
  // The lobby start action: the seats in slot order, each with its owner key and pick.
  [FRAME.START]: {
    size: (f) => 2 + 1 + f.seats.reduce((n, s) => n + 1 + 1 + utf8(s.owner, MAX_OWNER_BYTES, 'owner').length + 1 + utf8(s.pick, MAX_PICK_BYTES, 'pick').length, 0),
    write(w, f) {
      if (f.seats.length > MAX_PLAYERS) throw new WireError('RANGE', `seats ${f.seats.length} > ${MAX_PLAYERS}`);
      w.u8(f.seats.length, 'seatCount');
      for (const s of f.seats) {
        const owner = utf8(s.owner, MAX_OWNER_BYTES, 'owner');
        const pick = utf8(s.pick, MAX_PICK_BYTES, 'pick');
        w.u8(s.slot, 'slot');
        w.u8(owner.length, 'ownerLength');
        w.raw(owner);
        w.u8(pick.length, 'pickLength');
        w.raw(pick);
      }
    },
    read(r) {
      const count = r.u8();
      if (count > MAX_PLAYERS) throw new WireError('COUNT_OUT_OF_RANGE', `seats ${count} > ${MAX_PLAYERS}`);
      const seats = [];
      for (let i = 0; i < count; i++) {
        const slot = r.u8();
        const ownerLength = r.u8();
        if (ownerLength > MAX_OWNER_BYTES) throw new WireError('COUNT_OUT_OF_RANGE', `owner ${ownerLength} > ${MAX_OWNER_BYTES}`);
        const owner = text(r.take(ownerLength), 'owner');
        const pickLength = r.u8();
        if (pickLength > MAX_PICK_BYTES) throw new WireError('COUNT_OUT_OF_RANGE', `pick ${pickLength} > ${MAX_PICK_BYTES}`);
        seats.push({ slot, owner, pick: text(r.take(pickLength), 'pick') });
      }
      return { seats };
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
      return { room: text(r.take(len), 'room') };
    },
  },
  // One piece of a pack file. A text or small asset travels as chunks (offset, bytes) of `size` total bytes; a large asset
  // travels as a reference (PACK_FLAG_REF, no bytes) and the peer asks for the bytes by hash with PACK_WANT.
  [FRAME.PACK_FILE]: {
    size: (f) => 2 + 2 + encoder.encode(f.name).length + 4 + 4 + 4 + 1 + 2 + (f.bytes ? f.bytes.length : 0),
    write(w, f) {
      const name = utf8(f.name, MAX_PACK_PATH_BYTES, 'pack name');
      const flags = f.flags | 0;
      const data = f.bytes || new Uint8Array(0);
      if (flags & PACK_FLAG_REF && data.length) throw new WireError('RANGE', 'a reference carries no bytes');
      if (data.length > MAX_PACK_CHUNK_BYTES) throw new WireError('RANGE', `chunk is ${data.length} bytes, max ${MAX_PACK_CHUNK_BYTES}`);
      w.u16(name.length, 'nameLength');
      w.raw(name);
      w.u32(f.hash, 'hash');
      w.u32(f.size, 'size');
      w.u32(f.offset, 'offset');
      w.u8(flags, 'flags');
      w.u16(data.length, 'chunkLength');
      w.raw(data);
    },
    read(r) {
      const nameLength = r.u16();
      if (nameLength > MAX_PACK_PATH_BYTES) throw new WireError('COUNT_OUT_OF_RANGE', `pack name ${nameLength} > ${MAX_PACK_PATH_BYTES}`);
      const name = text(r.take(nameLength), 'pack name');
      const hash = r.u32();
      const size = r.u32();
      const offset = r.u32();
      const flags = r.u8();
      const len = r.u16();
      if (len > MAX_PACK_CHUNK_BYTES) throw new WireError('COUNT_OUT_OF_RANGE', `chunk ${len} > ${MAX_PACK_CHUNK_BYTES}`);
      if (flags & PACK_FLAG_REF && len) throw new WireError('RANGE', 'a reference carries no bytes');
      return { name, hash, size, offset, flags, bytes: r.take(len).slice() };
    },
  },
  // A peer asks the host for the bytes of a referenced pack file, by its hash.
  [FRAME.PACK_WANT]: {
    size: () => 2 + 4,
    write(w, f) { w.u32(f.hash, 'hash'); },
    read(r) { return { hash: r.u32() }; },
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
      return { code, message: text(r.take(len), 'message') };
    },
  },
  // A joined player's input, for the host to hand to the pack: the action name and its JSON body.
  [FRAME.ACTION]: {
    size: (f) => 2 + 2 + utf8(f.name, MAX_ACTION_NAME_BYTES, 'name').length + 2 + utf8(f.body, MAX_ACTION_BYTES, 'body').length,
    write(w, f) {
      const name = utf8(f.name, MAX_ACTION_NAME_BYTES, 'name');
      const body = utf8(f.body, MAX_ACTION_BYTES, 'body');
      w.u16(name.length, 'nameLength');
      w.raw(name);
      w.u16(body.length, 'bodyLength');
      w.raw(body);
    },
    read(r) {
      const nameLen = r.u16();
      if (nameLen > MAX_ACTION_NAME_BYTES) throw new WireError('COUNT_OUT_OF_RANGE', `name ${nameLen} > ${MAX_ACTION_NAME_BYTES}`);
      const name = text(r.take(nameLen), 'name');
      const bodyLen = r.u16();
      if (bodyLen > MAX_ACTION_BYTES) throw new WireError('COUNT_OUT_OF_RANGE', `body ${bodyLen} > ${MAX_ACTION_BYTES}`);
      return { name, body: text(r.take(bodyLen), 'body') };
    },
  },
  // A UI event the host's pack emitted, for every page: its JSON body.
  [FRAME.UI]: {
    size: (f) => 2 + 2 + utf8(f.body, MAX_UI_BYTES, 'body').length,
    write(w, f) {
      const body = utf8(f.body, MAX_UI_BYTES, 'body');
      w.u16(body.length, 'bodyLength');
      w.raw(body);
    },
    read(r) {
      const bodyLen = r.u16();
      if (bodyLen > MAX_UI_BYTES) throw new WireError('COUNT_OUT_OF_RANGE', `body ${bodyLen} > ${MAX_UI_BYTES}`);
      return { body: text(r.take(bodyLen), 'body') };
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

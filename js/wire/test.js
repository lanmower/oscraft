import assert from 'node:assert/strict';
import { createSeating, createStateEncoder, applyFrame, startFrame } from '../client/delta.js';
import { swapGate, swapIsNew } from '../client/swap_protocol.js';

import { FRAME, SPECTATOR, WIRE_VERSION, WireError, encodeFrame, decodeFrame, frameTypeName } from './frame.js';

const samples = [
  { type: FRAME.ORDER, unitId: 0, x: 0, y: 0 },
  { type: FRAME.ORDER, unitId: 0xffffffff, x: -0x80000000, y: 0x7fffffff },
  { type: FRAME.STATE, tick: 0, hash: 0, units: [] },
  {
    type: FRAME.STATE, tick: 42, hash: 0xdeadbeef,
    units: [
      { id: 1, kind: 7, player: 0, x: -5, y: 9, hp: 100 },
      { id: 2, kind: 65535, player: 255, x: 1, y: -1, hp: 0 },
    ],
  },
  { type: FRAME.DELTA, tick: 0, hash: 0, units: [], removed: [] },
  {
    type: FRAME.DELTA, tick: 41, hash: 0x12345678,
    units: [{ id: 3, kind: 2, player: 1, x: 7, y: -7, hp: 50 }],
    removed: [9, 0xffffffff],
  },
  { type: FRAME.CAST, unitId: 0, slot: 0, target: -1, x: 0, y: 0 },
  { type: FRAME.CAST, unitId: 0xffffffff, slot: 255, target: 0x7fffffff, x: -0x80000000, y: 0x7fffffff },
  { type: FRAME.ATTACK, unitId: 4, targetId: -1 },
  { type: FRAME.ATTACK, unitId: 0xffffffff, targetId: 0x7fffffff },
  { type: FRAME.HOLD, unitId: 4 },
  { type: FRAME.HOLD, unitId: 0xffffffff },
  { type: FRAME.CLASS_OVERRIDE, unitId: 4, class: 2 },
  { type: FRAME.CLASS_OVERRIDE, unitId: 0xffffffff, class: 255 },
  { type: FRAME.HOLD_OVERRIDE, unitId: 4, hold: 0 },
  { type: FRAME.HOLD_OVERRIDE, unitId: 0xffffffff, hold: 255 },
  { type: FRAME.ATTACK_MOVE, unitId: 4, x: -3, y: 1310720 },
  { type: FRAME.USE_ITEM, unitId: 4, slot: 0, target: -1, x: 0, y: 0 },
  { type: FRAME.USE_ITEM, unitId: 0xffffffff, slot: 5, target: 0x7fffffff, x: -0x80000000, y: 0x7fffffff },
  { type: FRAME.SWAP, tick: 0, hash: 0, file: "" },
  { type: FRAME.SWAP, tick: 0xffffffff, hash: 0xffffffff, file: "moba-units.lua" },
  { type: FRAME.SWAP_NOT_READY, tick: 0, hash: 0 },
  { type: FRAME.SWAP_NOT_READY, tick: 0xffffffff, hash: 0xffffffff },
  { type: FRAME.SWAP_READY, tick: 0, hash: 0 },
  { type: FRAME.SWAP_READY, tick: 0xffffffff, hash: 0xffffffff },
  { type: FRAME.SWAP_CANCEL, tick: 0, hash: 0 },
  { type: FRAME.SWAP_CANCEL, tick: 0xffffffff, hash: 0xffffffff },
  { type: FRAME.JOIN, lastTick: 0 },
  { type: FRAME.JOIN, lastTick: 0xffffffff },
  { type: FRAME.ROSTER, you: 255, team: 255, players: [] },
  { type: FRAME.START, seats: [] },
  { type: FRAME.HELLO, room: '' },
  { type: FRAME.HELLO, room: 'lobby-ünïcode' },
  { type: FRAME.PACK_FILE, name: 'scenarios/moba/hero.lua', hash: 0xdeadbeef, size: 3, offset: 0, flags: 0, bytes: new Uint8Array([1, 2, 3]) },
  { type: FRAME.PACK_FILE, name: '', hash: 0, size: 0, offset: 0, flags: 0, bytes: new Uint8Array(0) },
  { type: FRAME.PACK_FILE, name: 'big.png', hash: 7, size: 90000000, offset: 0, flags: 1, bytes: new Uint8Array(0) },
  { type: FRAME.PACK_WANT, hash: 0xffffffff },
  { type: FRAME.ERROR, code: 0, message: '' },
  { type: FRAME.ERROR, code: 404, message: 'room not found' },
];

const tenUnits = Array.from({ length: 10 }, (_, p) => ({ id: p + 1, kind: 1, player: p, x: p * 100, y: 7 - p * 100, hp: 1000 - p }));
samples.push({ type: FRAME.STATE, tick: 7, hash: 1, units: tenUnits });
samples.push({ type: FRAME.DELTA, tick: 8, hash: 2, units: tenUnits, removed: [] });
samples.push({
  type: FRAME.ROSTER, you: 7, team: 1,
  players: Array.from({ length: 10 }, (_, p) => ({ slot: p, team: p < 5 ? 0 : 1, connected: p % 2, gold: 500 * p - 1, kills: p, deaths: 65535 - p })),
});
samples.push({ type: FRAME.START, seats: Array.from({ length: 10 }, (_, p) => ({ slot: p, owner: `${p}`.repeat(64), pick: 'hero' })) });
samples.push({ type: FRAME.ACTION, name: '', body: '' });
samples.push({ type: FRAME.ACTION, name: 'buy-item', body: '{"slot":2,"item":"ward"}' });
samples.push({ type: FRAME.UI, body: '{"panel":"shop","open":true}' });

const typesSeen = new Set();
let roundtrip = 0;

for (const frame of samples) {
  const bytes = encodeFrame(frame);
  assert.equal(bytes[0], frame.type, 'first byte is the frame type');
  assert.equal(bytes[1], WIRE_VERSION, 'second byte is the wire version');
  const back = decodeFrame(bytes);
  const { version, ...payload } = back;
  assert.equal(version, WIRE_VERSION);
  assert.deepStrictEqual(payload, frame, `round-trip ${frameTypeName(frame.type)}`);
  typesSeen.add(frame.type);
  roundtrip += 1;
}

function expectReject(bytes, code, label) {
  assert.throws(
    () => decodeFrame(bytes),
    (err) => err instanceof WireError && err.code === code,
    `${label}: expected WireError ${code}`,
  );
}

for (const frame of samples) {
  const bytes = encodeFrame(frame);
  for (const v of [0, WIRE_VERSION + 1, 0xff]) {
    const forged = Uint8Array.from(bytes);
    forged[1] = v;
    expectReject(forged, 'UNSUPPORTED_VERSION', `${frameTypeName(frame.type)} version ${v}`);
  }
  expectReject(bytes.subarray(0, bytes.length - 1), 'TRUNCATED', `${frameTypeName(frame.type)} truncated`);
  const padded = new Uint8Array(bytes.length + 1);
  padded.set(bytes);
  expectReject(padded, 'LENGTH_MISMATCH', `${frameTypeName(frame.type)} trailing byte`);
}

expectReject(Uint8Array.from([0x7f, WIRE_VERSION]), 'UNKNOWN_TYPE', 'unknown type');
expectReject(Uint8Array.from([FRAME.ORDER]), 'TRUNCATED', 'one-byte frame');

const overCount = Uint8Array.from([FRAME.STATE, WIRE_VERSION, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0x10]);
expectReject(overCount, 'COUNT_OUT_OF_RANGE', 'state count above bound');
expectReject(Uint8Array.from([FRAME.DELTA, WIRE_VERSION, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0x10]), 'COUNT_OUT_OF_RANGE', 'delta count above bound');

assert.throws(() => encodeFrame({ type: FRAME.ORDER, unitId: -1, x: 0, y: 0 }), WireError);
assert.throws(() => encodeFrame({ type: FRAME.HELLO, room: 'x'.repeat(65) }), WireError);

expectReject(Uint8Array.from([FRAME.ROSTER, WIRE_VERSION, 0, 0, 11]), 'COUNT_OUT_OF_RANGE', 'roster above ten players');
expectReject(Uint8Array.from([FRAME.START, WIRE_VERSION, 11]), 'COUNT_OUT_OF_RANGE', 'start above ten seats');
assert.throws(() => encodeFrame({ type: FRAME.ROSTER, you: 0, team: 0, players: tenUnits.concat(tenUnits[0]) }), WireError);

const keys = Array.from({ length: 10 }, (_, p) => `peer${p}`.padEnd(64, 'k'));
const started = { slots: keys.map((owner, slot) => ({ owner, hero: 'hero', ready: true })) };
const sent = decodeFrame(encodeFrame(startFrame(started)));
assert.equal(sent.seats.length, 10);
const seating = createSeating();
assert.equal(seating.start(sent.seats), 10);
for (const [i, key] of keys.entries()) {
  const mine = decodeFrame(encodeFrame(seating.join(key) && seating.roster(key)));
  assert.equal(mine.you, i);
  assert.equal(mine.team, i < 5 ? 0 : 1);
  assert.equal(mine.players.length, 10);
}
assert.equal(seating.roster('stranger').you, SPECTATOR);
seating.leave(keys[3]);
assert.equal(seating.roster(keys[0]).players[3].connected, 0);
assert.equal(seating.join(keys[3]).you, 3, 'reconnect keeps the seat');
const resumed = createSeating();
resumed.restore(seating.export());
assert.equal(resumed.seat(keys[8]).you, 8, 'snapshot restore keeps the seats');

const encoder = createStateEncoder();
encoder.encode(1, 0, tenUnits);
encoder.encode(2, 0, tenUnits);
encoder.keyframe();
const lateFrame = decodeFrame(encodeFrame(encoder.encode(3, 0, tenUnits)));
assert.equal(lateFrame.type, FRAME.STATE);
const lateView = new Map();
assert.ok(applyFrame(lateView, lateFrame));
const next = decodeFrame(encodeFrame(encoder.encode(4, 0, tenUnits.map((u) => ({ ...u, hp: u.hp - 1 })))));
assert.equal(next.type, FRAME.DELTA);
applyFrame(lateView, next);
assert.equal(lateView.size, 10);
assert.equal(new Set([...lateView.values()].map((u) => u.player)).size, 10);

const manyUnits = Array.from({ length: 2000 }, (_, i) => ({ id: i + 1, kind: 1, player: i % 10, x: i, y: 0 - i, hp: 5 }));
assert.deepStrictEqual(decodeFrame(encodeFrame({ type: FRAME.STATE, tick: 9, hash: 3, units: manyUnits })).units, manyUnits, 'state of 2000 units round-trips');

const longName = 'a'.repeat(65);
assert.throws(() => encodeFrame({ type: FRAME.SWAP, tick: 1, hash: 0, file: longName }), (err) => err instanceof WireError && err.code === 'RANGE', 'swap writer refuses 65 bytes');
const swap64 = encodeFrame({ type: FRAME.SWAP, tick: 1, hash: 0x811c9dc5, file: 'a'.repeat(64) });
assert.equal(decodeFrame(swap64).file.length, 64, 'swap of 64 bytes round-trips');
assert.equal(decodeFrame(swap64).hash, 0x811c9dc5, 'swap carries the image hash');
expectReject(Uint8Array.from([FRAME.SWAP, WIRE_VERSION, 1, 0, 0, 0, 0, 0, 0, 0, 65, ...new Uint8Array(65).fill(0x61)]), 'COUNT_OUT_OF_RANGE', 'swap decode of 65 bytes');

expectReject(Uint8Array.from([FRAME.HELLO, WIRE_VERSION, 1, 0xff]), 'BAD_UTF8', 'hello with invalid UTF-8');
expectReject(Uint8Array.from([FRAME.ERROR, WIRE_VERSION, 0, 0, 1, 0, 0xc3]), 'BAD_UTF8', 'error message truncated mid-character');

assert.equal(encodeFrame({ type: FRAME.SWAP_NOT_READY, tick: 5, hash: 6 }).length, 10, 'not-ready report is 10 bytes');
expectReject(Uint8Array.from([FRAME.SWAP_NOT_READY, WIRE_VERSION, 0, 0, 0, 5, 0, 0]), 'TRUNCATED', 'truncated not-ready report');

assert.equal(encodeFrame({ type: FRAME.SWAP_READY, tick: 5, hash: 6 }).length, 10, 'ready report is 10 bytes');
assert.deepStrictEqual(({ ...decodeFrame(encodeFrame({ type: FRAME.SWAP_READY, tick: 40, hash: 0xabcdef01 })) }), { type: FRAME.SWAP_READY, tick: 40, hash: 0xabcdef01, version: WIRE_VERSION }, 'ready report round-trips');
expectReject(Uint8Array.from([FRAME.SWAP_READY, WIRE_VERSION, 0, 0, 0, 5, 0, 0]), 'TRUNCATED', 'truncated ready report');
expectReject(Uint8Array.from([FRAME.SWAP_CANCEL, WIRE_VERSION, 0, 0, 0, 5]), 'TRUNCATED', 'truncated cancel');

const dupSwap = decodeFrame(encodeFrame({ type: FRAME.SWAP, tick: 100, hash: 0x1234, file: 'terrain.js' }));
let lastApplied = null;
const appliedTicks = [];
for (const f of [dupSwap, { ...dupSwap }, { ...dupSwap, tick: 99 }, { ...dupSwap, tick: 132 }]) {
  if (swapIsNew(lastApplied, f.tick)) { appliedTicks.push(f.tick); lastApplied = f.tick; }
}
assert.deepStrictEqual(appliedTicks, [100, 132], 'a repeated or older SWAP tick is ignored, a later one applies');
assert.equal(swapIsNew(null, 0), true, 'the first swap always applies');
assert.equal(swapIsNew(100, 100), false, 'the applied tick itself is not applied twice');

const peersSet = new Set(['a', 'b']);
const pendingSwap = { tick: 100, deferrals: 0, readyPeers: new Set(['a']) };
assert.equal(swapGate(91, pendingSwap, peersSet), 'wait', 'no decision before the gate tick T-8');
assert.equal(swapGate(92, pendingSwap, peersSet), 'defer', 'a missing peer at T-8 moves the swap');
pendingSwap.readyPeers.add('b');
assert.equal(swapGate(99, pendingSwap, peersSet), 'commit', 'every peer ready commits the swap');
assert.equal(swapGate(92, { tick: 100, deferrals: 3, readyPeers: new Set() }, peersSet), 'cancel', 'a fourth miss cancels the swap');
assert.equal(swapGate(92, { tick: 100, deferrals: 0, readyPeers: new Set() }, new Set()), 'commit', 'no connected peers commits at the gate');

const types = typesSeen.size;
assert.equal(types, Object.keys(FRAME).length, 'every frame type round-trips');
console.log(`wire frames ok types=${types} roundtrip=${roundtrip}`);
console.log('net ten ok players=10');

assert.throws(() => encodeFrame({ type: FRAME.PACK_FILE, name: 'a.lua', hash: 0, size: 1, offset: 0, flags: 0, bytes: new Uint8Array(16385) }), WireError);
assert.throws(() => encodeFrame({ type: FRAME.PACK_FILE, name: 'a.png', hash: 0, size: 1, offset: 0, flags: 1, bytes: new Uint8Array(1) }), WireError);
assert.throws(() => encodeFrame({ type: FRAME.PACK_FILE, name: 'x'.repeat(257), hash: 0, size: 0, offset: 0, flags: 0, bytes: new Uint8Array(0) }), WireError);
expectReject(Uint8Array.from([FRAME.PACK_FILE, WIRE_VERSION, 0xff, 0x01]), 'COUNT_OUT_OF_RANGE', 'pack name above bound');
console.log(`wire test: ${roundtrip} frames round-trip, ${typesSeen.size} types seen`);

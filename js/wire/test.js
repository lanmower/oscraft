import assert from 'node:assert/strict';
import { FRAME, WIRE_VERSION, WireError, encodeFrame, decodeFrame, frameTypeName } from './frame.js';

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
  { type: FRAME.HELLO, room: '' },
  { type: FRAME.HELLO, room: 'lobby-ünïcode' },
  { type: FRAME.ERROR, code: 0, message: '' },
  { type: FRAME.ERROR, code: 404, message: 'room not found' },
];

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

const overCount = Uint8Array.from([FRAME.STATE, WIRE_VERSION, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0x04]);
expectReject(overCount, 'COUNT_OUT_OF_RANGE', 'state count above bound');
expectReject(Uint8Array.from([FRAME.DELTA, WIRE_VERSION, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0x04]), 'COUNT_OUT_OF_RANGE', 'delta count above bound');

assert.throws(() => encodeFrame({ type: FRAME.ORDER, unitId: -1, x: 0, y: 0 }), WireError);
assert.throws(() => encodeFrame({ type: FRAME.HELLO, room: 'x'.repeat(65) }), WireError);

const types = typesSeen.size;
assert.equal(types, Object.keys(FRAME).length, 'every frame type round-trips');
console.log(`wire frames ok types=${types} roundtrip=${roundtrip}`);

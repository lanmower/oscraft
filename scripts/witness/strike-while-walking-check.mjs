import assert from "node:assert/strict";
import { verdict } from "./strike-while-walking.mjs";

const leg = (overrides) => ({
  label: "leg",
  lock: "true",
  ready: true,
  aborted: null,
  strikeSamplesWhileMoving: 4,
  inReachMoving: 9,
  ...overrides,
});

const greenRedGreen = (red = {}, green = {}) => [
  leg({ label: "lock on", lock: "true", ...green }),
  leg({ label: "lock off", lock: "false", strikeSamplesWhileMoving: 0, ...red }),
  leg({ label: "lock on again", lock: "true", ...green }),
];

const cases = [
  { name: "green, red, green with the expected counts passes", legs: greenRedGreen(), ok: true },
  { name: "a red leg with strikes while moving fails", legs: greenRedGreen({ strikeSamplesWhileMoving: 2 }), ok: false },
  { name: "a green leg with no strike while moving fails", legs: greenRedGreen({}, { strikeSamplesWhileMoving: 0 }), ok: false },
  { name: "a red leg never in reach fails, since zero proves nothing", legs: greenRedGreen({ inReachMoving: 0 }), ok: false },
  { name: "a leg that did not reach the stage at start hp fails", legs: greenRedGreen({}, { ready: false }), ok: false },
  { name: "an aborted leg fails", legs: greenRedGreen({ aborted: "hero dead" }), ok: false },
];

for (const c of cases) {
  const result = verdict(c.legs);
  assert.equal(result.ok, c.ok, c.name);
}

assert.equal(verdict([leg({ lock: "true", strikeSamplesWhileMoving: 0 })]).reasons[0], "leg: no strike while moving with one-hand on");

console.log(`strike-while-walking verdict checks ok (${cases.length} cases)`);

import assert from "node:assert/strict";
import { CONFIG, strikeWhileWalkingLeg, verdict } from "./strike-while-walking.mjs";

const leg = (overrides) => ({
  label: "leg",
  lock: "true",
  ready: true,
  aborted: null,
  samples: 320,
  strikeSamplesWhileMoving: 4,
  launchesWhileMoving: 0,
  inReachMoving: 9,
  kinds: { melee_hit: 3 },
  ...overrides,
});

const greenRedGreen = (red = {}, green = {}) => [
  leg({ label: "lock on", lock: "true", ...green }),
  leg({ label: "lock off", lock: "false", strikeSamplesWhileMoving: 0, ...red }),
  leg({ label: "lock on again", lock: "true", ...green }),
];

const cases = [
  { name: "green, red, green with the expected counts passes", legs: greenRedGreen(), ok: true },
  { name: "a red leg with strike samples but no launch passes, since a landed strike is not a launch", legs: greenRedGreen({ strikeSamplesWhileMoving: 2 }), ok: true },
  { name: "a green leg with no strike while moving fails", legs: greenRedGreen({}, { strikeSamplesWhileMoving: 0 }), ok: false },
  { name: "a red leg that launches while moving fails", legs: greenRedGreen({ launchesWhileMoving: 1 }), ok: false },
  { name: "a red leg never in reach fails, since zero proves nothing", legs: greenRedGreen({ inReachMoving: 0 }), ok: false },
  { name: "a leg that did not reach the stage at start hp fails", legs: greenRedGreen({}, { ready: false }), ok: false },
  { name: "an aborted leg fails", legs: greenRedGreen({ aborted: "hero dead" }), ok: false },
  { name: "a green leg with no hero-sourced effect kinds fails", legs: greenRedGreen({}, { kinds: {} }), ok: false },
  { name: "a red leg with no hero-sourced effect kinds still passes", legs: greenRedGreen({ kinds: {} }), ok: true },
  { name: "a leg at the sample floor passes", legs: greenRedGreen({ samples: CONFIG.sampleFloor }), ok: true },
  { name: "a leg below the sample floor fails, even when its counts are green", legs: greenRedGreen({ samples: CONFIG.sampleFloor - 1 }), ok: false },
];

for (const c of cases) {
  const result = verdict(c.legs);
  assert.equal(result.ok, c.ok, c.name);
}

assert.equal(verdict([leg({ lock: "true", strikeSamplesWhileMoving: 0 })]).reasons[0], "leg: no strike while moving with one-hand on");
assert.equal(
  verdict([leg({ samples: 100, reruns: [{ attempt: 1 }, { attempt: 2 }] })]).reasons[0],
  `leg: 100 samples, below the floor of ${CONFIG.sampleFloor} after 3 attempts`,
);

assert.equal(CONFIG.legTicks, 40 * 16, "a leg is 40 s of sim ticks at 16 ticks per second");
const legSource = strikeWhileWalkingLeg.toString();
assert.doesNotMatch(legSource, /Date\.now\(/, "a leg is measured in sim ticks, not wall-clock");
assert.match(legSource, /__ciTicks\(\)/, "a leg reads the sim tick counter");

console.log(`strike-while-walking verdict checks ok (${cases.length} cases, leg timing in sim ticks)`);

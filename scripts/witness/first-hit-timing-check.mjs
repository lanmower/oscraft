import assert from "node:assert/strict";
import { verdictFirstHit } from "./first-hit-timing.mjs";

const leg = (overrides) => ({
  label: "leg",
  lock: "true",
  judged: true,
  ready: true,
  aborted: null,
  targetId: 7,
  reachTick: 100,
  launchTick: 101,
  releaseTick: 108,
  hitTick: 108,
  attackPoint: 7,
  latencyTicks: 8,
  samples: 40,
  ...overrides,
});

const stoppedAndWalking = (walkOverrides = {}, stopOverrides = {}) => [
  leg({ label: "stopped, lock on", ...stopOverrides }),
  leg({ label: "walking, lock on", ...walkOverrides }),
  leg({ label: "walking, lock off (control)", lock: "false", judged: false, hitTick: null, latencyTicks: null, reachTick: null }),
];

const cases = [
  { name: "both judged legs hit within the attack point plus one tick passes", legs: stoppedAndWalking(), ok: true },
  { name: "a hit exactly at the attack point plus one tick passes", legs: stoppedAndWalking({ latencyTicks: 8, hitTick: 108 }), ok: true },
  { name: "a stopped hero whose first hit is two ticks past the attack point fails", legs: stoppedAndWalking({}, { latencyTicks: 9, hitTick: 109 }), ok: false },
  { name: "a walking hero whose first hit is late fails", legs: stoppedAndWalking({ latencyTicks: 20, hitTick: 120 }), ok: false },
  { name: "a judged leg with no first hit fails", legs: stoppedAndWalking({ hitTick: null, latencyTicks: null }), ok: false },
  { name: "a judged leg that never came into reach fails", legs: stoppedAndWalking({ reachTick: null, latencyTicks: null, hitTick: null }), ok: false },
  { name: "a judged leg with no observed attack point fails", legs: stoppedAndWalking({ attackPoint: 0 }), ok: false },
  { name: "a judged leg that did not reach the stage at start hp fails", legs: stoppedAndWalking({ ready: false }), ok: false },
  { name: "a judged leg with the lock not engaged fails", legs: stoppedAndWalking({ lock: "false" }), ok: false },
  { name: "an aborted judged leg fails", legs: stoppedAndWalking({ aborted: "hero dead" }), ok: false },
  { name: "a late lock-off control does not fail the verdict", legs: stoppedAndWalking({}, {}).map((l) => (l.judged ? l : { ...l, latencyTicks: 50, hitTick: 150 })), ok: true },
];

for (const c of cases) {
  const result = verdictFirstHit(c.legs);
  assert.equal(result.ok, c.ok, `${c.name}: ${JSON.stringify(result.reasons)}`);
}

const late = verdictFirstHit(stoppedAndWalking({}, { latencyTicks: 9, hitTick: 109 }));
assert.equal(
  late.reasons[0],
  "stopped, lock on: first hit 9 ticks after reach, later than the attack point 7 plus 1 tick",
);

console.log(`first-hit-timing verdict checks ok (${cases.length} cases)`);

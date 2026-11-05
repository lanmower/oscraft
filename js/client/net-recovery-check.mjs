import { FRAME } from "./wire/frame.js";
import { createFrameRecovery } from "./net_recovery.js";

const failures = [];
const expect = (name, got, want) => {
  const ok = JSON.stringify(got) === JSON.stringify(want);
  if (!ok) failures.push(`${name}: got ${JSON.stringify(got)} want ${JSON.stringify(want)}`);
  console.log(`${ok ? "ok" : "FAIL"} ${name} ${JSON.stringify(got)}`);
};

const requests = [];
const logs = [];
let baseline = false;
const applyFrame = (units, frame) => {
  if (frame.type === FRAME.STATE) {
    units.clear();
    baseline = true;
    for (const u of frame.units) units.set(u.id, u);
    return units;
  }
  if (!baseline) return null;
  for (const id of frame.removed) units.delete(id);
  for (const u of frame.units) units.set(u.id, u);
  return units;
};
const recovery = createFrameRecovery({
  applyFrame,
  requestKeyframe: (key) => requests.push(key),
  log: (key) => logs.push(key),
});
const units = new Map();
const delta = (tick) => ({ type: FRAME.DELTA, tick, units: [{ id: tick, kind: 1, player: 0, x: tick, y: 0, hp: 10 }], removed: [] });
const keyframe = { type: FRAME.STATE, tick: 9, units: [{ id: 1, kind: 1, player: 0, x: 3, y: 4, hp: 10 }] };

expect("delta without a baseline is dropped", recovery.apply(units, delta(1), "host-a").applied, false);
expect("null baseline triggers one recovery request", requests.length, 1);
expect("recovery request names the frame sender", requests[0], "host-a");
expect("null baseline is logged once", logs.length, 1);
expect("guest is awaiting a keyframe", recovery.isAwaiting(), true);
expect("second delta is dropped", recovery.apply(units, delta(2), "host-a").applied, false);
expect("third delta is dropped", recovery.apply(units, delta(3), "host-a").applied, false);
expect("repeated deltas keep one recovery request", requests.length, 1);
expect("repeated deltas keep one log", logs.length, 1);
const resumed = recovery.apply(units, keyframe, "host-a");
expect("later keyframe applies", resumed.applied, true);
expect("keyframe replaces the units", [...units.values()].map((u) => u.id), [1]);
expect("keyframe clears the wait", recovery.isAwaiting(), false);
expect("delta after the keyframe applies", recovery.apply(units, delta(4), "host-a").applied, true);
expect("units carry the post-keyframe delta", [...units.keys()], [1, 4]);
expect("resume adds no recovery request", requests.length, 1);
expect("resume adds no log", logs.length, 1);

if (failures.length) {
  console.log(failures.join(String.fromCharCode(10)));
  process.exit(1);
}
console.log("net-recovery-check: all passed");

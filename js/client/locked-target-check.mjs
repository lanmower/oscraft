import { createHeroRun, stepHeroRun, TARGET_CELLS_PER_S } from "./hero_run.js";
import { heroDirection } from "./hero_view_lock.js";

const fail = (what) => { throw new Error(what); };
const FRAME_S = 1 / 16;
const STEP = TARGET_CELLS_PER_S * FRAME_S;
const BOUNDS = { x0: 0, z0: 0, x1: 2000, z1: 2000 };
const CENTRE = { x: 1000, z: 1000 };
const EDGE_STARTS = [{ x: 1000, z: 1 }, { x: 1, z: 1000 }, { x: 1999, z: 1999 }];
const KEY_SETS = [
  ["ArrowUp"], ["ArrowUp", "ArrowRight"], ["ArrowRight"], ["ArrowDown", "ArrowRight"],
  ["ArrowDown"], ["ArrowDown", "ArrowLeft"], ["ArrowLeft"], ["ArrowUp", "ArrowLeft"],
];
const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));

const directionFor = (keys) => heroDirection(new Set(keys), 0, { x: 0, y: 0 });

function targetAfterOneFrame(start, direction) {
  const hero = { id: 3, x: start.x, z: start.z };
  const r = stepHeroRun(createHeroRun(), { hero, direction, held: true, dtS: FRAME_S, enemyIds: [], bounds: BOUNDS });
  return r.state.target;
}

function checkDirections(directionOf) {
  const moved = [];
  KEY_SETS.forEach((keys, i) => {
    const d = directionOf(keys);
    if (d === null) fail(`direction ${i} (${keys.join("+")}) has no direction`);
    const target = targetAfterOneFrame(CENTRE, d);
    const length = Math.hypot(target.x - CENTRE.x, target.z - CENTRE.z);
    if (Math.abs(length - STEP) > 1e-9) fail(`direction ${i} (${keys.join("+")}) locked: target moved ${length.toFixed(4)} cells from centre, want ${STEP.toFixed(4)}`);
    moved.push(`${keys.join("+")} ${length.toFixed(4)}`);
  });
  for (const start of EDGE_STARTS) {
    KEY_SETS.forEach((keys) => {
      const d = directionOf(keys);
      const target = targetAfterOneFrame(start, d);
      const want = { x: clamp(start.x + d.x * STEP, BOUNDS.x0, BOUNDS.x1), z: clamp(start.z + d.z * STEP, BOUNDS.z0, BOUNDS.z1) };
      if (Math.abs(target.x - want.x) > 1e-9 || Math.abs(target.z - want.z) > 1e-9) fail(`from (${start.x},${start.z}) ${keys.join("+")} landed at (${target.x}, ${target.z}), want the clamped (${want.x}, ${want.z})`);
    });
  }
  return moved;
}

const moved = checkDirections(directionFor);
const locked = (keys) => (keys.join("+") === "ArrowUp+ArrowRight" ? null : directionFor(keys));
let caught = false;
try {
  checkDirections(locked);
} catch {
  caught = true;
}
if (!caught) fail("negative control: a locked north-east direction was accepted");

console.log(`locked target ok: 8 directions from centre (${moved.join(", ")}); edge and corner starts land on the clamped point; negative control rejects a locked direction`);

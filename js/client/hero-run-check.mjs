import { createHeroRun, enemiesInSight, REORDER_S, stepHeroRun, TARGET_CELLS_PER_S } from "./hero_run.js";

const fail = (what) => { throw new Error(what); };
const near = (a, b, eps = 1e-6) => Math.abs(a - b) <= eps;
const FRAME_S = 0.016;
const BOUNDS = { x0: 0, z0: 0, x1: 200, z1: 200 };
const SQRT_HALF = Math.SQRT1_2;
const ENEMY = 7;
const hero = { id: 3, x: 100, z: 100 };
const SQUARE = [
  { x: 0, z: -1 },
  { x: SQRT_HALF, z: -SQRT_HALF },
  { x: 1, z: 0 },
  { x: SQRT_HALF, z: SQRT_HALF },
  { x: 0, z: 1 },
  { x: -SQRT_HALF, z: SQRT_HALF },
  { x: -1, z: 0 },
  { x: -SQRT_HALF, z: -SQRT_HALF },
];

const frame = (state, over = {}) => stepHeroRun(state, { hero, direction: null, held: false, dtS: FRAME_S, enemyIds: [], bounds: BOUNDS, ...over });
const orders = (r) => r.commands.filter((c) => c.kind === "order");
const holds = (r) => r.commands.filter((c) => c.kind === "hold");

const movesAlong = (target, from, d) => {
  const dx = target.x - from.x;
  const dz = target.z - from.z;
  return near(Math.hypot(dx, dz), TARGET_CELLS_PER_S * FRAME_S, 1e-9) && dx * d.x + dz * d.z > 0;
};
if (SQUARE.length !== 8) fail("the check needs eight directions");
for (const [i, d] of SQUARE.entries()) {
  const target = frame(createHeroRun(), { held: true, direction: d }).state.target;
  if (!movesAlong(target, hero, d)) fail(`direction ${i} did not move the target along its arrow`);
}

let state = createHeroRun();
let r = frame(state, { held: true, direction: { x: 0, z: -1 } });
if (orders(r).length !== 1) fail("a press issues exactly one order");
state = r.state;
let reorders = 0;
for (let i = 0; i < 30; i++) {
  r = frame(state, { held: true, direction: { x: 0, z: -1 } });
  reorders += orders(r).length;
  state = r.state;
}
if (reorders !== 1) fail(`a held key re-ordered ${reorders} times in 30 frames, want 1 at the ${REORDER_S}s cadence`);

const slow = [{ dt: 0.01, n: 10 }];
const fast = [{ dt: 0.1, n: 1 }];
const runSteps = (plan) => {
  let s = createHeroRun();
  for (const { dt, n } of plan) for (let i = 0; i < n; i++) s = frame(s, { held: true, direction: SQUARE[3], dtS: dt }).state;
  return s.target;
};
const slowTarget = runSteps(slow);
const fastTarget = runSteps(fast);
if (!near(slowTarget.x, fastTarget.x, 1e-9) || !near(slowTarget.z, fastTarget.z, 1e-9)) fail(`frame rate changed the target: ${JSON.stringify(slowTarget)} against ${JSON.stringify(fastTarget)}`);

const EDGE = { x0: 0, z0: 0, x1: 200, z1: 200 };
let atEdge = frame(createHeroRun(), { hero: { id: 3, x: 0.5, z: 100 }, held: true, direction: { x: -SQRT_HALF, z: SQRT_HALF }, dtS: 0.5, bounds: EDGE });
if (atEdge.state.target.x !== 0) fail(`target left the walkable bounds on x: ${atEdge.state.target.x}`);
if (!(atEdge.state.target.z > 100)) fail("a diagonal into the west edge did not keep moving on z");

r = frame(createHeroRun(), { held: true, direction: { x: 0, z: -1 } });
let pending = r.state;
for (let i = 0; i < 10; i++) pending = frame(pending, { held: true, direction: { x: 0, z: -1 } }).state;
const beforeRelease = pending.target;
const released = frame(pending, { held: false });
if (orders(released).length !== 1) fail("release issued no final order to the target");
if (!near(orders(released)[0].z, beforeRelease.z) || !near(orders(released)[0].x, beforeRelease.x)) fail("the final order is not the target");
let walk = released.state;
for (let i = 0; i < 5; i++) {
  const next = frame(walk, { held: false });
  if (orders(next).length + holds(next).length !== 0) fail("walking to a target issued a command after its final order");
  walk = next.state;
}
const arrived = frame(walk, { held: false, hero: { id: 3, x: beforeRelease.x, z: beforeRelease.z } });
if (arrived.state.target !== null) fail("the hero at its target did not finish the run");

const seen = frame(createHeroRun(), { held: true, direction: { x: 0, z: -1 }, enemyIds: [] });
const stopped = frame(seen.state, { held: true, direction: { x: 0, z: -1 }, enemyIds: [ENEMY] });
if (holds(stopped).length !== 1 || stopped.state.target !== null) fail("a new enemy in sight did not stop the run");

const idle = frame(createHeroRun());
if (idle.commands.length !== 0 || idle.state.target !== null) fail("an idle hero issued a command");

const sightLine = enemiesInSight({ x: 0, z: 0 }, [{ id: ENEMY, player: 1, x: 10, z: 0 }], () => true);
if (sightLine.length !== 1) fail("an enemy inside sight was not reported");

let continued = frame(createHeroRun(), { held: true, direction: { x: 0, z: -1 } }).state;
continued = frame(continued, { held: true, direction: { x: 0, z: -1 } }).state;
const pendingTarget = continued.target;
const repress = frame(frame(continued, { held: false }).state, { held: true, direction: { x: 1, z: 0 } });
if (!near(repress.state.target.x, pendingTarget.x + TARGET_CELLS_PER_S * FRAME_S, 1e-9)) fail("a press during a pending run did not continue from the target");

const EDGE_HERO = { id: 3, x: 0, z: 100 };
for (const d of SQUARE) {
  const target = frame(createHeroRun(), { hero: EDGE_HERO, held: true, direction: d }).state.target;
  if (target.x < BOUNDS.x0 || target.x > BOUNDS.x1 || target.z < BOUNDS.z0 || target.z > BOUNDS.z1) fail(`a target left the walkable bounds: ${JSON.stringify(target)}`);
  const inward = d.x >= 0;
  const moved = Math.hypot(target.x - EDGE_HERO.x, target.z - EDGE_HERO.z) > 1e-9;
  if (inward && !moved) fail(`an inward direction ${JSON.stringify(d)} from the west edge did not move the target`);
}
let walking = createHeroRun();
let previous = null;
for (let i = 0; i < 5; i++) {
  walking = frame(walking, { held: true, direction: SQUARE[2] }).state;
  if (previous !== null && !(walking.target.x > previous.x)) fail("a held direction did not keep advancing the target");
  previous = walking.target;
}
if (movesAlong(hero, hero, { x: 0, z: -1 })) fail("negative control: a target that never moves was accepted");

console.log("hero run ok (8 directions move the target, frame rate independent, clamped to the walkable bounds, release walks to the target)");

import {
  approachTarget,
  centredOnHero,
  createHeroViewLock,
  findHeroOf,
  followTarget,
  heroDirection,
  heroOrderPoint,
} from "./hero_view_lock.js";

const fail = (what) => { throw new Error(what); };
const near = (a, b) => Math.abs(a - b) < 1e-9;
const same = (got, want, what) => {
  const matches = want === null ? got === null : got !== null && near(got.x, want.x) && near(got.z, want.z);
  if (!matches) fail(`${what}: got ${JSON.stringify(got)}, want ${JSON.stringify(want)}`);
};
const keys = (...names) => new Set(names);
const SQRT_HALF = Math.SQRT1_2;
const STEP_CELLS = 4;

same(heroDirection(keys("ArrowUp"), 0), { x: 0, z: -1 }, "up at yaw 0 is forward");
same(heroDirection(keys("ArrowDown"), 0), { x: 0, z: 1 }, "down at yaw 0 is back");
same(heroDirection(keys("ArrowRight"), 0), { x: 1, z: 0 }, "right at yaw 0 is right");
same(heroDirection(keys("ArrowLeft"), 0), { x: -1, z: 0 }, "left at yaw 0 is left");
same(heroDirection(keys("ArrowUp"), Math.PI / 2), { x: -1, z: 0 }, "up at yaw 90 deg follows the camera");
same(heroDirection(keys("ArrowRight"), Math.PI / 2), { x: 0, z: -1 }, "right at yaw 90 deg follows the camera");
same(heroDirection(keys("ArrowUp", "ArrowRight"), 0), { x: SQRT_HALF, z: -SQRT_HALF }, "diagonal is unit length");
same(heroDirection(keys("ArrowUp", "ArrowDown"), 0), null, "opposite keys cancel");
if (heroDirection(keys("a", "Shift"), 0) !== null) fail("non-arrow keys produced a direction");
if (heroDirection(new Set(), 0) !== null) fail("no keys produced a direction");
for (const yaw of [0.3, 1.7, 4.2]) {
  const d = heroDirection(keys("ArrowLeft", "ArrowUp"), yaw);
  if (!near(Math.hypot(d.x, d.z), 1)) fail(`direction at yaw ${yaw} is not unit length`);
}

const groundAt = () => 7;
const hero = { id: 3, x: 100, z: 200 };
const target = followTarget(hero, groundAt);
if (target[0] !== 100 || target[1] !== 7 || target[2] !== 200) fail(`follow target ${JSON.stringify(target)}`);
const lobbyView = { target: [32, 0, 24], distance: 40, yaw: 0.5, pitch: 0.9 };
const centred = centredOnHero(lobbyView, hero, groundAt);
if (centred.target[0] !== 100 || centred.target[1] !== 7 || centred.target[2] !== 200) fail(`centred target ${JSON.stringify(centred.target)}`);
if (centred.distance !== 40 || centred.yaw !== 0.5 || centred.pitch !== 0.9) fail("centring changed the saved distance, yaw or pitch");
if (lobbyView.target[0] !== 32) fail("centring changed the saved view");
if (centredOnHero(lobbyView, null, groundAt) !== lobbyView) fail("no hero did not keep the saved view");
same(heroOrderPoint(hero, { x: 0, z: -1 }, STEP_CELLS), { x: 100, z: 200 - STEP_CELLS }, "order point ahead of the hero");
same(heroOrderPoint(hero, { x: SQRT_HALF, z: SQRT_HALF }, STEP_CELLS), { x: 100 + SQRT_HALF * STEP_CELLS, z: 200 + SQRT_HALF * STEP_CELLS }, "diagonal order point");

const units = [
  { id: 1, player: 0, type: 5, x: 0, z: 0 },
  { id: 2, player: 1, type: 6, x: 0, z: 0 },
  { id: 3, player: 0, type: 7, x: 50, z: 60 },
];
const flags = [];
flags[5] = 1;
flags[6] = 1;
if (findHeroOf(units, flags, 0)?.id !== 1) fail("local hero not found");
if (findHeroOf(units, flags, 1)?.id !== 2) fail("other player's hero not found");
if (findHeroOf(units, flags, 2) !== null) fail("a player without a hero found one");
if (findHeroOf(units, [], 0) !== null) fail("no flags still found a hero");

const goal = [10, 4, -6];
const from = [0, 0, 0];
same({ x: approachTarget(from, goal, 0)[0], z: approachTarget(from, goal, 0)[2] }, { x: 0, z: 0 }, "alpha 0 moved the focus");
same({ x: approachTarget(from, goal, 1)[0], z: approachTarget(from, goal, 1)[2] }, { x: 10, z: -6 }, "alpha 1 did not reach the goal");
same({ x: approachTarget(from, goal, 0.5)[0], z: approachTarget(from, goal, 0.5)[2] }, { x: 5, z: -3 }, "alpha 0.5 is not halfway");
if (approachTarget(from, goal, 2)[0] !== 10) fail("alpha above 1 overshot the goal");
if (approachTarget(from, goal, -1)[0] !== 0) fail("negative alpha moved the focus");
if (approachTarget(null, goal, 0.1)[0] !== 10 || approachTarget(null, goal, 0.1) === goal) fail("first focus did not snap to a copy of the goal");
if (approachTarget(from, goal, 1) === goal) fail("approach returned the goal array itself");

const lock = createHeroViewLock();
if (lock.isLocked()) fail("lock starts on");
if (lock.toggle() !== true || !lock.isLocked()) fail("toggle did not lock");
if (lock.toggle() !== false || lock.isLocked()) fail("second toggle did not unlock");
lock.toggle();
if (!lock.isLocked()) fail("lock did not stay on before a pan");
if (lock.release() !== false || lock.isLocked()) fail("pan release did not unlock");
if (lock.release() !== false) fail("release of an unlocked view changed state");

console.log("hero view lock ok");

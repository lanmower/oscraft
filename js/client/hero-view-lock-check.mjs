import {
  createHeroViewLock,
  findHeroOf,
  followTarget,
  heroDirection,
  heroOrderPoint,
  HERO_ORDER_INTERVAL_S,
  HERO_STEP_CELLS,
  stepHeroOrderClock,
} from "./hero_view_lock.js";

const fail = (what) => { throw new Error(what); };
const near = (a, b) => Math.abs(a - b) < 1e-9;
const same = (got, want, what) => {
  const matches = want === null ? got === null : got !== null && near(got.x, want.x) && near(got.z, want.z);
  if (!matches) fail(`${what}: got ${JSON.stringify(got)}, want ${JSON.stringify(want)}`);
};
const keys = (...names) => new Set(names);
const SQRT_HALF = Math.SQRT1_2;

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
same(heroOrderPoint(hero, { x: 0, z: -1 }), { x: 100, z: 200 - HERO_STEP_CELLS }, "order point ahead of the hero");
same(heroOrderPoint(hero, { x: SQRT_HALF, z: SQRT_HALF }), { x: 100 + SQRT_HALF * HERO_STEP_CELLS, z: 200 + SQRT_HALF * HERO_STEP_CELLS }, "diagonal order point");

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

const idle = stepHeroOrderClock(0.4, 0.1, false);
if (idle.due || idle.clockS !== 0) fail("idle clock is not reset");
const first = stepHeroOrderClock(0, 0.016, true);
if (!first.due || first.clockS !== HERO_ORDER_INTERVAL_S) fail("first held frame does not send at once");
const waiting = stepHeroOrderClock(HERO_ORDER_INTERVAL_S, 0.1, true);
if (waiting.due || !near(waiting.clockS, HERO_ORDER_INTERVAL_S - 0.1)) fail("held key sends before the interval");
const again = stepHeroOrderClock(0.05, 0.06, true);
if (!again.due || again.clockS !== HERO_ORDER_INTERVAL_S) fail("held key does not resend after the interval");

const lock = createHeroViewLock();
if (lock.isLocked()) fail("lock starts on");
if (lock.toggle() !== true || !lock.isLocked()) fail("toggle did not lock");
if (lock.toggle() !== false || lock.isLocked()) fail("second toggle did not unlock");
lock.toggle();
if (!lock.isLocked()) fail("lock did not stay on before a pan");
if (lock.release() !== false || lock.isLocked()) fail("pan release did not unlock");
if (lock.release() !== false) fail("release of an unlocked view changed state");

console.log("hero view lock ok");

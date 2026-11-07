import { heroOrderPoint } from "./hero_view_lock.js";

export const RUN_CELLS = 64;
export const TAP_MAX_S = 0.2;
export const HERO_SIGHT_CELLS = 28.125;
export const REORDER_S = 0.25;
const ARRIVE_CELLS = 1;

export function createHeroRun() {
  return { pressing: false, pressedS: 0, sinceOrderS: 0, heading: null, target: null, ordered: false, seenEnemies: null, run: null };
}

export function enemiesInSight(hero, units, isHostile) {
  if (!hero) return [];
  return units.filter((u) => isHostile(u.player) && distance(hero, u) <= HERO_SIGHT_CELLS).map((u) => u.id);
}

export function stepHeroRun(state, input) {
  if (!input.hero) return { state: createHeroRun(), commands: [] };
  if (input.held && !state.pressing) {
    return holding({ ...createHeroRun(), pressing: true, seenEnemies: new Set(input.enemyIds) }, input);
  }
  if (input.held) return holding(state, input);
  if (state.pressing) return released(state, input);
  return running(state, input);
}

function holding(state, input) {
  const next = { ...state, pressedS: state.pressedS + input.dtS, sinceOrderS: state.sinceOrderS + input.dtS };
  if (input.direction === null) return { state: next, commands: [] };
  const turned = !sameDirection(input.direction, state.heading);
  if (!turned && next.sinceOrderS < REORDER_S) return { state: next, commands: [] };
  const target = clampToMap(heroOrderPoint(input.hero, input.direction, RUN_CELLS), input.bounds);
  return { state: { ...next, heading: input.direction, target, ordered: true, sinceOrderS: 0 }, commands: [order(target)] };
}

function released(state, input) {
  if (!state.ordered) return { state: createHeroRun(), commands: [] };
  if (state.pressedS >= TAP_MAX_S) return { state: createHeroRun(), commands: [{ kind: "hold" }] };
  return { state: { ...createHeroRun(), run: { target: state.target, seenEnemies: state.seenEnemies } }, commands: [] };
}

function running(state, input) {
  if (!state.run) return { state, commands: [] };
  const { target, seenEnemies } = state.run;
  if (distance(input.hero, target) < ARRIVE_CELLS) return { state: createHeroRun(), commands: [] };
  if (input.enemyIds.some((id) => !seenEnemies.has(id))) {
    return { state: createHeroRun(), commands: [{ kind: "hold" }] };
  }
  return { state: { ...createHeroRun(), run: { target, seenEnemies: new Set(input.enemyIds) } }, commands: [] };
}

function sameDirection(direction, heading) {
  return heading !== null && direction.x === heading.x && direction.z === heading.z;
}

function clampToMap(point, bounds) {
  return { x: clamp(point.x, 0, bounds.w), z: clamp(point.z, 0, bounds.h) };
}

function clamp(value, low, high) {
  return Math.min(high, Math.max(low, value));
}

function distance(a, b) {
  return Math.hypot(a.x - b.x, a.z - b.z);
}

function order(target) {
  return { kind: "order", x: target.x, z: target.z };
}

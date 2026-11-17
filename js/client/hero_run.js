export const TARGET_CELLS_PER_S = 24;
export const HERO_SIGHT_CELLS = 28.125;
export const REORDER_S = 0.25;
const ARRIVE_CELLS = 1;

export function createHeroRun() {
  return { held: false, target: null, ordered: null, sinceOrderS: 0, seenEnemies: null };
}

export function enemiesInSight(hero, units, isHostile) {
  if (!hero) return [];
  return units.filter((u) => isHostile(u.player) && distance(hero, u) <= HERO_SIGHT_CELLS).map((u) => u.id);
}

export function stepHeroRun(state, input) {
  if (!input.hero) return { state: createHeroRun(), commands: [] };
  if (state.target !== null && input.enemyIds.some((id) => !state.seenEnemies.has(id))) {
    return { state: createHeroRun(), commands: [{ kind: "hold" }] };
  }
  const target = advanceTarget(state, input);
  if (target === null) return { state: createHeroRun(), commands: [] };
  if (!input.held && distance(input.hero, target) < ARRIVE_CELLS) return { state: createHeroRun(), commands: [] };
  const sinceOrderS = state.sinceOrderS + input.dtS;
  const pressed = input.held && !state.held;
  const due = state.ordered === null || pressed || (input.held ? sinceOrderS >= REORDER_S : !samePoint(state.ordered, target));
  const next = {
    held: input.held,
    target,
    ordered: due ? target : state.ordered,
    sinceOrderS: due ? 0 : sinceOrderS,
    seenEnemies: state.target === null ? new Set(input.enemyIds) : state.seenEnemies,
  };
  return { state: next, commands: due ? [order(target)] : [] };
}

function advanceTarget(state, input) {
  const start = state.target ?? (input.direction ? { x: input.hero.x, z: input.hero.z } : null);
  if (start === null || !input.direction) return start;
  const step = TARGET_CELLS_PER_S * input.dtS;
  return clampToBounds({ x: start.x + input.direction.x * step, z: start.z + input.direction.z * step }, input.bounds);
}

function samePoint(a, b) {
  return a.x === b.x && a.z === b.z;
}

function clampToBounds(point, bounds) {
  return { x: clamp(point.x, bounds.x0, bounds.x1), z: clamp(point.z, bounds.z0, bounds.z1) };
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

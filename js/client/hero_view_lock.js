export const HERO_STEP_WORLD = 300;
export const HERO_ORDER_INTERVAL_S = 0.25;

const HERO_FLAG = 1;

const ARROW_AXES = new Map([
  ["ArrowLeft", [-1, 0]],
  ["ArrowRight", [1, 0]],
  ["ArrowUp", [0, 1]],
  ["ArrowDown", [0, -1]],
]);

export function createHeroViewLock() {
  let locked = false;
  return {
    isLocked: () => locked,
    toggle() {
      locked = !locked;
      return locked;
    },
    release() {
      locked = false;
      return locked;
    },
  };
}

export function heroDirection(keys, yaw) {
  let right = 0;
  let forward = 0;
  for (const key of keys) {
    const axis = ARROW_AXES.get(key);
    if (axis) {
      right += axis[0];
      forward += axis[1];
    }
  }
  const length = Math.hypot(right, forward);
  if (length === 0) return null;
  const r = right / length;
  const f = forward / length;
  return {
    x: r * Math.cos(yaw) - f * Math.sin(yaw),
    z: -r * Math.sin(yaw) - f * Math.cos(yaw),
  };
}

export function followTarget(hero, groundAt) {
  return [hero.x, groundAt(hero.x, hero.z), hero.z];
}

export function heroOrderPoint(hero, direction, distance = HERO_STEP_WORLD) {
  return { x: hero.x + direction.x * distance, z: hero.z + direction.z * distance };
}

export function findHeroOf(units, flags, player) {
  return units.find((u) => u.player === player && flags[u.type] === HERO_FLAG) ?? null;
}

export function stepHeroOrderClock(clockS, dtS, moving) {
  if (!moving) return { due: false, clockS: 0 };
  const left = clockS - dtS;
  if (left > 0) return { due: false, clockS: left };
  return { due: true, clockS: HERO_ORDER_INTERVAL_S };
}

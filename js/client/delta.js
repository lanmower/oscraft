// State frames for the browser client: a full state frame every FULL_EVERY ticks, a delta frame otherwise.
// Pure: no DOM and no environment access, so Node CI can import it.
import { FRAME } from './wire/frame.js';

export const FULL_EVERY = 20;

const sameUnit = (a, b) => a.kind === b.kind && a.player === b.player && a.x === b.x && a.y === b.y && a.hp === b.hp;

// encode(tick, hash, units) returns the frame for this tick. The first frame and every fullEvery-th frame
// are full; the rest carry only units whose record changed since the previous frame, plus the ids that left.
export function createStateEncoder({ fullEvery = FULL_EVERY } = {}) {
  let last = null;
  let count = 0;
  return {
    encode(tick, hash, units) {
      const full = last === null || count % fullEvery === 0;
      count += 1;
      const now = new Map(units.map((u) => [u.id, u]));
      let frame;
      if (full) {
        frame = { type: FRAME.STATE, tick, hash, units };
      } else {
        const changed = units.filter((u) => !last.has(u.id) || !sameUnit(last.get(u.id), u));
        const removed = [...last.keys()].filter((id) => !now.has(id));
        frame = { type: FRAME.DELTA, tick, hash, units: changed, removed };
      }
      last = now;
      return frame;
    },
  };
}

// Applies a state or delta frame to units (a Map of id -> unit) and returns it, or returns null when the frame
// was ignored. Each Map keeps its own baseline: deltas are ignored until a state frame has been applied to that Map.
const baselined = new WeakSet();

export function applyFrame(units, frame) {
  if (frame.type === FRAME.STATE) {
    units.clear();
    baselined.add(units);
  } else {
    if (!baselined.has(units)) return null;
    for (const id of frame.removed) units.delete(id);
  }
  for (const u of frame.units) units.set(u.id, u);
  return units;
}

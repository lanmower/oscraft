// State frames for the browser client: a full state frame every FULL_EVERY frames, a delta frame otherwise.
// Pure: no DOM and no environment access, so Node CI can import it.
import { FRAME, MAX_PLAYERS, TEAM_SIZE, SPECTATOR } from './wire/frame.js';

export const FULL_EVERY = 20;

const sameUnit = (a, b) => a.kind === b.kind && a.player === b.player && a.x === b.x && a.y === b.y && a.hp === b.hp;

// encode(tick, hash, units) returns the frame for this tick. The first frame and every fullEvery-th frame
// are full; the rest carry only units whose record changed since the previous frame, plus the ids that left.
export function createStateEncoder({ fullEvery = FULL_EVERY } = {}) {
  let last = null;
  let count = 0;
  let forceFull = false;
  return {
    // The next frame is a full state frame: a late joiner or a reconnecting peer needs a baseline.
    keyframe() { forceFull = true; },
    encode(tick, hash, units) {
      const full = forceFull || last === null || count % fullEvery === 0;
      forceFull = false;
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

export const teamOfSlot = (slot) => (slot < TEAM_SIZE ? 0 : 1);

// Host-side seating. start(seats) takes the seats of the lobby start frame; a peer key then maps to its player id
// (the slot) and team. A key that was seated gets the same seat back when it reconnects, a key without a seat
// watches as SPECTATOR. export()/restore() carry the seats through a snapshot so a resumed host keeps them.
export function createSeating() {
  const seatOf = new Map();
  const connected = new Set();
  const stats = new Map();
  const seat = (key) => (seatOf.has(key) ? { you: seatOf.get(key), team: teamOfSlot(seatOf.get(key)) } : { you: SPECTATOR, team: SPECTATOR });
  return {
    start(seats) {
      seatOf.clear();
      for (const s of seats) if (s.slot < MAX_PLAYERS) seatOf.set(s.owner, s.slot);
      return seatOf.size;
    },
    join(key) { connected.add(key); return seat(key); },
    leave(key) { connected.delete(key); },
    seat,
    setStats(slot, value) { stats.set(slot, value); },
    size: () => seatOf.size,
    // ROSTER frame for one peer: its own seat plus the per-player data of every seat.
    roster(key) {
      const players = [...seatOf].sort((a, b) => a[1] - b[1]).map(([owner, slot]) => {
        const st = stats.get(slot) || {};
        return { slot, team: teamOfSlot(slot), connected: connected.has(owner) ? 1 : 0, gold: st.gold || 0, kills: st.kills || 0, deaths: st.deaths || 0 };
      });
      return { type: FRAME.ROSTER, ...seat(key), players };
    },
    export() { return [...seatOf].map(([owner, slot]) => ({ owner, slot })); },
    restore(saved) { seatOf.clear(); for (const s of saved) seatOf.set(s.owner, s.slot); return seatOf.size; },
  };
}

// START frame from a lobby state (the reduceLobby state with started true): the owned seats in slot order.
export function startFrame(lobby) {
  const seats = [];
  lobby.slots.forEach((s, slot) => { if (s.owner) seats.push({ slot, owner: s.owner, pick: s.pick || '' }); });
  return { type: FRAME.START, seats };
}

import { createAutoSeats, createSeating } from "./delta.js";
import { MAX_PLAYERS, SPECTATOR } from "./wire/frame.js";

const fail = (what) => { throw new Error(what); };
const peer = (name) => `peer-${name}`;

function refreshStorm(rounds, newFirst) {
  const seating = createSeating();
  const autoSeats = createAutoSeats(seating, MAX_PLAYERS);
  const slots = [];
  let previous = null;
  for (let n = 0; n < rounds; n++) {
    const key = peer(n);
    if (!newFirst && previous !== null) autoSeats.leave(previous);
    autoSeats.join(key);
    slots.push(seating.seat(key).you);
    if (newFirst && previous !== null) autoSeats.leave(previous);
    previous = key;
  }
  return slots;
}

for (const newFirst of [true, false]) {
  const order = newFirst ? "new joins before old leaves" : "old leaves before new joins";
  const slots = refreshStorm(40, newFirst);
  const spectators = slots.filter((slot) => slot === SPECTATOR).length;
  if (spectators > 0) fail(`refresh storm (${order}) left ${spectators} of 40 refreshed peers as spectators: ${slots.join(",")}`);
  console.log(`refresh storm (${order}): 40 refreshes, 0 spectators, slots ${slots.slice(0, 12).join(",")}...`);
}

{
  const seating = createSeating();
  const autoSeats = createAutoSeats(seating, MAX_PLAYERS);
  const table = Array.from({ length: MAX_PLAYERS }, (_, i) => `table-${i}`);
  table.forEach((key) => autoSeats.join(key));
  if (table.some((key) => seating.seat(key).you === SPECTATOR)) fail("ten peers did not each get a seat");
  autoSeats.join("late");
  if (seating.seat("late").you !== SPECTATOR) fail("an eleventh peer got a seat while ten were present");
  autoSeats.leave(table[3]);
  autoSeats.join("after-leave");
  const reused = seating.seat("after-leave").you;
  if (reused !== 3) fail(`the seat freed by table-3 went to slot ${reused}, not 3`);
  if (seating.roster("after-leave").you !== 3) fail("the roster does not report the reused seat");
  console.log(`full table: ten seated, eleventh spectates, freed slot 3 reused by the next peer (roster you=${seating.roster("after-leave").you})`);
}

console.log("lobby seats ok");

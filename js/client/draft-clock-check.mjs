import { createDraftClock } from "./draft_clock.js";

const fail = (what) => { throw new Error(what); };
const sent = [];
let timer = null;
let permitted = true;
const clock = createDraftClock({
  send: (ticks) => sent.push(ticks),
  allowed: () => permitted,
  schedule: (fn, ms) => { timer = { fn, ms }; return timer; },
  cancel: (handle) => { if (handle === timer) timer = null; },
});

if (timer !== null) fail("a timer runs before a draft is active");
clock.setActive(true);
if (timer === null || timer.ms !== 1000) fail("an active draft does not run a one second timer");
timer.fn();
if (sent.length !== 1 || sent[0] !== 16) fail("a beat does not send sixteen ticks");
permitted = false;
timer.fn();
if (sent.length !== 1) fail("a guest sent a clock beat");
permitted = true;
clock.setActive(true);
timer.fn();
if (sent.length !== 2) fail("a repeated activation changed the timer");
clock.setActive(false);
if (timer !== null || clock.isActive()) fail("the timer outlives the draft");
clock.beat();
if (sent.length !== 2) fail("an inactive clock sent a beat");
console.log("draft clock ok");

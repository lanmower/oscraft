import { createHeroRun, REORDER_S, stepHeroRun } from "./hero_run.js";
import { approachTarget, followTarget } from "./hero_view_lock.js";

const MAX_FRAME_S = 0.1;
const SIM_TICK_S = 1 / 16;
const LOCK_FOLLOW_RATE = 14;
const ASSUMED_SPEEDS_CELLS_PER_S = [12, 4];
const START = { x: 2000, z: 2000 };
const BOUNDS = { w: 4000, h: 4000 };
const NORTH = { x: 0, z: -1 };
const EPSILON = 1e-9;
const SETTLE_CELLS = 0.01;
const UNIFORM_60_HZ = [1 / 60];
const JITTERED_LOW_FPS = [1 / 60, 1 / 60, 1 / 30, 1 / 60, 1 / 7, 1 / 60, 1 / 45, 1 / 3, 1 / 60, 1 / 20, 1 / 60, 1 / 60, 1 / 12];
const PROFILES = [
  { name: "uniform60", intervals: UNIFORM_60_HZ, strictFrameCap: true },
  { name: "jitteredLowFps", intervals: JITTERED_LOW_FPS, strictFrameCap: false },
];
const SCHEDULE = [
  { held: true, seconds: 3 },
  { held: false, seconds: 2 },
  { held: true, seconds: 2 },
  { held: false, seconds: 2 },
];

const fail = (what) => { throw new Error(what); };
const groundAt = () => 0;
const span = (a, b) => Math.hypot(a.x - b.x, a.z - b.z);
const planar = (a, b) => Math.hypot(a[0] - b[0], a[2] - b[2]);

function advanceTick(sim) {
  sim.prevX = sim.x;
  sim.prevZ = sim.z;
  if (!sim.target) return;
  const dx = sim.target.x - sim.x;
  const dz = sim.target.z - sim.z;
  const distance = Math.hypot(dx, dz);
  if (distance === 0) return;
  const step = Math.min(distance, sim.speed * SIM_TICK_S);
  sim.x += (dx / distance) * step;
  sim.z += (dz / distance) * step;
}

function advanceClock(sim, dt) {
  sim.tickAcc += dt;
  while (sim.tickAcc >= SIM_TICK_S) {
    sim.tickAcc -= SIM_TICK_S;
    advanceTick(sim);
  }
  const alpha = sim.tickAcc / SIM_TICK_S;
  return { x: sim.prevX + (sim.x - sim.prevX) * alpha, z: sim.prevZ + (sim.z - sim.prevZ) * alpha };
}

function simulate({ speed, intervals, schedule, strictFrameCap = false, mutate = {} }) {
  const sim = { speed, x: START.x, z: START.z, prevX: START.x, prevZ: START.z, target: null, tickAcc: 0 };
  let state = createHeroRun();
  let focus = null;
  let t = 0;
  let frameIndex = 0;
  const record = { speed, strictFrameCap, frames: [], orders: [], holds: [], segments: [] };
  for (const segment of schedule) {
    const segmentStart = t;
    const firstFrame = record.frames.length;
    let segmentTime = 0;
    while (segmentTime < segment.seconds - EPSILON) {
      const dt = Math.min(MAX_FRAME_S, intervals[frameIndex % intervals.length]);
      frameIndex += 1;
      segmentTime += dt;
      t += dt;
      const hero = advanceClock(sim, dt);
      const step = stepHeroRun(state, {
        hero: { id: 1, x: sim.x, z: sim.z },
        direction: segment.held ? NORTH : null,
        held: segment.held,
        dtS: dt,
        enemyIds: [],
        bounds: BOUNDS,
      });
      state = step.state;
      for (const command of step.commands) {
        if (command.kind === "order") {
          sim.target = { x: command.x, z: command.z };
          record.orders.push({ t, held: segment.held, from: { x: sim.x, z: sim.z }, to: { x: command.x, z: command.z } });
        } else {
          sim.target = { x: sim.x, z: sim.z };
          record.holds.push({ t, held: segment.held });
        }
      }
      focus = approachTarget(focus, followTarget(hero, groundAt), 1 - Math.exp(-dt * LOCK_FOLLOW_RATE));
      record.frames.push({ t, dt, held: segment.held, hero: { x: sim.x, z: sim.z }, target: followTarget(hero, groundAt), focus: [...focus] });
    }
    record.segments.push({ held: segment.held, startT: segmentStart, endT: t, firstFrame, lastFrame: record.frames.length });
  }
  if (mutate.dropEveryThirdOrder) record.orders = record.orders.filter((_, i) => i % 3 !== 2);
  if (mutate.dropHolds) record.holds = [];
  return record;
}

function violations(record) {
  const out = [];
  const speed = record.speed;
  const frames = record.frames;
  const maxDt = Math.max(...frames.map((f) => f.dt));
  const orderGapCap = REORDER_S + maxDt + EPSILON;
  const orderGapFloor = REORDER_S - EPSILON;

  for (const segment of record.segments.filter((s) => s.held)) {
    const orders = record.orders.filter((o) => o.t > segment.startT - EPSILON && o.t <= segment.endT + EPSILON);
    if (orders.length === 0) { out.push(`press at ${segment.startT.toFixed(3)}s issued no order`); continue; }
    const firstFrameEnd = segment.startT + frames[segment.firstFrame].dt;
    const atPress = orders.filter((o) => o.t <= firstFrameEnd + EPSILON).length;
    if (atPress !== 1) out.push(`press at ${segment.startT.toFixed(3)}s issued ${atPress} orders on its first frame, want 1`);
    for (let i = 1; i < orders.length; i++) {
      const gap = orders[i].t - orders[i - 1].t;
      if (gap > orderGapCap) out.push(`order gap ${gap.toFixed(4)}s exceeds cadence ${REORDER_S}s plus one frame`);
      if (gap < orderGapFloor) out.push(`order gap ${gap.toFixed(4)}s is shorter than cadence ${REORDER_S}s`);
    }
    const tail = segment.endT - orders[orders.length - 1].t;
    if (tail > orderGapCap) out.push(`no order in the last ${tail.toFixed(4)}s of a held press`);
    const start = record.frames[segment.firstFrame].hero;
    const end = record.frames[segment.lastFrame - 1].hero;
    const seconds = segment.endT - segment.startT;
    if (!(span(start, end) >= 0.7 * speed * seconds)) out.push(`held press travelled ${span(start, end).toFixed(2)} cells, want at least ${(0.7 * speed * seconds).toFixed(2)}`);
  }

  for (const segment of record.segments.filter((s) => !s.held)) {
    const ordersInside = record.orders.filter((o) => o.t > segment.startT + EPSILON && o.t <= segment.endT + EPSILON);
    if (ordersInside.length > 0) out.push(`${ordersInside.length} order(s) issued during a release`);
    const holdsInside = record.holds.filter((h) => h.t > segment.startT - EPSILON && h.t <= segment.startT + frames[segment.firstFrame].dt + EPSILON);
    if (holdsInside.length !== 1) out.push(`release at ${segment.startT.toFixed(3)}s issued ${holdsInside.length} holds on its first frame, want 1`);
    const stopped = record.frames[segment.firstFrame + 1];
    const last = record.frames[segment.lastFrame - 1];
    if (stopped && span(stopped.hero, last.hero) > SIM_TICK_S * speed * 2 + EPSILON) out.push(`hero kept moving after release: ${span(stopped.hero, last.hero).toFixed(3)} cells`);
    if (planar(last.focus, last.target) > SETTLE_CELLS) out.push(`focus did not converge after release: ${planar(last.focus, last.target).toFixed(4)} cells`);
  }

  const lagCap = speed / LOCK_FOLLOW_RATE;
  for (let i = 1; i < frames.length; i++) {
    const dt = frames[i].dt;
    const step = planar(frames[i].focus, frames[i - 1].focus);
    const derivedCap = (1 - Math.exp(-LOCK_FOLLOW_RATE * dt)) * (lagCap + speed * dt);
    if (step > derivedCap + EPSILON) out.push(`focus step ${step.toFixed(4)} exceeds derived cap ${derivedCap.toFixed(4)} at ${frames[i].t.toFixed(3)}s`);
    if (record.strictFrameCap && step > speed * dt + EPSILON) out.push(`focus step ${step.toFixed(4)} exceeds speed*dt ${(speed * dt).toFixed(4)} at ${frames[i].t.toFixed(3)}s`);
    const lag = planar(frames[i].target, frames[i].focus);
    if (lag > lagCap + EPSILON) out.push(`focus lag ${lag.toFixed(4)} exceeds speed/k ${lagCap.toFixed(4)} at ${frames[i].t.toFixed(3)}s`);
  }
  return out;
}

function witness(record) {
  const frames = record.frames;
  const focusRatios = [];
  for (let i = 1; i < frames.length; i++) focusRatios.push(planar(frames[i].focus, frames[i - 1].focus) / (record.speed * frames[i].dt));
  const gaps = [];
  for (const segment of record.segments.filter((s) => s.held)) {
    const orders = record.orders.filter((o) => o.t > segment.startT - EPSILON && o.t <= segment.endT + EPSILON);
    for (let i = 1; i < orders.length; i++) gaps.push(orders[i].t - orders[i - 1].t);
  }
  return {
    frames: frames.length,
    maxDt: Math.max(...frames.map((f) => f.dt)),
    orders: record.orders.length,
    holds: record.holds.length,
    minGap: Math.min(...gaps),
    maxGap: Math.max(...gaps),
    maxFocusRatio: Math.max(...focusRatios),
  };
}

const ran = [];
for (const speed of ASSUMED_SPEEDS_CELLS_PER_S) {
  for (const profile of PROFILES) {
    const record = simulate({ speed, intervals: profile.intervals, schedule: SCHEDULE, strictFrameCap: profile.strictFrameCap === true });
    const found = violations(record);
    if (found.length > 0) fail(`speed ${speed} ${profile.name}: ${found.slice(0, 3).join("; ")}`);
    const w = witness(record);
    ran.push(`speed=${speed} profile=${profile.name} frames=${w.frames} maxDt=${w.maxDt.toFixed(4)} orders=${w.orders} holds=${w.holds} gap=[${w.minGap.toFixed(4)},${w.maxGap.toFixed(4)}] maxFocusRatio=${w.maxFocusRatio.toFixed(4)}`);
    console.log(ran[ran.length - 1]);
  }
}

const droppedOrder = violations(simulate({ speed: 12, intervals: JITTERED_LOW_FPS, schedule: SCHEDULE, mutate: { dropEveryThirdOrder: true } }));
if (droppedOrder.length === 0) fail("negative control: a dropped order did not fail the gate");
const missedRelease = violations(simulate({ speed: 12, intervals: JITTERED_LOW_FPS, schedule: SCHEDULE, mutate: { dropHolds: true } }));
if (missedRelease.length === 0) fail("negative control: a missed release did not fail the gate");
console.log(`negative control dropped order: ${droppedOrder.length} violation(s), first: ${droppedOrder[0]}`);
console.log(`negative control missed release: ${missedRelease.length} violation(s), first: ${missedRelease[0]}`);
console.log(`hero trajectory ok (${ASSUMED_SPEEDS_CELLS_PER_S.length} assumed speeds x ${PROFILES.length} frame profiles, 2 negative controls)`);

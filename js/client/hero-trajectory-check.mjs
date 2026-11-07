import { createHeroRun, REORDER_S, RUN_CELLS, stepHeroRun } from "./hero_run.js";
import { approachTarget, followTarget } from "./hero_view_lock.js";

const FRAME_S = 1 / 60;
const SIM_TICK_S = 1 / 16;
const HOLD_S = 3;
const RELEASE_S = 4;
const SETTLE_LIMIT_S = 1;
const SETTLE_CELLS = 0.01;
const LOCK_FOLLOW_RATE = 14;
const ASSUMED_SPEEDS_CELLS_PER_S = [12, 4];
const START = { x: 2000, z: 2000 };
const BOUNDS = { w: 4000, h: 4000 };
const NORTH = { x: 0, z: -1 };
const FOCUS_ALPHA = 1 - Math.exp(-FRAME_S * LOCK_FOLLOW_RATE);

const fail = (what) => { throw new Error(what); };
const span = (a, b) => Math.hypot(a.x - b.x, a.z - b.z);
const groundAt = () => 0;

function makeSim(speed) {
  return { speed, x: START.x, z: START.z, prevX: START.x, prevZ: START.z, target: null, tickAcc: 0 };
}

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

function advanceFrame(sim) {
  sim.tickAcc += FRAME_S;
  while (sim.tickAcc >= SIM_TICK_S) {
    sim.tickAcc -= SIM_TICK_S;
    advanceTick(sim);
  }
  const alpha = sim.tickAcc / SIM_TICK_S;
  return {
    hero: { x: sim.prevX + (sim.x - sim.prevX) * alpha, z: sim.prevZ + (sim.z - sim.prevZ) * alpha },
    tick: { x: sim.x, z: sim.z },
  };
}

function runFrames(run, seconds, held) {
  const frames = Math.round(seconds / FRAME_S);
  for (let i = 0; i < frames; i++) {
    const { hero, tick } = advanceFrame(run.sim);
    const step = stepHeroRun(run.state, {
      hero: { id: 1, x: run.sim.x, z: run.sim.z },
      direction: held ? NORTH : null,
      held,
      dtS: FRAME_S,
      enemyIds: [],
      bounds: BOUNDS,
    });
    run.state = step.state;
    for (const command of step.commands) {
      if (command.kind === "order") {
        run.sim.target = { x: command.x, z: command.z };
        run.orders.push({ t: run.t, from: { x: run.sim.x, z: run.sim.z }, to: { x: command.x, z: command.z } });
      } else {
        run.sim.target = { x: run.sim.x, z: run.sim.z };
        run.holds += 1;
      }
    }
    run.focus = approachTarget(run.focus, followTarget(hero, groundAt), FOCUS_ALPHA);
    run.frames.push({ tick: [tick.x, tick.z], focus: [...run.focus], target: followTarget(hero, groundAt) });
    run.t += FRAME_S;
  }
}

function measure(speed) {
  const run = { sim: makeSim(speed), state: createHeroRun(), focus: null, orders: [], holds: 0, frames: [], t: 0 };
  runFrames(run, HOLD_S, true);
  const holdFrames = run.frames.length;
  const orderGaps = run.orders.slice(1).map((o, i) => o.t - run.orders[i].t);
  const maxOrderGap = Math.max(...orderGaps);
  const heroTravel = span(run.sim, START);
  const tickSteps = [];
  const focusSteps = [];
  for (let i = 1; i < holdFrames; i++) {
    const a = run.frames[i - 1];
    const b = run.frames[i];
    tickSteps.push(Math.hypot(b.tick[0] - a.tick[0], b.tick[1] - a.tick[1]));
    focusSteps.push(Math.hypot(b.focus[0] - a.focus[0], b.focus[2] - a.focus[2]));
  }
  const oldTickMaxStep = Math.max(...tickSteps);
  const focusMaxStep = Math.max(...focusSteps);
  const focusMeanStep = focusSteps.reduce((a, b) => a + b, 0) / focusSteps.length;
  const lastFrame = run.frames[holdFrames - 1];
  const lagAtEnd = Math.hypot(lastFrame.focus[0] - lastFrame.target[0], lastFrame.focus[2] - lastFrame.target[2]);

  const holdsBefore = run.holds;
  runFrames(run, RELEASE_S, false);
  const releaseFrames = run.frames.slice(holdFrames);
  const settleIndex = releaseFrames.findIndex((f) => Math.hypot(f.focus[0] - f.target[0], f.focus[2] - f.target[2]) < SETTLE_CELLS);
  const settleS = settleIndex < 0 ? null : (settleIndex + 1) * FRAME_S;

  return {
    speed,
    orders: run.orders.length,
    maxOrderGap,
    heroTravel,
    oldTickMaxStep,
    focusMaxStep,
    focusMeanStep,
    lagAtEnd,
    holdsOnRelease: run.holds - holdsBefore,
    settleS,
  };
}

const EPSILON = 1e-9;
const results = ASSUMED_SPEEDS_CELLS_PER_S.map(measure);
for (const r of results) {
  if (r.orders < Math.floor(HOLD_S / REORDER_S)) fail(`speed ${r.speed}: ${r.orders} orders in ${HOLD_S}s held, want at least ${Math.floor(HOLD_S / REORDER_S)}`);
  if (!(r.maxOrderGap <= REORDER_S + FRAME_S + EPSILON)) fail(`speed ${r.speed}: order gap ${r.maxOrderGap} exceeds cadence ${REORDER_S}`);
  if (!(r.heroTravel >= 0.7 * r.speed * HOLD_S)) fail(`speed ${r.speed}: hero travelled ${r.heroTravel} cells, want at least ${0.7 * r.speed * HOLD_S}`);
  if (!(r.oldTickMaxStep <= r.speed * SIM_TICK_S + EPSILON)) fail(`speed ${r.speed}: tick step ${r.oldTickMaxStep} exceeds one tick of travel`);
  if (!(r.focusMaxStep <= r.speed * FRAME_S + EPSILON)) fail(`speed ${r.speed}: focus step ${r.focusMaxStep} exceeds one frame of hero travel`);
  if (!(r.focusMaxStep < r.oldTickMaxStep)) fail(`speed ${r.speed}: focus step ${r.focusMaxStep} is not below the old tick step ${r.oldTickMaxStep}`);
  if (!(r.lagAtEnd <= 1.05 * r.speed / LOCK_FOLLOW_RATE)) fail(`speed ${r.speed}: focus lag ${r.lagAtEnd} exceeds speed over follow rate`);
  if (r.holdsOnRelease !== 1) fail(`speed ${r.speed}: ${r.holdsOnRelease} holds on release, want 1`);
  if (r.settleS === null || r.settleS > SETTLE_LIMIT_S) fail(`speed ${r.speed}: focus did not settle within ${SETTLE_LIMIT_S}s`);
  console.log(`speed=${r.speed} cells/s orders=${r.orders} maxOrderGap=${r.maxOrderGap.toFixed(4)} heroTravel=${r.heroTravel.toFixed(2)} oldTickMaxStep=${r.oldTickMaxStep.toFixed(4)} focusMaxStep=${r.focusMaxStep.toFixed(4)} focusMeanStep=${r.focusMeanStep.toFixed(4)} lagAtEnd=${r.lagAtEnd.toFixed(4)} settleS=${r.settleS.toFixed(4)}`);
}
console.log(`hero trajectory ok (${results.length} assumed speeds, deterministic 60 Hz loop over a 16 Hz sim)`);

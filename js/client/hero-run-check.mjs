import { createHeroRun, RUN_CELLS, stepHeroRun, TAP_MAX_S } from "./hero_run.js";

const fail = (what) => { throw new Error(what); };
const expectCommands = (result, want, what) => {
  const got = JSON.stringify(result.commands);
  const expected = JSON.stringify(want);
  if (got !== expected) fail(`${what}: got ${got}, want ${expected}`);
};

const FRAME_S = 0.016;
const BIG = { w: 200, h: 200 };
const NORTH = { x: 0, z: -1 };
const EAST = { x: 1, z: 0 };
const WEST = { x: -1, z: 0 };
const SOUTH_EAST = { x: Math.SQRT1_2, z: Math.SQRT1_2 };
const ENEMY = 7;
const NONE = [];
const HOLD = [{ kind: "hold" }];
const orderTo = (x, z) => [{ kind: "order", x, z }];
const hero = { id: 3, x: 100, z: 100 };

const frame = (state, over = {}) => stepHeroRun(state, { hero, direction: null, held: false, dtS: FRAME_S, enemyIds: [], bounds: BIG, ...over });

let state = createHeroRun();
let r = frame(state, { held: true, direction: NORTH });
expectCommands(r, orderTo(100, 100 - RUN_CELLS), "press issues one order to a far point");
state = r.state;
for (let i = 0; i < 30; i++) {
  r = frame(state, { held: true, direction: NORTH });
  expectCommands(r, NONE, "a held key re-issues an order");
  state = r.state;
}
r = frame(state, { held: true, direction: EAST });
expectCommands(r, orderTo(100 + RUN_CELLS, 100), "a direction change issues one order");
state = r.state;
for (let i = 0; i < 10; i++) {
  r = frame(state, { held: true, direction: EAST });
  expectCommands(r, NONE, "holding the new direction re-issues an order");
  state = r.state;
}
r = frame(state, { held: false });
expectCommands(r, HOLD, "a hold press released after a long run issues hold");
state = createHeroRun();

r = frame(state, { held: true, direction: NORTH, dtS: 0.05 });
expectCommands(r, orderTo(100, 100 - RUN_CELLS), "tap issues its order");
r = frame(r.state, { held: true, direction: NORTH, dtS: 0.05 });
expectCommands(r, NONE, "tap re-issues during the press");
r = frame(r.state, { held: false, dtS: 0.05 });
expectCommands(r, NONE, "a short tap issues nothing on release");
if (r.state.run === null) fail("a short tap did not start a run");
state = r.state;
for (let i = 0; i < 20; i++) {
  r = frame(state, { held: false });
  expectCommands(r, NONE, "a tap run re-issues while nothing is in vision");
  state = r.state;
}
if (state.run === null) fail("a tap run ended with no enemy and no arrival");
r = frame(state, { held: false, enemyIds: [ENEMY] });
expectCommands(r, HOLD, "an enemy entering vision does not stop the tap run");
if (r.state.run !== null) fail("an enemy entering vision left the run active");
r = frame(r.state, { held: false, enemyIds: [ENEMY] });
expectCommands(r, NONE, "a stopped run issues again");

state = createHeroRun();
r = frame(state, { held: true, direction: NORTH, enemyIds: [ENEMY] });
r = frame(r.state, { held: false, enemyIds: [ENEMY] });
expectCommands(r, NONE, "an enemy visible at tap start stopped the run");
for (let i = 0; i < 10; i++) {
  r = frame(r.state, { held: false, enemyIds: [ENEMY] });
  expectCommands(r, NONE, "an enemy visible at tap start stopped the run later");
}
if (r.state.run === null) fail("an enemy visible at tap start ended the run");

state = createHeroRun();
r = frame(state, { held: true, direction: NORTH, enemyIds: [] });
r = frame(r.state, { held: true, direction: NORTH, enemyIds: [ENEMY] });
expectCommands(r, NONE, "an enemy entering during the press issued a command");
r = frame(r.state, { held: false, enemyIds: [ENEMY] });
expectCommands(r, NONE, "an enemy entering during the press was not held for the run");
r = frame(r.state, { held: false, enemyIds: [ENEMY] });
expectCommands(r, HOLD, "an enemy entering during the press did not stop the run");

state = createHeroRun();
r = frame(state, { held: true, direction: NORTH, enemyIds: [ENEMY] });
r = frame(r.state, { held: false, enemyIds: [ENEMY] });
r = frame(r.state, { held: false, enemyIds: [] });
expectCommands(r, NONE, "an enemy leaving vision issued a command");
r = frame(r.state, { held: false, enemyIds: [ENEMY] });
expectCommands(r, HOLD, "an enemy returning to vision did not stop the run");

state = createHeroRun();
r = frame(state, { held: true, direction: NORTH, dtS: 0.1 });
expectCommands(r, orderTo(100, 100 - RUN_CELLS), "long hold issues its order");
state = r.state;
for (let i = 0; i < 2; i++) {
  r = frame(state, { held: true, direction: NORTH, dtS: 0.1, enemyIds: [ENEMY] });
  expectCommands(r, NONE, "a hold run stopped for an enemy");
  state = r.state;
}
if (!(state.pressedS >= TAP_MAX_S)) fail("the long hold did not reach the tap limit");
r = frame(state, { held: false, enemyIds: [ENEMY] });
expectCommands(r, HOLD, "a long hold did not stop on release");
if (r.state.run !== null) fail("a long hold left a run behind");
r = frame(r.state, { held: false });
expectCommands(r, NONE, "a long hold issued hold twice");

state = createHeroRun();
r = frame(state, { held: true, direction: null, dtS: 0.1 });
expectCommands(r, NONE, "a press with no direction issued an order");
for (let i = 0; i < 4; i++) {
  r = frame(r.state, { held: true, direction: null, dtS: 0.1 });
  expectCommands(r, NONE, "opposite keys issued an order");
}
r = frame(r.state, { held: true, direction: NORTH, dtS: 0.1 });
expectCommands(r, orderTo(100, 100 - RUN_CELLS), "a direction appearing mid-press issued no order");
r = frame(r.state, { held: false, dtS: 0.1 });
expectCommands(r, HOLD, "a long press with a direction did not stop on release");
state = createHeroRun();
r = frame(state, { held: true, direction: null, dtS: 0.5 });
r = frame(r.state, { held: false, dtS: 0.5 });
expectCommands(r, NONE, "a press with no direction issued hold on release");

state = createHeroRun();
r = frame(state, { held: true, direction: NORTH });
r = frame(r.state, { held: false });
const runningState = r.state;
const idle = stepHeroRun(runningState, { hero: null, direction: null, held: false, dtS: FRAME_S, enemyIds: [ENEMY], bounds: BIG });
expectCommands(idle, NONE, "a null hero emitted a command");
if (JSON.stringify(idle.state) !== JSON.stringify(createHeroRun())) fail("a null hero did not reset the state");
const pressing = frame(createHeroRun(), { held: true, direction: NORTH });
const pressedWithoutHero = stepHeroRun(pressing.state, { hero: null, direction: NORTH, held: true, dtS: FRAME_S, enemyIds: [], bounds: BIG });
expectCommands(pressedWithoutHero, NONE, "a null hero emitted a command while held");
if (pressedWithoutHero.state.pressing) fail("a null hero kept the press");

const small = { w: 32, h: 24 };
const edgeHero = (x, z) => ({ id: 4, x, z });
const edgeFrame = (at, direction) => stepHeroRun(createHeroRun(), { hero: edgeHero(...at), direction, held: true, dtS: FRAME_S, enemyIds: [], bounds: small });
expectCommands(edgeFrame([16, 12], NORTH), orderTo(16, 0), "order north was not clamped to the top edge");
expectCommands(edgeFrame([16, 12], EAST), orderTo(32, 12), "order east was not clamped to the right edge");
expectCommands(edgeFrame([2, 12], WEST), orderTo(0, 12), "order west was not clamped to the left edge");
expectCommands(edgeFrame([30, 22], SOUTH_EAST), orderTo(32, 24), "diagonal order was not clamped to the corner");

state = createHeroRun();
r = frame(state, { held: true, direction: NORTH, dtS: 0.05 });
r = frame(r.state, { held: false, dtS: 0.05 });
const tapped = r.state;
if (tapped.run === null) fail("tap did not start a run before the arrival check");
r = frame(tapped, { hero: { id: 3, x: 100, z: 36.5 }, enemyIds: [ENEMY] });
expectCommands(r, NONE, "arrival issued a command");
if (r.state.run !== null) fail("arrival did not end the run");

r = frame(tapped, { held: true, direction: EAST, enemyIds: [ENEMY] });
expectCommands(r, orderTo(100 + RUN_CELLS, 100), "a new press during a tap run did not issue its order alone");
if (r.state.run !== null) fail("a new press did not end the tap run");

console.log("hero run ok");

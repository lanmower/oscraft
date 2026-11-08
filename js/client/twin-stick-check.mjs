import { AIM_CELLS, PAD_CAST_SLOTS, STICK_DEADZONE, aimPoint, pressedDownEdges, twinStick } from "./twin_stick.js";

const fail = (what) => { throw new Error(what); };
const near = (a, b) => Math.abs(a - b) < 1e-9;
const sameStick = (got, want, what) => {
  const matches = want === null ? got === null : got !== null && near(got.x, want.x) && near(got.y, want.y);
  if (!matches) fail(`${what}: got ${JSON.stringify(got)}, want ${JSON.stringify(want)}`);
};
const samePoint = (got, want, what) => {
  if (!near(got.x, want.x) || !near(got.z, want.z)) fail(`${what}: got ${JSON.stringify(got)}, want ${JSON.stringify(want)}`);
};
const pad = (axes, buttons = [], connected = true) => ({ connected, axes, buttons });

if (STICK_DEADZONE !== 0.2) fail(`the deadzone is ${STICK_DEADZONE}, the spec says 0.2`);
if (AIM_CELLS !== 8) fail(`the aim distance is ${AIM_CELLS} cells, the spec says 8`);
if (PAD_CAST_SLOTS.join(",") !== "0,1,2,3") fail("the pad cast buttons are not 0..3");

const none = twinStick([]);
if (none.move !== null || none.aim !== null || none.pressed.length !== 0) fail("no pad reads as no stick and no buttons");
const nullPads = twinStick(null);
if (nullPads.move !== null || nullPads.aim !== null) fail("a null pad list reads as no pad");

sameStick(twinStick([pad([0.1, -0.1, 0, 0])]).move, null, "inside the deadzone the left stick is idle");
sameStick(twinStick([pad([0.2, 0, 0, 0])]).move, null, "exactly at the deadzone edge is idle");
sameStick(twinStick([pad([0.6, 0, 0, 0])]).move, { x: 0.5, y: 0 }, "0.6 scales to half after the deadzone");
sameStick(twinStick([pad([-1, 0, 0, 0])]).move, { x: -1, y: 0 }, "full left is -1");
sameStick(twinStick([pad([0, -1, 0, 0])]).move, { x: 0, y: 1 }, "stick pushed up (negative axis) is forward (+y)");
sameStick(twinStick([pad([0, 0, 0, 1])]).aim, { x: 0, y: -1 }, "right stick pushed down is aim back (-y)");
sameStick(twinStick([pad([0, 0, 0, 0])]).aim, null, "a centred right stick aims nothing");

const chosen = twinStick([null, pad([-1, 0, 0, 0], [], false), pad([1, 0, 0, 0])]);
sameStick(chosen.move, { x: 1, y: 0 }, "the first connected pad is used, disconnected pads are skipped");

const pressed = twinStick([pad([0, 0, 0, 0], [{ pressed: true }, null, { pressed: false }, { value: 0.9 }])]).pressed;
if (JSON.stringify(pressed) !== JSON.stringify([true, false, false, false])) fail(`button pressed flags: ${JSON.stringify(pressed)}`);

const downs = (previous, now) => JSON.stringify(pressedDownEdges(previous, now));
if (downs([false, false, false], [true, true, false]) !== "[0,1]") fail("two new presses give two down edges");
if (downs([true, false], [true, false]) !== "[]") fail("a held button does not repeat its down edge");
if (downs([true], [false]) !== "[]") fail("a release is not a down edge");
if (downs([false], [true]) !== "[0]") fail("a press from rest is a down edge");

const hero = { x: 10, z: 20 };
samePoint(aimPoint(hero, null, 0), { x: 10, z: 20 }, "no aim stays on the hero");
samePoint(aimPoint(hero, { x: 1, y: 0 }, 0), { x: 10 + AIM_CELLS, z: 20 }, "aim right at yaw 0");
samePoint(aimPoint(hero, { x: 0, y: 1 }, 0), { x: 10, z: 20 - AIM_CELLS }, "aim forward at yaw 0");
samePoint(aimPoint(hero, { x: 0, y: 1 }, Math.PI / 2), { x: 10 - AIM_CELLS, z: 20 }, "aim forward at yaw 90 follows the camera");

console.log("twin stick checks ok (deadzone, scaling, y sign, pad choice, button edges, aim point)");

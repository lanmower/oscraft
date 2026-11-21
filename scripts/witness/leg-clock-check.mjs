import { expect } from "../../js/client/check-expect.mjs";
import fs from "node:fs";
import vm from "node:vm";

const src = fs.readFileSync(new URL("./leg-clock.js", import.meta.url), "utf8");
const load = () => {
  const sandbox = {};
  sandbox.window = sandbox;
  vm.runInNewContext(src, sandbox);
  return sandbox.window.legClock;
};
const endTick = (clock, start, legTicks, cap, counts) => {
  const c = clock.create(start, legTicks, cap);
  let t = start;
  while (c.advance(t, counts(t))) {
    t += 1;
    if (t > start + 100000) throw new Error("the leg clock never ends");
  }
  return t;
};

const LEG = 640;
const CAP = LEG * 3;
const START = 100;
const clock = load();

const plain = endTick(clock, START, LEG, CAP, () => true);
expect(plain === START + LEG, `a leg with a live creep the whole time ends after legTicks: got ${plain - START}`);

const pendingFrom = 300;
const pendingTicks = 100;
const withPick = endTick(clock, START, LEG, CAP, (t) => !(t >= pendingFrom && t < pendingFrom + pendingTicks));
expect(Math.abs(withPick - (START + LEG + pendingTicks)) <= 1, `a re-pick of ${pendingTicks} ticks extends the leg by that much: ended after ${withPick - START} ticks`);

const idle = endTick(clock, START, LEG, CAP, () => false);
expect(idle === START + CAP, `a leg with no live creep stops at the cap: got ${idle - START}`);

console.log("leg clock ok (3 cases)");

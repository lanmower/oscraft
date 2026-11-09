import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";

const context = {};
runInNewContext(readFileSync(new URL("./clear-target.js", import.meta.url), "utf8"), context);
const { pick, place, choose, blockersOf, stopPointOf } = context.clearTarget;

const HERO = 1;
const unit = (id, x, z, player = 255, type = 46) => ({ id, x, z, player, type });
const hero = unit(HERO, 0, 0, 0, 1);
const rule = (overrides = {}) => ({
  isolationCells: 4,
  reachCells: 2,
  laneCells: 10,
  targetOwner: 255,
  excluded: [],
  placeTarget: true,
  placeRadii: [8, 12, 16],
  placeStepDeg: 15,
  alive: () => true,
  hostile: () => true,
  ...overrides,
});

const cases = [
  {
    name: "the nearest lone creep is picked",
    units: [hero, unit(10, 10, 0), unit(30, 30, 0)],
    expect: (r) => assert.equal(r.creep.id, 10),
  },
  {
    name: "a creep whose camp-mate sits nearer its stop point is blocked; the next clear creep is picked",
    units: [hero, unit(10, 10, 0), unit(11, 9, 0), unit(20, 0, 20)],
    expect: (r) => assert.equal(r.creep.id, 20),
  },
  {
    name: "a camp-mate three cells beyond the candidate is not on the approach and does not block it",
    units: [hero, unit(10, 10, 0), unit(13, 13, 0)],
    expect: (r) => assert.equal(r.creep.id, 10),
  },
  {
    name: "a creep on the approach corridor blocks the farther candidate, so the nearer clear creep is picked",
    units: [hero, unit(10, 10, 0), unit(5, 5, 0)],
    expect: (r) => assert.equal(r.creep.id, 5),
  },
  {
    name: "a living non-neutral unit on the approach blocks the only candidate",
    units: [hero, unit(10, 10, 0), unit(50, 5, 1, 0, 1)],
    expect: (r) => assert.equal(r.creep, null),
  },
  {
    name: "a refusal names the nearest candidate and each blocker by id, player, type and cause",
    units: [hero, unit(10, 10, 0), unit(11, 9, 0)],
    expect: (r) => {
      assert.equal(r.creep, null);
      assert.match(r.aborted, /^no clear creep: all 2 candidates are blocked; nearest 11 is blocked by 10 \(player 255, 46, on the approach,/);
    },
  },
  {
    name: "a dead hostile does not block",
    units: [hero, unit(10, 10, 0), { ...unit(11, 9, 0), dead: true }],
    alive: (u) => !u.dead,
    expect: (r) => assert.equal(r.creep.id, 10),
  },
];

for (const c of cases) {
  const r = pick(rule({ alive: c.alive ?? (() => true) }), c.units, hero, HERO);
  c.expect(r);
}

{
  const stop = stopPointOf(hero, unit(10, 10, 0), 2);
  assert.ok(Math.abs(stop.x - 8) < 1e-9 && Math.abs(stop.z) < 1e-9, "the stop point sits reachCells from the candidate on the hero's side");
  assert.equal(blockersOf(rule(), [hero, unit(10, 10, 0)], unit(10, 10, 0), hero, HERO).length, 0);
}

{
  const units = [hero, unit(10, 10, 0), unit(11, 9, 0)];
  const placed = place(rule(), units, hero, HERO);
  assert.equal(placed.aborted, null);
  assert.equal(placed.kind, 46, "a placed creep takes the kind of the existing neutral creeps");
  assert.ok(Math.hypot(placed.point.x - hero.x, placed.point.z - hero.z) >= 8, "the point is at least the smallest placement radius from the hero");
}

{
  const lane = unit(90, 14, 0, 0, 1);
  const units = [hero, unit(10, 10, 0), lane];
  const placed = place(rule({ placeRadii: [8] }), units, hero, HERO);
  assert.notEqual(placed.point, null, "a point exists at 8 cells that keeps 10 cells from the lane unit");
  const d = Math.hypot(lane.x - placed.point.x, lane.z - placed.point.z);
  assert.ok(d > 10, `a placed point keeps 10 cells from a non-neutral hostile, got ${d.toFixed(2)}`);
}

{
  const units = [hero, unit(10, 10, 0), unit(11, 9, 0)];
  const extra = [];
  const spawn = (kind, x, z) => {
    extra.push(unit(300, x, z, 255, kind));
    return 300;
  };
  const r = choose(rule(), () => [...units, ...extra], hero, HERO, spawn);
  assert.equal(extra.length, 1, "with no clear creep, one creep is placed");
  assert.equal(r.creep.id, 300, "the placed creep is the one picked");
  assert.equal(r.aborted, null);
}

{
  const units = [hero, unit(10, 10, 0)];
  const extra = [];
  const spawn = (kind, x, z) => {
    extra.push(unit(301, x, z, 255, kind));
    return 301;
  };
  const r = choose(rule(), () => [...units, ...extra], hero, HERO, spawn);
  assert.equal(r.creep.id, 10, "a clear creep is chosen before any placement");
  assert.equal(extra.length, 0, "no creep is placed when a clear one exists");
}

{
  const units = [hero, unit(10, 10, 0), unit(11, 9, 0)];
  const spawn = () => {
    throw new Error("placement must not spawn when placement is off");
  };
  const r = choose(rule({ placeTarget: false }), () => units, hero, HERO, spawn);
  assert.match(r.aborted, /^no clear creep/, "with placement off, the refusal stands");
}

console.log(`clear-target checks ok (${cases.length + 5} cases)`);

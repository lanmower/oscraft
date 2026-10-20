// Node check for the MOBA selection logic and its bindings: run with `node js/client/moba_select_check.mjs`.
// Synthetic units carry screen positions; heroes are a set of ids. Exits 1 on the first failed assertion.
import assert from "node:assert/strict";
import UI from "./moba_ui.json" with { type: "json" };
import {
  boxSelect, clickSelect, ownIds, ControlGroups, resolveKey, cooldownTicks, formationPoints,
  mergeConfig, fill, teamOf, CONTROL_ACTIONS, targetSpec, refusalText,
} from "./moba_select.js";

const ME = 0;
const HEROES = new Set([1]);
const isHero = (id) => HEROES.has(id);
const units = [
  { id: 1, player: ME, sx: 100, sy: 100 },  // own hero
  { id: 2, player: ME, sx: 110, sy: 105 },  // own creep, inside the box with the hero
  { id: 3, player: ME, sx: 500, sy: 500 },  // own creep, far away
  { id: 4, player: 1, sx: 105, sy: 102 },   // enemy hero, inside the box
  { id: 5, player: ME, sx: 120, sy: 300 },  // own creep, outside the box
];
const box = { x0: 90, y0: 90, x1: 130, y1: 120 };
const sel = (current, extra) => boxSelect(units, box, { localPlayer: ME, isHero, current, ...extra });

const checks = [];
const check = (name, fn) => checks.push({ name, fn });

check("box selects own units inside the box only", () => {
  const far = { x0: 480, y0: 480, x1: 520, y1: 520 };
  assert.deepEqual(boxSelect(units, far, { localPlayer: ME, isHero }), [3]);
});

check("box with an own hero keeps heroes and drops creeps", () => {
  assert.deepEqual(sel([]), [1]);
});

check("box with no own hero keeps the creeps", () => {
  const creepsOnly = { x0: 0, y0: 290, x1: 200, y1: 320 };
  assert.deepEqual(boxSelect(units, creepsOnly, { localPlayer: ME, isHero }), [5]);
});

check("box never picks enemy units", () => {
  const enemyOnly = { x0: 103, y0: 100, x1: 108, y1: 104 };
  assert.deepEqual(boxSelect(units, enemyOnly, { localPlayer: ME, isHero }), []);
});

check("box is order-independent (drag up-left works)", () => {
  const backwards = { x0: 130, y0: 120, x1: 90, y1: 90 };
  assert.deepEqual(boxSelect(units, backwards, { localPlayer: ME, isHero }), [1]);
});

check("shift box adds to the current selection", () => {
  assert.deepEqual(sel([5]), [1]);
  assert.deepEqual(sel([5], { additive: true }), [5, 1]);
});

check("click selects one unit, shift toggles it", () => {
  assert.deepEqual(clickSelect(2, { current: [1] }), [2]);
  assert.deepEqual(clickSelect(2, { current: [1], additive: true }), [1, 2]);
  assert.deepEqual(clickSelect(1, { current: [1, 2], additive: true }), [2]);
  assert.deepEqual(clickSelect(-1, { current: [1] }), []);
});

check("ownIds drops enemy and dead ids", () => {
  assert.deepEqual(ownIds([1, 4, 9, 3], units, ME), [1, 3]);
});

check("formation points spread a group around the target", () => {
  const pts = formationPoints(4, 10, 10);
  assert.equal(pts.length, 4);
  assert.equal(new Set(pts.map((p) => `${p.x.toFixed(2)},${p.z.toFixed(2)}`)).size, 4);
});

check("control groups set, recall and centre on a double tap", () => {
  const groups = new ControlGroups(350);
  groups.set(0, [1, 2, 2]);
  const first = groups.recall(0, 1000);
  assert.deepEqual(first.ids, [1, 2]);
  assert.equal(first.centre, false);
  assert.equal(groups.recall(0, 1200).centre, true);
  assert.equal(groups.recall(0, 2000).centre, false);
  assert.deepEqual(groups.recall(5, 0).ids, []);
});

check("keys resolve to actions: plain, ctrl and custom", () => {
  const keys = UI.keys;
  assert.deepEqual(resolveKey(keys, "a"), { action: "attackMove" });
  assert.deepEqual(resolveKey(keys, "S"), { action: "stop" });
  assert.deepEqual(resolveKey(keys, "h"), { action: "hold" });
  assert.deepEqual(resolveKey(keys, "Tab"), { action: "scoreboard" });
  assert.deepEqual(resolveKey(keys, "p"), { action: "shop" });
  assert.deepEqual(resolveKey(keys, "b"), { action: "item", slot: 4 });
  assert.deepEqual(resolveKey(keys, "z"), { action: "item", slot: 0 });
  assert.deepEqual(resolveKey(keys, "n"), { action: "item", slot: 5 });
  assert.deepEqual(resolveKey(keys, " "), { action: "centre" });
  assert.deepEqual(resolveKey(keys, "q"), { action: "cast", slot: 0 });
  assert.deepEqual(resolveKey(keys, "q", { ctrl: true }), { action: "skillUp", slot: 0 });
  assert.deepEqual(resolveKey(keys, "3"), { action: "recallGroup", slot: 2 });
  assert.deepEqual(resolveKey(keys, "3", { ctrl: true }), { action: "setGroup", slot: 2 });
  assert.deepEqual(resolveKey(keys, "x"), { action: "item", slot: 1 });
  assert.equal(resolveKey(keys, "g"), null);
  const custom = { ...keys, custom: { f: "recall_hero" } };
  assert.deepEqual(resolveKey(custom, "F"), { action: "custom", name: "recall_hero" });
});

check("cooldown ticks read the held rank, cooldown alone for flat abilities", () => {
  assert.equal(cooldownTicks({ max_rank: 3, cooldowns: [50, 40, 30] }, 2), 40);
  assert.equal(cooldownTicks({ max_rank: 3, cooldowns: [50, 40, 30] }, 0), 50);
  assert.equal(cooldownTicks({ max_rank: 0, cooldown: 70 }, 0), 70);
  assert.equal(cooldownTicks(null, 0), 0);
});

check("every control has a binding", () => {
  const bound = {
    attackMove: UI.keys.attackMove,
    stop: UI.keys.stop,
    hold: UI.keys.hold,
    centre: UI.keys.centre,
    item: UI.keys.item,
    scoreboard: UI.keys.scoreboard,
    shop: UI.keys.shop,
    cast: UI.keys.cast,
    skillUp: UI.keys.skillUp.keys,
    group: UI.keys.group.keys,
  };
  for (const action of CONTROL_ACTIONS) {
    const value = bound[action];
    assert.ok(value && (Array.isArray(value) ? value.length : Object.keys(value).length), `unbound control: ${action}`);
  }
  assert.equal(UI.keys.group.keys.length, 9, "control groups need keys 1..9");
});

check("targeted entries arm, plain entries cast on press; refusals read as words", () => {
  assert.deepEqual(targetSpec({ target: "point", radius: 4 }), { targetKind: "point", radius: 4 });
  assert.deepEqual(targetSpec({ target: "unit" }), { targetKind: "unit", radius: 0 });
  assert.deepEqual(targetSpec({ target: "none" }), { targetKind: "none", radius: 0 });
  assert.deepEqual(targetSpec({}), { targetKind: "none", radius: 0 });
  assert.equal(refusalText("cast_refused_cooldown"), "on cooldown");
  assert.equal(refusalText("NotEnoughMana"), "no mana");
  assert.equal(refusalText("OutOfRange"), "out of range");
  assert.equal(refusalText("cast_dropped no_hero_selected slot=0"), "select a hero first");
});

check("config merges and text fills", () => {
  const merged = mergeConfig(UI, { doubleTapMs: 200, keys: { shop: ["o"] } });
  assert.equal(merged.doubleTapMs, 200);
  assert.deepEqual(merged.keys.shop, ["o"]);
  assert.deepEqual(merged.keys.cast, UI.keys.cast);
  assert.equal(fill("hp {hp} gold {gold}", { hp: 5 }), "hp 5 gold ");
  assert.equal(teamOf(UI.teams, 7), 1);
  assert.equal(teamOf(UI.teams, 42), -1);
});

let failed = 0;
for (const { name, fn } of checks) {
  try {
    fn();
    console.log(`ok   ${name}`);
  } catch (err) {
    failed += 1;
    console.log(`FAIL ${name}: ${err.message}`);
  }
}
console.log(failed === 0 ? `PASS ${checks.length} checks` : `FAIL ${failed} of ${checks.length}`);
process.exit(failed === 0 ? 0 : 1);

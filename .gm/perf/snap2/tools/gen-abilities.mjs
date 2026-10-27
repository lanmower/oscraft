// Generates scenarios/moba/abilities_ruleset.osd from reference per-hero ability definitions and the recipes in
// scenarios/moba/tools/ability-recipes.mjs. Every number (cooldown, mana, cast range, cast point, damage, durations, radii, per-level
// tables, talent, scepter and shard deltas) is read from the reference file; a recipe only says which engine effect carries which
// reference key. Source: SteamDatabase tracking repo @282fad29f0f74504fad465c52d197fb61f4d612b
// source/scripts/npc/heroes/hero_*.txt (AbilityDefinitions) and npc/npc_abilities.txt (generic talents).
// Usage: node scenarios/moba/tools/gen-abilities.mjs [--src dir] [--out scenarios/moba]
// Scale (shared with scenarios/moba/tools/gen-heroes.mjs): one reference unit is 15.625 milli, 16 ticks per second.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { HEROES, HELPERS, RECIPES, SUMMON_UNITS, H } from "./ability-recipes.mjs";

const COMMIT = "282fad29f0f74504fad465c52d197fb61f4d612b";
const RAW = `https://raw.githubusercontent.com/SteamDatabase/GameTracking-Dota2/${COMMIT}/game/dota/pak01_dir/scripts/npc/`;
const args = process.argv.slice(2);
const opt = (name, dflt) => (args.includes(name) ? args[args.indexOf(name) + 1] : dflt);
const outDir = opt("--out", "scenarios/moba");
const dir = opt("--src", path.join(os.tmpdir(), "ref-heroes-src"));

function parseKV(text) {
  const tok = [];
  const re = /\/\/[^\n]*|"((?:[^"\\]|\\.)*)"|([{}])|#base\s+"[^"]*"/g;
  let m;
  while ((m = re.exec(text))) {
    if (m[1] !== undefined) tok.push(m[1]);
    else if (m[2]) tok.push(m[2]);
  }
  let i = 0;
  function block() {
    const o = {};
    while (i < tok.length && tok[i] !== "}") {
      const k = tok[i++];
      if (tok[i] === "{") { i++; o[k] = block(); i++; } else o[k] = tok[i++];
    }
    return o;
  }
  const root = {};
  while (i < tok.length) {
    const k = tok[i++];
    if (tok[i] === "{") { i++; root[k] = block(); i++; }
  }
  return root;
}

async function fetchTo(file, url) {
  if (!fs.existsSync(file)) fs.writeFileSync(file, Buffer.from(await (await fetch(url)).arrayBuffer()));
}
fs.mkdirSync(dir, { recursive: true });
await fetchTo(path.join(dir, "npc_abilities.txt"), RAW + "npc_abilities.txt");
for (const hero of Object.keys(HEROES)) await fetchTo(path.join(dir, `npc_dota_hero_${hero}.txt`), `${RAW}heroes/npc_dota_hero_${hero}.txt`);

const generic = Object.values(parseKV(fs.readFileSync(path.join(dir, "npc_abilities.txt"), "utf8")))[0];
const MILLI = 15.625;
const TPS = 16;
const round = (x) => (x < 0 ? -Math.round(-x) : Math.round(x));
const levels = (s) => String(s).trim().split(/\s+/).filter((t) => t !== "").map(Number);
const META = new Set(["affected_by_aoe_increase", "CalculateSpellDamageTooltip", "DamageTypeTooltip", "display_type", "dynamic_value", "RequiresShard", "RequiresScepter", "hero_levelup", "levelup_interval", "LinkedSpecialBonus", "LinkedSpecialBonusField", "LinkedSpecialBonusOperation", "value"]);

// Value conversions from reference units to engine units. d() is the change of the engine value for a reference delta.
export const CONV = {
  int: (x) => x,
  dist: (x) => x * MILLI,
  time: (x) => x * TPS,
  speed: (x) => (x * MILLI) / TPS,
  neg: (x) => -x,
  hundredths: (x) => x * 100,
  slowmul: (x) => 65536 * (1 + x / 100),
  slowpos: (x) => 65536 * (1 - x / 100),
};
const delta = (conv, x) => CONV[conv](x) - CONV[conv](0);

// A reference key read from AbilityValues (or the ability root): per-level numbers and the modifiers keyed by talent, scepter, shard.
function readKey(def, key) {
  let raw = def.AbilityValues?.[key];
  if (raw === undefined) raw = def[key];
  if (raw === undefined) return null;
  if (typeof raw === "string") return { base: levels(raw), mods: {} };
  let value = raw.value;
  while (value && typeof value === "object") value = value.value;
  const mods = {};
  for (const [k, v] of Object.entries(raw)) if (k.startsWith("special_bonus_") && typeof v === "string") mods[k] = v;
  return { base: value === undefined ? [] : levels(value), mods };
}

// Modifier text to {mode 0 add | 1 percent, amount} or null when the engine cannot carry it.
function readMod(text, hasBase) {
  const t = text.trim();
  const m = /^([+-]?)(\d+(?:\.\d+)?)(%?)$/.exec(t);
  if (!m) return null;
  if (m[1] === "" && hasBase) return null;
  const amount = Number(m[2]) * (m[1] === "-" ? -1 : 1);
  return { mode: m[3] ? 1 : 0, amount };
}

const out = [];
const notes = [];
const usedConds = new Set();
const emit = (s) => out.push(s);
const q = (s) => JSON.stringify(s);

function condOf(name, talents) {
  if (name === "special_bonus_scepter") return 0;
  if (name === "special_bonus_shard") return 1;
  if (name.startsWith("special_bonus_facet")) return -1;
  const k = talents.indexOf(name);
  return k < 0 ? -2 : 2 + k;
}

// Resolves a parameter specification to per-rank engine values and the modifiers that apply to it.
function resolveParam(p, ctx) {
  if (typeof p === "number") return { ranks: [p], mods: [] };
  if (Array.isArray(p)) return { ranks: p, mods: [] };
  const rk = readKey(ctx.def, p.v);
  if (!rk) throw new Error(`${ctx.name}: Valve key ${p.v} not found`);
  ctx.used.add(p.v);
  const conv = p.conv ?? "int";
  const mul = p.mul ?? 1;
  const fn = p.fn ?? ((x) => x);
  const ranks = (rk.base.length ? rk.base : [0]).map((x) => round(fn(CONV[conv](x)) * mul) + (p.add ?? 0));
  const mods = [];
  for (const [name, text] of Object.entries(rk.mods)) {
    const cond = condOf(name, ctx.talents);
    if (cond === -1) continue;
    const mod = readMod(text, rk.base.length > 0);
    if (cond === -2 || !mod) { notes.push(`${ctx.name}: ${p.v} ${name} ${text} not modelled`); continue; }
    const d = mod.mode === 1 ? mod.amount : round(delta(conv, mod.amount) * mul);
    mods.push({ cond, mode: mod.mode, delta: d });
  }
  return { ranks, mods };
}

function headerField(def, key, conv, ctx) {
  const rk = readKey(def, key);
  if (!rk) return null;
  const ranks = rk.base.map((x) => round(CONV[conv](x)));
  const mods = [];
  for (const [name, text] of Object.entries(rk.mods)) {
    const cond = condOf(name, ctx.talents);
    if (cond === -1) continue;
    const mod = readMod(text, true);
    if (cond === -2 || !mod) { notes.push(`${ctx.name}: ${key} ${name} ${text} not modelled`); continue; }
    mods.push({ cond, mode: mod.mode, delta: mod.mode === 1 ? mod.amount : round(delta(conv, mod.amount)) });
  }
  return { ranks, mods };
}

const expand = (arr, n) => Array.from({ length: n }, (_, k) => arr[Math.min(k, arr.length - 1)]);
const distinct = (arr) => arr.some((v) => v !== arr[0]);
const TYPE = { legacy: 0, physical: 1, magical: 2, pure: 3 };
const TARGETS = { none: "none", unit: "unit", point: "point" };

function flatten(effects, list = []) {
  for (const e of effects) {
    list.push(e);
    if (e.nested) flatten(e.nested, list);
  }
  return list;
}

function emitAbility(name, def, spec, talents) {
  const ctx = { name, def, used: new Set(), talents };
  const ultimate = def.AbilityType === "DOTA_ABILITY_TYPE_ULTIMATE" || def.AbilityType === "ABILITY_TYPE_ULTIMATE";
  const learnable = !spec.noSkill;
  const maxLevel = learnable ? Math.min(Number(def.MaxLevel) || (ultimate ? 3 : 4), ultimate ? 3 : 4) : 1;
  const R = spec.ranks ?? maxLevel;
  const none = { ranks: [0], mods: [] };
  const cd = spec.helper ? none : headerField(def, "AbilityCooldown", "time", ctx) ?? none;
  const mana = spec.helper ? none : headerField(def, "AbilityManaCost", "int", ctx) ?? none;
  const range = spec.helper || spec.noRange ? null : headerField(def, "AbilityCastRange", "dist", ctx);
  const point = spec.helper ? null : readKey(def, "AbilityCastPoint");
  const chan = spec.helper ? null : readKey(def, "AbilityChannelTime");
  const cooldown = expand(cd.ranks.length ? cd.ranks : [0], R);
  const energy = expand(mana.ranks.length ? mana.ranks : [0], R);
  const castRange = range && range.ranks.length ? expand(range.ranks, R) : [0];
  const castPoint = point ? round(point.base[0] * TPS) : 0;
  const channel = chan ? round(chan.base[0] * TPS) : 0;
  for (const k of ["AbilityCooldown", "AbilityManaCost", "AbilityCastRange", "AbilityCastPoint", "AbilityChannelTime"]) ctx.used.add(k);

  const flat = flatten(spec.effects);
  const lines = [];
  const scaled = [];
  const mods = [];
  const effLines = flat.map((e, idx) => {
    const ps = e.p.map((p) => resolveParam(p, ctx));
    const vals = ps.map((r) => expand(r.ranks, R));
    ps.forEach((r, j) => {
      if (distinct(vals[j])) scaled.push([idx, j, vals[j]]);
      for (const m of r.mods) mods.push([m.cond, idx, j, m.delta, m.mode]);
    });
    const named = e.unit ? `, unit=${q(e.unit)}` : e.ability ? `, ability=${q(e.ability)}` : "";
    const pp = vals.map((v, j) => `p${j}=${v[0]}`).join(", ");
    return `effect(${q(e.k)}, ${pp}${named})`;
  });
  const target = TARGETS[spec.target ?? "none"];
  const proj = spec.projectile ? Math.round(resolveParam(spec.projectile, ctx).ranks[0]) : 0;
  const filt = spec.filters?.length ? `, target_filters=[${spec.filters.map(q).join(", ")}]` : "";
  lines.push(`ability(${q(name)}, ${cooldown[0]}, ${energy[0]}, ${q(target)}, [${effLines.join(", ")}]${filt}${proj ? `, projectile_speed_milli=${proj}` : ""})`);
  if (learnable && !spec.passiveFree) {
    const gates = ultimate ? [6, 12, 18].slice(0, R) : [1, 3, 5, 7].slice(0, R);
    lines.push(`skill(${q(name)}, ultimate=${ultimate ? "True" : "False"}, gates=[${gates}], damage=[${expand([0], R)}], cooldown=[${cooldown}], energy=[${energy}])`);
  }
  if (channel > 0) lines.push(`channel(${q(name)}, ${channel})`);
  const ext = [];
  const x = spec.ext ?? {};
  const rangeScaled = distinct(castRange);
  if (castRange[0] > 0) ext.push(`cast_range=${castRange[0]}`);
  if (castPoint > 0) ext.push(`cast_point=${castPoint}`);
  for (const key of ["period", "chance", "scatter_min", "scatter_max", "charges", "charge_restore", "break_range", "line_extra"]) {
    if (x[key] === undefined) continue;
    const r = resolveParam(x[key], ctx);
    const v = expand(r.ranks, R);
    ext.push(`${key}=${v[0]}`);
    if (distinct(v) && (key === "period" || key === "chance")) scaled.push([key === "period" ? 253 : 254, 0, v]);
    if (key === "charge_restore") for (const m of r.mods) mods.push([m.cond, 254, 0, m.delta, m.mode]);
  }
  for (const flag of spec.flags ?? []) ext.push(`${flag}=True`);
  if (ext.length) lines.push(`ability_ext(${q(name)}, ${ext.join(", ")})`);
  if (rangeScaled) scaled.push([252, 0, castRange]);
  for (const [e, p, v] of scaled) lines.push(`ability_scaled(${q(name)}, ${e}, ${p}, [${v}])`);
  for (const m of cd.mods) mods.push([m.cond, 250, 0, m.delta, m.mode]);
  for (const m of mana.mods) mods.push([m.cond, 251, 0, m.delta, m.mode]);
  if (range) for (const m of range.mods) mods.push([m.cond, 252, 0, m.delta, m.mode]);
  for (const [cond] of mods) usedConds.add(cond);
  for (const [cond, e, p, d, mode] of mods) lines.push(`ability_mod(${q(name)}, ${cond}, ${e}, ${p}, ${d}${mode ? ", mode=1" : ""})`);

  const unmodelled = Object.keys(def.AbilityValues ?? {}).filter((k) => !ctx.used.has(k) && !(spec.ignore ?? []).includes(k) && !META.has(k));
  if (unmodelled.length) lines.unshift(`# ${name} unmodelled reference keys: ${unmodelled.join(", ")}`);
  if (spec.note) lines.unshift(`# ${name}: ${spec.note}`);
  lines.forEach(emit);
}

// Generic talents (npc_abilities.txt special_bonus_*) as stat modifiers on a passive per-hero ability.
const GENERIC = [
  [/^special_bonus_hp_\d+$/, 2, "int"],
  [/^special_bonus_mp_\d+$/, 15, "int"],
  [/^special_bonus_mp_regen_\d+$/, 13, "hundredths"],
  [/^special_bonus_hp_regen_\d+$/, 14, "hundredths"],
  [/^special_bonus_strength_\d+$/, 22, "int"],
  [/^special_bonus_agility_\d+$/, 23, "int"],
  [/^special_bonus_intelligence_\d+$/, 24, "int"],
  [/^special_bonus_all_stats_\d+$/, [22, 23, 24], "int"],
  [/^special_bonus_agility_and_intelligence_\d+$/, [23, 24], "int"],
  [/^special_bonus_armor_\d+$/, 1, "int"],
  [/^special_bonus_attack_base_damage_\d+$/, 0, "int"],
  [/^special_bonus_attack_damage_\d+$/, 0, "int"],
  [/^special_bonus_attack_speed_\d+$/, 12, "int"],
  [/^special_bonus_movement_speed_\d+$/, 3, "speed"],
  [/^special_bonus_movement_speed_percentage_\d+$/, 19, "int"],
  [/^special_bonus_magic_resistance_\d+$/, 11, "int"],
  [/^special_bonus_spell_amplify_\d+$/, 16, "int"],
  [/^special_bonus_cooldown_reduction_\d+$/, 20, "int"],
  [/^special_bonus_evasion_\d+$/, 5, "int"],
  [/^special_bonus_lifesteal_\d+$/, 10, "int"],
  [/^special_bonus_spell_lifesteal_\d+$/, 25, "int"],
  [/^special_bonus_status_resistance_\d+$/, 21, "int"],
  [/^special_bonus_cast_range_\d+$/, 27, "dist"],
  [/^special_bonus_attack_range_\d+$/, 4, "dist"],
];

function emitTalents(hero, talents) {
  const name = `${hero}_talents`;
  const effs = [];
  const mods = [];
  talents.forEach((t, k) => {
    if (!t) return;
    const g = generic[t];
    const rule = GENERIC.find(([re]) => re.test(t));
    if (!g || !rule) return;
    let v = g.AbilityValues?.value;
    while (v && typeof v === "object") v = v.value;
    const amount = round(CONV[rule[2]](Number(v)));
    for (const stat of [].concat(rule[1])) {
      effs.push(`effect("stat", p0=${stat}, p1=0)`);
      usedConds.add(2 + k);
      mods.push(`ability_mod(${q(name)}, ${2 + k}, ${effs.length - 1}, 1, ${amount})`);
    }
  });
  if (!effs.length) return null;
  emit(`# ${name}: generic talents of ${hero} (reference special_bonus_* values) as stat modifiers; unique talents are modifiers on the abilities they change.`);
  emit(`ability(${q(name)}, 100000, 0, "none", [${effs.join(", ")}])`);
  emit(`ability_ext(${q(name)}, passive=True)`);
  mods.forEach(emit);
  return name;
}

emit("# Generated by scenarios/moba/tools/gen-abilities.mjs from reference hero ability definitions (SteamDatabase tracking repo @" + COMMIT + "). Do not edit by hand.");
emit("# Distances are reference units x 15.625 milli, durations seconds x 16 ticks. A line starting with `# name unmodelled reference keys` lists the");
emit("# reference values the engine does not carry for that ability; docs/GAME.md lists what each one changes.");
const assigned = [];
for (const [hero, slots] of Object.entries(HEROES)) {
  const kv = Object.values(parseKV(fs.readFileSync(path.join(dir, `npc_dota_hero_${hero}.txt`), "utf8")))[0];
  const h = Object.values(kv)[0];
  const talents = [10, 11, 12, 13, 14, 15, 16, 17].map((n) => h[`Ability${n}`] ?? null);
  emit(`# ---- ${hero} ----`);
  const names = [];
  for (const slot of slots) {
    if (slot === "@talents") {
      const t = emitTalents(hero, talents);
      if (t) names.push(t);
      continue;
    }
    const def = h.AbilityDefinitions?.[slot];
    if (!def) throw new Error(`${hero}: ability ${slot} not in the Valve file`);
    const recipe = RECIPES[slot];
    if (!recipe) throw new Error(`${hero}: no recipe for ${slot}`);
    emitAbility(slot, def, recipe(H), talents);
    names.push(slot);
    for (const [helper, parent] of Object.entries(HELPERS)) {
      if (parent === slot) emitAbility(helper, def, { ...RECIPES[helper](H), helper: true }, talents);
    }
  }
  const missing = talents.map((t, k) => [t, k]).filter(([t, k]) => t && !usedConds.has(2 + k)).map(([t]) => t);
  if (missing.length) emit(`# ${hero}: talents with no modelled effect: ${missing.join(", ")}`);
  usedConds.clear();
  assigned.push(`hero_abilities(${q(hero)}, [${names.map(q).join(", ")}])`);
}
assigned.forEach(emit);
if (notes.length) {
  emit("# Modifiers the engine cannot carry:");
  [...new Set(notes)].forEach((n) => emit(`#   ${n}`));
}
fs.writeFileSync(path.join(outDir, "abilities_ruleset.osd"), out.join("\n") + "\n");
fs.writeFileSync(path.join(outDir, "units_summons.osd"), ["# Generated by scenarios/moba/tools/gen-abilities.mjs. Do not edit by hand.", ...SUMMON_UNITS, ""].join("\n"));
console.log(`wrote ${path.join(outDir, "abilities_ruleset.osd")}: ${out.length} lines, ${notes.length} unmodelled modifiers`);

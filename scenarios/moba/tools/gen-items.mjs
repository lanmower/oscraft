// Generates scenarios/moba/items.osd from reference items.txt and neutral_items.txt.
// Source of truth: SteamDatabase tracking repo @282fad29f0f74504fad465c52d197fb61f4d612b
// source/scripts/npc/items.txt and neutral_items.txt.
// Usage: node scenarios/moba/tools/gen-items.mjs [--src dir] [--out scenarios/moba/items.osd]
// Without --src both files are downloaded into a temp directory first.
// Scale (same as gen-heroes.mjs and gen-reference-map.mjs): one reference unit is 125/8 = 15.625 milli; speed is milli per
// tick at 16 ticks per second; durations and cooldowns are ticks at 16 per second.
// Cost is reference ItemCost, the full price of the item. A recipe's requirement list gives the components; the
// recipe scroll is not an item here, its price is the difference between the cost and the components' costs.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const COMMIT = "282fad29f0f74504fad465c52d197fb61f4d612b";
const RAW = `https://raw.githubusercontent.com/SteamDatabase/GameTracking-Dota2/${COMMIT}/game/dota/pak01_dir/scripts/npc/`;
const args = process.argv.slice(2);
const opt = (name, dflt) => (args.includes(name) ? args[args.indexOf(name) + 1] : dflt);
const outFile = opt("--out", "scenarios/moba/items.osd");
const MILLI = 15.625;
const TPS = 16;
const unitsToMilli = (v) => Math.round(v * MILLI);
const speedToMilliPerTick = (v) => Math.round((v * MILLI) / TPS);
const ticks = (seconds) => Math.round(seconds * TPS);
const UNLIMITED_STACK = 65535;
const hundredths = (v) => Math.round(v * 100);

function parseKV(text) {
  const tokens = [];
  const re = /\/\/[^\n]*|"((?:[^"\\]|\\.)*)"|([{}])/g;
  let m;
  while ((m = re.exec(text))) {
    if (m[1] !== undefined) tokens.push(m[1]);
    else if (m[2]) tokens.push(m[2]);
  }
  let at = 0;
  const object = () => {
    const o = {};
    while (at < tokens.length && tokens[at] !== "}") {
      const key = tokens[at++];
      if (tokens[at] === "{") {
        at++;
        o[key] = object();
      } else {
        o[key] = tokens[at++];
      }
    }
    at++;
    return o;
  };
  const root = {};
  while (at < tokens.length) {
    const key = tokens[at++];
    at++;
    root[key] = object();
  }
  return root;
}

async function load(name) {
  const dir = opt("--src", null);
  if (dir) return fs.readFileSync(path.join(dir, name), "utf8");
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "items-"));
  const body = await (await fetch(RAW + name)).text();
  fs.writeFileSync(path.join(tmp, name), body);
  return body;
}

const items = parseKV(await load("items.txt")).DOTAAbilities;
const neutral = parseKV(await load("neutral_items.txt")).neutral_items;

const first = (v) => {
  if (v && typeof v === "object") return first(v.value);
  return typeof v === "string" ? v.trim().split(/\s+/)[0] : v;
};
const level = (v, n) => {
  if (v && typeof v === "object") return level(v.value, n);
  const parts = String(v).trim().split(/\s+/);
  return parts[Math.min(n, parts.length) - 1];
};
const num = (v) => {
  const x = Number.parseFloat(first(v));
  return Number.isFinite(x) ? x : 0;
};
const values = (it) => it.AbilityValues ?? {};

// Stat keys of AbilityValues by item stat. `x` scales the value; `all` adds the value to the three attributes.
const STAT_KEYS = [
  ["armor", ["bonus_armor", "armor_bonus", "bonus_physical_armor", "armor"], hundredths],
  ["damage", ["bonus_damage", "bonus_attack_damage", "bonus_damage_melee"]],
  ["health", ["bonus_health", "bonus_hp", "health_bonus", "bonus_max_health"]],
  ["mana", ["bonus_mana", "mana_bonus", "bonus_max_mana"]],
  ["strength", ["bonus_strength", "bonus_str"]],
  ["agility", ["bonus_agility", "bonus_agi"]],
  ["intelligence", ["bonus_intellect", "bonus_int", "bonus_intelligence"]],
  ["move_speed", ["bonus_movement_speed", "bonus_movement", "bonus_speed", "bonus_move_speed", "bonus_movespeed"], speedToMilliPerTick],
  ["attack_speed", ["bonus_attack_speed", "bonus_attack_speed_pct", "attack_speed_bonus"]],
  ["health_regen", ["bonus_health_regen", "bonus_hp_regen", "bonus_regen", "health_regen", "hp_regen"], hundredths],
  ["mana_regen", ["bonus_mana_regen", "bonus_mp_regen", "mana_regen"], hundredths],
  ["magic_resist", ["bonus_magical_armor", "bonus_magic_resistance", "bonus_spell_resist", "bonus_magic_resist", "magic_resistance"]],
  ["evasion", ["bonus_evasion", "evasion"]],
  ["spell_amp", ["bonus_spell_amp", "bonus_spell_damage", "bonus_amp", "spell_amp"]],
  ["lifesteal", ["bonus_lifesteal"]],
  ["spell_lifesteal", ["bonus_spell_lifesteal", "spell_lifesteal"]],
  ["cast_range", ["bonus_cast_range", "cast_range_bonus"], unitsToMilli],
  ["attack_range", ["bonus_attack_range", "attack_range_bonus"], unitsToMilli],
  ["status_resist", ["bonus_status_resist", "bonus_status_resistance", "status_resistance"]],
  ["cooldown_reduction", ["bonus_cooldown"]],
  ["gold_per_min", ["bonus_gpm"]],
  ["xp_per_min", ["bonus_xpm"]],
  ["crit_chance", ["crit_chance"]],
  ["crit_multiplier", ["crit_multiplier"]],
  ["cleave", ["cleave_damage_percent"]],
  ["cleave_distance", ["cleave_distance"], unitsToMilli],
  ["cleave_start_width", ["cleave_starting_width"], unitsToMilli],
  ["cleave_end_width", ["cleave_ending_width"], unitsToMilli],
  ["bash_chance", ["bash_chance_melee", "bash_chance"]],
  ["bash_duration", ["bash_duration"], ticks],
  ["speed_pct", ["movement_speed_percent_bonus", "bonus_movement_speed_pct", "bonus_movement_speed_percent", "movement_speed_percent_bonus_melee"]],
  ["slow_resist", ["slow_resistance", "bonus_slow_resist", "slow_resist"]],
  ["hp_regen_amp", ["hp_regen_amp"]],
  ["mana_regen_amp", ["mana_regen_multiplier"]],
  ["mana_cost_reduction", ["manacost_reduction"]],
  ["night_vision", ["bonus_night_vision"], unitsToMilli],
  ["scepter", []],
  ["shard", []],
];
// Items that hold an upgrade of the hero while carried.
const HELD_UPGRADES = { ultimate_scepter: { scepter: 1 } };
// Items spent when used.
const SPENT_ON_USE = new Set(["aghanims_shard", "ultimate_scepter_2"]);
const ALL_STATS_KEYS = ["bonus_all_stats", "bonus_stats", "all_stats"];
// Neutral items name the same stats without the bonus_ prefix.
const NEUTRAL_ALIASES = {
  mp_regen: "mana_regen", magic_resist: "magic_resist", attack_lifesteal: "lifesteal", all_stats: "all",
};

function statsOf(it, lvl = 1, isNeutral = false, name = "") {
  const av = values(it);
  const out = { ...(HELD_UPGRADES[name] ?? {}) };
  const add = (stat, v) => {
    if (v) out[stat] = (out[stat] ?? 0) + v;
  };
  for (const [stat, keys, scale = (v) => Math.round(v)] of STAT_KEYS) {
    for (const key of keys) {
      if (key in av) {
        add(stat, scale(num(level(av[key], lvl))));
        break;
      }
    }
  }
  for (const key of ALL_STATS_KEYS) {
    if (key in av) {
      const v = Math.round(num(level(av[key], lvl)));
      for (const stat of ["strength", "agility", "intelligence"]) add(stat, v);
      break;
    }
  }
  if (isNeutral) {
    for (const [key, stat] of Object.entries(NEUTRAL_ALIASES)) {
      if (!(key in av)) continue;
      const v = num(level(av[key], lvl));
      if (stat === "all") {
        if (ALL_STATS_KEYS.some((k) => k in av)) continue;
        for (const s of ["strength", "agility", "intelligence"]) add(s, Math.round(v));
      } else if (!(stat in out)) {
        const scale = STAT_KEYS.find(([s]) => s === stat)?.[2] ?? ((x) => Math.round(x));
        add(stat, scale(v));
      }
    }
  }
  return out;
}

// Tier and level of every neutral item and enhancement.
const tierOf = new Map();
const enhancementLevels = new Map();
for (const [tier, t] of Object.entries(neutral.neutral_tiers)) {
  for (const name of Object.keys(t.items ?? {})) tierOf.set(name, Number(tier));
  for (const group of Object.values(t.enhancements ?? {})) {
    for (const [name, lvl] of Object.entries(group)) {
      if (!enhancementLevels.has(name)) enhancementLevels.set(name, new Set());
      enhancementLevels.get(name).add(Number(lvl));
    }
  }
}

// Recipe components by result item: the first requirement set of the recipe whose ItemResult it is.
const recipeOf = new Map();
for (const [key, it] of Object.entries(items)) {
  if (typeof it !== "object" || !it.ItemRecipe || !it.ItemResult) continue;
  const req = it.ItemRequirements?.["01"];
  if (req) recipeOf.set(it.ItemResult, req.split(";").filter(Boolean).map((c) => c.replace(/\*$/, "")));
}

const isRecipe = (it) => it.ItemRecipe === "1";
const isNeutralDrop = (it) => it.ItemIsNeutralActiveDrop === "1" || it.ItemIsNeutralPassiveDrop === "1";
const RETIRED = (key, it) =>
  it.IsObsolete === "1" || key.startsWith("item_river_painter") || /^item_tier\d_token$/.test(key);

const catalog = [];
for (const [key, it] of Object.entries(items)) {
  if (!key.startsWith("item_") || typeof it !== "object" || isRecipe(it) || RETIRED(key, it)) continue;
  const name = key.slice(5);
  if (key.startsWith("item_enhancement_")) {
    for (const lvl of [...(enhancementLevels.get(key) ?? [])].sort()) {
      catalog.push({ key, name: `${name}_${lvl}`, it, enhancementLevel: lvl });
    }
    continue;
  }
  if (isNeutralDrop(it) && !tierOf.has(key)) continue;
  catalog.push({ key, name, it, tier: tierOf.get(key) ?? 0 });
}
const known = new Set(catalog.map((c) => c.key));

// Neutral drops are not crafted, whatever an obsolete recipe says. A component that is retired is dropped.
const componentsOf = (c) => (c.tier || c.enhancementLevel ? [] : (recipeOf.get(c.key) ?? []).filter((k) => known.has(k)));

// Components declared before the item that uses them.
const ordered = [];
const placed = new Set();
const byKey = new Map(catalog.map((c) => [c.key, c]));
const place = (c) => {
  if (placed.has(c.key + c.name)) return;
  placed.add(c.key + c.name);
  for (const dep of componentsOf(c)) place(byKey.get(dep));
  ordered.push(c);
};
catalog.forEach(place);

const cost = (c) => Math.round(num(c.it.ItemCost));

// Active abilities. Each entry builds the ability from the item's own AbilityValues, cooldown and mana cost
// out of the engine's effect kinds. Codes are the engine's (rust/crates/data/src/format.rs).
const STAT = { damage: 0, armor: 1, health: 2, speed: 3, range: 4, evasion: 5, lifesteal: 10, magic_resist: 11, attack_speed: 12, mana_regen: 13, health_regen: 14, speed_pct: 19, status_resist: 21 };
const STATUS = { physical_immunity: 12, stun: 0, silence: 2, root: 3, invisibility: 4, hex: 6, disarm: 7, spell_immunity: 8, invulnerable: 9, mute: 11 };
const TEAM = { enemies: 0, friends: 1, others: 2, all: 3 };
const MAGICAL = 2;
const Q16 = 65536;
const v = (it, key, lvl = 1) => num(level(values(it)[key] ?? "0", lvl));
const timed = (stat, amount, seconds) => ["timed_stat", { p0: STAT[stat], p1: Math.round(amount), p2: ticks(seconds), p3: 0 }];
const status = (code, seconds) => ["status", { p0: STATUS[code], p1: ticks(seconds) }];
const AT_CASTER = 4;
const area = (radius, team, effects, at = AT_CASTER) => [["aoe", { p0: unitsToMilli(radius), p1: TEAM[team] | at, p2: effects.length, p3: 0 }], ...effects];
const slowTo = (multiplier, seconds) => ["slow", { p0: ticks(seconds), p1: Math.round(multiplier * Q16) }];
const hurt = (amount, type = MAGICAL) => ["damage", { p0: Math.round(amount), p1: 0, p2: type }];
const heal = (amount) => ["heal", { p0: Math.round(amount), p1: 0, p2: 1 }];
const mana = (amount) => ["mana", { p0: Math.round(amount), p1: 0 }];
const portal = (it, channelKey, target, filters) => ({
  target,
  filters,
  channel: channelKey ? v(it, channelKey) || 3 : 3,
  effects: [["teleport", { p0: unitsToMilli(v(it, "minimum_distance") || 70) }]],
});
const dagon = (it, n) => ({ target: "unit", filters: ["ground"], effects: [hurt(v(it, "damage", n))], lvl: n });
const ACTIVES = {
  blink: (it) => ({ target: "point", effects: [["move", { p0: unitsToMilli(v(it, "blink_range")) }]] }),
  overwhelming_blink: (it) => ACTIVES.blink(it),
  swift_blink: (it) => ACTIVES.blink(it),
  arcane_blink: (it) => ACTIVES.blink(it),
  flask: (it) => ({
    target: "unit",
    filters: ["army"],
    effects: [["create_healer", { p0: Math.round(v(it, "health_regen")) * Q16, p1: 0, p2: TPS, p3: ticks(v(it, "buff_duration")) }]],
  }),
  // The upgrade bits these carry are set by scenarios/moba/upgrades.lua, not by an effect.
  aghanims_shard: () => ({ target: "none", cooldown: 0, mana: 0, effects: [] }),
  ultimate_scepter_2: () => ({ target: "none", cooldown: 0, mana: 0, effects: [] }),
  manta: (it) => ({
    target: "none",
    effects: [["illusion", { p0: Math.round(v(it, "images_count")), p1: ticks(v(it, "illusion_duration")), p2: Math.round(v(it, "tooltip_damage_outgoing_melee")), p3: Math.round(v(it, "tooltip_damage_incoming_total_pct")) }]],
  }),
  ghost: (it) => ({ target: "none", effects: [status("physical_immunity", v(it, "duration")), status("disarm", v(it, "duration"))] }),
  faerie_fire: (it) => ({ target: "none", effects: [heal(v(it, "hp_restore"))] }),
  clarity: (it) => ({ target: "unit", filters: ["army"], effects: [timed("mana_regen", v(it, "mana_regen") * 100, v(it, "buff_duration"))] }),
  enchanted_mango: (it) => ({ target: "none", effects: [mana(v(it, "replenish_amount"))] }),
  cheese: (it) => ({ target: "none", effects: [heal(v(it, "health_restore")), mana(v(it, "mana_restore"))] }),
  royale_with_cheese: (it) => ({ target: "unit", filters: ["army"], effects: [heal(v(it, "health_restore")), mana(v(it, "mana_restore"))] }),
  phase_boots: (it) => ({ target: "none", effects: [timed("speed_pct", v(it, "phase_movement_speed"), v(it, "phase_duration"))] }),
  mekansm: (it) => ({ target: "none", effects: area(v(it, "heal_radius"), "friends", [heal(v(it, "heal_amount"))]) }),
  guardian_greaves: (it) => ({ target: "none", effects: area(v(it, "replenish_radius"), "friends", [heal(v(it, "replenish_health")), mana(v(it, "replenish_mana"))]) }),
  arcane_boots: (it) => ({ target: "none", effects: area(v(it, "replenish_radius"), "friends", [mana(v(it, "replenish_amount"))]) }),
  pipe: (it) => ({ target: "none", effects: area(v(it, "barrier_radius"), "friends", [["shield_bonus", { p0: Math.round(v(it, "barrier_block")) }]]) }),
  sheepstick: (it) => ({ target: "unit", filters: ["ground"], effects: [status("hex", v(it, "sheep_duration"))] }),
  orchid: (it) => ({ target: "unit", filters: ["ground"], effects: [["silence", { p0: ticks(v(it, "silence_duration")) }]] }),
  bloodthorn: (it) => ACTIVES.orchid(it),
  abyssal_blade: (it) => ({ target: "unit", filters: ["ground"], effects: [["stun", { p0: ticks(v(it, "stun_duration")) }]] }),
  heavens_halberd: (it) => ({ target: "unit", filters: ["ground"], effects: [status("disarm", v(it, "disarm_melee"))] }),
  nullifier: (it) => ({ target: "unit", filters: ["ground"], effects: [status("mute", v(it, "mute_duration"))] }),
  rod_of_atos: (it) => ({ target: "unit", filters: ["ground"], effects: [["root", { p0: ticks(v(it, "duration")) }]] }),
  gungir: (it) => ({ target: "point", effects: area(v(it, "radius"), "enemies", [["root", { p0: ticks(v(it, "duration")) }]], 0) }),
  tpscroll: (it) => ({ ...portal(it, "tooltip_channel_time", "unit", ["structure"]) }),
  travel_boots: (it) => ({ ...portal(it, "tp_channel_time", "unit"), cooldown: ticks(v(it, "tp_cooldown")), mana: 75 }),
  travel_boots_2: (it) => ({ ...portal(it, "tp_channel_time", "unit"), cooldown: ticks(v(it, "tp_cooldown")), mana: 75 }),
  refresher: () => ({ target: "none", effects: [["refresh", { p0: 3 }]] }),
  refresher_shard: () => ({ target: "none", effects: [["refresh", { p0: 3 }]] }),
  dagon: (it) => dagon(it, 1),
  dagon_2: (it) => dagon(it, 2),
  dagon_3: (it) => dagon(it, 3),
  dagon_4: (it) => dagon(it, 4),
  dagon_5: (it) => dagon(it, 5),
  ethereal_blade: (it) => ({
    target: "unit",
    filters: ["ground"],
    effects: [slowTo(1 + v(it, "blast_movement_slow") / 100, v(it, "duration")), hurt(v(it, "blast_damage_base"))],
  }),
  shivas_guard: (it) => ({
    target: "none",
    effects: area(v(it, "blast_radius"), "enemies", [hurt(v(it, "blast_damage")), slowTo(1 + v(it, "blast_movement_speed") / 100, v(it, "blast_debuff_duration"))]),
  }),
  meteor_hammer: (it) => ({
    target: "point",
    effects: area(v(it, "impact_radius"), "enemies", [hurt(v(it, "impact_damage_units")), ["stun", { p0: ticks(v(it, "stun_duration")) }]], 0),
  }),
  black_king_bar: (it) => ({ target: "none", effects: [status("spell_immunity", v(it, "duration"))] }),
  cyclone: (it) => ({ target: "unit", effects: [status("invulnerable", v(it, "cyclone_duration")), ["stun", { p0: ticks(v(it, "cyclone_duration")) }]] }),
  wind_waker: (it) => ACTIVES.cyclone(it),
  satanic: (it) => ({ target: "none", effects: [timed("lifesteal", v(it, "unholy_lifesteal_percent"), v(it, "unholy_duration"))] }),
  mask_of_madness: (it) => ({
    target: "none",
    effects: [
      timed("attack_speed", v(it, "berserk_bonus_attack_speed"), v(it, "berserk_duration")),
      timed("speed_pct", v(it, "berserk_bonus_movement_speed_melee"), v(it, "berserk_duration")),
      timed("armor", -v(it, "berserk_armor_reduction"), v(it, "berserk_duration")),
    ],
  }),
  invis_sword: (it) => ({ target: "none", effects: [["invisibility", { p0: ticks(v(it, "windwalk_duration")) }], timed("speed_pct", v(it, "windwalk_movement_speed"), v(it, "windwalk_duration"))] }),
  silver_edge: (it) => ACTIVES.invis_sword(it),
  soul_ring: (it) => ({ target: "none", effects: [mana(v(it, "mana_gain"))] }),
  ancient_janggo: (it) => ({
    target: "none",
    effects: area(v(it, "radius"), "friends", [timed("speed_pct", v(it, "bonus_movement_speed_pct"), v(it, "duration")), timed("attack_speed", v(it, "bonus_attack_speed_pct"), v(it, "duration"))]),
  }),
  boots_of_bearing: (it) => ACTIVES.ancient_janggo(it),
  pavise: (it) => ({ target: "unit", filters: ["army"], effects: [["shield_bonus", { p0: Math.round(v(it, "absorb_amount")) }]] }),
  solar_crest: (it) => ({
    target: "unit",
    filters: ["army"],
    effects: [timed("armor", v(it, "target_armor"), v(it, "duration")), timed("speed_pct", v(it, "target_movement_speed"), v(it, "duration")), timed("attack_speed", v(it, "target_attack_speed"), v(it, "duration"))],
  }),
  glimmer_cape: (it) => ({
    target: "unit",
    filters: ["army"],
    effects: [["invisibility", { p0: ticks(v(it, "duration")) }], timed("magic_resist", v(it, "bonus_magical_armor"), v(it, "duration"))],
  }),
  shadow_amulet: (it) => ({ target: "unit", filters: ["army"], effects: [["invisibility", { p0: ticks(v(it, "fade_duration")) }]] }),
  diffusal_blade: (it) => ({ target: "unit", filters: ["ground"], effects: [["mana", { p0: Math.round(v(it, "feedback_mana_burn")), p1: 1 }]] }),
  disperser: (it) => ACTIVES.diffusal_blade(it),
  dust: (it) => ({ target: "none", effects: area(v(it, "radius"), "enemies", [slowTo(1 + v(it, "movespeed") / 100, v(it, "duration")), hurt(v(it, "damage"))]) }),
  smoke_of_deceit: (it) => ({ target: "none", effects: area(v(it, "application_radius"), "friends", [["invisibility", { p0: ticks(v(it, "duration")) }], timed("speed_pct", v(it, "bonus_movement_speed"), v(it, "duration"))]) }),
  blood_grenade: (it) => ({
    target: "point",
    effects: area(v(it, "radius"), "enemies", [hurt(v(it, "impact_damage")), slowTo(1 + v(it, "movespeed_slow") / 100, v(it, "debuff_duration"))], 0),
  }),
};
// Passive item abilities: an aura pulses on the holder every `period` ticks; a proc rolls `chance` percent on every
// attack. The holder runs them while the item is in an active slot (spell.rs, passive_sources). An item has one ability,
// so an item with an active keeps the active.
const PASSIVES = {
  maelstrom: (it) => procChain(it, "chain_damage", "chain_strikes", "chain_radius", "chain_chance", "chain_delay", "chain_cooldown"),
  mjollnir: (it) => procChain(it, "chain_damage", "chain_strikes", "chain_radius", "chain_chance", "chain_delay", "chain_cooldown"),
  desolator: (it) => proc(100, 0, [timed("armor", v(it, "corruption_armor"), v(it, "corruption_duration"))]),
  skadi: (it) => proc(100, 0, [slowTo(1 + v(it, "cold_slow_melee") / 100, v(it, "cold_duration")), timed("attack_speed", v(it, "cold_attack_slow_melee"), v(it, "cold_duration"))]),
  orb_of_venom: (it) => proc(100, 0, [["dot", { p0: Math.round(v(it, "damage")), p1: ticks(v(it, "tick_interval")), p2: ticks(v(it, "poison_duration")), p3: MAGICAL }]]),
  monkey_king_bar: (it) => proc(v(it, "bonus_chance"), 0, [hurt(v(it, "bonus_chance_damage"))]),
  radiance: (it) => aura(TPS, 1, area(v(it, "aura_radius"), "enemies", [hurt(v(it, "aura_damage"))])),
  vladmir: (it) => aura(TPS, 1, area(v(it, "aura_radius"), "friends", [
    timed("armor", v(it, "armor_aura"), 2), timed("mana_regen", v(it, "mana_regen_aura") * 100, 2),
    timed("lifesteal", v(it, "lifesteal_aura"), 2), ["timed_stat", { p0: STAT.damage, p1: Math.round(v(it, "damage_aura")), p2: ticks(2), p3: 1 }],
  ])),
  headdress: (it) => aura(TPS, 1, area(v(it, "aura_radius"), "friends", [timed("health_regen", v(it, "aura_health_regen") * 100, 2)])),
  ring_of_basilius: (it) => aura(TPS, 1, area(v(it, "aura_radius"), "friends", [timed("mana_regen", v(it, "aura_mana_regen") * 100, 2)])),
  buckler: (it) => aura(TPS, 1, area(v(it, "bonus_aoe_radius"), "friends", [timed("armor", v(it, "bonus_aoe_armor"), 2)])),
};
function proc(chance, period, effects) {
  return { target: "unit", cooldown: 0, effects, ext: { proc_attack: true, passive: true, chance: Math.round(chance), ...(period ? { period } : {}) } };
}
function procChain(it, damage, strikes, radius, chance, delay, cooldown) {
  return {
    target: "unit",
    cooldown: ticks(v(it, cooldown)),
    effects: [["chain", { p0: Math.round(v(it, damage)), p1: Math.round(v(it, strikes)), p2: unitsToMilli(v(it, radius)), p3: MAGICAL }]],
    ext: { proc_attack: true, passive: true, chance: Math.round(v(it, chance)), period: ticks(v(it, delay)) },
  };
}
function aura(period, ticksCooldown, effects) {
  return { target: "none", cooldown: 0, effects, ext: { aura: true, passive: true, period } };
}
// Cast range and the side an active may target, from the item's own keys.
function targeting(it, active) {
  if (!active) return {};
  const out = {};
  const range = num(level(it.AbilityCastRange ?? "0", 1));
  const clamps = active.effects.some(([kind]) => kind === "move");
  if (range > 0 && active.target !== "none" && !clamps) out.cast_range = unitsToMilli(range);
  const team = it.AbilityUnitTargetTeam ?? "";
  if (/FRIENDLY/.test(team) && !/ENEMY|BOTH|CUSTOM/.test(team)) out.ally_only = true;
  if (/ENEMY/.test(team) && !/FRIENDLY|BOTH|CUSTOM/.test(team)) out.enemy_only = true;
  return out;
}
const cooldownOf = (it, lvl = 1) => ticks(num(level(it.AbilityCooldown ?? "0", lvl)));
const manaOf = (it, lvl = 1) => Math.round(num(level(it.AbilityManaCost ?? "0", lvl)));
const FIXED_ABILITIES = { ward_observer: "place_observer", ward_sentry: "place_sentry" };

const lines = [];
const abilityLines = [];
const q = (s) => JSON.stringify(s);
const kw = (o) =>
  Object.entries(o)
    .map(([k, v]) => `${k}=${typeof v === "string" ? q(v) : v === true ? "True" : v}`)
    .join(", ");

for (const c of ordered) {
  const build = ACTIVES[c.name] ?? PASSIVES[c.name];
  if (!build) continue;
  const a = build(c.it);
  const ext = { ...targeting(c.it, ACTIVES[c.name] !== undefined ? a : null), ...(a.ext ?? {}) };
  const effects = a.effects.map(([kind, p]) => `effect(${q(kind)}, ${kw(p)})`).join(", ");
  const filters = a.filters ? `, target_filters=${JSON.stringify(a.filters)}` : "";
  const cooldown = a.cooldown ?? cooldownOf(c.it, a.lvl);
  const energy = a.mana ?? manaOf(c.it, a.lvl);
  abilityLines.push(`ability(${q("item_" + c.name)}, ${cooldown}, ${energy}, ${q(a.target)}, [${effects}]${filters})`);
  if (a.channel) abilityLines.push(`channel(${q("item_" + c.name)}, ${ticks(a.channel)})`);
  if (Object.keys(ext).length) abilityLines.push(`ability_ext(${q("item_" + c.name)}, ${kw(ext)})`);
}

for (const c of ordered) {
  const it = c.it;
  const parts = [];
  const comps = componentsOf(c).map((k) => k.slice(5));
  const partsCost = componentsOf(c).reduce((sum, k) => sum + cost(byKey.get(k)), 0);
  if (partsCost > cost(c)) console.error(`cost rule: ${c.name} costs ${cost(c)}, components ${partsCost}`);
  const quality = it.ItemQuality ?? "";
  const buyable = it.ItemPurchasable !== "0" && !c.tier && !c.enhancementLevel;
  const stats = c.enhancementLevel
    ? statsOf(it, c.enhancementLevel, true, c.name)
    : statsOf(it, 1, c.tier > 0, c.name);
  const kws = [];
  if (Object.keys(stats).length) kws.push(`stats=${JSON.stringify(stats)}`);
  const ability = ACTIVES[c.name] || PASSIVES[c.name] ? "item_" + c.name : FIXED_ABILITIES[c.name];
  if (ability) kws.push(`ability=${q(ability)}`);
  if (comps.length) kws.push(`components=${JSON.stringify(comps)}`);
  const consumable = (quality.includes("consumable") && it.ItemPermanent !== "1") || SPENT_ON_USE.has(c.name);
  if (consumable) kws.push("consumable=True");
  if (c.tier) kws.push(`neutral_tier=${c.tier}`);
  if (c.enhancementLevel) kws.push("enhancement=True");
  if (!buyable) kws.push("no_buy=True");
  if (it.ItemSellable === "0" || c.tier || c.enhancementLevel) kws.push("no_sell=True");
  if (buyable) {
    const shops = it.SecretShop === "1" || quality === "secret_shop" ? ["secret"] : ["home"];
    const basic = comps.length === 0 && !shops.includes("secret") && /component|consumable|common/.test(quality);
    if (basic) shops.push("side");
    kws.push(`tags=${JSON.stringify(shops)}`);
  }
  const charges = Math.round(num(it.ItemInitialCharges ?? "0"));
  if (charges > 0) kws.push(`charges=${charges}`);
  if (it.ItemStackable === "1") kws.push(`stack_max=${it.ItemStackableMax ? Math.round(num(it.ItemStackableMax)) : UNLIMITED_STACK}`);
  const stockMax = Math.round(num(it.ItemStockMax ?? "0"));
  if (stockMax > 0) {
    kws.push(`stock_max=${stockMax}`, `stock_initial=${Math.round(num(it.ItemStockInitial ?? "0"))}`);
    kws.push(`stock_time_s=${Math.round(num(it.ItemStockTime ?? "0"))}`);
    if (it.ItemInitialStockTime) kws.push(`stock_start_s=${Math.round(num(it.ItemInitialStockTime))}`);
  }
  lines.push(`item(${q(c.name)}, ${cost(c)}${kws.length ? ", " + kws.join(", ") : ""})`);
}

const header = `# Generated by scenarios/moba/tools/gen-items.mjs from SteamDatabase tracking repo @${COMMIT}
# source/scripts/npc/items.txt and neutral_items.txt. Do not edit by hand: change the generator.
# item(name, cost, stats={}, ability=, components=, consumable=, neutral_tier=, enhancement=, no_buy=, no_sell=, tags=,
# charges=, stack_max=, stock_*=): cost is reference ItemCost (the full price); components come from the recipe's
# first requirement set. Percent stats are whole percents, regeneration is hundredths per second, move_speed is milli per
# tick, ranges are milli (one reference unit is 15.625 milli). Side shop membership is a heuristic (basic components and
# consumables), not reference data: shops.txt is not in the dump.
`;
fs.writeFileSync(outFile, header + abilityLines.join("\n") + "\n" + 'item_tags(["home", "side", "secret"])\n' + lines.join("\n") + "\n");
console.error(`${ordered.length} items, ${abilityLines.length} item abilities -> ${outFile}`);

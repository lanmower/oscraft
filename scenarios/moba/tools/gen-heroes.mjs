// Generates scenarios/moba/units_heroes.osd and the generated block of scenarios/moba/hero_rules.txt
// from reference npc_heroes (one KV file per hero under scripts/npc/heroes/ plus hero_base.txt).
// Source of truth: SteamDatabase tracking repo @282fad29f0f74504fad465c52d197fb61f4d612b
// source/scripts/npc/heroes/*.txt.
// Usage: node scenarios/moba/tools/gen-heroes.mjs [--src dir] [--out scenarios/moba] [--standins]
// Without --src the 129 hero files are downloaded into a temp directory first. --standins also rewrites the
// stat fields of the placeholder heroes in units.osd to those of the reference hero they stand in for (STANDINS).
// Scale (same as scenarios/moba/tools/gen-reference-map.mjs and the creep rows in units.osd): one reference unit is 125/8 = 15.625
// milli; speed is milli per tick at 16 ticks per second; weapon period is ticks; health regeneration of a hero is its `hp_regen`
// key in hero_rules.txt.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const COMMIT = "282fad29f0f74504fad465c52d197fb61f4d612b";
const RAW = `https://raw.githubusercontent.com/SteamDatabase/GameTracking-Dota2/${COMMIT}/game/dota/pak01_dir/scripts/npc/`;
const args = process.argv.slice(2);
const opt = (name, dflt) => (args.includes(name) ? args[args.indexOf(name) + 1] : dflt);
const outDir = opt("--out", "scenarios/moba");
const MILLI = 15.625;
const TPS = 16;

// Placeholder hero in units.osd and the reference hero whose stats it carries. The other placeholders have no 6.88g hero and were dropped (apply-creeps-688g.py).
const STANDINS = {
  hero: "dragon_knight",
};

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

async function source() {
  const dir = opt("--src", path.join(os.tmpdir(), "ref-heroes-src"));
  fs.mkdirSync(dir, { recursive: true });
  const idx = path.join(dir, "npc_heroes.txt");
  if (!fs.existsSync(idx)) fs.writeFileSync(idx, Buffer.from(await (await fetch(RAW + "npc_heroes.txt")).arrayBuffer()));
  const names = [...fs.readFileSync(idx, "utf8").matchAll(/#base "heroes\/(npc_dota_hero_\w+)\.txt"/g)].map((m) => m[1]);
  for (const n of names) {
    const p = path.join(dir, n + ".txt");
    if (!fs.existsSync(p)) fs.writeFileSync(p, Buffer.from(await (await fetch(`${RAW}heroes/${n}.txt`)).arrayBuffer()));
  }
  return { dir, names };
}

const { dir, names } = await source();
const load = (n) => Object.values(Object.values(parseKV(fs.readFileSync(path.join(dir, n + ".txt"), "utf8")))[0])[0];
const base = load("npc_dota_hero_base");
const num = (h, k) => Number(h[k] ?? base[k] ?? 0);
const x100 = (v) => Math.round(v * 100);
const PRIMARY = { DOTA_ATTRIBUTE_STRENGTH: 0, DOTA_ATTRIBUTE_AGILITY: 1, DOTA_ATTRIBUTE_INTELLECT: 2, DOTA_ATTRIBUTE_ALL: 3 };

const heroes = [];
for (const full of names) {
  if (full === "npc_dota_hero_base" || full === "npc_dota_hero_target_dummy") continue;
  const h = load(full);
  if ((h.Enabled ?? "0") !== "1") continue;
  const id = full.replace("npc_dota_hero_", "");
  const melee = (h.AttackCapabilities ?? base.AttackCapabilities) === "DOTA_UNIT_CAP_MELEE_ATTACK";
  const bat = num(h, "AttackRate");
  heroes.push({
    id,
    female: (h.Adjectives?.Female ?? "0") === "1",
    health: num(h, "StatusHealth"),
    speed: Math.round((num(h, "MovementSpeed") * MILLI) / TPS),
    range: Math.round(num(h, "AttackRange") * MILLI),
    sight: Math.round(num(h, "VisionDaytimeRange") * MILLI),
    period: Math.max(1, Math.round(bat * TPS)),
    melee,
    stats: {
      primary: PRIMARY[h.AttributePrimary ?? base.AttributePrimary] ?? 0,
      str: x100(num(h, "AttributeBaseStrength")), str_gain: x100(num(h, "AttributeStrengthGain")),
      agi: x100(num(h, "AttributeBaseAgility")), agi_gain: x100(num(h, "AttributeAgilityGain")),
      int: x100(num(h, "AttributeBaseIntelligence")), int_gain: x100(num(h, "AttributeIntelligenceGain")),
      mana: num(h, "StatusMana"), mana_regen: x100(num(h, "StatusManaRegen")), hp_regen: x100(num(h, "StatusHealthRegen")),
      armor: x100(num(h, "ArmorPhysical")), mr: num(h, "MagicalResistance"),
      bat: x100(bat), damage: x100((num(h, "AttackDamageMin") + num(h, "AttackDamageMax")) / 2),
    },
  });
}

const kw = (s) => Object.entries(s).map(([k, v]) => `${k}=${v}`).join(" ");
const statLine = (name, h) => `hero ${name} ${kw(h.stats)}`;

// Heroes whose hy3d export ships as js/client/models/<name>.glb, keyed by hero id. This file rewrites
// units_heroes.osd whole, so a per-hero model set only in that file is reverted by the next run.
const HY3D_MODEL = {
  abaddon: "abaddon-skinned",
  alchemist: "alchemist-skinned",
  ancient_apparition: "ancient-apparition-skinned",
  antimage: "anti-mage-skinned",
  axe: "axe-skinned",
  bane: "bane-elemental-skinned",
  batrider: "batrider-skinned",
  beastmaster: "beastmaster-skinned",
  bloodseeker: "bloodseeker-skinned",
  bounty_hunter: "bounty-hunter-skinned",
  arc_warden: "arc-warden-skinned",
  broodmother: "broodmother-skinned",
  chaos_knight: "chaos-knight-skinned",
  bristleback: "bristleback-skinned",
  death_prophet: "death-prophet-skinned",
  disruptor: "disruptor-skinned",
  doom_bringer: "doom-bringer-skinned",
  dragon_knight: "dragon-knight-skinned",
  drow_ranger: "drow-ranger-skinned",
  earthshaker: "earthshaker-skinned",
  crystal_maiden: "crystal-maiden-skinned",
  dark_seer: "dark-seer-skinned",
};

const osd = [
  "# Generated by scenarios/moba/tools/gen-heroes.mjs from reference npc_heroes at SteamDatabase tracking repo @" + COMMIT + ". Do not edit by hand.",
  "# One unit per enabled reference hero, appended after every hand-written unit so earlier indices hold. Per-hero attributes, growth,",
  "# mana, health regeneration, armor, magic resistance and base attack time are the `hero` lines of hero_rules.txt. Abilities are filled by the ability data.",
  "# Fixed: radius 375 = HULL_SIZE_HERO 24 units (hull sizes are engine constants, not in the dump: UNVERIFIED).",
  ...heroes.flatMap((h) => {
    const model = HY3D_MODEL[h.id] ?? (h.female ? "character-female-a" : "character-male-a");
    return [
      `unit("${h.id}", ${h.health}, ${h.speed}, 375, build_time=480, resource_a=0, capacity=1, filters=["ground", "armored", "army"], weapon_damage=${Math.round(h.stats.damage / 100)}, weapon_range_milli=${h.range}, weapon_period=${h.period}, sight_milli=${h.sight}, armor=${Math.round(h.stats.armor / 100)}, model="${model}", deny_pct=0)`,
      `unit_extra("${h.id}", player_controlled=True, deny_xp_pct=0)`,
    ];
  }),
  "",
];
fs.writeFileSync(path.join(outDir, "units_heroes.osd"), osd.join("\n"));

const BEGIN = "# BEGIN generated by scenarios/moba/tools/gen-heroes.mjs";
const END = "# END generated by scenarios/moba/tools/gen-heroes.mjs";
const byId = Object.fromEntries(heroes.map((h) => [h.id, h]));
const block = [
  BEGIN + " from npc_heroes @" + COMMIT,
  "# hero <unit> primary(0 str,1 agi,2 int,3 universal) str/agi/int + *_gain (hundredths), mana, mana_regen and hp_regen (hundredths per s),",
  "# armor (hundredths), mr (percent), bat (hundredths of a second), damage (hundredths, min/max midpoint).",
  ...heroes.map((h) => statLine(h.id, h)),
  ...Object.entries(STANDINS).map(([unit, id]) => statLine(unit, byId[id])),
  END,
];
const rulesPath = path.join(outDir, "hero_rules.txt");
let rules = fs.readFileSync(rulesPath, "utf8");
const a = rules.indexOf(BEGIN);
const b = rules.indexOf(END);
if (a >= 0 && b > a) rules = rules.slice(0, a) + block.join("\n") + rules.slice(b + END.length);
else rules = rules.replace(/\n*$/, "\n") + block.join("\n") + "\n";
fs.writeFileSync(rulesPath, rules);

if (args.includes("--standins")) {
  const up = path.join(outDir, "units.osd");
  const lines = fs.readFileSync(up, "utf8").split("\n").map((line) => {
    const m = line.match(/^unit\("(\w+)",/);
    const id = m && STANDINS[m[1]];
    if (!id) return line;
    const h = byId[id];
    const set = (l, k, v) => l.replace(new RegExp(`(\\b${k}=)-?\\d+`), `$1${v}`);
    let l = line.replace(/^unit\("(\w+)", \d+, \d+, \d+/, `unit("$1", ${h.health}, ${h.speed}, 375`);
    l = set(l, "weapon_damage", Math.round(h.stats.damage / 100));
    l = set(l, "weapon_range_milli", h.range);
    l = set(l, "weapon_period", h.period);
    l = set(l, "sight_milli", h.sight);
    l = set(l, "armor", Math.round(h.stats.armor / 100));
    l = l.replace(/, health_regen=\d+/, "");
    return l;
  });
  fs.writeFileSync(up, lines.join("\n"));
}
console.log(`${heroes.length} heroes`);

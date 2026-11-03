import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { notesByAnchor } from "./provenance.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const opt = (name, dflt) => (args.includes(name) ? args[args.indexOf(name) + 1] : dflt);
const outDir = path.resolve(opt("--out", path.join(here, "..")));
const check = args.includes("--check");
const read = (file) => fs.readFileSync(path.join(outDir, file), "utf8");

export const SCHOOLS = ["fire", "frost", "lightning", "nature", "shadow", "holy", "arcane", "poison", "blood", "earth", "wind", "physical"];
export const FORMS = ["bolt", "beam", "chain", "nova", "zone", "column", "aura", "cone", "summon", "blink", "orbit", "spiral"];
const DEFAULT_FORMS = { unit: ["bolt", "bolt", "beam", "aura"], point: ["zone", "column", "nova", "zone"], none: ["nova", "aura", "column", "spiral", "orbit", "cone", "aura", "nova"] };

const KIND_SCHOOL = { slow: "frost", root: "nature", dot: "poison", silence: "arcane", invisibility: "shadow", heal: "holy", shield_bonus: "holy", chain: "lightning", illusion: "arcane", teleport: "arcane", mana: "arcane", dispel: "arcane", refresh: "arcane", create_healer: "holy" };

const KEYWORDS = {
  fire: ["fire", "ember", "flame", "burn", "blaze", "inferno", "magma", "lava", "sun", "meteor", "scorch", "immolat", "torch", "ignite", "phoenix", "searing", "volcan", "incinerat", "napalm", "dragon"],
  frost: ["frost", "ice", "cold", "freez", "glacial", "chill", "winter", "snow", "blizzard", "arctic", "crystal", "cryo", "shatter"],
  lightning: ["lightning", "thunder", "storm", "shock", "static", "spark", "volt", "electr", "charge", "overload", "bolt", "tesla"],
  nature: ["nature", "entangle", "vine", "thorn", "briar", "treant", "spore", "leaf", "forest", "bloom", "wild", "grove", "living", "overgrowth", "spiderling", "spider", "web", "druid", "bear"],
  shadow: ["shadow", "dark", "night", "curse", "death", "soul", "void", "nether", "dusk", "requiem", "doom", "dread", "nightmare", "hex", "spectral", "wraith", "reaper", "plague"],
  holy: ["holy", "light", "divine", "heal", "mend", "bless", "purif", "guard", "sanct", "radian", "glyph", "fortif", "restor", "renew", "avatar", "hallow", "salve", "mercy"],
  arcane: ["arcane", "magic", "mana", "rune", "sigil", "spell", "astral", "cosmic", "illusion", "mirror", "ethereal", "rift", "portal", "invoke", "sorcer", "mystic", "essence"],
  poison: ["poison", "venom", "toxic", "acid", "corros", "sting", "noxious", "virulent", "contagion", "epidemic", "blight", "toxin"],
  blood: ["blood", "rage", "vampir", "bleed", "lacerat", "carnage", "feast", "thirst", "rupture", "gore", "frenzy", "berserk", "rend", "hunger", "bite"],
  earth: ["earth", "stone", "rock", "quake", "boulder", "fissure", "sand", "geo", "crush", "tremor", "avalanche", "magnet", "mountain", "echo_slam", "cleave_earth", "slam", "stomp"],
  wind: ["wind", "gale", "gust", "cyclone", "tornado", "dash", "blink", "leap", "jump", "glide", "whirl", "breeze", "zephyr", "tailwind", "hurricane", "windrun", "flurry", "evade", "hawk"],
  physical: ["strike", "slash", "cleave", "blade", "sword", "arrow", "shot", "attack", "bash", "axe", "cut", "spear", "hammer", "dagger", "shield", "bulwark", "stance", "pulse", "battle", "fury", "charge_of", "javelin", "ward"],
};

const FORM_KEYWORDS = [
  ["bolt", ["bolt", "coil", "missile", "dart", "arrow", "shot", "throw", "javelin", "touch", "orb", "fireball", "snipe", "shuriken"]],
  ["nova", ["nova", "clap", "slam", "shockwave", "pulse", "burst", "explo", "roar", "stomp", "shout", "cry", "requiem", "ravage"]],
  ["beam", ["beam", "ray", "laser", "drain", "siphon", "link", "lash", "leash", "tether", "lance"]],
  ["cone", ["breath", "wave", "cleave", "shotgun", "sweep", "fan", "spray", "scatter"]],
  ["column", ["pillar", "smite", "wrath", "judg", "geyser", "eruption", "fissure", "rain", "call_down", "inferno", "thunder", "blast"]],
  ["orbit", ["orbit", "satellite", "guard", "bulwark", "barrier", "halo", "ring"]],
  ["spiral", ["spiral", "tornado", "cyclone", "whirl", "vortex", "spin", "twister", "swirl", "gyre"]],
  ["summon", ["summon", "spawn", "totem", "golem", "minion", "clone", "spiderling", "hound", "ward", "wolf", "skeleton", "zombie", "raise", "army", "legion", "familiar", "elemental", "treant", "spirit_bear", "invoke"]],
  ["blink", ["blink", "dash", "leap", "jump", "glide", "phase", "shift", "portal", "teleport", "step", "rush", "charge", "flash", "vault", "warp", "tp_scroll", "hook", "pounce"]],
];

const OFFENCE = new Set(["damage", "stun", "slow", "silence", "root", "dot", "dispel", "mana", "if_kill", "attack", "aoe", "status"]);
const SUMMON_KINDS = new Set(["spawn", "ward", "illusion", "create_healer"]);
const BLINK_KINDS = new Set(["move", "teleport"]);
const SUSTAINED = new Set(["zone", "aura", "orbit", "spiral", "beam"]);
const FORM_DUR_MS = { bolt: 700, beam: 900, chain: 800, nova: 700, zone: 2500, column: 900, aura: 2500, cone: 700, summon: 1200, blink: 500, orbit: 3000, spiral: 1500 };
const TICK_MS = 1000 / 16;
const PASSIVE_COOLDOWN = 100000;

export function hash(name) {
  let h = 2166136261;
  for (let i = 0; i < name.length; i++) h = Math.imul(h ^ name.charCodeAt(i), 16777619) >>> 0;
  return h;
}
const spread = (name, salt, lo, hi) => lo + (hash(name + "#" + salt) % (hi - lo + 1));
const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, Math.round(v)));
const tokens = (name) => name.replace(/^item_/, "").split("_").filter(Boolean);
const matchesStem = (name, stems) => {
  const hay = "_" + tokens(name).join("_");
  return stems.filter((s) => hay.includes("_" + s) || (s.length > 4 && hay.includes(s)));
};

function parseEffects(text) {
  return [...text.matchAll(/effect\("(\w+)"((?:, p\d=-?\d+)*)/g)].map((m) => ({
    kind: m[1],
    p: [...m[2].matchAll(/p(\d)=(-?\d+)/g)].reduce((a, q) => ((a[Number(q[1])] = Number(q[2])), a), []),
  }));
}

function loadAbilities() {
  const abilities = new Map();
  const files = fs.readdirSync(outDir).filter((f) => /^(abilities.*|items)\.osd$/.test(f)).sort();
  for (const file of files) {
    let code = null;
    const notes = notesByAnchor(outDir, file);
    for (const line of read(file).split("\n")) {
      const named = /^ability\("([^"]+)"/.exec(line);
      const sidecarCode = named && (notes.get(named[1]) ?? []).map((n) => /^\S+ 6\.88g (\S+) /.exec(n)).filter(Boolean).at(-1)?.[1];
      if (sidecarCode) code = sidecarCode;
      const c = /^# \S+ 6\.88g (\S+) /.exec(line);
      if (c) code = c[1];
      const m = /^ability\("([^"]+)", (-?\d+), (-?\d+), "(\w+)", \[(.*?)\](?:, target_filters=\[[^\]]*\])?(?:, projectile_speed_milli=(\d+))?\)\s*$/.exec(line);
      if (!m) continue;
      const row = { name: m[1], cooldown: Number(m[2]), target: m[4], effects: parseEffects(m[5]), projectile: m[6] ? Number(m[6]) : 0, code };
      code = null;
      const had = abilities.get(row.name);
      if (!had || had.effects.length < row.effects.length) abilities.set(row.name, row);
    }
  }
  for (const line of read("abilities-inventory.tsv").split("\n").slice(1)) {
    const f = line.split("\t");
    if (f[3] === "ability" && f[2] !== "generic_hidden" && !abilities.has(f[2])) abilities.set(f[2], { name: f[2], cooldown: 0, target: "none", effects: [], projectile: 0, code: null, inventoryOnly: true });
  }
  return abilities;
}

const AREA_LABEL = /(?:作用|影响|爆炸|伤害)?范围|半径/;
const NOT_AREA = /施法|攻击|自动|施放|最大|侦察/;
const PLACEHOLDER_AREA = 9999;
const PLACEHOLDER_DUR_S = 0.1;
function loadReference() {
  const byCode = new Map();
  for (const line of read("abilities_688g_perlevel.tsv").split("\n")) {
    if (line.startsWith("#") || line.startsWith("unit_code")) continue;
    const f = line.split("\t");
    if (f.length < 10) continue;
    const dur = parseFloat((f[8] || "").split("|")[0]);
    let area = 0;
    for (const m of (f[9] || "").matchAll(/Data\w\[([^\]]*)\]=([^\[]*?)(?=\s{2}Data|$)/g)) {
      const v = parseFloat(m[1].split("/")[0]);
      if (AREA_LABEL.test(m[2]) && !NOT_AREA.test(m[2]) && v > 0 && v < PLACEHOLDER_AREA) {
        area = v;
        break;
      }
    }
    byCode.set(f[3], { dur: dur > PLACEHOLDER_DUR_S ? dur : 0, area });
  }
  for (const line of read("abilities_688g_area.tsv").split("\n")) {
    if (line.startsWith("#") || line.startsWith("code\t")) continue;
    const f = line.split("\t");
    const area = parseFloat(f[1]);
    if (f[0] && area > 0) byCode.set(f[0], { dur: byCode.get(f[0])?.dur ?? 0, area });
  }
  const byName = new Map();
  for (const line of read("abilities_688g.tsv").split("\n")) {
    if (line.startsWith("#") || line.startsWith("id\t")) continue;
    const f = line.split("\t");
    const area = parseFloat(f[7]);
    const dur = parseFloat(f[8]);
    const row = { dur: dur > PLACEHOLDER_DUR_S ? dur : 0, area: area > 0 && area < PLACEHOLDER_AREA ? area : 0 };
    if (f[0] && (row.area || row.dur)) byName.set(f[0], row);
  }
  return { byCode, byName };
}

function schoolOf(a) {
  const hits = {};
  for (const [school, stems] of Object.entries(KEYWORDS)) {
    const n = matchesStem(a.name, stems).length;
    if (n) hits[school] = n;
  }
  const kinds = a.effects.map((e) => e.kind);
  const kindSchool = kinds.map((k) => KIND_SCHOOL[k]).find(Boolean);
  const ranked = Object.entries(hits).sort((x, y) => y[1] - x[1] || SCHOOLS.indexOf(x[0]) - SCHOOLS.indexOf(y[0]));
  if (kindSchool && (!ranked.length || ranked[0][0] === kindSchool || !kinds.includes("damage"))) return { school: kindSchool, evidence: "kind", doubt: ranked.length && ranked[0][0] !== kindSchool ? 1 : 0 };
  if (ranked.length) return { school: ranked[0][0], evidence: "keyword", doubt: ranked.length > 1 && ranked[1][1] === ranked[0][1] ? 1 : 0 };
  if (kindSchool) return { school: kindSchool, evidence: "kind", doubt: 0 };
  if (kinds.includes("damage") || kinds.includes("aoe") || kinds.includes("attack") || kinds.includes("stun")) return { school: "physical", evidence: "kind", doubt: 1 };
  return { school: SCHOOLS[hash(a.name) % SCHOOLS.length], evidence: "default", doubt: 2 };
}

function formOf(a) {
  const kinds = new Set(a.effects.map((e) => e.kind));
  if (kinds.has("chain")) return { form: "chain", evidence: "kind", doubt: 0 };
  if ([...kinds].some((k) => SUMMON_KINDS.has(k))) return { form: "summon", evidence: "kind", doubt: 0 };
  if ([...kinds].some((k) => BLINK_KINDS.has(k)) && !kinds.has("damage")) return { form: "blink", evidence: "kind", doubt: 0 };
  if (kinds.has("zone")) return { form: "zone", evidence: "kind", doubt: 0 };
  for (const [form, stems] of FORM_KEYWORDS) if (matchesStem(a.name, stems).length) return { form, evidence: "keyword", doubt: kinds.size ? 1 : 0 };
  const offence = [...kinds].some((k) => OFFENCE.has(k));
  const passive = a.cooldown >= PASSIVE_COOLDOWN;
  if (a.effects.length) {
    if (passive) return { form: "aura", evidence: "kind", doubt: 0 };
    if (a.target === "unit") return { form: offence ? "bolt" : "aura", evidence: "target", doubt: 0 };
    if (a.target === "point") return { form: kinds.has("aoe") || offence ? "zone" : "column", evidence: "target", doubt: 0 };
    return { form: offence ? "nova" : "aura", evidence: "target", doubt: 0 };
  }
  const pool = DEFAULT_FORMS[a.target] ?? DEFAULT_FORMS.none;
  return { form: a.cooldown === 0 && a.target === "none" && !a.inventoryOnly ? "aura" : pool[hash(a.name) % pool.length], evidence: "default", doubt: 2 };
}

function sizeOf(a, ref) {
  const radius = a.effects.filter((e) => e.kind === "aoe" && e.p[0] < 100000).map((e) => e.p[0]);
  if (radius.length) return clamp((Math.max(...radius) * 1000) / 4000, 600, 2500);
  const move = a.effects.find((e) => e.kind === "move" || e.kind === "teleport");
  if (move && move.p[0] > 0) return clamp(move.p[0] / 15, 700, 1500);
  if (ref?.area) return clamp((ref.area * 15.625 * 1000) / 4000, 600, 2500);
  return spread(a.name, "size", 800, 1300);
}

function speedOf(a) {
  return a.projectile ? clamp((a.projectile * 1000) / 12000, 600, 2000) : spread(a.name, "speed", 850, 1150);
}

function durOf(a, form, ref) {
  if (SUSTAINED.has(form)) {
    const ticks = a.effects.flatMap((e) => (e.kind === "dot" ? [e.p[2]] : e.kind === "timed_stat" ? [e.p[2]] : ["stun", "slow", "root", "silence"].includes(e.kind) ? [e.p[0]] : [])).filter((t) => t > 0);
    if (ticks.length) return clamp(Math.max(...ticks) * TICK_MS, 800, 6000);
    if (ref?.dur) return clamp(ref.dur * 1000, 800, 6000);
  }
  return clamp((FORM_DUR_MS[form] * spread(a.name, "dur", 80, 120)) / 100, 300, 6000);
}

function hitsOf(a, form) {
  if (form === "chain") return spread(a.name, "hits", 3, 5);
  if (form === "orbit") return spread(a.name, "hits", 3, 6);
  if (form === "spiral") return spread(a.name, "hits", 2, 4);
  if (form === "cone") return spread(a.name, "hits", 5, 8);
  return 1;
}

const abilities = loadAbilities();
const ref = loadReference();
const rows = [];
for (const name of [...abilities.keys()].sort()) {
  const a = abilities.get(name);
  const r = ref.byName.get(name) ?? (a.code ? ref.byCode.get(a.code) : undefined);
  const s = schoolOf(a);
  const f = formOf(a);
  rows.push({
    name,
    school: s.school,
    form: f.form,
    size: sizeOf(a, r),
    speed: speedOf(a),
    dur: durOf(a, f.form, r),
    hits: hitsOf(a, f.form),
    schoolDefault: s.evidence === "default",
    formDefault: f.evidence === "default",
    doubt: s.doubt + f.doubt,
  });
}

const lua = [
  "-- Generated by scenarios/moba/tools/gen-ability-fx.mjs. Do not edit by hand.",
  "return {",
  ...rows.map((r) => `  [${JSON.stringify(r.name)}] = { school = "${r.school}", form = "${r.form}", size = ${r.size}, speed = ${r.speed}, dur = ${r.dur}, hits = ${r.hits} },`),
  "}",
  "",
].join("\n");

const file = path.join(outDir, "ability_fx.lua");
if (check) {
  if (!fs.existsSync(file) || fs.readFileSync(file, "utf8") !== lua) {
    console.error("ability_fx.lua is stale: run node scenarios/moba/tools/gen-ability-fx.mjs");
    process.exit(1);
  }
  console.log(`ability_fx.lua current (${rows.length} abilities)`);
} else {
  fs.writeFileSync(file, lua);
  const tally = (key, all) => Object.fromEntries(all.map((k) => [k, rows.filter((r) => r[key] === k).length]));
  console.log(`wrote ${file}: ${rows.length} abilities`);
  console.log("schools", JSON.stringify(tally("school", SCHOOLS)));
  console.log("forms", JSON.stringify(tally("form", FORMS)));
  console.log(`default school ${rows.filter((r) => r.schoolDefault).length}, default form ${rows.filter((r) => r.formDefault).length}, both ${rows.filter((r) => r.schoolDefault && r.formDefault).length}`);
  console.log("least sure:", rows.filter((r) => r.doubt).sort((x, y) => y.doubt - x.doubt || x.name.localeCompare(y.name)).slice(0, 10).map((r) => `${r.name}(${r.school}/${r.form})`).join(" "));
}

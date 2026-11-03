import fs from "node:fs";

const check = process.argv.includes("--check");
const dir = new URL("..", import.meta.url).pathname;
const osdPath = dir + "items.osd";
const table = fs.readFileSync(dir + "items_688g.tsv", "utf8").trim().split("\n").map((l) => l.split("\t"));
const head = table.shift();
const rows = table.map((r) => Object.fromEntries(head.map((k, i) => [k, r[i] ?? ""])));
const ORDER = ["stats", "ability", "aura", "components", "consumable", "neutral_tier", "enhancement", "no_buy", "no_sell", "no_drop", "tags", "charges", "stack_max", "stock_max", "stock_initial", "stock_time_s", "stock_start_s"];

function splitTop(text) {
  const out = []; let depth = 0, inStr = false, cur = "";
  for (const ch of text) {
    if (inStr) { cur += ch; if (ch === '"') inStr = false; continue; }
    if (ch === '"') { inStr = true; cur += ch; continue; }
    if ("[{".includes(ch)) depth++;
    if ("]}".includes(ch)) depth--;
    if (ch === "," && depth === 0) { out.push(cur.trim()); cur = ""; continue; }
    cur += ch;
  }
  if (cur.trim()) out.push(cur.trim());
  return out;
}

function rewrite(line, row) {
  const m = line.match(/^item\("([^"]+)", (-?\d+)(?:, (.*))?\)$/);
  const kws = new Map();
  for (const part of splitTop(m[3] ?? "")) { const i = part.indexOf("="); kws.set(part.slice(0, i), part.slice(i + 1)); }
  const full = ["applied", "applied_floor", "added"].includes(row.status);
  if (full) {
    const stats = JSON.parse(row.stats);
    if (Object.keys(stats).length) kws.set("stats", JSON.stringify(stats)); else kws.delete("stats");
    if (row.components !== "KEEP") {
      const comps = row.components ? row.components.split(",") : [];
      if (comps.length) kws.set("components", JSON.stringify(comps)); else kws.delete("components");
    }
  }
  if (row.method === "added_state") { kws.set("no_buy", "True"); kws.set("no_sell", "True"); kws.delete("tags"); }
  if (row.method === "added_form") { kws.set("no_buy", "True"); kws.delete("tags"); }
  for (const flag of (row.item_flags ?? "").split(",").filter(Boolean)) {
    if (flag.startsWith("charges=")) kws.set("charges", flag.slice(8));
    else kws.set(flag, "True");
  }
  if (row.ability_osd && row.ability_osd.split(" ;; ").some((l) => l.startsWith(`ability("item_${row.id}",`))) kws.set("ability", `"item_${row.id}"`);
  if (row.aura_ref) kws.set("aura", `"${row.aura_ref}"`);
  if (row.ability_ref) kws.set("ability", `"${row.ability_ref}"`);
  const rest = [...ORDER.filter((k) => kws.has(k)), ...[...kws.keys()].filter((k) => !ORDER.includes(k))].map((k) => `${k}=${kws.get(k)}`);
  return `item("${m[1]}", ${row.cost}${rest.length ? ", " + rest.join(", ") : ""})`;
}

let lines = fs.readFileSync(osdPath, "utf8").split("\n");
const byId = new Map(rows.map((r) => [r.id, r]));
const itemId = (l) => l.match(/^item\("([^"]+)"/)?.[1];
const added = [];
lines = lines.map((l) => {
  const id = itemId(l);
  const row = id && byId.get(id);
  if (!row || ["candidate", "not_in_688g"].includes(row.status)) return l;
  return rewrite(l, row);
});
for (const row of rows.filter((r) => r.status === "added")) {
  if (lines.some((l) => itemId(l) === row.id)) continue;
  const line = rewrite(`item("${row.id}", 0, tags=${row.components ? '["home"]' : '["home","side"]'})`, row);
  const user = lines.findIndex((l) => itemId(l) && new RegExp(`components=\\[[^\\]]*"${row.id}"`).test(l));
  if (user >= 0) lines.splice(user, 0, line); else added.push(line);
}
if (added.length) { const end = lines.length - (lines[lines.length - 1] === "" ? 1 : 0); lines.splice(end, 0, ...added); }
for (const row of rows.filter((r) => r.ability_osd)) {
  const names = new Set(row.ability_osd.split(" ;; ").map((l) => l.match(/^\w+\("([^"]+)"/)[1]));
  const first = lines.findIndex((l) => /^(ability|ability_ext|channel)\(/.test(l) && names.has(l.match(/^\w+\("([^"]+)"/)[1]));
  lines = lines.filter((l) => !(/^(ability|ability_ext|channel)\(/.test(l) && names.has(l.match(/^\w+\("([^"]+)"/)[1])));
  const at = first >= 0 ? Math.min(first, lines.length) : lines.reduce((n, l, i) => (/^(ability|ability_ext|channel)\(/.test(l) ? i + 1 : n), 0);
  lines.splice(at, 0, ...row.ability_osd.split(" ;; "));
}
const cooldowns = new Map(rows.filter((r) => r.ability_cooldown_ticks !== "" && ["applied", "applied_floor", "added"].includes(r.status)).map((r) => ["item_" + r.id, r.ability_cooldown_ticks]));
lines = lines.map((l) => {
  const m = l.match(/^ability\("([^"]+)", (\d+)(.*)$/);
  return m && cooldowns.has(m[1]) ? `ability("${m[1]}", ${cooldowns.get(m[1])}${m[3]}` : l;
});
for (let moved = true, guard = 0; moved && guard < 1000; guard++) {
  moved = false;
  const at = new Map();
  lines.forEach((l, i) => { const id = itemId(l); if (id) at.set(id, i); });
  for (let i = 0; i < lines.length && !moved; i++) {
    if (!itemId(lines[i])) continue;
    for (const c of JSON.parse((lines[i].match(/components=(\[[^\]]*\])/) ?? [0, "[]"])[1])) {
      const j = at.get(c);
      if (j > i) { const [line] = lines.splice(j, 1); lines.splice(i, 0, line); moved = true; break; }
    }
  }
}
const out = lines.join("\n");
const STAT_KEYS = new Set(["armor", "damage", "health", "mana", "strength", "agility", "intelligence", "move_speed", "attack_speed", "health_regen", "mana_regen", "magic_resist", "evasion", "spell_amp", "lifesteal", "spell_lifesteal", "cast_range", "attack_range", "status_resist", "cooldown_reduction", "gold_per_min", "xp_per_min", "crit_chance", "crit_multiplier", "cleave", "cleave_distance", "cleave_start_width", "cleave_end_width", "bash_chance", "bash_duration", "speed_pct", "slow_resist", "hp_regen_amp", "mana_regen_amp", "mana_cost_reduction", "night_vision", "scepter", "shard", "bash_damage", "bash_chance_ranged", "bash_cooldown", "block_chance", "block_amount", "spell_block", "true_strike", "block_chance_nonhero", "creep_damage_pct", "true_sight"]);
const problems = [];
const seen = new Map();
for (const l of out.split("\n")) {
  const m = l.match(/^item\("([^"]+)", (-?\d+)(?:, (.*))?\)$/);
  if (!m) continue;
  const kw = Object.fromEntries(splitTop(m[3] ?? "").map((p) => { const i = p.indexOf("="); return [p.slice(0, i), p.slice(i + 1)]; }));
  const comps = JSON.parse(kw.components ?? "[]");
  let sum = 0;
  for (const c of comps) { if (!seen.has(c)) problems.push(`${m[1]}: component ${c} is not declared before it`); sum += seen.get(c) ?? 0; }
  if (sum > +m[2]) problems.push(`${m[1]}: cost ${m[2]} is below its components' cost ${sum}`);
  for (const k of Object.keys(JSON.parse(kw.stats ?? "{}"))) if (!STAT_KEYS.has(k)) problems.push(`${m[1]}: unknown stat ${k}`);
  seen.set(m[1], +m[2]);
}
if (problems.length) { console.error(problems.join("\n")); process.exit(1); }
if (check) {
  if (out !== fs.readFileSync(osdPath, "utf8")) { console.error("items.osd differs from items_688g.tsv: run node scenarios/moba/tools/apply-items-688g.mjs"); process.exit(1); }
  console.log("items.osd matches items_688g.tsv");
} else {
  fs.writeFileSync(osdPath, out);
  console.log(`${rows.filter((r) => r.status !== "candidate" && r.status !== "not_in_688g").length} rows applied, ${rows.filter((r) => r.status === "added").length} added`);
}

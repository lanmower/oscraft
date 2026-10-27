#!/usr/bin/env node
// Lists every roster hero's abilities as the reference names them, and says which of them a recipe
// already covers.
//
// Usage: node scenarios/moba/tools/gen-ability-inventory.mjs [--out scenarios/moba] [--src dir]
//
// abilities_ruleset.osd is built from HEROES and RECIPES in ability-recipes.mjs. HEROES says which
// abilities a hero has; RECIPES says what each one does, and a recipe cannot be derived from the dump,
// it has to be written. So the work of adding a hero is: learn its ability names (this script) and then
// write a recipe for each (a person, against the reference's AbilityValues keys).
//
// The names come from the same commit gen-abilities.mjs reads: each
// npc_dota_hero_<hero>.txt lists Ability1..Ability6. Files are cached under --src, so a second run
// costs nothing. The output is a table, not a generator input: nothing here is applied to the pack.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { HEROES } from "./ability-recipes.mjs";

const COMMIT = "282fad29f0f74504fad465c52d197fb61f4d612b";
const RAW = `https://raw.githubusercontent.com/SteamDatabase/GameTracking-Dota2/${COMMIT}/game/dota/pak01_dir/scripts/npc/`;

const args = process.argv.slice(2);
const opt = (name, fallback) => {
  const hit = args.find((a) => a.startsWith(`--${name}=`));
  return hit ? hit.slice(name.length + 3) : fallback;
};
const out = opt("out", "scenarios/moba");
const dir = opt("src", path.join(os.tmpdir(), "ref-heroes-src"));
fs.mkdirSync(dir, { recursive: true });

const heroes = [
  ...new Set(
    fs
      .readFileSync(path.join(out, "units_heroes.osd"), "utf8")
      .split("\n")
      .map((l) => l.match(/^\s*unit\(\s*"([^"]+)"/)?.[1])
      .filter(Boolean),
  ),
].sort();

const covered = new Set(Object.values(HEROES).flat().filter((a) => a !== "@talents"));

async function abilityNames(hero) {
  const file = path.join(dir, `npc_dota_hero_${hero}.txt`);
  if (!fs.existsSync(file)) {
    const res = await fetch(`${RAW}heroes/npc_dota_hero_${hero}.txt`);
    if (!res.ok) return null;
    fs.writeFileSync(file, await res.text());
  }
  const text = fs.readFileSync(file, "utf8");
  // Ability1..Ability6 name the hero's abilities in slot order; the values are the internal names.
  const slots = new Map();
  for (const m of text.matchAll(/^\s*"(Ability\d+)"\s+"([^"]+)"/gm)) slots.set(Number(m[1].slice(7)), m[2]);
  return [...slots.entries()].sort((a, b) => a[0] - b[0]).map(([, name]) => name);
}

const rows = ["hero\tslot\tability\tkind\trecipe"];
let withRecipe = 0;
let without = 0;
const missing = [];
for (const hero of heroes) {
  const names = await abilityNames(hero);
  if (!names || names.length === 0) {
    missing.push(hero);
    rows.push([hero, 0, "-", "-", "no reference file"].join("\t"));
    continue;
  }
  const seen = new Set();
  names.forEach((name, i) => {
    // The reference lists scepter and shard variants as further slots under the same name, and every
    // talent as a shared special_bonus_* ability, so neither is a second ability to write a recipe for.
    if (seen.has(name)) return;
    seen.add(name);
    const kind = name.startsWith("special_bonus_") ? "talent" : "ability";
    const has = covered.has(name);
    if (kind === "ability") {
      if (has) withRecipe += 1;
      else without += 1;
    }
    rows.push([hero, i + 1, name, kind, has ? "yes" : "no"].join("\t"));
  });
}
const outPath = path.join(out, "abilities-inventory.tsv");
fs.writeFileSync(outPath, rows.join("\n") + "\n");
console.log(`${heroes.length} heroes, ${withRecipe} abilities with a recipe, ${without} without`);
if (missing.length) console.log(`no reference file for: ${missing.join(" ")}`);
console.log(`written to ${outPath}`);

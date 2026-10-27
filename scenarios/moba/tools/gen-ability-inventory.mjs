#!/usr/bin/env node
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
  const namesAlreadyListed = new Set();
  names.forEach((name, i) => {
    if (namesAlreadyListed.has(name)) return;
    namesAlreadyListed.add(name);
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

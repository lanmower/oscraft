#!/usr/bin/env node
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const flag = process.argv.indexOf("--root");
const root = flag > 0 ? resolve(process.argv[flag + 1]) : join(dirname(fileURLToPath(import.meta.url)), "..");
const moba = join(root, "scenarios", "moba");
const read = (name) => readFileSync(join(moba, name), "utf8");

const heroBase = read("panels.lua").match(/^local HERO_BASE = \{\n([\s\S]*?)\n\}/m)?.[1] ?? "";
const displayName = new Map([...heroBase.matchAll(/^\s+(\w+) = \{ "([^"]+)"/gm)].map((m) => [m[1], m[2]]));

const controlled = new Set();
const modelled = new Set();
for (const file of readdirSync(moba).filter((f) => /^units.*\.osd$/.test(f))) {
  const text = read(file);
  for (const m of text.matchAll(/^unit_extra\("(\w+)",[^\n]*player_controlled=True/gm)) controlled.add(m[1]);
  for (const m of text.matchAll(/^unit\("(\w+)",[^\n]*\bmodel="([^"]+)"/gm)) modelled.add(m[1]);
}
const roster = [...controlled].filter((id) => modelled.has(id));

const table = new Map([...read("hero_acronyms.lua").matchAll(/\["([^"]+)"\]\s*=\s*"([^"]*)"/g)].map((m) => [m[1], m[2]]));
const overrides = JSON.parse(read("moba_ui.json")).heroAcronyms ?? {};

const problems = [];
const owners = new Map();
if (roster.length === 0) problems.push("no player-controlled unit kind with a model found in scenarios/moba/units*.osd");
for (const id of roster) {
  const shown = displayName.get(id) ?? id;
  const acronym = overrides[shown] ?? table.get(shown);
  if (acronym === undefined) { problems.push(`${id} (${shown}): no acronym in hero_acronyms.lua or heroAcronyms`); continue; }
  if (!/^[A-Z]{1,3}$/.test(acronym)) problems.push(`${id} (${shown}): acronym "${acronym}" is not 1 to 3 letters A-Z`);
  owners.set(acronym, [...(owners.get(acronym) ?? []), shown]);
}
for (const [acronym, heroes] of owners) if (heroes.length > 1) problems.push(`acronym "${acronym}" is shared by ${heroes.join(", ")}`);

if (problems.length > 0) {
  console.log(problems.join("\n"));
  console.log(`RESULT: FAIL (${problems.length} problem${problems.length === 1 ? "" : "s"} over ${roster.length} heroes)`);
  process.exit(1);
}
console.log(`RESULT: PASS (${roster.length} heroes, ${owners.size} acronyms)`);

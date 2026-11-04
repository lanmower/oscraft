import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");
const moba = path.join(root, "scenarios", "moba");
const table = new Map([...fs.readFileSync(path.join(moba, "hero_acronyms.lua"), "utf8").matchAll(/^\s*\["([^"]+)"\] = "([^"]*)",/gm)].map((m) => [m[1], m[2]]));
const overrides = JSON.parse(fs.readFileSync(path.join(moba, "moba_ui.json"), "utf8")).heroAcronyms ?? {};
const acronyms = [...table.values()];
const invalid = acronyms.filter((value) => !/^[A-Z]{1,3}$/.test(value));
const duplicates = [...new Set(acronyms.filter((value, index) => acronyms.indexOf(value) !== index))];
const overrideMismatch = Object.entries(overrides).filter(([name, value]) => table.get(name) !== value).map(([name]) => name);
const problems = [
  ...invalid.map((value) => `not 1-3 letters: ${value}`),
  ...duplicates.map((value) => `duplicate acronym: ${value}`),
  ...overrideMismatch.map((name) => `override not in table: ${name}`),
];
if (problems.length > 0) {
  console.error(problems.join("\n"));
  process.exit(1);
}
console.log(`check-acronyms: ${acronyms.length} acronyms, all 1-3 letters and unique`);

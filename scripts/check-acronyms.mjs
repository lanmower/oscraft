import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");
const moba = path.join(root, "scenarios", "moba");
const derived = path.join(root, "scenarios", "moba-acronyms");
const bota = path.join(root, "scenarios", "bota");
const tableText = fs.readFileSync(path.join(moba, "hero_acronyms.lua"), "utf8");
const table = new Map([...tableText.matchAll(/^\s*\["([^"]+)"\] = "([^"]*)",/gm)].map((m) => [m[1], m[2]]));
const overrides = JSON.parse(fs.readFileSync(path.join(moba, "moba_ui.json"), "utf8")).heroAcronyms ?? {};
const acronyms = [...table.values()];
const invalid = acronyms.filter((value) => !/^[A-Z]{1,3}$/.test(value));
const duplicates = [...new Set(acronyms.filter((value, index) => acronyms.indexOf(value) !== index))];
const overrideMismatch = Object.entries(overrides).filter(([name, value]) => table.get(name) !== value).map(([name]) => name);
const roster = fs.readFileSync(path.join(moba, "units_heroes.osd"), "utf8");
const heroCount = [...roster.matchAll(/^unit\("/gm)].length;
const rosterGap = heroCount !== acronyms.length ? [`roster coverage: ${acronyms.length} acronyms for ${heroCount} heroes`] : [];
const derivedTable = [...fs.readFileSync(path.join(derived, "hero_acronyms.lua"), "utf8").matchAll(/^\s*\["([^"]+)"\] = "([^"]*)",/gm)].map((m) => [m[1], m[2]]);
const derivedUnitIds = [...fs.readFileSync(path.join(derived, "units_heroes.osd"), "utf8").matchAll(/^unit\("(\w+)"/gm)].map((m) => m[1]);
const derivedUi = JSON.parse(fs.readFileSync(path.join(derived, "moba_ui.json"), "utf8"));
const loader = fs.readFileSync(path.join(root, "js", "client", "pack_load.js"), "utf8");
const botaTable = [...fs.readFileSync(path.join(bota, "hero_acronyms.lua"), "utf8").matchAll(/^\s*\["([^"]+)"\] = "([^"]*)",/gm)].map((m) => [m[1], m[2]]);
const botaUnitIds = [...fs.readFileSync(path.join(bota, "units_heroes.osd"), "utf8").matchAll(/^unit\("(\w+)"/gm)].map((m) => m[1]);
const botaUi = JSON.parse(fs.readFileSync(path.join(bota, "bota_ui.json"), "utf8"));
const problems = [
  ...invalid.map((value) => `default authority not 1-3 letters: ${value}`),
  ...duplicates.map((value) => `duplicate default authority acronym: ${value}`),
  ...overrideMismatch.map((name) => `override not in default authority: ${name}`),
  ...rosterGap,
  ...(derivedTable.length !== acronyms.length ? [`derived acronym table has ${derivedTable.length} entries`] : []),
  ...derivedTable.filter(([key, value]) => !/^[a-z]{1,3}$/.test(key) || key.toUpperCase() !== value).map(([key]) => `derived active ID is not an acronym: ${key}`),
  ...derivedUnitIds.filter((id) => !/^[a-z]{1,3}$/.test(id)).map((id) => `derived unit ID is not an acronym: ${id}`),
  ...(derivedUi.identifierMode !== "default-dota-acronyms" ? ["derived UI identifier mode is not default-dota-acronyms"] : []),
  ...(!/PACK_PARAM[^\n]*"scenarios\/moba-acronyms"/.test(loader) ? ["default PACK_PARAM is not scenarios/moba-acronyms"] : []),
  ...botaTable.filter(([key]) => /^[a-z]{1,3}$/.test(key) && key !== "npc").map(([key]) => `BOTA active ID is an acronym-only key: ${key}`),
  ...botaUnitIds.filter((id) => /^[a-z]{1,3}$/.test(id) && id !== "npc").map((id) => `BOTA unit ID is acronym-only: ${id}`),
  ...(botaUi.identifierMode !== "bota-fullname" ? ["BOTA UI identifier mode is not bota-fullname"] : []),
];
if (problems.length > 0) {
  console.error(problems.join("\n"));
  process.exit(1);
}
console.log(`check-acronyms: default ${derivedTable.length} acronym-only active IDs, BOTA ${botaTable.length} fullname active IDs; authority ${acronyms.length} unique 1-3 letter labels`);

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const pack = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const SCOURGE = [
  "creep", "creep_super", "creep_mega",
  "creep_ranged", "creep_ranged_super", "creep_ranged_mega",
  "creep_siege", "creep_siege_super", "creep_siege_mega",
];
const EXTRA_OVERRIDES = {
  creep_ranged: { attack_point_ms: 500 },
  creep_ranged_super: { attack_point_ms: 500, acquire: 800 },
  creep_ranged_mega: { attack_point_ms: 500, acquire: 800 },
};

const read = (f) => fs.readFileSync(path.join(pack, f), "utf8").split("\n");
const units = read("units.osd");
const extras = read("units_extra.osd");
const line = (lines, fn, name) => lines.find((l) => l.startsWith(`${fn}("${name}",`));
const rename = (l, name) => l.replace(/^(\w+)\("[^"]+"/, `$1("sentinel_${name}"`);
const setKey = (l, key, value) => (new RegExp(`\\b${key}=\\d+`).test(l) ? l.replace(new RegExp(`\\b${key}=\\d+`), `${key}=${value}`) : l.replace(/\)\s*$/, `, ${key}=${value})`));

const out = [];
for (const name of SCOURGE) out.push(rename(line(units, "unit", name), name));
for (const name of SCOURGE) {
  let l = rename(line(extras, "unit_extra", name), name);
  for (const [key, value] of Object.entries(EXTRA_OVERRIDES[name] ?? {})) l = setKey(l, key, value);
  out.push(l);
}
fs.writeFileSync(path.join(pack, "units_lane.osd"), `${out.join("\n")}\n`);
console.log(`gen-sentinel-creeps: ${SCOURGE.length} kinds`);

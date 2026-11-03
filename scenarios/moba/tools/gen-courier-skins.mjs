import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const pack = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const GROUND = ["penguin", "raccoon", "crab", "seal", "skink", "dog", "dune_worm", "carrion_beetle", "fel_boar", "rabbit", "pack_horse", "mini_pudge", "grunt", "pandaren", "mini_techies"];
const FLYING = ["zeppelin", "vulture", "owl", "viper", "frost_wyrm", "flying_panda"];

const read = (f) => fs.readFileSync(path.join(pack, f), "utf8").split("\n");
const line = (lines, fn, name) => lines.find((l) => l.startsWith(`${fn}("${name}",`));
const rename = (l, name) => l.replace(/^(\w+)\("[^"]+"/, `$1("${name}"`);

const units = read("units.osd");
const extras = read("units_extra.osd");
const out = [];
for (const [base, skins] of [["courier", GROUND], ["courier_flying", FLYING]]) {
  for (const skin of skins) out.push(rename(line(units, "unit", base), `courier_${skin}`));
  for (const skin of skins) out.push(rename(line(extras, "unit_extra", base), `courier_${skin}`));
}
fs.writeFileSync(path.join(pack, "units_courier_skins.osd"), `${out.join("\n")}\n`);
console.log(`gen-courier-skins: ${GROUND.length + FLYING.length} kinds`);

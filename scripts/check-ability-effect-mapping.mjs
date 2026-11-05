import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const mapPath = join(root, "scenarios", "moba", "abilities_688g_label_effects.tsv");
const batchPath = join(root, "scenarios", "moba", "abilities_688g_b01.osd");
const rulesPath = join(root, "scenarios", "moba", "abilities_ruleset.osd");
const rows = readFileSync(mapPath, "utf8").trim().split(/\r?\n/).slice(1).map((line) => line.split("\t"));
const damageLabels = rows.filter((cols) => cols[1] === "damage" && cols[2] !== "UNVERIFIED");
const batch = readFileSync(batchPath, "utf8");
const rules = readFileSync(rulesPath, "utf8");
const encoded = batch.match(/ability\("([^"]+)"[^\n]*effect\("damage"[^)]*\)/);
const execute = /effect\("execute"/.test(rules);
if (!damageLabels.length || !encoded || !execute) {
  console.log(`check-ability-effect-mapping: FAIL labels=${damageLabels.length} encoded=${!!encoded} execute=${execute}`);
  process.exitCode = 1;
} else {
  const effect = encoded[0].match(/effect\("damage"[^)]*\)/)[0];
  console.log(`check-ability-effect-mapping: ${damageLabels.length} damage labels bound; ${encoded[1]} encodes ${effect}; abilities_ruleset.osd encodes execute`);
}

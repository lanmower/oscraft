import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { RETIRED_ABILITIES } from "./retired-abilities.mjs";

const moba = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const check = process.argv.includes("--check");
const retired = (name) => RETIRED_ABILITIES.has(name);

const ABILITY_DEFINITION = /^(ability|skill|ability_ext|ability_scaled)\("([^"]+)"/;
const ABILITY_LIST = /, abilities=\[([^\]]*)\]/;

const dropDefinitionLines = (text) =>
  text
    .split("\n")
    .filter((line) => {
      const m = ABILITY_DEFINITION.exec(line);
      return !(m && retired(m[2]));
    })
    .join("\n");

const dropListEntries = (text) =>
  text
    .split("\n")
    .map((line) => {
      const m = ABILITY_LIST.exec(line);
      if (!m) return line;
      const names = m[1].split(",").map((s) => s.trim().replace(/^"|"$/g, "")).filter(Boolean);
      if (!names.some(retired)) return line;
      const kept = names.filter((name) => !retired(name));
      const list = kept.length ? `, abilities=[${kept.map((name) => `"${name}"`).join(", ")}]` : "";
      return line.replace(ABILITY_LIST, list);
    })
    .join("\n");

const dropRowsByColumn = (column) => (text) =>
  text
    .split("\n")
    .filter((line) => !retired(line.split("\t")[column]))
    .join("\n");

const TARGETS = [
  ["abilities_688g_b05.osd", dropDefinitionLines],
  ["abilities_688g_b07.osd", dropDefinitionLines],
  ["units_heroes.osd", dropListEntries],
  ["ability-688g-ids.tsv", dropRowsByColumn(1)],
  ["assets/icons/ABILITY-ICONS.tsv", dropRowsByColumn(0)],
];

const stale = [];
for (const [name, rewrite] of TARGETS) {
  const file = path.join(moba, name);
  const before = fs.readFileSync(file, "utf8");
  const after = rewrite(before);
  if (check) {
    if (after !== before) stale.push(name);
    continue;
  }
  if (after !== before) fs.writeFileSync(file, after);
  console.log(`${name}: ${after === before ? "unchanged" : "retired entries removed"}`);
}
if (check) {
  if (stale.length) {
    console.error(`retired ability entries still present in: ${stale.join(", ")}`);
    process.exit(1);
  }
  console.log(`no retired ability entries in ${TARGETS.length} files`);
}

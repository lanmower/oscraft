import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const pack = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const rows = fs.readFileSync(path.join(pack, "hero_melee_688g.tsv"), "utf8").split("\n").slice(1).filter(Boolean).map((l) => l.split("\t"));
const melee = new Map(rows.map((c) => [c[0], c[1] === "1"]));
const file = path.join(pack, "units_heroes.osd");
let changed = 0;
const lines = fs.readFileSync(file, "utf8").split("\n").map((line) => {
  const m = /^unit_extra\("(\w+)", /.exec(line);
  if (!m || !melee.has(m[1])) return line;
  const next = line.replace(/, melee=True/, "").replace(/\)\s*$/, melee.get(m[1]) ? ", melee=True)" : ")");
  if (next !== line) changed++;
  return next;
});
fs.writeFileSync(file, lines.join("\n"));
console.log(`apply-hero-melee: ${changed} unit_extra lines changed`);

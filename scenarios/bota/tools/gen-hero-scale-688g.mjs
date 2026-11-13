import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const moba = path.join(here, "..");
const [, , jassPath = "war3map.j", outPath = path.join(moba, "hero_scale_688g.lua")] = process.argv;

const jass = fs.readFileSync(jassPath, "utf8");
const scales = new Map();
for (const m of jass.matchAll(/^call P3\(\d+,'(\w{4})','\w{4}','\w{4}',"[^"]*",([\d.]+),\d+\)$/gm)) scales.set(m[1], Number(m[2]));
for (const m of jass.matchAll(/^call Pu\('(\w{4})',([\d.]+),/gm)) if (!scales.has(m[1])) scales.set(m[1], Number(m[2]));

const rows = fs.readFileSync(path.join(moba, "heroes_688g.tsv"), "utf8").trim().split("\n");
const header = rows.shift().split("\t");
const [idCol, srcCol] = [header.indexOf("id"), header.indexOf("src")];
const lines = [];
const missing = [];
for (const row of rows) {
  const cells = row.split("\t");
  const scale = scales.get(cells[srcCol]);
  if (scale === undefined) { missing.push(cells[idCol]); continue; }
  if (scale !== 1) lines.push(`  ${cells[idCol]} = ${Math.round(scale * 100)},`);
}
fs.writeFileSync(outPath, `return {\n${lines.join("\n")}\n}\n`);
console.log(`${lines.length} heroes scaled, ${missing.length} without a Qu entry (scale 1): ${missing.join(" ")}`);

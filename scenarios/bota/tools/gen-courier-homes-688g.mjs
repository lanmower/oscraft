import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const moba = path.join(here, "..");
const [, , jassPath = "war3map.j", outPath = path.join(moba, "courier_homes_688g.lua")] = process.argv;

const MAP_OFFSET = 35 * 64 + 8192;
const MILLI_PER_UNIT = 1000 / 64;

const sumOf = (expr) =>
  [...expr.replaceAll(/\.(?=-)/g, "").matchAll(/[+-]?\d+/g)].reduce((total, m) => total + Number(m[0]), 0);

const indexOfId = new Map(
  fs
    .readFileSync(path.join(moba, "starts-688g.tsv"), "utf8")
    .trim()
    .split("\n")
    .slice(1)
    .map((row) => {
      const [index, id] = row.split("\t");
      return [Number(id), Number(index)];
    }),
);

const jass = fs.readFileSync(jassPath, "utf8");
const homes = [];
for (const m of jass.matchAll(/^set \w+=CreateUnit\(Player\(([\w]+)\),'ncop',([^,]+),([^,]+),270\)\r?$/gm)) {
  const player = Number(m[1]);
  if (!indexOfId.has(player)) throw new Error(`no pack seat for 6.88g player ${player}`);
  const wx = sumOf(m[2]);
  const wy = sumOf(m[3]);
  homes.push([
    indexOfId.get(player),
    Math.round((MAP_OFFSET + wx) * MILLI_PER_UNIT),
    Math.round((MAP_OFFSET - wy) * MILLI_PER_UNIT),
  ]);
}
homes.sort((a, b) => a[0] - b[0]);

const rows = homes.map(([index, x, y]) => `  [${index}] = { x = ${x}, y = ${y} },`);
fs.writeFileSync(outPath, `return {\n${rows.join("\n")}\n}\n`);
console.log(`${rows.length} courier homes`);

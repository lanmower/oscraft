// Re-places the mapped (non-tree) doodad types into scenarios/moba/map.osd from the decoded placements
// (doodads-688g.tsv) and the type table (doodad-types-688g.tsv). Tree lines are left as gen-w3-map.mjs wrote them.
// Usage: node scenarios/moba/tools/place-doodads-688g.mjs [--dir scenarios/moba]
import fs from "node:fs";
import path from "node:path";
import { MAPPED_ID_BASE, parseRoutes, parseBases, parseTypeTable, mappedDoodadLines } from "./doodad-kinds.mjs";

const args = process.argv.slice(2);
const dir = args.includes("--dir") ? args[args.indexOf("--dir") + 1] : "scenarios/moba";
const GX = 35, GY = 35, UNIT_MILLI = 1000 / 64;
const toMilli = (wx, wy) => [Math.round((GX * 64 + wx + 8192) * UNIT_MILLI), Math.round((GY * 64 + 8192 - wy) * UNIT_MILLI)];

const kinds = parseTypeTable(fs.readFileSync(path.join(dir, "doodad-types-688g.tsv"), "utf8"));
const records = fs.readFileSync(path.join(dir, "doodads-688g.tsv"), "utf8").trim().split("\n").slice(1).map((l) => {
  const c = l.split("\t");
  const [x, y] = toMilli(Number(c[2]), Number(c[3]));
  return { type: c[0], x, y };
});
const mapOsdPath = path.join(dir, "map.osd");
const mapLines = fs.readFileSync(mapOsdPath, "utf8").split("\n");
const bases = parseBases(mapLines.join("\n"));
const routes = parseRoutes(fs.readFileSync(path.join(dir, "routes.osd"), "utf8"));
const { lines, excludedLane, excludedBase } = mappedDoodadLines(records, kinds, { bases, routes });

const isMapped = (l) => { const m = l.match(/^doodad\((\d+),/); return m && Number(m[1]) >= MAPPED_ID_BASE; };
const body = mapLines.filter((l) => !isMapped(l));
while (body.length && body[body.length - 1] === "") body.pop();
fs.writeFileSync(mapOsdPath, [...body, ...lines, ""].join("\n"));
console.log(JSON.stringify({ mappedTypes: kinds.size, records: records.length, placed: lines.length, excludedLane, excludedBase }));

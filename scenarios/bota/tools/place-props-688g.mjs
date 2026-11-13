import fs from "node:fs";
import path from "node:path";
import { MAPPED_ID_BASE, PILLAR_ID_BASE, parseRoutes, parseBases, parseTypeTable, mappedPropLines } from "./prop-kinds.mjs";

const args = process.argv.slice(2);
const KNOWN = new Set(["dir", "types", "placements", "dry-run"]);
for (const a of args) if (a.startsWith("--") && !KNOWN.has(a.slice(2).split("=")[0])) throw new Error(`unknown argument ${a}; use --dir <dir>, --types=<file>, --placements=<file>, --dry-run`);
const opt = (name, fallback) => {
  const i = args.indexOf(`--${name}`);
  if (i >= 0) {
    if (!args[i + 1] || args[i + 1].startsWith("--")) throw new Error(`--${name} needs a value`);
    return args[i + 1];
  }
  const hit = args.find((a) => a.startsWith(`--${name}=`));
  return hit ? hit.slice(name.length + 3) : fallback;
};
const dir = opt("dir", "scenarios/moba");
const dryRun = args.includes("--dry-run");
const placementsName = opt("placements", "props-688g.tsv");
const typeTableName = opt("types", "prop-types-688g.tsv");
const GX = 35, GY = 35, UNIT_MILLI = 1000 / 64;
const toMilli = (wx, wy) => [Math.round((GX * 64 + wx + 8192) * UNIT_MILLI), Math.round((GY * 64 + 8192 - wy) * UNIT_MILLI)];

const kinds = parseTypeTable(fs.readFileSync(path.join(dir, typeTableName), "utf8"));
const records = fs.readFileSync(path.join(dir, placementsName), "utf8").trim().split("\n").slice(1).map((l) => {
  const c = l.split("\t");
  const [x, y] = toMilli(Number(c[2]), Number(c[3]));
  const rawScale = Math.round(Number(c[6]) * 1000);
  const scale = Number.isFinite(rawScale) && rawScale > 0 && rawScale <= 100000 ? rawScale : undefined;
  return { type: c[0], x, y, scale };
});
const mapOsdPath = path.join(dir, "map.osd");
const mapLines = fs.readFileSync(mapOsdPath, "utf8").split("\n");
const bases = parseBases(mapLines.join("\n"));
const routes = parseRoutes(fs.readFileSync(path.join(dir, "routes.osd"), "utf8"));
const { lines, excludedLane, excludedBase } = mappedPropLines(records, kinds, { bases, routes });

const isMapped = (l) => { const m = l.match(/^prop\((\d+),/); return m && Number(m[1]) >= MAPPED_ID_BASE && Number(m[1]) < PILLAR_ID_BASE; };
const body = mapLines.filter((l) => !isMapped(l));
while (body.length && body[body.length - 1] === "") body.pop();
if (!dryRun) fs.writeFileSync(mapOsdPath, [...body, ...lines, ""].join("\n"));
console.log(JSON.stringify({ dryRun, source: placementsName, types: typeTableName, mappedTypes: kinds.size, records: records.length, placed: lines.length, excludedLane, excludedBase }));

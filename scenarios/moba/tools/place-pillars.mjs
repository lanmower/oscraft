import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { propSizes } from "./prop-sizes.mjs";
import { PILLAR_ID_BASE } from "./prop-kinds.mjs";

const toolDir = path.dirname(fileURLToPath(import.meta.url));
const mobaDir = path.join(toolDir, "..");
const mapPath = path.join(mobaDir, "map.osd");
const cellsPath = path.join(mobaDir, "pillar-cells.tsv");
const sizesPath = path.join(mobaDir, "assets", "prop_sizes.json");
const modelsDir = new URL("file://" + path.join(mobaDir, "assets", "models") + "/");
const KINDS = [1, 2, 3, 4, 5, 6].map((n) => "pillar-rock-" + n + "-skinned");
const isPillarLine = (line) => line.startsWith("prop(") && line.includes('"pillar-rock-');

const cells = fs.readFileSync(cellsPath, "utf8").trim().split("\n").slice(1)
  .map((line) => line.split("\t"))
  .filter((cols) => cols.length >= 2 && cols[0] !== "")
  .map((cols) => ({ x: Number(cols[0]), y: Number(cols[1]) }));
if (!cells.length) throw new Error("pillar-cells.tsv has no cells");
if (!cells.every((c) => Number.isInteger(c.x) && Number.isInteger(c.y))) throw new Error("pillar-cells.tsv has a non-integer cell");

const pillarLines = cells.map((c, i) => "prop(" + (PILLAR_ID_BASE + i) + ', "' + KINDS[i % KINDS.length] + '", ' + (c.x * 1000) + ", " + (c.y * 1000) + ")");

const text = fs.readFileSync(mapPath, "utf8");
const eol = text.includes("\r\n") ? "\r\n" : "\n";
const existing = text.split(eol);
while (existing.length && existing[existing.length - 1] === "") existing.pop();
const kept = existing.filter((line) => !isPillarLine(line));
const next = kept.concat(pillarLines, [""]).join(eol);
if (next !== text) fs.writeFileSync(mapPath, next);

const rawSizes = fs.readFileSync(sizesPath, "utf8");
const sizes = JSON.parse(rawSizes);
const missing = KINDS.filter((k) => !(k in sizes));
let footprintsAdded = 0;
if (missing.length) {
  const computed = propSizes(modelsDir, new Set(missing)).sizes;
  const addable = missing.filter((k) => computed[k] !== undefined);
  const absent = missing.filter((k) => computed[k] === undefined);
  if (absent.length) throw new Error("no computed footprint for " + absent.join(", "));
  const inner = rawSizes.trimEnd().slice(0, -1).trimEnd();
  const added = addable.map((k) => " " + JSON.stringify(k) + ": " + JSON.stringify(computed[k]));
  fs.writeFileSync(sizesPath, inner + ",\n" + added.join(",\n") + "\n}\n");
  footprintsAdded = addable.length;
}

console.log(JSON.stringify({ cells: cells.length, pillarLines: pillarLines.length, footprintsAdded }));


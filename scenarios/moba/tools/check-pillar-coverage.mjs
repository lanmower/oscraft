import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { findCandidates, loadGrid } from "./pillar-slopes.mjs";

const toolDir = path.dirname(fileURLToPath(import.meta.url));
const mapPath = path.join(toolDir, "..", "map.osd");
const PILLAR_PROP = /^prop\((\d+), "pillar-rock-(\d+)-skinned", (-?\d+), (-?\d+)\)$/;

const slopes = findCandidates(loadGrid(mapPath));
const slopeKeys = new Set(slopes.map((c) => `${c.x},${c.y}`));

const decorated = [];
for (const raw of fs.readFileSync(mapPath, "utf8").split(/\r?\n/)) {
  const match = PILLAR_PROP.exec(raw.trim());
  if (!match) continue;
  const [, , kind, x, y] = match;
  decorated.push({ key: `${Number(x) / 1000},${Number(y) / 1000}`, kind: Number(kind) });
}
const decoratedKeys = new Set(decorated.map((d) => d.key));
const missing = [...slopeKeys].filter((key) => !decoratedKeys.has(key));
const extra = [...decoratedKeys].filter((key) => !slopeKeys.has(key));
const duplicates = decorated.length - decoratedKeys.size;
const kinds = [...new Set(decorated.map((d) => d.kind))].sort((a, b) => a - b);

const report = {
  impassableSlopeCells: slopeKeys.size,
  decoratedCells: decoratedKeys.size,
  pillarProps: decorated.length,
  duplicates,
  missing,
  extra,
  rockKinds: kinds,
};
console.log(JSON.stringify(report, null, 2));
if (missing.length || extra.length || duplicates || decorated.length !== slopeKeys.size) {
  console.error("pillar coverage: FAIL");
  process.exitCode = 1;
} else {
  console.log(`pillar coverage: ok (${decoratedKeys.size} of ${slopeKeys.size} impassable slope cells decorated, one prop each)`);
}

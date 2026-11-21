import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { findCandidates, loadGrid } from "../scenarios/moba/tools/pillar-slopes.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const PILLAR_PROP = /^prop\((\d+), "pillar-rock-(\d+)-skinned", (-?\d+), (-?\d+)\)$/;

function coverage(mapPath) {
  const slopeKeys = new Set(findCandidates(loadGrid(mapPath)).map((c) => c.x + "," + c.y));
  const decorated = [];
  for (const raw of fs.readFileSync(mapPath, "utf8").split(/\r?\n/)) {
    const match = PILLAR_PROP.exec(raw.trim());
    if (!match) continue;
    const [, , kind, x, y] = match;
    decorated.push({ key: Number(x) / 1000 + "," + Number(y) / 1000, kind: Number(kind) });
  }
  const decoratedKeys = new Set(decorated.map((d) => d.key));
  return {
    impassableSlopeCells: slopeKeys.size,
    decoratedCells: decoratedKeys.size,
    pillarProps: decorated.length,
    duplicates: decorated.length - decoratedKeys.size,
    missing: [...slopeKeys].filter((key) => !decoratedKeys.has(key)),
    extra: [...decoratedKeys].filter((key) => !slopeKeys.has(key)),
    rockKinds: [...new Set(decorated.map((d) => d.kind))].sort((a, b) => a - b),
  };
}

for (const scenario of ["moba", "bota"]) {
  const report = coverage(path.join(root, "scenarios", scenario, "map.osd"));
  if (report.missing.length || report.extra.length || report.duplicates || report.pillarProps !== report.impassableSlopeCells) {
    console.error("pillar coverage " + scenario + ": FAIL " + JSON.stringify(report));
    process.exitCode = 1;
  } else {
    console.log("pillar coverage " + scenario + ": ok (" + report.decoratedCells + " of " + report.impassableSlopeCells + " impassable slope cells decorated, one prop each)");
  }
}

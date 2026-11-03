import fs from "node:fs";
import path from "node:path";
import { loadFootprints, readWpmTsv, readPropRecords, deriveBlocked, openPaintLines } from "./pathing-688g.mjs";

const args = process.argv.slice(2);
const dir = args.includes("--dir") ? args[args.indexOf("--dir") + 1] : "scenarios/moba";
const check = args.includes("--check");
const { blocked } = deriveBlocked(readWpmTsv(path.join(dir, "pathing-wpm-688g.tsv")), readPropRecords(path.join(dir, "props-688g.tsv")), loadFootprints(path.join(dir, "pathing-footprints-688g.tsv")));
const paints = openPaintLines(blocked);
const file = path.join(dir, "map.osd");
const lines = fs.readFileSync(file, "utf8").split("\n");
const isPaint = (l) => /^paint\(\d+, \d+, \d+, \d+, 0\)$/.test(l);
const first = lines.findIndex(isPaint);
if (first < 0) throw new Error("map.osd has no paint(..., 0) lines");
const next = [...lines.slice(0, first), ...paints, ...lines.slice(first).filter((l) => !isPaint(l))].join("\n");
const cur = lines.join("\n");
let nb = 0; for (const r of blocked) for (const v of r) nb += v;
if (check) {
  if (next !== cur) { console.error(`map.osd open lines differ from the derived pathing (${paints.length} derived); run apply-pathing-688g.mjs`); process.exit(1); }
  console.log(`pathing in sync: ${nb} blocked sim cells, ${paints.length} paint lines`);
} else {
  fs.writeFileSync(file, next);
  console.log(`wrote ${paints.length} paint lines, ${nb} blocked sim cells`);
}

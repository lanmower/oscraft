// Witness: edits the shipped map.bin, exports map.osd text, compiles it with osdt-data and reads the image back.
// Usage: node js/client/editor-check.mjs [path/to/osdt-data]
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { addRegion, deleteNear, exportOsd, kindAt, modelFromMap, paintKind, paintLevel, placeDoodad, placeSpawn } from "./editor.js";
import { readDoodads, readGrid } from "./terrain.js";
import { cellKind } from "./tiles.js";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "../..");
const tool = process.argv[2] || path.join(root, "rust/target/debug/osdt-data");
const m = modelFromMap(new Uint8Array(fs.readFileSync(path.join(here, "map.bin"))));

paintKind(m, 5, 5, "water", 2);
paintKind(m, 20, 20, "cliff", 1);
paintKind(m, 40, 10, "lane", 1);
paintKind(m, 8, 40, "dirt", 0);
paintLevel(m, 10, 10, 4, 2);
const doodad = placeDoodad(m, "tree", 12500, 14500);
const gone = deleteNear(m, m.doodads[0].x, m.doodads[0].y, 100);
if (!gone || gone.kind !== "doodad") throw new Error("delete");
placeSpawn(m, "skirmisher", 0, 21000, 17000);
placeSpawn(m, "warden", -1, 6000, 5000);
addRegion(m, "mid", 28, 28, 35, 31);

const text = exportOsd(m);
const dir = fs.mkdtempSync(path.join(os.tmpdir(), "editor-"));
try {
  const data = path.join(root, "rust/data");
  for (const f of fs.readdirSync(data)) if (f.endsWith(".osd") && f !== "map.osd") fs.copyFileSync(path.join(data, f), path.join(dir, f));
  fs.writeFileSync(path.join(dir, "map.osd"), text);
  const out = path.join(dir, "out.osdt");
  execFileSync(tool, [dir, out], { stdio: ["ignore", "inherit", "inherit"] });
  const bytes = new Uint8Array(fs.readFileSync(out));
  const g = readGrid(bytes);
  if (g.width !== m.width || g.height !== m.height) throw new Error("grid size");
  let cells = 0;
  for (let i = 0; i < g.width * g.height; i++, cells++) {
    if (g.levels[i] !== m.level[i]) throw new Error(`level at ${i}`);
    if (cellKind(g.flags[i], g.tiles[i]) !== kindAt(m, i % m.width, Math.floor(i / m.width))) throw new Error(`kind at ${i}`);
  }
  const doodads = readDoodads(bytes);
  if (doodads.length !== m.doodads.length || !doodads.some((d) => d.id === doodad.id && d.x === 12500)) throw new Error("doodads");
  if (!/^# region mid 28 28 36 32$/m.test(text) || !/^spawn\("warden", -1, 6000, 5000\)$/m.test(text)) throw new Error("spawn or region lines");
  console.log(`editor export ok cells=${cells}`);
} finally {
  fs.rmSync(dir, { recursive: true, force: true });
}

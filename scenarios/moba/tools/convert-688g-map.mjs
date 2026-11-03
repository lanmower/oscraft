import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { buildingPlan, buildingLines, shopLines, provenanceRows, sidecarText } from "./buildings-688g.mjs";
import { classify, provenanceFile } from "./provenance.mjs";
import { wpmBlockedGrid, writeWpmTsv, loadFootprints, deriveBlocked, openPaintLines } from "./pathing-688g.mjs";

const args = process.argv.slice(2);
const opt = (name, dflt) => (args.includes(name) ? args[args.indexOf(name) + 1] : dflt);
const src = opt("--src", null);
const outDir = opt("--out", "scenarios/moba");
if (!src) throw new Error("--src <dir> is required");

const GRID = 327;
const SIM = 256;
const GX = 35;
const GY = 35;
const UNIT_MILLI = 1000 / 64;
const toMilli = (wx, wy) => [Math.round((GX * 64 + wx + 8192) * UNIT_MILLI), Math.round((GY * 64 + 8192 - wy) * UNIT_MILLI)];

const read = (name) => fs.readFileSync(path.join(src, name));

function decodeTerrain(buf) {
  const dv = new DataView(buf.buffer, buf.byteOffset, buf.byteLength);
  let p = 4;
  const i32 = () => { const v = dv.getInt32(p, true); p += 4; return v; };
  const version = i32();
  p += 1;
  i32();
  const tilesets = Array.from({ length: i32() }, () => { const s = buf.toString("latin1", p, p + 4); p += 4; return s; });
  const cliffs = Array.from({ length: i32() }, () => { const s = buf.toString("latin1", p, p + 4); p += 4; return s; });
  const width = i32(), height = i32();
  p += 8;
  const corners = [];
  for (let k = 0; k < width * height; k++, p += 7) {
    const raw = dv.getUint16(p, true);
    const water = dv.getUint16(p + 2, true);
    corners.push({
      height: (raw & 0x3fff) - 8192,
      waterLevel: (water & 0x3fff) - 8192,
      mapEdge: (water & 0x4000) !== 0,
      groundTexture: buf[p + 4] & 0x0f,
      flags: buf[p + 4] & 0xf0,
      variation: buf[p + 5] & 0x0f,
      cliffTexture: buf[p + 5] >> 4,
      layerHeight: (buf[p + 6] & 0x0f) - 8,
    });
  }
  return { version, tilesets, cliffs, width, height, corners };
}

function decodePathing(buf) {
  const width = buf.readInt32LE(8), height = buf.readInt32LE(12);
  if (buf.length !== 16 + width * height) throw new Error("pathing size does not match its header");
  return { width, height, cells: buf.subarray(16) };
}

const DOO_RECORD = 54;
function decodeProps(buf) {
  const dv = new DataView(buf.buffer, buf.byteOffset, buf.byteLength);
  const count = dv.getInt32(12, true);
  const recs = [];
  for (let i = 0, o = 16; i < count; i++, o += DOO_RECORD) {
    recs.push({
      type: buf.toString("latin1", o, o + 4),
      variation: dv.getInt32(o + 4, true),
      x: dv.getFloat32(o + 8, true),
      y: dv.getFloat32(o + 12, true),
      z: dv.getFloat32(o + 16, true),
      angle: dv.getFloat32(o + 20, true),
      scale: [dv.getFloat32(o + 24, true), dv.getFloat32(o + 28, true), dv.getFloat32(o + 32, true)],
      skin: buf.toString("latin1", o + 36, o + 40),
      flags: buf[o + 40],
      life: buf[o + 41],
      itemTable: dv.getInt32(o + 42, true),
      itemSets: dv.getInt32(o + 46, true),
      uid: dv.getInt32(o + 50, true),
    });
  }
  const tail = buf.length - (16 + count * DOO_RECORD);
  if (tail !== 8) throw new Error(`prop block tail is ${tail} bytes, expected 8`);
  return recs;
}

function decodeStarts(buf) {
  const dv = new DataView(buf.buffer, buf.byteOffset, buf.byteLength);
  const starts = [];
  for (let o = 0; o + 40 <= buf.length; o += 4) {
    const id = dv.getInt32(o, true), type = dv.getInt32(o + 4, true), race = dv.getInt32(o + 8, true), fixed = dv.getInt32(o + 12, true);
    if (id < 0 || id > 11 || type < 0 || type > 3 || race < 0 || race > 8 || fixed < 0 || fixed > 1) continue;
    if (!buf.toString("latin1", o + 16, o + 24).startsWith("TRIGSTR_")) continue;
    const end = buf.indexOf(0, o + 16);
    if (end < 0 || end - o > 40) continue;
    const name = buf.toString("latin1", o + 16, end);
    starts.push({ id, type, race, fixed, name, x: dv.getFloat32(end + 1, true), y: dv.getFloat32(end + 5, true) });
  }
  return starts.sort((a, b) => a.id - b.id);
}

const terrain = decodeTerrain(read("terrain.bin"));
const pathing = decodePathing(read("pathing.bin"));
const props = decodeProps(read("doodads.bin"));
const starts = decodeStarts(read("info.bin"));
const W = terrain.width;
const corner = (i, j) => terrain.corners[j * W + i];

const wpmBlocked = wpmBlockedGrid(pathing.cells);
writeWpmTsv(path.join(outDir, "pathing-wpm-688g.tsv"), wpmBlocked);
const { blocked: blockedGrid } = deriveBlocked(wpmBlocked, props, loadFootprints(path.join(outDir, "pathing-footprints-688g.tsv")));
const tileOf = (cx, cy) => ({ i: cx >> 1, j: (SIM / 2 - 1) - (cy >> 1) });
const cornerZ = (c) => (c.layerHeight - 2) * 128 + c.height;
const cellZ = (cx, cy) => {
  const { i, j } = tileOf(cx, cy);
  const r = cy >> 1;
  const fx = (cx + 0.5) / 2 - i;
  const t = (cy + 0.5) / 2 - r;
  const at = (ci, cj) => cornerZ(corner(ci, cj));
  return (1 - fx) * (1 - t) * at(i, j + 1) + fx * (1 - t) * at(i + 1, j + 1) + (1 - fx) * t * at(i, j) + fx * t * at(i + 1, j);
};
const Z_PER_LEVEL = 8;
const Z_OFFSET = Number(opt("--z-offset", "384"));
const heightLevel = (z) => Math.max(0, Math.min(255, Math.round((z + Z_OFFSET) / Z_PER_LEVEL)));
const HEIGHT_STEP_MILLI = Math.round((Z_PER_LEVEL / 64) * 1000);
const WATER_TILE = 255;
const GRASS_TILE = 0;
const TILESET_TILE = Object.freeze({ Cdrt: 2, Cdrd: 2, Cvin: 3, Cgrs: 4, Clvg: 5, Zvin: 6, Crck: 7, Ndrd: 8, Zbks: 9 });
const isWaterCell = (cx, cy) => { const { i, j } = tileOf(cx, cy); return (corner(i, j).flags & 0x40) !== 0; };
const tileId = (cx, cy) => {
  if (isWaterCell(cx, cy)) return WATER_TILE;
  const { i, j } = tileOf(cx, cy);
  const name = terrain.tilesets[corner(i, j).groundTexture];
  return TILESET_TILE[name] ?? GRASS_TILE;
};

const open = blockedGrid.map((row) => Array.from(row, (v) => !v));
const runs = (valueAt) => {
  const out = [];
  for (let cy = 0; cy < SIM; cy++) {
    let cx = 0;
    while (cx < SIM) {
      const v = valueAt(cx, cy);
      let e = cx + 1;
      while (e < SIM && valueAt(e, cy) === v) e++;
      out.push([cx, cy, e, cy + 1, v]);
      cx = e;
    }
  }
  return out;
};
const rects = (list) => {
  const out = [];
  const openRects = new Map();
  for (const [x0, y0, x1, y1, v] of list) {
    const key = `${x0}:${x1}:${v}`;
    const r = openRects.get(key);
    if (r && r[3] === y0) r[3] = y1;
    else { const nr = [x0, y0, x1, y1, v]; openRects.set(key, nr); out.push(nr); }
  }
  return out;
};
const gridRect = ([x0, y0, x1, y1, v]) => [x0 + GX, y0 + GY, x1 + GX, y1 + GY, v];
const heightRuns = rects(runs((cx, cy) => heightLevel(cellZ(cx, cy)))).map(gridRect);
const tileRuns = rects(runs((cx, cy) => tileId(cx, cy))).map(gridRect);
const rampAt = (cx, cy) => { const { i, j } = tileOf(cx, cy); return (corner(i, j).flags & 0x10) !== 0 ? 1 : 0; };
const rampRuns = rects(runs((cx, cy) => rampAt(cx, cy)).filter((r) => r[4] === 1)).map(gridRect);

const legacyLines = fs.readFileSync(path.join(outDir, "map.osd"), "utf8").split("\n");
const script = fs.readFileSync(path.join(src, "script.txt"), "latin1");
const plan = buildingPlan(script);
const ownedLine = (l) => /^start\(|^spawn\("(base|hero|tower|tower_t2|tower_t3|tower_t4|barracks_melee|barracks_ranged|fountain|base_outbuilding|base_outbuilding_large)"/.test(l);
const spawnLines = legacyLines.filter((l) => l.startsWith("spawn(") && !ownedLine(l));
const generatedLines = buildingLines(plan);
const shopDepotLines = shopLines(plan);

const treeRule = script.match(/function pU takes destructable d returns boolean[\s\S]*?endfunction/)[0];
const treeCodes = new Set([...treeRule.matchAll(/'([A-Za-z0-9]{4})'/g)].map((m) => m[1]));
const treeLines = props.filter((d) => treeCodes.has(d.type)).map((d, k) => {
  const [x, y] = toMilli(d.x, d.y);
  return `prop(${k + 100}, "tree", ${x}, ${y})`;
});

const headerNotes = [
  `${opt("--label", "Dota 6.88g reference map")}, converted by scenarios/moba/tools/convert-688g-map.mjs from the extracted map members.`,
  "World to grid: 1 source unit = 15.625 milli; the 16384-unit playable square sits at grid cell (35, 35) as a 256x256 sim block.",
  "Walkable: no unwalkable pathing cell in the 2x2 pathing block of the sim cell: bit 0x02 of the wpm, or a prop pathTex footprint (pathing-688g.mjs). Outside the block: blocked.",
  `Height: each sim cell bilinear over its tile's corners, z = (layer - 2) * 128 + ground offset, level = (z + 384) / 8, height_step ${HEIGHT_STEP_MILLI} milli. Tile: ground texture of the tile's corner.`,
  "Props: tree records are placed here at their original positions; the other types are placed by scenarios/moba/tools/place-props-688g.mjs from prop-types-688g.tsv (shipped_kind).",
  "Start, building and shop lines are derived from the script by scenarios/moba/tools/buildings-688g.mjs; the other spawn lines are kept as committed.",
];
const provenance = [...headerNotes.map((note) => ({ anchor: "file", kind: classify(note), note })), ...provenanceRows(plan)];
const body = [
  `grid(${GRID}, ${GRID}, cell_milli=1000, fill=1)`,
  `height_step(${HEIGHT_STEP_MILLI})`,
  ...heightRuns.map((r) => `paint_height(${r[0]}, ${r[1]}, ${r[2]}, ${r[3]}, ${r[4]})`),
  ...tileRuns.map((r) => `paint_tile(${r[0]}, ${r[1]}, ${r[2]}, ${r[3]}, ${r[4]})`),
  ...rampRuns.map((r) => `paint_ramp(${r[0]}, ${r[1]}, ${r[2]}, ${r[3]})`),
  ...openPaintLines(blockedGrid),
  ...generatedLines,
  ...spawnLines,
  ...shopDepotLines,
  ...treeLines,
];
fs.writeFileSync(path.join(outDir, "map.osd"), [...body, ""].join("\n"));
fs.mkdirSync(path.dirname(provenanceFile(outDir, "map.osd")), { recursive: true });
fs.writeFileSync(provenanceFile(outDir, "map.osd"), sidecarText(provenance));

fs.writeFileSync(path.join(outDir, "starts-688g.tsv"), ["player_index\tid\tkey\tx\ty", ...plan.starts.map((s) => { const id = (s.side === 0 ? 1 : 7) + s.slot; return [s.k, id, `slot${id}`, s.wx, s.wy].join("\t"); })].join("\n") + "\n");

const tsv = [
  "type\tvariation\tx\ty\tz\tangle\tscale_x\tscale_y\tscale_z\tskin\tflags\tlife\titem_table\titem_sets\tuid",
  ...props.map((d) => [d.type, d.variation, d.x, d.y, d.z, d.angle, ...d.scale, d.skin, d.flags, d.life, d.itemTable, d.itemSets, d.uid].join("\t")),
];
fs.writeFileSync(path.join(outDir, "props-688g.tsv"), tsv.join("\n") + "\n");

const typeCounts = new Map();
for (const d of props) typeCounts.set(d.type, (typeCounts.get(d.type) || 0) + 1);
const priorPath = path.join(outDir, "prop-types-688g.tsv");
const priorRows = fs.existsSync(priorPath) ? fs.readFileSync(priorPath, "utf8").trim().split("\n").slice(1).map((l) => l.split("\t")) : [];
const priorMapped = new Map(priorRows.filter((c) => c[2] !== "tree" && c[4]).map((c) => [c[0], c]));
const typeRows = [...typeCounts].sort((a, b) => b[1] - a[1]).map(([type, n]) => {
  const tree = treeCodes.has(type);
  if (!tree && priorMapped.has(type)) { const c = priorMapped.get(type); return [type, n, "", "", c[4], c[5]].join("\t"); }
  return [type, n, tree ? "tree" : "", "", tree ? "tree" : "", tree ? "script tree rule (pU)" : "no name in source, no shipped kind"].join("\t");
});
fs.writeFileSync(path.join(outDir, "prop-types-688g.tsv"), ["type\tcount\ttree_rule\tname\tshipped_kind\treason", ...typeRows].join("\n") + "\n");

execFileSync(process.execPath, [fileURLToPath(new URL("./place-props-688g.mjs", import.meta.url)), "--dir", outDir], { stdio: "inherit" });
execFileSync(process.execPath, [fileURLToPath(new URL("./pillar-cells.mjs", import.meta.url))], { stdio: "inherit" });
execFileSync(process.execPath, [fileURLToPath(new URL("./place-pillars.mjs", import.meta.url))], { stdio: "inherit" });

const waterCells = Array.from({ length: SIM * SIM }, (_, k) => isWaterCell(k % SIM, Math.floor(k / SIM))).filter(Boolean).length;
console.log(JSON.stringify({
  water: { cells: waterCells, rule: "W3E corner ground flag 0x40, tile id 255" },
  terrainCorners: W * terrain.height,
  blockedSimCells: open.flat().filter((v) => !v).length,
  props: props.length,
  propTypes: typeCounts.size,
  treeProps: treeLines.length,
  editorStarts: starts.length,
  heightRuns: heightRuns.length,
  tileRuns: tileRuns.length,
  rampRuns: rampRuns.length,
  openRuns: openPaintLines(blockedGrid).length,
}));

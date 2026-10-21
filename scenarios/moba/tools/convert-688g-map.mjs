// Converts the static members of the 6.88g reference map into the scenario data under scenarios/moba.
// Input: a local directory holding the extracted members as terrain.bin, pathing.bin, doodads.bin, info.bin and script.txt.
// Output: map.osd (terrain paint lines, ramps, open cells, script-driven spawns, mapped doodads), doodads-688g.tsv,
// doodad-types-688g.tsv and starts-688g.tsv. Unit and item tables are not converted here.
// Usage: node scenarios/moba/tools/convert-688g-map.mjs --src <extract dir> [--out scenarios/moba]
import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const args = process.argv.slice(2);
const opt = (name, dflt) => (args.includes(name) ? args[args.indexOf(name) + 1] : dflt);
const src = opt("--src", null);
const outDir = opt("--out", "scenarios/moba");
if (!src) throw new Error("--src <dir> is required");

// Grid: a sim cell is 1000 milli, a source unit is 1/64 of a cell (15.625 milli). The 16384-unit playable square
// occupies a 256x256 block of the 327x327 grid at (GX, GY).
const GRID = 327;
const SIM = 256;
const GX = 35;
const GY = 35;
const UNIT_MILLI = 1000 / 64;
const toMilli = (wx, wy) => [Math.round((GX * 64 + wx + 8192) * UNIT_MILLI), Math.round((GY * 64 + 8192 - wy) * UNIT_MILLI)];

const read = (name) => fs.readFileSync(path.join(src, name));

// Terrain member: int32 version, one byte, tileset list, cliff list, width, height, two offsets, then 7-byte corners.
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

// Pathing member: 'MP3W' magic, version, width, height, then one flag byte per pathing cell. Bit 0x02 is unwalkable.
const PATH_UNWALKABLE = 0x02;
function decodePathing(buf) {
  const width = buf.readInt32LE(8), height = buf.readInt32LE(12);
  if (buf.length !== 16 + width * height) throw new Error("pathing size does not match its header");
  return { width, height, cells: buf.subarray(16) };
}

// Doodad member: header of 16 bytes, 54-byte records, then an 8-byte special-doodad block.
const DOO_RECORD = 54;
function decodeDoodads(buf) {
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
  if (tail !== 8) throw new Error(`doodad block tail is ${tail} bytes, expected 8`);
  return recs;
}

// Start info member (format 31): player records are found by their name key followed by two start floats.
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
const doodads = decodeDoodads(read("doodads.bin"));
const starts = decodeStarts(read("info.bin"));
const W = terrain.width;
const corner = (i, j) => terrain.corners[j * W + i];

// Sim cell (cx, cy): cy counts down from the north edge. Its pathing block is 2x2 pathing cells.
const cellBlocked = (cx, cy) => {
  const rows = [510 - 2 * cy, 511 - 2 * cy], cols = [2 * cx, 2 * cx + 1];
  return rows.some((r) => cols.some((c) => (pathing.cells[r * pathing.width + c] & PATH_UNWALKABLE) !== 0));
};
const tileOf = (cx, cy) => ({ i: cx >> 1, j: (SIM / 2 - 1) - (cy >> 1) });
const cornerZ = (c) => (c.layerHeight - 2) * 128 + c.height;
// Sim cell height in z units, bilinear over the four corners of its tile.
const cellZ = (cx, cy) => {
  const { i, j } = tileOf(cx, cy);
  const r = cy >> 1;
  const fx = (cx + 0.5) / 2 - i;
  const t = (cy + 0.5) / 2 - r;
  const at = (ci, cj) => cornerZ(corner(ci, cj));
  return (1 - fx) * (1 - t) * at(i, j + 1) + fx * (1 - t) * at(i + 1, j + 1) + (1 - fx) * t * at(i, j) + fx * t * at(i + 1, j);
};
const Z_PER_LEVEL = 8;
const Z_OFFSET = 384;
const heightLevel = (z) => Math.max(0, Math.min(255, Math.round((z + Z_OFFSET) / Z_PER_LEVEL)));
const HEIGHT_STEP_MILLI = Math.round((Z_PER_LEVEL / 64) * 1000);
// Water comes from the W3E corner flags: bit 0x40 of a corner's ground byte marks its tile as water. A water cell takes the
// water tile id below (its ground under the water is the riverbed). Every other cell takes its tile id from its ground
// tileset name: the Cdrt and Cdrd dirt patches are the lane paths (packed earth), Cvin is the dirt, everything else is grass.
// The index into the W3E tileset list is not a tile id, so the mapping goes through the tileset name.
const WATER_TILE = 255;
const GRASS_TILE = 0;
const TILESET_TILE = Object.freeze({ Cdrt: 2, Cdrd: 2, Cvin: 3 });
const isWaterCell = (cx, cy) => { const { i, j } = tileOf(cx, cy); return (corner(i, j).flags & 0x40) !== 0; };
const tileId = (cx, cy) => {
  if (isWaterCell(cx, cy)) return WATER_TILE;
  const { i, j } = tileOf(cx, cy);
  const name = terrain.tilesets[corner(i, j).groundTexture];
  return TILESET_TILE[name] ?? GRASS_TILE;
};

const open = Array.from({ length: SIM }, (_, cy) => Array.from({ length: SIM }, (_, cx) => !cellBlocked(cx, cy)));
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
// Joins horizontal runs with the same x span and value on consecutive rows into rectangles.
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
const openRuns = rects(runs((cx, cy) => (open[cy][cx] ? 0 : 1)).filter((r) => r[4] === 0)).map(gridRect);
const heightRuns = rects(runs((cx, cy) => heightLevel(cellZ(cx, cy)))).map(gridRect);
const tileRuns = rects(runs((cx, cy) => tileId(cx, cy))).map(gridRect);
const rampAt = (cx, cy) => { const { i, j } = tileOf(cx, cy); return (corner(i, j).flags & 0x10) !== 0 ? 1 : 0; };
const rampRuns = rects(runs((cx, cy) => rampAt(cx, cy)).filter((r) => r[4] === 1)).map(gridRect);

// The homes and script-driven lines stay as they were in the committed scenario; only the decoded members change.
const legacyLines = fs.readFileSync(path.join(outDir, "map.osd"), "utf8").split("\n");
// Player starts come from info.bin: sorted by slot id, player index k takes the k-th start, team alternates by index.
// Names are the source's trigger-string keys, so the committed table carries the neutral key slot<id> instead.
const startLines = starts.map((s, k) => {
  const [x, y] = toMilli(s.x, s.y);
  return `start(${k}, ${x}, ${y}, team=${k % 2})`;
});
const script = fs.readFileSync(path.join(src, "script.txt"), "latin1");
const spawnLines = legacyLines.filter((l) => l.startsWith("spawn(") && !l.startsWith('spawn("base"'));
const shopLines = legacyLines.filter((l) => /^doodad\(\d+, "(shop|side_shop|secret_shop)"/.test(l));

// Ancients: the literal create calls in the script for the two ancient codes; the side convention sets the player.
const ANCIENT_SIDES = { etol: 0, unpl: 1 };
const ancientLines = [];
for (const m of script.matchAll(/CreateUnit\([^,]+,'(etol|unpl)',(-?\d+),(-?\d+),/g)) {
  const [, code, wx, wy] = m;
  if (ancientLines.some((l) => l.includes(`# ancient ${code} `))) continue;
  const [mx, my] = toMilli(Number(wx), Number(wy));
  ancientLines.push(`# ancient ${code} at ${wx},${wy} from the source script`, `spawn("base", ${ANCIENT_SIDES[code]}, ${mx}, ${my})`);
}
if (ancientLines.filter((l) => l.startsWith("spawn(")).length !== 2) throw new Error("expected two literal ancient placements in the script");

// Trees: the script's tree rule lists the destructable codes that are trees. Each tree record is one tree doodad at its
// original position (no re-anchoring); other types go through the type table and the placement script.
const treeRule = script.match(/function pU takes destructable d returns boolean[\s\S]*?endfunction/)[0];
const treeCodes = new Set([...treeRule.matchAll(/'([A-Za-z0-9]{4})'/g)].map((m) => m[1]));
const treeLines = doodads.filter((d) => treeCodes.has(d.type)).map((d, k) => {
  const [x, y] = toMilli(d.x, d.y);
  return `doodad(${k + 100}, "tree", ${x}, ${y})`;
});

const header = [
  "# Dota 6.88g reference map, converted by scenarios/moba/tools/convert-688g-map.mjs from the extracted map members.",
  "# World to grid: 1 source unit = 15.625 milli; the 16384-unit playable square sits at grid cell (35, 35) as a 256x256 sim block.",
  "# Walkable: no unwalkable pathing cell (bit 0x02) in the 2x2 pathing block of the sim cell. Outside the block: blocked.",
  `# Height: each sim cell bilinear over its tile's corners, z = (layer - 2) * 128 + ground offset, level = (z + 384) / 8, height_step ${HEIGHT_STEP_MILLI} milli. Tile: ground texture of the tile's corner.`,
  "# Doodads: tree records are placed here at their original positions; the other types are placed by scenarios/moba/tools/place-doodads-688g.mjs from doodad-types-688g.tsv (shipped_kind).",
  "# Spawn lines are script-driven and kept as they were. Do not edit by hand; edit the script.",
];
const body = [
  `grid(${GRID}, ${GRID}, cell_milli=1000, fill=1)`,
  `height_step(${HEIGHT_STEP_MILLI})`,
  ...heightRuns.map((r) => `paint_height(${r[0]}, ${r[1]}, ${r[2]}, ${r[3]}, ${r[4]})`),
  ...tileRuns.map((r) => `paint_tile(${r[0]}, ${r[1]}, ${r[2]}, ${r[3]}, ${r[4]})`),
  ...rampRuns.map((r) => `paint_ramp(${r[0]}, ${r[1]}, ${r[2]}, ${r[3]})`),
  ...openRuns.map((r) => `paint(${r[0]}, ${r[1]}, ${r[2]}, ${r[3]}, 0)`),
  ...startLines,
  ...ancientLines,
  ...spawnLines,
  ...shopLines,
  ...treeLines,
];
fs.writeFileSync(path.join(outDir, "map.osd"), [...header, ...body, ""].join("\n"));

fs.writeFileSync(path.join(outDir, "starts-688g.tsv"), ["player_index\tid\tkey\tx\ty", ...starts.map((s, k) => [k, s.id, `slot${s.id}`, s.x, s.y].join("\t"))].join("\n") + "\n");

const tsv = [
  "type\tvariation\tx\ty\tz\tangle\tscale_x\tscale_y\tscale_z\tskin\tflags\tlife\titem_table\titem_sets\tuid",
  ...doodads.map((d) => [d.type, d.variation, d.x, d.y, d.z, d.angle, ...d.scale, d.skin, d.flags, d.life, d.itemTable, d.itemSets, d.uid].join("\t")),
];
fs.writeFileSync(path.join(outDir, "doodads-688g.tsv"), tsv.join("\n") + "\n");

// Doodad type table: one row per four-letter type code. The source holds no type names, so the name column is empty.
// Tree codes come from the script rule; every other code keeps its previously recorded shipped kind.
const typeCounts = new Map();
for (const d of doodads) typeCounts.set(d.type, (typeCounts.get(d.type) || 0) + 1);
const priorPath = path.join(outDir, "doodad-types-688g.tsv");
const priorRows = fs.existsSync(priorPath) ? fs.readFileSync(priorPath, "utf8").trim().split("\n").slice(1).map((l) => l.split("\t")) : [];
const priorMapped = new Map(priorRows.filter((c) => c[2] !== "tree" && c[4]).map((c) => [c[0], c]));
const typeRows = [...typeCounts].sort((a, b) => b[1] - a[1]).map(([type, n]) => {
  const tree = treeCodes.has(type);
  if (!tree && priorMapped.has(type)) { const c = priorMapped.get(type); return [type, n, "", "", c[4], c[5]].join("\t"); }
  return [type, n, tree ? "tree" : "", "", tree ? "tree" : "", tree ? "script tree rule (pU)" : "no name in source, no shipped kind"].join("\t");
});
fs.writeFileSync(path.join(outDir, "doodad-types-688g.tsv"), ["type\tcount\ttree_rule\tname\tshipped_kind\treason", ...typeRows].join("\n") + "\n");

execFileSync(process.execPath, [fileURLToPath(new URL("./place-doodads-688g.mjs", import.meta.url)), "--dir", outDir], { stdio: "inherit" });

// Water: every sim cell whose W3E tile has the water flag (0x40), whatever its walkability.
const waterCells = Array.from({ length: SIM * SIM }, (_, k) => isWaterCell(k % SIM, Math.floor(k / SIM))).filter(Boolean).length;
console.log(JSON.stringify({
  water: { cells: waterCells, rule: "W3E corner ground flag 0x40, tile id 255" },
  terrainCorners: W * terrain.height,
  blockedSimCells: open.flat().filter((v) => !v).length,
  doodads: doodads.length,
  doodadTypes: typeCounts.size,
  treeDoodads: treeLines.length,
  starts: starts.length,
  heightRuns: heightRuns.length,
  tileRuns: tileRuns.length,
  rampRuns: rampRuns.length,
  openRuns: openRuns.length,
}));

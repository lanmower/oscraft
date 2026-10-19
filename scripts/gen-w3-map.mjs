// Decodes the static data of a Warcraft III map archive (DotA Allstars 6.88g) into scenarios/moba:
// terrain (war3map.w3e), pathing (war3map.wpm), doodads (war3map.doo) and player starts (war3map.w3i).
// Usage: node scripts/gen-w3-map.mjs --src <dir with the extracted war3map.* members> [--out scenarios/moba]
// Script-driven lines of map.osd (spawn) are kept verbatim; every other script-driven file is untouched.
import fs from "node:fs";
import path from "node:path";

const args = process.argv.slice(2);
const opt = (name, dflt) => (args.includes(name) ? args[args.indexOf(name) + 1] : dflt);
const src = opt("--src", null);
const outDir = opt("--out", "scenarios/moba");
if (!src) throw new Error("--src <dir> is required");

// World to grid. A w3 world unit is 1/64 of a sim cell (64 units, one 1000 milli cell), so one unit is 15.625 milli.
// The 16384-unit w3 playable square (w3e corners at -8192..8192) occupies a 256x256 block of the 327x327 grid at (GX, GY).
const GRID = 327;
const SIM = 256;
const GX = 35;
const GY = 35;
const UNIT_MILLI = 1000 / 64;
const toMilli = (wx, wy) => [Math.round((GX * 64 + wx + 8192) * UNIT_MILLI), Math.round((GY * 64 + 8192 - wy) * UNIT_MILLI)];

const read = (name) => fs.readFileSync(path.join(src, name));

// war3map.w3e: header 'W3E!', version, tileset, custom flag, tileset ids, cliff ids, width, height, offsets, then 7-byte corners.
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
  const offsetX = dv.getFloat32(p, true), offsetY = dv.getFloat32(p + 4, true);
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
  return { version, tilesets, cliffs, width, height, offsetX, offsetY, corners };
}

// war3map.wpm: 'MP3W', version, width, height, then one flag byte per pathing cell. Bit 0x02 is unwalkable.
const PATH_UNWALKABLE = 0x02;
function decodePathing(buf) {
  const width = buf.readInt32LE(8), height = buf.readInt32LE(12);
  if (buf.length !== 16 + width * height) throw new Error("wpm size does not match its header");
  return { width, height, cells: buf.subarray(16) };
}

// war3map.doo: 'W3do', version, subversion, count, then 54-byte records, then an 8-byte special-doodad block.
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
  if (tail !== 8) throw new Error(`doo tail is ${tail} bytes, expected the 8-byte special-doodad block`);
  return recs;
}

// war3map.w3i (format 31): each player record is id, type, race, fixed start, name, x, y, then four ints.
// Records are found by their TRIGSTR_ name followed by two start floats; ids 0..11 are the w3 player slots.
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

const terrain = decodeTerrain(read("war3map.w3e"));
const pathing = decodePathing(read("war3map.wpm"));
const doodads = decodeDoodads(read("war3map.doo"));
const starts = decodeStarts(read("war3map.w3i"));
const W = terrain.width;
const corner = (i, j) => terrain.corners[j * W + i];

// Sim cell (cx, cy): cy counts down from the top (north) edge, as the osd y axis does.
// Pathing cells (2cx..2cx+1, 510-2cy..511-2cy) cover it; pathing row index is floor((wy + 8192) / 32), no flip.
const cellBlocked = (cx, cy) => {
  const rows = [510 - 2 * cy, 511 - 2 * cy], cols = [2 * cx, 2 * cx + 1];
  return rows.some((r) => cols.some((c) => (pathing.cells[r * pathing.width + c] & PATH_UNWALKABLE) !== 0));
};
const tileOf = (cx, cy) => ({ i: cx >> 1, j: (SIM / 2 - 1) - (cy >> 1) });
const cellHeight = (cx, cy) => {
  const { i, j } = tileOf(cx, cy);
  const hs = [corner(i, j), corner(i + 1, j), corner(i, j + 1), corner(i + 1, j + 1)].map((c) => c.height);
  return Math.round(hs.reduce((a, b) => a + b, 0) / 4);
};
const HEIGHT_OFFSET = 105;
const HEIGHT_PER_LEVEL = 2;
const heightLevel = (h) => Math.max(0, Math.min(255, Math.floor((h + HEIGHT_OFFSET) / HEIGHT_PER_LEVEL)));
const tileId = (cx, cy) => { const { i, j } = tileOf(cx, cy); return corner(i, j).groundTexture; };

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
  const open = new Map();
  for (const [x0, y0, x1, y1, v] of list) {
    const key = `${x0}:${x1}:${v}`;
    const r = open.get(key);
    if (r && r[3] === y0) r[3] = y1;
    else { const nr = [x0, y0, x1, y1, v]; open.set(key, nr); out.push(nr); }
  }
  return out;
};
const gridRect = ([x0, y0, x1, y1, v]) => [x0 + GX, y0 + GY, x1 + GX, y1 + GY, v];
const openRuns = rects(runs((cx, cy) => (open[cy][cx] ? 0 : 1)).filter((r) => r[4] === 0)).map(gridRect);
const heightRuns = rects(runs((cx, cy) => heightLevel(cellHeight(cx, cy)))).map(gridRect);
const tileRuns = rects(runs((cx, cy) => tileId(cx, cy))).map(gridRect);

const startLines = starts.map((s, k) => {
  const [mx, my] = toMilli(s.x, s.y);
  const team = k < 5 ? 0 : 1;
  return `start(${k}, ${mx}, ${my}, team=${team})`;
});

const old = fs.readFileSync(path.join(outDir, "map.osd"), "utf8").split("\n");
const spawnLines = old.filter((l) => l.startsWith("spawn("));

const header = [
  "# Dota Allstars 6.88g map (war3map.w3e, war3map.wpm, war3map.w3i) decoded by scripts/gen-w3-map.mjs.",
  "# World to grid: 1 w3 unit = 15.625 milli; the 16384-unit playable square sits at grid cell (35, 35) as a 256x256 sim block.",
  "# Walkable: no unwalkable pathing cell (bit 0x02) in the 2x2 pathing block of the sim cell. Outside the block: blocked.",
  "# Height: ground height averaged over the tile corners, level = (h + 105) / 2, height_step 31 milli. Tile: ground texture of the tile's corner.",
  "# Doodads are in doodads-688g.tsv (decoded, not placed: the w3 doodad type to kind table is not in the archive).",
  "# Spawn lines are script-driven and kept as they were. Do not edit by hand; edit the script.",
];
const body = [
  `grid(${GRID}, ${GRID}, cell_milli=1000, fill=1)`,
  "height_step(31)",
  ...heightRuns.map((r) => `paint_height(${r[0]}, ${r[1]}, ${r[2]}, ${r[3]}, ${r[4]})`),
  ...tileRuns.map((r) => `paint_tile(${r[0]}, ${r[1]}, ${r[2]}, ${r[3]}, ${r[4]})`),
  ...openRuns.map((r) => `paint(${r[0]}, ${r[1]}, ${r[2]}, ${r[3]}, 0)`),
  ...startLines,
  ...spawnLines,
];
fs.writeFileSync(path.join(outDir, "map.osd"), [...header, ...body, ""].join("\n"));

const tsv = [
  "type\tvariation\tx\ty\tz\tangle\tscale_x\tscale_y\tscale_z\tskin\tflags\tlife\titem_table\titem_sets\tuid",
  ...doodads.map((d) => [d.type, d.variation, d.x, d.y, d.z, d.angle, ...d.scale, d.skin, d.flags, d.life, d.itemTable, d.itemSets, d.uid].join("\t")),
];
fs.writeFileSync(path.join(outDir, "doodads-688g.tsv"), tsv.join("\n") + "\n");

const summary = {
  terrain: { corners: W * terrain.height, tiles: (W - 1) * (terrain.height - 1), tilesets: terrain.tilesets.length, cliffs: terrain.cliffs.length },
  pathing: { cells: pathing.width * pathing.height, sim: `${SIM}x${SIM}`, blockedSimCells: open.flat().filter((v) => !v).length },
  doodads: doodads.length,
  starts: starts.map((s, k) => ({ k, id: s.id, name: s.name, x: s.x, y: s.y, cellOpen: open[Math.floor((8192 - s.y) / 64)][Math.floor((s.x + 8192) / 64)] })),
  openRuns: openRuns.length, heightRuns: heightRuns.length, tileRuns: tileRuns.length,
};
console.log(JSON.stringify(summary));

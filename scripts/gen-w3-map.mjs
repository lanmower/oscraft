// Decodes the static data of a Warcraft III map archive (DotA Allstars 6.88g) into scenarios/moba:
// terrain (war3map.w3e), pathing (war3map.wpm), doodads (war3map.doo) and player starts (war3map.w3i).
// Usage: node scripts/gen-w3-map.mjs --src <dir with the extracted war3map.* members> [--out scenarios/moba]
// Script-driven lines of map.osd (spawn) are kept verbatim; every other script-driven file is untouched.
import fs from "node:fs";
import path from "node:path";
import { reanchor } from "./w3-placements.mjs";

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
// Corner height in w3 z units: its layer (layer 2 is the ground, one layer is 128 z) plus its ground offset.
const cornerZ = (c) => (c.layerHeight - 2) * 128 + c.height;
// Sim cell height in z units, bilinear over the four corners of its tile: the corners are the tile's edges, so a
// one-layer step between neighbouring tiles becomes a slope across them, the way the 6.88 engine draws it.
// Tile r (sim rows 2r, 2r+1) has its north edge at corner row j+1 and its south edge at corner row j, j = 127 - r.
const cellZ = (cx, cy) => {
  const { i, j } = tileOf(cx, cy);
  const r = cy >> 1;
  const fx = (cx + 0.5) / 2 - i;
  const t = (cy + 0.5) / 2 - r;
  const at = (ci, cj) => cornerZ(corner(ci, cj));
  return (1 - fx) * (1 - t) * at(i, j + 1) + fx * (1 - t) * at(i + 1, j + 1) + (1 - fx) * t * at(i, j) + fx * t * at(i + 1, j);
};
// One level is 8 z, which is 125 milli of a 1000-milli sim cell (64 w3 units to a cell).
const Z_PER_LEVEL = 8;
const Z_OFFSET = 384;
const heightLevel = (z) => Math.max(0, Math.min(255, Math.round((z + Z_OFFSET) / Z_PER_LEVEL)));
const HEIGHT_STEP_MILLI = Math.round((Z_PER_LEVEL / 64) * 1000);
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
const heightRuns = rects(runs((cx, cy) => heightLevel(cellZ(cx, cy)))).map(gridRect);
const tileRuns = rects(runs((cx, cy) => tileId(cx, cy))).map(gridRect);
// The W3E ramp flag (bit 0x10 of the ground byte) of the tile's corner, the same corner the tile id is read from.
const rampAt = (cx, cy) => { const { i, j } = tileOf(cx, cy); return (corner(i, j).flags & 0x10) !== 0 ? 1 : 0; };
const rampRuns = rects(runs((cx, cy) => rampAt(cx, cy)).filter((r) => r[4] === 1)).map(gridRect);

// The sim takes each start as the player home (loader: open_player, base_fields), and the decoded w3i starts are not
// the team homes: the 6.88 start of player 1 is about 160 cells from the Dire ancient. The homes therefore come from
// --legacy (the previous map.osd: start, spawn and shop doodad lines), and the decoded starts go to starts-688g.tsv.
const legacy = opt("--legacy", null);
const legacyLines = legacy ? fs.readFileSync(legacy, "utf8").split("\n") : fs.readFileSync(path.join(outDir, "map.osd"), "utf8").split("\n");
const startLines = legacyLines.filter((l) => l.startsWith("start("));
const jass = fs.readFileSync(path.join(src, "war3map.j"), "latin1");
const spawnLinesLegacy = legacyLines.filter((l) => l.startsWith("spawn(") && !l.startsWith('spawn("base"'));

// Towers and barracks move to their 6.88 positions. Each group is one side's script codes; its positions are the literal
// CreateUnit calls for those codes in war3map.j. The variable-coordinate calls re-create the same points (their X/Y are
// GetUnitX/Y of the literal units), so they add no position. Tower codes: uzig (Scourge, player 1; the Ziggurat, from
// docs/CONVERGENCE_688G.md) and emow (Sentinel, player 0; the same tower-type table as uzig, war3map.j:91724 and 91751).
// Barracks codes are inferred, not named in the archive: on each side the two codes that have exactly six literal creates
// (three each) and sit in the lane-sized groups are taken as the barracks: eaom+eaoe (Sentinel, player 0) and usep+utod
// (Scourge, player 1). Each legacy line of a group takes the nearest unclaimed decoded position that is an open cell; a
// line with no open position keeps its legacy position.
const literalPositions = (codes) => {
  const out = [];
  for (const m of jass.matchAll(/CreateUnit\([^,]+,'([^']+)',(-?[\d.]+),(-?[\d.]+),/g)) {
    if (!codes.includes(m[1])) continue;
    const wx = Number(m[2]), wy = Number(m[3]);
    if (!out.some((p) => p.wx === wx && p.wy === wy)) out.push({ wx, wy });
  }
  return out;
};
const groups = [
  { player: 0, match: /^spawn\("(tower(?:_t\d)?)", 0, /, codes: ["emow"] },
  { player: 1, match: /^spawn\("(tower(?:_t\d)?)", 1, /, codes: ["uzig"] },
  { player: 0, match: /^spawn\("(barracks_(?:melee|ranged))", 0, /, codes: ["eaom", "eaoe"] },
  { player: 1, match: /^spawn\("(barracks_(?:melee|ranged))", 1, /, codes: ["usep", "utod"] },
];
const groupPositions = groups.map((g) => literalPositions(g.codes));
const groupClaimed = groups.map(() => new Set());
const towerCell = (p) => ({ cx: Math.floor((p.wx + 8192) / 64), cy: Math.floor((8192 - p.wy) / 64) });
const spawnLines = spawnLinesLegacy.map((line) => {
  const gi = groups.findIndex((g) => g.match.test(line));
  if (gi < 0) return line;
  const m = line.match(/^spawn\("([^"]+)", (\d+), (-?\d+), (-?\d+)\)$/);
  const lx = Number(m[3]) / 1000 - GX, ly = Number(m[4]) / 1000 - GY;
  let best = null;
  for (const p of groupPositions[gi]) {
    const { cx, cy } = towerCell(p);
    if (groupClaimed[gi].has(`${cx},${cy}`) || !open[cy]?.[cx]) continue;
    const d = (cx + 0.5 - lx) ** 2 + (cy + 0.5 - ly) ** 2;
    if (!best || d < best.d) best = { p, cx, cy, d };
  }
  if (!best) return line;
  groupClaimed[gi].add(`${best.cx},${best.cy}`);
  const [x, y] = toMilli(best.p.wx, best.p.wy);
  return `spawn("${m[1]}", ${m[2]}, ${x}, ${y})`;
});

// Ancients come from the script (war3map.j): the literal CreateUnit lines for the ancient codes (etol Sentinel,
// unpl Scourge). The player for each side is the side convention (Sentinel = player 0, Scourge = player 1).
const ANCIENT_SIDES = { etol: 0, unpl: 1 };
const ancientLines = [];
for (const m of jass.matchAll(/CreateUnit\([^,]+,'(etol|unpl)',(-?\d+),(-?\d+),/g)) {
  const [, code, wx, wy] = m;
  if (ancientLines.some((l) => l.includes(`# ancient ${code} `))) continue;
  const [mx, my] = toMilli(Number(wx), Number(wy));
  ancientLines.push(`# ancient ${code} at ${wx},${wy} from war3map.j`, `spawn("base", ${ANCIENT_SIDES[code]}, ${mx}, ${my})`);
}
if (ancientLines.filter((l) => l.startsWith("spawn(")).length !== 2) throw new Error("expected two literal ancient placements in war3map.j");

// Trees: the script tree rule (function pU) lists the destructable codes that are trees. Those doodad records become
// tree doodads, one per record. Every other type has no name in the archive and no shipped kind, so it stays unmatched.
const pU = jass.match(/function pU takes destructable d returns boolean[\s\S]*?endfunction/)[0];
const treeCodes = new Set([...pU.matchAll(/'([A-Za-z0-9]{4})'/g)].map((m) => m[1]));
const treeLines = doodads.filter((d) => treeCodes.has(d.type)).map((d, k) => {
  const [x, y] = toMilli(d.x, d.y);
  return `doodad(${k + 100}, "tree", ${x}, ${y})`;
});
const shopLines = legacyLines.filter((l) => /^doodad\(\d+, "(shop|side_shop|secret_shop)"/.test(l));

const header = [
  "# Dota Allstars 6.88g map (war3map.w3e, war3map.wpm, war3map.w3i) decoded by scripts/gen-w3-map.mjs.",
  "# World to grid: 1 w3 unit = 15.625 milli; the 16384-unit playable square sits at grid cell (35, 35) as a 256x256 sim block.",
  "# Walkable: no unwalkable pathing cell (bit 0x02) in the 2x2 pathing block of the sim cell. Outside the block: blocked.",
  `# Height: each sim cell bilinear over its tile's corners, z = (layer - 2) * 128 + ground offset, level = (z + 384) / 8, height_step ${HEIGHT_STEP_MILLI} milli. Tile: ground texture of the tile's corner.`,
  "# Doodads are in doodads-688g.tsv (decoded, not placed: the w3 doodad type to kind table is not in the archive).",
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

fs.writeFileSync(path.join(outDir, "starts-688g.tsv"), ["player_index\tid\tname\tx\ty", ...starts.map((s, k) => [k, s.id, s.name, s.x, s.y].join("\t"))].join("\n") + "\n");

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
  openRuns: openRuns.length, heightRuns: heightRuns.length, tileRuns: tileRuns.length, rampRuns: rampRuns.length, rampCells: rampRuns.reduce((n, r) => n + (r[2] - r[0]) * (r[3] - r[1]), 0),
};
console.log(JSON.stringify(summary));


if (args.includes("--reanchor")) {
  const rows = reanchor(outDir);
  console.log(["file", "unit", "player", "old_x", "old_y", "new_x", "new_y", "rule"].join("\t"));
  for (const r of rows) console.log(r);
}

// Doodad type table: one row per four-letter type code in war3map.doo. The archive holds no type names (the editor's
// Doodads.slk is not in it), so the name column is empty and the kind comes only from the script's tree rule.
const typeCounts = new Map();
for (const d of doodads) typeCounts.set(d.type, (typeCounts.get(d.type) || 0) + 1);
const typeRows = [...typeCounts].sort((a, b) => b[1] - a[1]).map(([type, n]) => {
  const tree = treeCodes.has(type);
  return [type, n, tree ? "tree" : "", "", tree ? "tree" : "", tree ? "script tree rule (pU)" : "no name in archive, no shipped kind"].join("\t");
});
fs.writeFileSync(path.join(outDir, "doodad-types-688g.tsv"), ["type\tcount\ttree_rule\tname\tshipped_kind\treason", ...typeRows].join("\n") + "\n");
console.log(JSON.stringify({ doodadTypes: typeCounts.size, treeTypes: [...typeCounts.keys()].filter((t) => treeCodes.has(t)).length, treeDoodads: treeLines.length, unmatchedRecords: doodads.length - treeLines.length, ancients: ancientLines.filter((l) => l.startsWith("spawn(")) }));

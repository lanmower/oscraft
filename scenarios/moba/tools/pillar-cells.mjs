import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { emitSplit } from "./provenance.mjs";

const toolDir = path.dirname(fileURLToPath(import.meta.url));
const mapPath = path.join(toolDir, "..", "map.osd");
const outPath = path.join(toolDir, "..", "pillar-cells.tsv");

const CLIFF_BIT = 2;
const BLOCKED_BIT = 1;
const WATER_TILE_ID = 255;
const CELL_RAMP = 4;
const MIN_RISE = 8;
const BLOCK_SIZE = 4;
const MAX_ROWS = 400;
const IGNORED_VERBS = new Set(["height_step", "start", "spawn", "prop"]);
const ARITY = { paint: 5, paint_tile: 5, paint_height: 5, paint_ramp: 4 };
const CALL = /^([a-z_]+)\((.*)\)$/;
const INT = /^-?\d+$/;
const NAMED = /^([a-z_]+)=(-?\d+)$/;
const NEIGHBOURS = [[-1, -1], [0, -1], [1, -1], [-1, 0], [1, 0], [-1, 1], [0, 1], [1, 1]];

function fail(lineNo, message) {
  throw new Error(`${path.basename(mapPath)}:${lineNo}: ${message}`);
}

function parseArgs(lineNo, body) {
  const positional = [];
  const named = new Map();
  if (body.trim() === "") return { positional, named };
  for (const part of body.split(",")) {
    const token = part.trim();
    const kv = NAMED.exec(token);
    if (kv) named.set(kv[1], Number(kv[2]));
    else if (INT.test(token)) positional.push(Number(token));
    else fail(lineNo, `argument is not an integer: ${token}`);
  }
  return { positional, named };
}

function loadGrid() {
  const lines = fs.readFileSync(mapPath, "utf8").split(/\r?\n/);
  let grid = null;
  lines.forEach((raw, index) => {
    const lineNo = index + 1;
    const line = raw.trim();
    if (line === "" || line.startsWith("#")) return;
    const call = CALL.exec(line);
    if (!call) fail(lineNo, `unparsed line: ${line.slice(0, 80)}`);
    const [, verb, body] = call;
    if (verb === "grid") {
      if (grid) fail(lineNo, "grid() appears twice");
      const { positional, named } = parseArgs(lineNo, body);
      if (positional.length !== 2) fail(lineNo, "grid() takes width and height");
      const [w, h] = positional;
      const fill = named.get("fill") ?? 0;
      if (w <= 0 || h <= 0) fail(lineNo, `grid ${w}x${h} is empty`);
      if (fill < 0 || fill > 255) fail(lineNo, `grid fill ${fill} must be 0..255`);
      const cells = w * h;
      grid = { w, h, flags: new Uint8Array(cells).fill(fill), tiles: new Uint8Array(cells), heights: new Uint8Array(cells) };
      return;
    }
    if (IGNORED_VERBS.has(verb)) return;
    if (!(verb in ARITY)) fail(lineNo, `unknown verb ${verb}`);
    if (!grid) fail(lineNo, `${verb}() before grid()`);
    const { positional } = parseArgs(lineNo, body);
    if (positional.length !== ARITY[verb]) fail(lineNo, `${verb}() takes ${ARITY[verb]} integers`);
    const [x0, y0, x1, y1, value = 0] = positional;
    if (!(0 <= x0 && x0 <= x1 && x1 <= grid.w && 0 <= y0 && y0 <= y1 && y1 <= grid.h)) {
      fail(lineNo, `${verb}(${x0}, ${y0}, ${x1}, ${y1}) is outside the ${grid.w}x${grid.h} grid`);
    }
    if (verb !== "paint_ramp" && (value < 0 || value > 255)) fail(lineNo, `${verb} value ${value} must be 0..255`);
    for (let y = y0; y < y1; y++) {
      for (let x = x0; x < x1; x++) {
        const i = y * grid.w + x;
        if (verb === "paint") grid.flags[i] = value;
        else if (verb === "paint_ramp") grid.flags[i] |= CELL_RAMP;
        else if (verb === "paint_tile") grid.tiles[i] = value;
        else grid.heights[i] = value;
      }
    }
  });
  if (!grid) throw new Error(`${path.basename(mapPath)}: no grid() line`);
  return grid;
}

function findCandidates({ w, h, flags, tiles, heights }) {
  const isWater = (i) => (flags[i] & CLIFF_BIT) === 0 && tiles[i] === WATER_TILE_ID;
  const isImpassable = (i) => (flags[i] & (BLOCKED_BIT | CLIFF_BIT)) !== 0 || isWater(i);
  const candidates = [];
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const i = y * w + x;
      if (!isImpassable(i) || isWater(i)) continue;
      let open = false;
      let neighborHeight = -1;
      for (const [dx, dy] of NEIGHBOURS) {
        const nx = x + dx;
        const ny = y + dy;
        if (nx < 0 || ny < 0 || nx >= w || ny >= h) continue;
        const j = ny * w + nx;
        if (isImpassable(j)) continue;
        open = true;
        neighborHeight = Math.max(neighborHeight, heights[j]);
      }
      if (!open || heights[i] - neighborHeight < MIN_RISE) continue;
      candidates.push({
        x,
        y,
        height: heights[i],
        neighborHeight,
        cliffBit: (flags[i] & CLIFF_BIT) !== 0 ? 1 : 0,
        blockedBit: (flags[i] & BLOCKED_BIT) !== 0 ? 1 : 0,
      });
    }
  }
  return candidates;
}

function spaceOut(candidates) {
  const ranked = [...candidates].sort((a, b) => b.height - a.height || a.y - b.y || a.x - b.x);
  const taken = new Set();
  const kept = [];
  for (const c of ranked) {
    const block = `${Math.floor(c.x / BLOCK_SIZE)},${Math.floor(c.y / BLOCK_SIZE)}`;
    if (taken.has(block)) continue;
    taken.add(block);
    kept.push(c);
  }
  return kept;
}

const grid = loadGrid();
const candidates = findCandidates(grid);
const spaced = spaceOut(candidates);
const rows = spaced.slice(0, MAX_ROWS).sort((a, b) => a.y - b.y || a.x - b.x);

const text = [
  `# Pillar candidates from ${path.basename(mapPath)} (${grid.w}x${grid.h} grid), generated by scenarios/moba/tools/pillar-cells.mjs: one per ${BLOCK_SIZE}x${BLOCK_SIZE} block, highest first, capped at ${MAX_ROWS} rows.`,
  ["x", "y", "height", "neighborHeight", "cliffBit", "blockedBit"].join("\t"),
  ...rows.map((r) => [r.x, r.y, r.height, r.neighborHeight, r.cliffBit, r.blockedBit].join("\t")),
].join("\n") + "\n";
emitSplit(path.dirname(outPath), path.basename(outPath), text, false);

console.log(`grid ${grid.w}x${grid.h}`);
console.log(`candidates before spacing: ${candidates.length}`);
console.log(`candidates after spacing: ${spaced.length}`);
console.log(`written: ${rows.length}${spaced.length > MAX_ROWS ? ` (cap ${MAX_ROWS}, ${spaced.length - MAX_ROWS} lowest-ranked dropped)` : ""} -> ${path.basename(outPath)}`);

#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const PROP_KINDS = ["rock", "tree", "crystal", "debris", "base", "shop", "fountain", "lane", "camp", "side_shop", "secret_shop"];
const CELL_KINDS = ["walkable", "blocked", "cliff", "blocked_cliff"];
const STRUCTURE_BIT = 1 << 6;
const NEUTRAL = 0xffff;
const START_FLAG = 1;

export function mapStats(buf) {
  if (buf.length < 16 || buf.toString("latin1", 0, 4) !== "OSDT") throw new Error("not an OSDT image (magic mismatch)");
  const version = buf.readUInt16LE(4);
  const sectionCount = buf.readUInt32LE(8);
  const declaredLength = buf.readUInt32LE(12);
  const sections = {};
  for (let i = 0; i < sectionCount; i++) {
    const e = 16 + i * 16;
    sections[buf.toString("latin1", e, e + 4)] = {
      offset: buf.readUInt32LE(e + 4),
      count: buf.readUInt32LE(e + 8),
      rec: buf.readUInt32LE(e + 12),
    };
  }
  const need = (tag) => {
    if (!sections[tag]) throw new Error(`section ${tag} missing`);
    return sections[tag];
  };

  const g = need("GRID");
  const width = buf.readUInt32LE(g.offset);
  const height = buf.readUInt32LE(g.offset + 4);
  const cellMilli = buf.readUInt32LE(g.offset + 8);
  const heightStep = buf.readUInt32LE(g.offset + 12);
  const n = width * height;
  const walk = g.offset + 16;
  const heights = walk + n;
  const tiles = walk + 2 * n;
  const cells = { walkable: 0, blocked: 0, cliff: 0, blocked_cliff: 0, invalid: 0 };
  const tileIds = new Set();
  let heightMax = 0;
  for (let i = 0; i < n; i++) {
    const b = buf[walk + i];
    if (b > 3) cells.invalid++;
    else cells[CELL_KINDS[b]]++;
    const h = buf[heights + i];
    if (h > heightMax) heightMax = h;
    tileIds.add(buf[tiles + i]);
  }

  const strs = need("STRS");
  const name = (off, len) => buf.toString("utf8", strs.offset + off, strs.offset + off + len);
  const u = need("UNIT");
  const units = [];
  for (let i = 0; i < u.count; i++) {
    const b = u.offset + i * u.rec;
    units.push({ name: name(buf.readUInt32LE(b), buf.readUInt16LE(b + 4)), filter: buf.readUInt32LE(b + 40) });
  }

  const plce = need("PLCE");
  const starts = {};
  const placements = {};
  const structures = {};
  const towers = {};
  for (let i = 0; i < plce.count; i++) {
    const b = plce.offset + i * plce.rec;
    const unit = buf.readUInt16LE(b);
    const player = buf.readUInt16LE(b + 2);
    const flags = buf.readUInt32LE(b + 12);
    const side = player === NEUTRAL ? "neutral" : `p${player}`;
    if (flags & START_FLAG) {
      starts[side] = (starts[side] || 0) + 1;
      continue;
    }
    placements[side] = (placements[side] || 0) + 1;
    const info = units[unit];
    if (!info) continue;
    if (info.filter & STRUCTURE_BIT) structures[side] = (structures[side] || 0) + 1;
    if (info.name.includes("tower")) towers[side] = (towers[side] || 0) + 1;
  }

  const props = Object.fromEntries(PROP_KINDS.map((k) => [k, 0]));
  const d = sections.DODA;
  if (d) {
    for (let i = 0; i < d.count; i++) {
      const kind = buf.readUInt16LE(d.offset + i * d.rec + 4);
      const label = PROP_KINDS[kind] ?? `kind${kind}`;
      props[label] = (props[label] || 0) + 1;
    }
  }

  const routes = { total: 0, points: 0, by_side: {} };
  const r = sections.ROUT;
  if (r) {
    for (let i = 0; i < r.count; i++) {
      const b = r.offset + i * r.rec;
      const count = buf.readUInt16LE(b + 4);
      if (count === 0) continue;
      const side = `p${buf.readUInt16LE(b)}`;
      routes.total++;
      routes.points += count;
      routes.by_side[side] = (routes.by_side[side] || 0) + 1;
    }
  }

  return {
    format: {
      magic: "OSDT",
      version,
      sections: sectionCount,
      declared_length: declaredLength,
      file_length: buf.length,
      length_ok: declaredLength === buf.length,
    },
    grid: { width, height, cell_milli: cellMilli, height_step: heightStep, cells: n },
    cells,
    tiles: { distinct: tileIds.size },
    heights: { max_level: heightMax },
    starts,
    placements: { by_side: placements },
    structures: { by_side: structures },
    towers: { by_side: towers },
    props,
    routes,
  };
}

export function flatten(obj, prefix = "", out = {}) {
  for (const [k, v] of Object.entries(obj)) {
    const key = prefix ? `${prefix}.${k}` : k;
    if (v && typeof v === "object") flatten(v, key, out);
    else out[key] = v;
  }
  return out;
}

function main() {
  const args = process.argv.slice(2);
  const file = args.find((a) => !a.startsWith("--"));
  if (!file) {
    console.error("usage: map-stats.mjs <file.bin|.osdt> [--json]");
    process.exit(2);
  }
  const stats = mapStats(readFileSync(file));
  if (args.includes("--json")) {
    console.log(JSON.stringify(stats, null, 2));
    return;
  }
  for (const [k, v] of Object.entries(flatten(stats))) console.log(`${k.padEnd(28)} ${v}`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main();

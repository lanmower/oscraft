import { roleOf, kindOf } from "./doodad-roles.mjs";
import { doodadSizes, SCATTER_ROLES } from "./doodad-sizes.mjs";
// Generates scenarios/moba/map.osd and scenarios/moba/neutral.osd from the Dota 2 map data
// dumped by leamare/dota-map-coordinates (data/gridnavdata.json, data/elevationdata.json,
// mapdata.json, patch 7.41). Usage: node scripts/gen-dota-map.mjs [--src dir] [--out scenarios/moba] [--no-trees]
// Without --src the three source files are downloaded into a temp directory first.
// Scale: one Dota unit is 125/8 = 15.625 milli, so one 64-unit GridNav cell is one 1000-milli sim cell.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const args = process.argv.slice(2);
const opt = (name, dflt) => (args.includes(name) ? args[args.indexOf(name) + 1] : dflt);
const outDir = opt("--out", "scenarios/moba");
const withTrees = !args.includes("--no-trees");
const RAW = "https://raw.githubusercontent.com/leamare/dota-map-coordinates/HEAD/";
const FILES = { grid: "data/gridnavdata.json", elev: "data/elevationdata.json", ents: "data/mapdata.json", map: "mapdata.json" };

async function source() {
  const dir = opt("--src", path.join(os.tmpdir(), "dota-map-src"));
  fs.mkdirSync(dir, { recursive: true });
  const out = {};
  for (const [k, f] of Object.entries(FILES)) {
    const p = path.join(dir, k + ".json");
    if (!fs.existsSync(p)) fs.writeFileSync(p, Buffer.from(await (await fetch(RAW + f)).arrayBuffer()));
    out[k] = JSON.parse(fs.readFileSync(p, "utf8"));
  }
  return out;
}

const S = await source();
const R = S.map.data;
const D = S.ents.data;
const WORLD_MIN = -10464;
const WORLD_MAX_Y = 10400;
const CELL_UNITS = 64;
const N = S.elev.data.length;
const MILLI_PER_UNIT = 15.625;
const mx = (wx) => Math.round((wx - WORLD_MIN) * MILLI_PER_UNIT);
const my = (wy) => Math.round((-WORLD_MIN - wy) * MILLI_PER_UNIT);
const cx = (wx) => Math.floor((wx - WORLD_MIN) / CELL_UNITS);
const cy = (wy) => Math.floor((-WORLD_MIN - wy) / CELL_UNITS);
const inb = (x, y) => x >= 0 && y >= 0 && x < N && y < N;

const blocked = new Uint8Array(N * N);
for (const c of S.grid.data) blocked[(N - 1 - (c.y - WORLD_MIN) / CELL_UNITS) * N + (c.x - WORLD_MIN) / CELL_UNITS] = 1;
const gridnavBlocked = blocked.reduce((a, b) => a + b, 0);
const treeCells = [];
if (withTrees) for (const t of D.ent_dota_tree) {
  const x = cx(t.x), y = cy(t.y);
  if (inb(x, y) && !blocked[y * N + x]) { blocked[y * N + x] = 1; treeCells.push(y * N + x); }
}

const level = new Uint8Array(N * N);
for (let j = 0; j < N; j++) for (let i = 0; i < N; i++) {
  const v = S.elev.data[j][i];
  level[(N - 1 - j) * N + i] = v < 0 ? 4 : Math.min(v, 4);
}

function carveFootprint(wx, wy, bounds) {
  const r = Math.max(1, Math.ceil(bounds / 2 / CELL_UNITS));
  for (let dy = -r; dy <= r; dy++) for (let dx = -r; dx <= r; dx++) {
    const x = cx(wx) + dx, y = cy(wy) + dy;
    if (inb(x, y)) blocked[y * N + x] = 0;
  }
}

const P = (team) => (team === 2 ? 0 : 1);
const things = [];
const add = (unit, player, wx, wy, bounds = 0) => { things.push({ unit, player, wx, wy }); if (bounds) carveFootprint(wx, wy, bounds); };

const bases = D.npc_dota_fort.map((f) => ({ team: f.team, wx: f.x, wy: f.y, b: f.bounds[0] })).sort((a, b) => P(a.team) - P(b.team));
const fountains = D.ent_dota_fountain.map((f) => ({ team: f.team, wx: f.x, wy: f.y })).sort((a, b) => P(a.team) - P(b.team));
for (const b of bases) add("base", P(b.team), b.wx, b.wy, b.b);

function nearestOpen(wx, wy, avoid = []) {
  const x0 = cx(wx), y0 = cy(wy);
  for (let r = 0; r < 40; r++) for (let dy = -r; dy <= r; dy++) for (let dx = -r; dx <= r; dx++) {
    if (Math.max(Math.abs(dx), Math.abs(dy)) !== r) continue;
    const x = x0 + dx, y = y0 + dy;
    if (inb(x, y) && !blocked[y * N + x] && avoid.every((a) => Math.hypot(a[0] - x, a[1] - y) >= 3)) return [x, y];
  }
  throw new Error("no open cell near " + wx + "," + wy);
}
const cellMilli = ([x, y]) => [x * 1000 + 500, y * 1000 + 500];

const starts = [];
const heroSpots = {};
for (const f of fountains) {
  const team = P(f.team), fort = bases[team];
  const toward = Math.atan2(fort.wy - f.wy, fort.wx - f.wx);
  const taken = [];
  heroSpots[team] = [];
  for (let k = 0; k < 10; k++) {
    const a = toward + (k - 4.5) * 0.55 + Math.PI * 0.0;
    const wx = f.wx + Math.cos(a) * 520, wy = f.wy + Math.sin(a) * 520;
    const c = nearestOpen(wx, wy, taken);
    taken.push(c);
    heroSpots[team].push(c);
  }
}
for (let p = 0; p < 10; p++) starts.push({ p, team: p % 2, c: heroSpots[p % 2][Math.floor(p / 2)] });

const tiers = { 1: "tower", 2: "tower_t2", 3: "tower_t3", 4: "tower_t4" };
const towerRows = D.npc_dota_tower.map((t) => ({ ...t, tier: +/tower(\d)/.exec(t.name)[1], lane: /_(top|mid|bot)$/.exec(t.name)?.[1] ?? "base" }));
const order = (r) => [r.tier, ["top", "mid", "bot", "base"].indexOf(r.lane)];
towerRows.sort((a, b) => P(a.team) - P(b.team) || a.tier - b.tier || order(a)[1] - order(b)[1] || a.y - b.y);
const raxRows = D.npc_dota_barracks.map((r) => ({ ...r, kind: /melee/.test(r.name) ? "barracks_melee" : "barracks_ranged" }));

const heroLines = [], courierLines = [];
for (const s of starts) {
  const [x, y] = cellMilli(s.c);
  heroLines.push(`spawn("hero", ${s.p}, ${x}, ${y})`);
}
for (const f of fountains) {
  const c = nearestOpen(f.wx + (P(f.team) ? -1 : 1) * 380, f.wy + (P(f.team) ? 1 : -1) * 380, heroSpots[P(f.team)]);
  const [x, y] = cellMilli(c);
  courierLines.push(`spawn("courier", ${P(f.team)}, ${x}, ${y})`);
}

for (const t of towerRows) add(tiers[t.tier], P(t.team), t.x, t.y, t.bounds[0]);
for (const r of raxRows.sort((a, b) => P(a.team) - P(b.team))) add(r.kind, P(r.team), r.x, r.y, r.bounds[0]);

const camps = R.npc_dota_neutral_spawner.map((s) => ({ ...s, side: s.triggerName.includes("good") ? 0 : 1, t: +s.neutralType }));
const compose = {
  0: ["neutral_kobold_taskmaster", "neutral_kobold_tunneler", "neutral_kobold"],
  1: ["neutral_alpha_wolf", "neutral_giant_wolf", "neutral_giant_wolf"],
  2: ["neutral_centaur_khan", "neutral_centaur_outrunner", "neutral_centaur_outrunner"],
  3: ["neutral_granite_golem", "neutral_rock_golem", "neutral_rock_golem"],
};
const campLines = [];
for (const c of camps.sort((a, b) => a.side - b.side || a.t - b.t || a.x - b.x)) {
  campLines.push(`# camp ${c.triggerName} type ${c.t} pull ${c.pullType}`);
  const taken = [];
  compose[c.t].forEach((u, k) => {
    const a = (k / 3) * Math.PI * 2 + 0.5;
    const cell = nearestOpen(c.x + Math.cos(a) * 110, c.y + Math.sin(a) * 110, taken);
    taken.push(cell);
    const [x, y] = cellMilli(cell);
    campLines.push(`spawn("${u}", -1, ${x}, ${y})`);
  });
}

const rosh = R.npc_dota_roshan_spawner[0];
const poiLines = [];
const rc = nearestOpen(rosh.x, rosh.y);
poiLines.push(`spawn("roshan", -1, ${cellMilli(rc).join(", ")})`);
for (const r of D.dota_item_rune_spawner_bounty) poiLines.push(`spawn("bounty_rune", -1, ${cellMilli(nearestOpen(r.x, r.y)).join(", ")})`);
const outposts = D.npc_dota_watch_tower.map((o) => ({ wx: o.x, wy: o.y }));
for (const o of outposts) { carveFootprint(o.wx, o.wy, 96); poiLines.push(`spawn("outpost", -1, ${mx(o.wx)}, ${my(o.wy)})`); }

let id = 1;
const doodads = [];
const placedBuildings = [];
const BUILDING_KINDS = new Set(["base", "fountain", "camp", "shop", "side_shop", "secret_shop"]);
const isBuildingKind = (kind) => BUILDING_KINDS.has(kind) || roleOf(kind.replace(/_/g, "-")) === "building";
const recordBuilding = (kind, x, y) => { if (isBuildingKind(kind)) placedBuildings.push({ kind, x: x / 1000, y: y / 1000 }); };
const dd = (kind, wx, wy) => { recordBuilding(kind, mx(wx), my(wy)); doodads.push(`doodad(${id++}, "${kind}", ${mx(wx)}, ${my(wy)})`); };
for (const b of bases) dd("base", b.wx, b.wy);
for (const f of fountains) dd("fountain", f.wx, f.wy);
// The dump has no shop type: the two entities beside the fountains (|y| > 6000) are the home shops; the two inner ones are taken as
// the secret shops (UNVERIFIED, no source names their kind). The dump lists no side shops.
for (const s of D.ent_dota_shop) dd(Math.abs(s.y) > 6000 ? "shop" : "secret_shop", s.x, s.y);
for (const c of camps) dd("camp", c.x, c.y);
for (const r of D.dota_item_rune_spawner_bounty) dd("lane", r.x, r.y);
for (const r of D.dota_item_rune_spawner_powerup) dd("lane", r.x, r.y);
for (const r of R.npc_dota_roshan_spawner) dd("rock", r.x, r.y);
for (const o of outposts) dd("crystal", o.wx, o.wy);
for (const o of [...D.npc_dota_lotus_pool, ...D.npc_dota_xp_fountain, ...D.npc_dota_unit_twin_gate, ...R.npc_dota_miniboss_spawner]) dd("crystal", o.x, o.y);
for (const l of R.landmarks ?? []) dd("debris", l.x, l.y);
for (const f of D.npc_dota_filler) dd("debris", f.x, f.y);
for (const l of D.npc_dota_lantern) dd("debris", l.x, l.y);
// Reachability from the Radiant fountain across open cells (4-connected).
const seen = new Uint8Array(N * N);
const [sx, sy] = heroSpots[0][0];
const q = [sy * N + sx];
seen[q[0]] = 1;
for (let h = 0; h < q.length; h++) {
  const i = q[h], x = i % N, y = (i / N) | 0;
  for (const [dx, dy] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) {
    const nx = x + dx, ny = y + dy;
    if (inb(nx, ny) && !blocked[ny * N + nx] && !seen[ny * N + nx]) { seen[ny * N + nx] = 1; q.push(ny * N + nx); }
  }
}
const missing = [];
for (const t of things) { const c = cy(t.wy) * N + cx(t.wx); if (!seen[c]) missing.push(t.unit + "@" + t.wx + "," + t.wy); }
for (const c of camps) if (!seen[cy(c.y) * N + cx(c.x)]) missing.push("camp@" + c.triggerName);
for (const s of starts) if (!seen[s.c[1] * N + s.c[0]]) missing.push("start" + s.p);
if (!seen[cy(rosh.y) * N + cx(rosh.x)]) missing.push("roshan");
const walkable = blocked.reduce((a, b) => a + (b ? 0 : 1), 0);
console.error(JSON.stringify({ N, gridnavBlocked, treeCells: treeCells.length, walkable, reachable: q.length, missing }));

// Open-cell and height rectangles: horizontal runs merged across identical consecutive rows.
function rects(valueAt, want) {
  const out = [];
  let open = new Map();
  for (let y = 0; y <= N; y++) {
    const runs = [];
    if (y < N) for (let x = 0; x < N;) {
      if (!want(valueAt(x, y))) { x++; continue; }
      let e = x;
      while (e < N && want(valueAt(e, y)) && valueAt(e, y) === valueAt(x, y)) e++;
      runs.push([x, e, valueAt(x, y)]);
      x = e;
    }
    const next = new Map();
    for (const r of runs) {
      const k = r.join(",");
      next.set(k, open.has(k) ? open.get(k) : y);
    }
    for (const [k, y0] of open) if (!next.has(k)) { const [a, b, v] = k.split(",").map(Number); out.push([a, y0, b, y, v]); }
    open = next;
  }
  return out;
}
const openRects = rects((x, y) => (blocked[y * N + x] ? 1 : 0), (v) => v === 0).map((r) => `paint(${r[0]}, ${r[1]}, ${r[2]}, ${r[3]}, 0)`);
const lvlRects = rects((x, y) => level[y * N + x], (v) => v !== 1).map((r) => `paint_height(${r[0]}, ${r[1]}, ${r[2]}, ${r[3]}, ${r[4]})`);

// Tile ids the client reads (js/client/tiles.js TILE_IDS): lane 2, dirt 3, water 4; every other open cell is grass.
// A river cell (level 0) is water only when blocked, because tiles.js draws water on blocked cells alone. Lane cells are
// the creep routes (tower waypoints per lane) rasterised LANE_HALF_WIDTH cells either side of each segment, on open cells.
// Dirt is limited to the zones around the bases and towers (footprint radius plus a margin) and a DIRT_LANE_EDGE rim
// beside each lane; the rest stays grass. Tiles never change walkability or height: they only pick the flat tile model.
const TILE = { lane: 2, dirt: 3, water: 4 };
const LANE_HALF_WIDTH = 2;
const DIRT_LANE_EDGE = 1;
const BASE_DIRT_MARGIN = 2;
const TOWER_DIRT_MARGIN = 1;
const tiles = new Uint8Array(N * N);
for (let i = 0; i < N * N; i++) if (blocked[i] && level[i] === 0) tiles[i] = TILE.water;
function markLane(a, b) {
  const [ax, ay] = a, [bx, by] = b;
  const dx = bx - ax, dy = by - ay, len2 = dx * dx + dy * dy || 1;
  for (let y = Math.floor(Math.min(ay, by) - LANE_HALF_WIDTH); y <= Math.ceil(Math.max(ay, by) + LANE_HALF_WIDTH); y++) {
    for (let x = Math.floor(Math.min(ax, bx) - LANE_HALF_WIDTH); x <= Math.ceil(Math.max(ax, bx) + LANE_HALF_WIDTH); x++) {
      if (!inb(x, y) || blocked[y * N + x]) continue;
      const t = Math.max(0, Math.min(1, ((x + 0.5 - ax) * dx + (y + 0.5 - ay) * dy) / len2));
      if (Math.hypot(x + 0.5 - (ax + t * dx), y + 0.5 - (ay + t * dy)) <= LANE_HALF_WIDTH) tiles[y * N + x] = TILE.lane;
    }
  }
}
for (const player of [0, 1]) {
  const own = bases[player];
  for (const name of ["top", "mid", "bot"]) {
    const dist = (t) => Math.hypot(t.x - own.wx, t.y - own.wy);
    const pts = towerRows.filter((t) => t.lane === name).sort((a, b) => dist(a) - dist(b)).map((t) => [cx(t.x) + 0.5, cy(t.y) + 0.5]);
    for (let k = 0; k + 1 < pts.length; k++) markLane(pts[k], pts[k + 1]);
  }
}
const footprintRadius = (bounds) => Math.max(1, Math.ceil(bounds / 2 / CELL_UNITS));
const dirtZone = new Uint8Array(N * N);
const structureZone = new Uint8Array(N * N);
function stampZone(zone, cellX, cellY, r) {
  for (let dy = -r; dy <= r; dy++) for (let dx = -r; dx <= r; dx++) if (inb(cellX + dx, cellY + dy)) zone[(cellY + dy) * N + cellX + dx] = 1;
}
for (const b of bases) {
  const r = footprintRadius(b.b) + BASE_DIRT_MARGIN;
  stampZone(dirtZone, cx(b.wx), cy(b.wy), r);
  stampZone(structureZone, cx(b.wx), cy(b.wy), r);
}
for (const t of towerRows) {
  const r = footprintRadius(t.bounds[0]) + TOWER_DIRT_MARGIN;
  stampZone(dirtZone, cx(t.x), cy(t.y), r);
  stampZone(structureZone, cx(t.x), cy(t.y), r);
}
for (let i = 0; i < N * N; i++) {
  if (tiles[i] !== TILE.lane) continue;
  const x0 = i % N, y0 = (i / N) | 0;
  for (let dy = -DIRT_LANE_EDGE; dy <= DIRT_LANE_EDGE; dy++) for (let dx = -DIRT_LANE_EDGE; dx <= DIRT_LANE_EDGE; dx++) if (inb(x0 + dx, y0 + dy)) dirtZone[(y0 + dy) * N + x0 + dx] = 1;
}
for (let i = 0; i < N * N; i++) if (!blocked[i] && dirtZone[i] && tiles[i] === 0) tiles[i] = TILE.dirt;
const tileRects = rects((x, y) => tiles[y * N + x], (v) => v !== 0).map((r) => `paint_tile(${r[0]}, ${r[1]}, ${r[2]}, ${r[3]}, ${r[4]})`);
const tileCount = (id) => tiles.reduce((a, v) => a + (v === id ? 1 : 0), 0);

// Visual pack props, sized by scripts/doodad-sizes.mjs and classed by scripts/doodad-roles.mjs. Doodads never block, so
// walkability is unchanged. Every jungle tree cell (a blocked cell) gets a tree, so each blocked cell reads as a tree; bushes,
// rocks and props sit on open cells. Nothing is placed on a lane cell or inside a base or tower footprint (structureZone);
// lane rims stay open for bushes along the lane edges. Each role's models cycle in order.
const modelsDir = new URL("../js/client/models/", import.meta.url);
const { sizes: doodadFootprints, units: unitFootprints } = doodadSizes(modelsDir);
const shippedStems = fs.readdirSync(modelsDir).filter((f) => f.endsWith(".glb")).map((f) => f.slice(0, -4)).sort().filter((stem) => doodadFootprints[stem] !== undefined);
const ROLE_KINDS = {};
for (const stem of shippedStems) (ROLE_KINDS[roleOf(stem)] ??= []).push(kindOf(stem));
const roleNext = {};
const pick = (role) => { const list = ROLE_KINDS[role]; const n = roleNext[role] = (roleNext[role] ?? -1) + 1; return list[n % list.length]; };
const clear = (i) => tiles[i] !== TILE.lane && !structureZone[i];
const clearAt = (wx, wy) => clear(cy(wy) * N + cx(wx));
const cellDd = (kind, c) => { const [x, y] = cellMilli(c); recordBuilding(kind, x, y); doodads.push(`doodad(${id++}, "${kind}", ${x}, ${y})`); };
let natureEvery = 0;
for (let i = 0; i < N * N; i++) {
  if (level[i] !== 1 || blocked[i] || !clear(i)) continue;
  if (natureEvery++ % 40 === 0) cellDd(pick("nature"), [i % N, (i / N) | 0]);
}
let cliffEdge = 0;
for (let y = 1; y < N - 1; y++) for (let x = 1; x < N - 1; x++) {
  const i = y * N + x;
  if (level[i] < 3 || (level[i - 1] >= 3 && level[i + 1] >= 3 && level[i - N] >= 3 && level[i + N] >= 3)) continue;
  if (clear(i) && cliffEdge++ % 9 === 0) cellDd(pick("rock"), [x, y]);
}
for (const t of towerRows) {
  if (t.lane === "base") continue;
  if (clearAt(t.x + 200, t.y)) dd(pick("prop"), t.x + 200, t.y);
  if (clearAt(t.x - 200, t.y)) dd(pick("prop"), t.x - 200, t.y);
}
for (const c of camps) if (clearAt(c.x + 150, c.y + 150)) dd(pick("character"), c.x + 150, c.y + 150);
for (const s of D.ent_dota_shop) if (clearAt(s.x + 250, s.y)) dd(pick("item"), s.x + 250, s.y);
for (const f of fountains) for (const dx of [300, -300]) if (clearAt(f.wx + dx, f.wy)) dd(pick("prop"), f.wx + dx, f.wy);
// Castle kit (wall and tower stems) as base and tower decoration: one piece on each diagonal corner of a ring just outside
// the structure zone (footprint plus margin plus CASTLE_RING_EXTRA cells), on clear cells only, cycling through the kit.
const CASTLE_RING_EXTRA = 2;
const CASTLE_STEM = /^(wall|tower-|brick-wall|stone-wall)/;
const castleKinds = shippedStems.filter((stem) => CASTLE_STEM.test(stem)).map(kindOf);
let castleNext = 0;
const castleRing = (cellX, cellY, r) => {
  for (const [dx, dy] of [[1, 1], [-1, 1], [1, -1], [-1, -1]]) {
    const x = cellX + dx * r, y = cellY + dy * r;
    if (inb(x, y) && clear(y * N + x)) cellDd(castleKinds[castleNext++ % castleKinds.length], [x, y]);
  }
};
for (const b of bases) castleRing(cx(b.wx), cy(b.wy), footprintRadius(b.b) + BASE_DIRT_MARGIN + CASTLE_RING_EXTRA);
for (const t of towerRows) castleRing(cx(t.x), cy(t.y), footprintRadius(t.bounds[0]) + TOWER_DIRT_MARGIN + CASTLE_RING_EXTRA);
// Trees go last so every building footprint is known. A tree is skipped whole when its fitted footprint (a square centred on
// its cell) covers a lane cell, a base or tower zone, or a placed building; placement order is fixed, so the result is deterministic.
// A kind is sized by the model doodads.json names for it (play.js reads the same file), falling back to its stem.
const KIND_MODEL = JSON.parse(fs.readFileSync(new URL("../js/client/doodads.json", import.meta.url), "utf8"));
const sizeOfKind = (kind) => doodadFootprints[(KIND_MODEL[kind] ?? "").replace(/^(js\/client\/)?models\//, "").replace(/\.glb$/, "") || kind.replace(/_/g, "-")] ?? 2 * 0.5;
const footprintClear = (i, kind) => {
  const half = sizeOfKind(kind) / 2, x = i % N + 0.5, y = ((i / N) | 0) + 0.5;
  for (let gy = Math.floor(y - half); gy < Math.ceil(y + half); gy++) for (let gx = Math.floor(x - half); gx < Math.ceil(x + half); gx++) {
    if (inb(gx, gy) && (tiles[gy * N + gx] === TILE.lane || structureZone[gy * N + gx])) return false;
  }
  return placedBuildings.every((b) => { const bh = sizeOfKind(b.kind) / 2; return !(x + half > b.x - bh && x - half < b.x + bh && y + half > b.y - bh && y - half < b.y + bh); });
};
const fieldCells = [];
for (let i = 0; i < N * N; i++) if (level[i] === 1 && !blocked[i] && clear(i)) fieldCells.push(i);
const scatterKinds = shippedStems.filter((stem) => SCATTER_ROLES.has(roleOf(stem))).map(kindOf);
const scatterStep = Math.max(1, Math.floor(fieldCells.length / scatterKinds.length));
if (fieldCells.length < scatterKinds.length) throw new Error(`field cells ${fieldCells.length} below models ${scatterKinds.length}`);
scatterKinds.forEach((kind, k) => { const c = fieldCells[k * scatterStep]; if ((roleOf(kind.replace(/_/g, "-")) === "tree" || /stump|trunk|log/.test(kind)) && !footprintClear(c, kind)) return; cellDd(kind, [c % N, (c / N) | 0]); });
treeCells.forEach((i) => { if (!clear(i)) return; const kind = pick("tree"); if (footprintClear(i, kind)) cellDd(kind, [i % N, (i / N) | 0]); });

fs.writeFileSync(new URL("../js/client/doodad_sizes.json", import.meta.url), JSON.stringify({ ...doodadFootprints, ...unitFootprints }, null, 1) + "\n");

const thingLine = (t) => `spawn("${t.unit}", ${t.player}, ${mx(t.wx)}, ${my(t.wy)})`;
const header = [
  `# Dota 2 map (patch 7.41) at ${N} x ${N} cells: the GridNav of 64-unit cells, generated by scripts/gen-dota-map.mjs.`,
  `# Source: leamare/dota-map-coordinates data/gridnavdata.json, data/elevationdata.json, mapdata.json (dumped from the game).`,
  `# Scale: 1 Dota unit = 15.625 milli, so one 64-unit GridNav cell is one 1000-milli cell. Radiant (team 0) is bottom-left,`,
  `# Dire (team 1) top-right; the y axis points down the screen, so Dota north is small y. Do not edit by hand; edit the script.`,
  `# Walkable: GridNav-open cells minus ${treeCells.length} tree cells (trees are static here). Height levels: 0 river/low, 1 field, 2 base high ground, 3-4 cliffs and rims.`,
  `grid(${N}, ${N}, cell_milli=1000, fill=1)`,
];
const mapLines = [
  ...header,
  ...openRects,
  `height_step(2000)`,
  `paint_height(0, 0, ${N}, ${N}, 1)`,
  ...lvlRects,
  ...tileRects,
  ...doodads,
  ...starts.map((s) => `start(${s.p}, ${cellMilli(s.c).join(", ")}, team=${s.team})`),
  ...bases.map((b) => thingLine({ unit: "base", player: P(b.team), wx: b.wx, wy: b.wy })),
  ...heroLines,
  ...towerRows.map((t) => thingLine({ unit: tiers[t.tier], player: P(t.team), wx: t.x, wy: t.y })),
  ...raxRows.map((r) => thingLine({ unit: r.kind, player: P(r.team), wx: r.x, wy: r.y })),
  ...courierLines,
];
fs.writeFileSync(path.join(outDir, "map.osd"), mapLines.join("\n") + "\n");
fs.writeFileSync(path.join(outDir, "neutral.osd"), [
  "# Neutral camps of the Dota 2 map: one camp per sourced spawner (14 per side), composition by neutralType 0..3 (small, medium, large, ancient).",
  "# Generated by scripts/gen-dota-map.mjs from mapdata.json npc_dota_neutral_spawner; camp compositions are UNVERIFIED.",
  ...campLines,
  "# Roshan, bounty runes and outposts come after the camps so the first neutral unit is a camp creep.",
  ...poiLines,
].join("\n") + "\n");
console.error(`map.osd lines ${mapLines.length} (open rects ${openRects.length}, height rects ${lvlRects.length}, tile rects ${tileRects.length}); neutral lines ${campLines.length}`);
console.error(`tile cells per id: lane(2) ${tileCount(TILE.lane)}, dirt(3) ${tileCount(TILE.dirt)}, water(4) ${tileCount(TILE.water)}`);

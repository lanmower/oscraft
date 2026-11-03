import fs from "node:fs";

export const PATH_W = 512, PATH_UNIT = 32, WORLD_HALF = 8192, SIM = 256;
export const PATH_UNWALKABLE = 0x02;
export const PATH_UNFLYABLE = 0x04;
export const PATH_UNBUILDABLE = 0x08;
export const PATH_BLIGHT = 0x20;
export const PATH_NO_WATER = 0x40;
export const PATH_TERRAIN_BLOCK = 0x80;

export function loadFootprints(tsvPath) {
  const map = new Map();
  for (const l of fs.readFileSync(tsvPath, "utf8").trim().split("\n").slice(1)) {
    const c = l.split("\t");
    const [type, , kind, , w, h, , rows] = c;
    if (!rows) continue;
    const W = Number(w), H = Number(h);
    const mask = new Uint8Array(W * H);
    rows.split("/").forEach((row, i) => { for (let x = 0; x < W; x++) mask[(H - 1 - i) * W + x] = row[x] === "1" ? 1 : 0; });
    map.set(type, { w: W, h: H, mask, kind });
  }
  return map;
}

export function quarterTurns(angle) {
  const q = Math.round((angle - (3 * Math.PI) / 2) / (Math.PI / 2));
  return ((q % 4) + 4) % 4;
}

export function rotate(fp, turns) {
  const t = ((turns % 4) + 4) % 4;
  if (t === 0) return fp;
  const swap = t === 1 || t === 3;
  const w = swap ? fp.h : fp.w, h = swap ? fp.w : fp.h;
  const mask = new Uint8Array(w * h);
  for (let y = 0; y < fp.h; y++) for (let x = 0; x < fp.w; x++) {
    const dx = t === 1 ? fp.h - 1 - y : t === 2 ? fp.w - 1 - x : y;
    const dy = t === 1 ? x : t === 2 ? fp.h - 1 - y : fp.w - 1 - x;
    mask[dy * w + dx] = fp.mask[y * fp.w + x];
  }
  return { w, h, mask, kind: fp.kind };
}

export const TREE_TYPES = new Set(["ATtr", "NTtw", "CTtr", "NTtc"]);

export function footprintCells(d, footprints, turned = new Map()) {
  const fp = footprints.get(d.type);
  if (!fp || (d.flags & 2) === 0) return [];
  const turns = quarterTurns(d.angle);
  const key = `${d.type}|${turns}`;
  if (!turned.has(key)) turned.set(key, rotate(fp, turns));
  const r = turned.get(key);
  const bx = Math.round((d.x - (r.w * PATH_UNIT) / 2 + WORLD_HALF) / PATH_UNIT);
  const by = Math.round((d.y - (r.h * PATH_UNIT) / 2 + WORLD_HALF) / PATH_UNIT);
  const out = [];
  for (let y = 0; y < r.h; y++) for (let x = 0; x < r.w; x++) {
    if (!r.mask[y * r.w + x]) continue;
    const cx = bx + x, cy = by + y;
    if (cx >= 0 && cy >= 0 && cx < PATH_W && cy < PATH_W) out.push(cy * PATH_W + cx);
  }
  return out;
}

export function stampFootprints(props, footprints) {
  const stamp = new Uint16Array(PATH_W * PATH_W);
  const turned = new Map();
  const stamped = [];
  for (const d of props) {
    const cells = footprintCells(d, footprints, turned);
    if (!cells.length) continue;
    for (const i of cells) stamp[i]++;
    stamped.push(d);
  }
  return { stamp, stamped };
}

export function simCellsOf(d, footprints, turned) {
  const set = new Set();
  for (const i of footprintCells(d, footprints, turned)) {
    const row = Math.floor(i / PATH_W), col = i % PATH_W;
    set.add(Math.floor((511 - row) / 2) * SIM + Math.floor(col / 2));
  }
  return [...set].map((k) => [k % SIM, Math.floor(k / SIM)]);
}

export const subCells = (cx, cy) => [[510 - 2 * cy, 2 * cx], [510 - 2 * cy, 2 * cx + 1], [511 - 2 * cy, 2 * cx], [511 - 2 * cy, 2 * cx + 1]];

export function blockedGrid(wpmCells, stamp, reduce = "any") {
  const need = reduce === "all" ? 4 : reduce === "majority" ? 2 : 1;
  const blocked = Array.from({ length: SIM }, () => new Uint8Array(SIM));
  for (let cy = 0; cy < SIM; cy++) for (let cx = 0; cx < SIM; cx++) {
    let n = 0;
    for (const [r, c] of subCells(cx, cy)) if ((wpmCells[r * PATH_W + c] & PATH_UNWALKABLE) || (stamp && stamp[r * PATH_W + c])) n++;
    blocked[cy][cx] = n >= need ? 1 : 0;
  }
  return blocked;
}

export const GX = 35, GY = 35;

export const wpmBlockedGrid = (wpmCells) => blockedGrid(wpmCells, null, "any");

export function writeWpmTsv(file, blocked) {
  const out = ["cy\tx0\tx1"];
  for (let cy = 0; cy < SIM; cy++) {
    let cx = 0;
    while (cx < SIM) {
      if (!blocked[cy][cx]) { cx++; continue; }
      let e = cx;
      while (e < SIM && blocked[cy][e]) e++;
      out.push(`${cy}\t${cx}\t${e}`);
      cx = e;
    }
  }
  fs.writeFileSync(file, out.join("\n") + "\n");
}
export function readWpmTsv(file) {
  const blocked = Array.from({ length: SIM }, () => new Uint8Array(SIM));
  for (const l of fs.readFileSync(file, "utf8").trim().split("\n").slice(1)) {
    const [cy, x0, x1] = l.split("\t").map(Number);
    for (let x = x0; x < x1; x++) blocked[cy][x] = 1;
  }
  return blocked;
}

export function deriveBlocked(wpmBlocked, props, footprints) {
  const solid = props.filter((d) => !TREE_TYPES.has(d.type));
  const { stamp, stamped } = stampFootprints(solid, footprints);
  const stampBlocked = blockedGrid(new Uint8Array(PATH_W * PATH_W), stamp, "any");
  const blocked = wpmBlocked.map((row, cy) => row.map((v, cx) => (v || stampBlocked[cy][cx] ? 1 : 0)));
  const turned = new Map();
  const treeBlocked = Array.from({ length: SIM }, () => new Uint8Array(SIM));
  for (const d of props) if (TREE_TYPES.has(d.type)) for (const [cx, cy] of simCellsOf(d, footprints, turned)) treeBlocked[cy][cx] = 1;
  const fullBlocked = blocked.map((row, cy) => row.map((v, cx) => (v || treeBlocked[cy][cx] ? 1 : 0)));
  return { blocked, treeBlocked, fullBlocked, stampBlocked, stamped, stamp };
}

export function readPropRecords(tsvFile) {
  return fs.readFileSync(tsvFile, "utf8").trim().split("\n").slice(1).map((l) => l.split("\t")).map((c) => ({ type: c[0], x: Number(c[2]), y: Number(c[3]), angle: Number(c[5]), flags: Number(c[10]) }));
}

const runsOf = (valueAt) => {
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
const rectsOf = (list) => {
  const out = [], openRects = new Map();
  for (const [x0, y0, x1, y1, v] of list) {
    const key = `${x0}:${x1}:${v}`, r = openRects.get(key);
    if (r && r[3] === y0) r[3] = y1;
    else { const nr = [x0, y0, x1, y1, v]; openRects.set(key, nr); out.push(nr); }
  }
  return out;
};
export function openPaintLines(blocked) {
  return rectsOf(runsOf((cx, cy) => (blocked[cy][cx] ? 1 : 0)).filter((r) => r[4] === 0))
    .map(([x0, y0, x1, y1]) => `paint(${x0 + GX}, ${y0 + GY}, ${x1 + GX}, ${y1 + GY}, 0)`);
}

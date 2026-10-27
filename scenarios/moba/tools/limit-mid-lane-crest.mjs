import fs from "node:fs";
import path from "node:path";
import { provenanceFile } from "./provenance.mjs";
import { parseSidecar, sidecarText } from "./buildings-688g.mjs";

const args = process.argv.slice(2);
const mapPath = args.includes("--map") ? args[args.indexOf("--map") + 1] : new URL("../map.osd", import.meta.url).pathname;
const SCRIPT = "scenarios/moba/tools/limit-mid-lane-crest.mjs";
const N = 327;
const WINDOW = { x0: 90, x1: 106, y0: 226, y1: 240 };
const FIXED = new Set([32, 48]);
const CREST_NOTE = `Mid-lane crest (cells ${WINDOW.x0}-${WINDOW.x1}, ${WINDOW.y0}-${WINDOW.y1}) flattened to one level per cell by ${SCRIPT}; that script edits this file's crest band.`;
const RE = /^paint_height\((\d+), (\d+), (\d+), (\d+), (-?\d+)\)$/;

const lines = fs.readFileSync(mapPath, "utf8").split("\n");
const sidecarPath = provenanceFile(path.dirname(mapPath), path.basename(mapPath));
const sidecarRows = fs.existsSync(sidecarPath) ? parseSidecar(fs.readFileSync(sidecarPath, "utf8")) : [];
const hasHeader = sidecarRows.some((r) => r.note === CREST_NOTE);

function paintGrid(src) {
  const h = new Int32Array(N * N).fill(-1);
  for (const l of src) {
    const m = RE.exec(l);
    if (!m) continue;
    const [x0, y0, x1, y1, v] = m.slice(1).map(Number);
    for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) h[y * N + x] = v;
  }
  return h;
}

const before = paintGrid(lines);
const h = Int32Array.from(before);
const at = (x, y) => h[y * N + x];
const inWindow = (x, y) => x >= WINDOW.x0 && x < WINDOW.x1 && y >= WINDOW.y0 && y < WINDOW.y1;
const mutable = (x, y) => inWindow(x, y) && before[y * N + x] >= 0 && !FIXED.has(before[y * N + x]);

const cells = [];
for (let y = WINDOW.y0; y < WINDOW.y1; y++) for (let x = WINDOW.x0; x < WINDOW.x1; x++) if (mutable(x, y)) cells.push([x, y]);
function sweep(upper) {
  let moved = true;
  for (let pass = 0; moved && pass < 10000; pass++) {
    moved = false;
    for (const [x, y] of cells) {
      const cur = at(x, y);
      let next = cur;
      for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) {
        if (!dx && !dy) continue;
        const v = at(x + dx, y + dy);
        if (v < 0) continue;
        next = upper ? Math.min(next, v + 1) : Math.max(next, v - 1);
      }
      if (next !== cur) { h[y * N + x] = next; moved = true; }
    }
  }
}
for (let round = 0; round < 1000; round++) {
  const prev = Int32Array.from(h);
  sweep(true);
  sweep(false);
  if (prev.every((v, i) => v === h[i])) break;
}

function steep(src, inBox) {
  let worst = 0, diag = 0, at2 = null;
  for (let y = 0; y < N; y++) for (let x = 0; x < N; x++) {
    if (inBox && !inWindow(x, y)) continue;
    const a = src[y * N + x];
    if (a < 0) continue;
    for (const [dx, dy] of [[1, 1], [1, -1], [1, 0], [0, 1]]) {
      const nx = x + dx, ny = y + dy;
      if (nx >= N || ny >= N) continue;
      const b = src[ny * N + nx];
      if (b < 0) continue;
      const d = Math.abs(a - b);
      if (dx && dy) diag = Math.max(diag, d);
      if (d > worst) { worst = d; at2 = [x, y, nx, ny]; }
    }
  }
  return { worst, diagonal: diag, at: at2 };
}

const changed = cells.filter(([x, y]) => h[y * N + x] !== before[y * N + x]);
const appended = [];
for (let y = WINDOW.y0; y < WINDOW.y1; y++) {
  let x = WINDOW.x0;
  while (x < WINDOW.x1) {
    if (!changed.some(([cx, cy]) => cx === x && cy === y)) { x++; continue; }
    const nv = h[y * N + x];
    let e = x + 1;
    while (e < WINDOW.x1 && changed.some(([cx, cy]) => cx === e && cy === y) && h[y * N + e] === nv) e++;
    appended.push(`paint_height(${x}, ${y}, ${e}, ${y + 1}, ${nv})`);
    x = e;
  }
}

const stats = (src) => ({ window: steep(src, true), map: steep(src, false) });
const report = { window: WINDOW, mutableCells: cells.length, changedCells: changed.length, appendedLines: appended.length, before: stats(before), after: stats(h) };

if (changed.length === 0 && hasHeader) {
  console.log(JSON.stringify({ ...report, note: "already flattened; nothing to write" }));
} else {
  let lastPaint = -1;
  lines.forEach((l, i) => { if (RE.test(l)) lastPaint = i; });
  const merged = [...lines.slice(0, lastPaint + 1), ...appended, ...lines.slice(lastPaint + 1)];
  if (!hasHeader) {
    fs.mkdirSync(path.dirname(sidecarPath), { recursive: true });
    fs.writeFileSync(sidecarPath, sidecarText([{ anchor: "file", kind: "rationale", note: CREST_NOTE }, ...sidecarRows]));
  }
  fs.writeFileSync(mapPath, merged.join("\n"));
  console.log(JSON.stringify(report));
}

import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const args = process.argv.slice(2);
const dir = args.includes("--dir") ? args[args.indexOf("--dir") + 1] : "scenarios/moba";
const here = path.dirname(fileURLToPath(import.meta.url));
const CORRIDOR = 12, RIVER_MIN = 1000;
let failures = 0;
const fail = (m) => { failures++; console.error("FAIL " + m); };

try { execFileSync(process.execPath, [path.join(here, "apply-pathing-688g.mjs"), "--dir", dir, "--check"], { stdio: ["ignore", "inherit", "inherit"] }); } catch { fail("map.osd open lines differ from the derived pathing"); }

try { execFileSync(process.execPath, [path.join(here, "gen-trees-688g.mjs"), "--dir", dir, "--check"], { stdio: ["ignore", "inherit", "inherit"] }); } catch { fail("trees_688g.lua differs from the derived trees"); }

const osd = fs.readFileSync(path.join(dir, "map.osd"), "utf8").split("\n");
const gl = osd.find((l) => l.startsWith("grid(")).match(/grid\((\d+), (\d+)/);
const W = Number(gl[1]), H = Number(gl[2]);
const flags = new Uint8Array(W * H).fill(1), tiles = new Uint8Array(W * H);
for (const l of osd) {
  const m = /^(paint|paint_tile)\((\d+), (\d+), (\d+), (\d+), (\d+)\)/.exec(l);
  if (!m) continue;
  const [x0, y0, x1, y1, v] = m.slice(2).map(Number);
  const a = m[1] === "paint" ? flags : tiles;
  for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) a[y * W + x] = v;
}
const treeCells = new Uint8Array(W * H);
for (const m of fs.readFileSync(path.join(dir, "trees_688g.lua"), "utf8").matchAll(/^  \{(\d+), (\d+), (\d+), (\d+), -?\d+\},$/gm)) {
  const [x0, y0, w, h] = m.slice(1).map(Number);
  for (let y = y0; y < y0 + h; y++) for (let x = x0; x < x0 + w; x++) treeCells[y * W + x] = 1;
}
const open = (x, y) => x >= 0 && y >= 0 && x < W && y < H && (flags[y * W + x] & 1) === 0 && treeCells[y * W + x] === 0;
const cell = (m) => [Math.floor(m / 1000), Math.floor(m / 1000)];
const at = (xm, ym) => [Math.floor(xm / 1000), Math.floor(ym / 1000)];

function flood(seeds, allowed = open) {
  const seen = new Uint8Array(W * H), q = [];
  for (const [x, y] of seeds) if (allowed(x, y) && !seen[y * W + x]) { seen[y * W + x] = 1; q.push(x, y); }
  for (let i = 0; i < q.length; i += 2) {
    const x = q[i], y = q[i + 1];
    for (const [dx, dy] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) {
      const nx = x + dx, ny = y + dy;
      if (allowed(nx, ny) && !seen[ny * W + nx]) { seen[ny * W + nx] = 1; q.push(nx, ny); }
    }
  }
  return seen;
}

const anchors = [];
const read = (f) => (fs.existsSync(path.join(dir, f)) ? fs.readFileSync(path.join(dir, f), "utf8").split("\n") : []);
for (const f of ["map.osd", "neutral.osd", "waves.osd"]) for (const l of read(f)) {
  let m = /^spawn\("([^"]+)", (-?\d+), (\d+), (\d+)/.exec(l);
  if (m) { anchors.push({ what: `${f} ${m[1]}`, side: Number(m[2]), xy: at(Number(m[3]), Number(m[4])) }); continue; }
  m = /^prop\(\d+, "depot", (\d+), (\d+)/.exec(l);
  if (m) { anchors.push({ what: `${f} shop depot`, side: -1, xy: at(Number(m[1]), Number(m[2])) }); continue; }
  m = /^start\((\d+), (\d+), (\d+)/.exec(l);
  if (m) anchors.push({ what: `${f} start ${m[1]}`, side: -1, xy: at(Number(m[2]), Number(m[3])) });
}
const routes = [];
for (const l of read("routes.osd")) {
  const m = /^route\((\d+), (\d+), \[([^\]]*)\]\)/.exec(l);
  if (!m) continue;
  const n = m[3].split(",").map(Number), pts = [];
  for (let i = 0; i + 1 < n.length; i += 2) pts.push(at(n[i], n[i + 1]));
  routes.push({ side: Number(m[1]), lane: Number(m[2]), pts });
  pts.forEach((p, i) => anchors.push({ what: `routes.osd route(${m[1]},${m[2]}) point ${i}`, side: -1, xy: p }));
}
const baseCells = anchors.filter((a) => /map\.osd base$/.test(a.what)).map((a) => a.xy);
if (baseCells.length !== 2) fail(`expected 2 base anchors, found ${baseCells.length}`);
const comp = flood(baseCells.slice(0, 1));
for (const a of anchors) {
  const [x, y] = a.xy;
  if (!open(x, y)) fail(`${a.what} at cell ${x},${y} is blocked`);
  else if (!comp[y * W + x] && !/ fountain$/.test(a.what)) fail(`${a.what} at cell ${x},${y} is open but not connected to the first base`);
}
if (baseCells.length === 2 && !comp[baseCells[1][1] * W + baseCells[1][0]]) fail("the two bases are not connected");

const dist = (pts, x, y) => { let best = 1e9; for (let i = 0; i + 1 < pts.length; i++) { const [ax, ay] = pts[i], [bx, by] = pts[i + 1]; const dx = bx - ax, dy = by - ay, t = Math.max(0, Math.min(1, ((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy || 1))); best = Math.min(best, Math.hypot(x - ax - t * dx, y - ay - t * dy)); } return best; };
const laneWidths = [];
for (const r of routes) {
  if (r.pts.length < 2) continue;
  const minC = (c) => { const near = (x, y) => open(x, y) && dist(r.pts, x, y) <= c; return flood([r.pts[0]], near)[r.pts[r.pts.length - 1][1] * W + r.pts[r.pts.length - 1][0]] === 1; };
  let lo = 0, hi = CORRIDOR;
  if (!minC(hi)) { fail(`route(${r.side},${r.lane}) is not connected inside a ${CORRIDOR}-cell corridor of its polyline`); continue; }
  while (lo < hi) { const mid = (lo + hi) >> 1; if (minC(mid)) hi = mid; else lo = mid + 1; }
  laneWidths.push(`route(${r.side},${r.lane}) corridor ${lo}`);
}
const riverOpen = [];
for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) if (tiles[y * W + x] === 255 && open(x, y)) riverOpen.push([x, y]);
const riverInComp = riverOpen.filter(([x, y]) => comp[y * W + x]).length;
if (riverInComp < RIVER_MIN) fail(`only ${riverInComp} open river cells lie in the base component (need ${RIVER_MIN})`);

console.log(`anchors ${anchors.length}, routes ${routes.length}, river open ${riverOpen.length} (${riverInComp} in base component); ${laneWidths.join("; ")}`);
if (failures) { console.error(`${failures} pathing check failure(s)`); process.exit(1); }
console.log("pathing checks passed");

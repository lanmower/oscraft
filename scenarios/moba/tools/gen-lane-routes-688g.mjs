import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { emitSplit } from "./provenance.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const mapPath = path.join(here, "..", "map.osd");
const outPath = path.join(here, "..", "routes.osd");

const GX = 35;
const UNIT_MILLI = 15.625;
const toMilli = ([wx, wy]) => [(GX * 64 + wx + 8192) * UNIT_MILLI, (GX * 64 + 8192 - wy) * UNIT_MILLI];
const centre = ([a, b, c, d]) => [(a + c) / 2, (b + d) / 2];

const SPAWN_RECT = {
  0: [[-6336, -4224, -5984, -3904], [-4896, -5504, -4608, -5184], [-3648, -6880, -3424, -6688]],
  1: [[2304, 5280, 2752, 5728], [3616, 3584, 3872, 3872], [6176, 3136, 6400, 3392]],
};
const WAYPOINT = [[-5805.1, 5258.6], [-561.7, -651.7], [5613.5, -6650]];
const BASE_TARGET = { 0: [4901.8, 4536.6], 1: [-5467.9, -5891.1] };
const SAMPLE_CELLS = 40;

const lines = fs.readFileSync(mapPath, "utf8").split("\n");
const header = lines.find((l) => l.startsWith("grid(")).match(/grid\((\d+), (\d+)/);
const W = Number(header[1]);
const H = Number(header[2]);
const cells = new Uint8Array(W * H).fill(1);
for (const l of lines) {
  let m = l.match(/^paint\((\d+), (\d+), (\d+), (\d+), (\d+)\)/);
  if (m) {
    const [, x0, y0, x1, y1, v] = m.map(Number);
    for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) cells[y * W + x] = v;
    continue;
  }
  m = l.match(/^paint_ramp\((\d+), (\d+), (\d+), (\d+)\)/);
  if (m) {
    const [, x0, y0, x1, y1] = m.map(Number);
    for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) cells[y * W + x] |= 4;
  }
}
const open = (cx, cy) => cx >= 0 && cy >= 0 && cx < W && cy < H && !(cells[cy * W + cx] & 1);
const roomy = (cx, cy) => {
  for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) if (!open(cx + dx, cy + dy)) return false;
  return true;
};
function snap([x, y]) {
  const cx0 = Math.floor(x / 1000);
  const cy0 = Math.floor(y / 1000);
  for (let r = 0; r < 40; r++)
    for (let dy = -r; dy <= r; dy++)
      for (let dx = -r; dx <= r; dx++)
        if (Math.max(Math.abs(dx), Math.abs(dy)) === r && roomy(cx0 + dx, cy0 + dy)) return [cx0 + dx, cy0 + dy];
  throw new Error(`no open cell near ${x},${y}`);
}

const edge = (cx, cy) => {
  let n = 0;
  for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) if (!open(cx + dx, cy + dy)) n++;
  return n;
};
function shortest(from, to) {
  const idx = ([x, y]) => y * W + x;
  const dist = new Float64Array(W * H).fill(Infinity);
  const prev = new Int32Array(W * H).fill(-1);
  const h = ([x, y]) => {
    const dx = Math.abs(x - to[0]);
    const dy = Math.abs(y - to[1]);
    return Math.max(dx, dy) + 0.414 * Math.min(dx, dy);
  };
  const heap = [[h(from), from[0], from[1]]];
  dist[idx(from)] = 0;
  const push = (e) => {
    heap.push(e);
    let i = heap.length - 1;
    while (i > 0) {
      const p = (i - 1) >> 1;
      if (heap[p][0] <= heap[i][0]) break;
      [heap[p], heap[i]] = [heap[i], heap[p]];
      i = p;
    }
  };
  const pop = () => {
    const top = heap[0];
    const last = heap.pop();
    if (heap.length) {
      heap[0] = last;
      let i = 0;
      for (;;) {
        const l = 2 * i + 1;
        const r = l + 1;
        let s = i;
        if (l < heap.length && heap[l][0] < heap[s][0]) s = l;
        if (r < heap.length && heap[r][0] < heap[s][0]) s = r;
        if (s === i) break;
        [heap[s], heap[i]] = [heap[i], heap[s]];
        i = s;
      }
    }
    return top;
  };
  while (heap.length) {
    const [, x, y] = pop();
    if (x === to[0] && y === to[1]) break;
    const here = dist[idx([x, y])];
    for (let dy = -1; dy <= 1; dy++)
      for (let dx = -1; dx <= 1; dx++) {
        if (!dx && !dy) continue;
        const nx = x + dx;
        const ny = y + dy;
        if (!open(nx, ny)) continue;
        if (dx && dy && (!open(x + dx, y) || !open(x, y + dy))) continue;
        const cost = (dx && dy ? 1.414 : 1) + 0.15 * edge(nx, ny);
        const d = here + cost;
        if (d < dist[idx([nx, ny])]) {
          dist[idx([nx, ny])] = d;
          prev[idx([nx, ny])] = idx([x, y]);
          push([d + h([nx, ny]), nx, ny]);
        }
      }
  }
  if (!Number.isFinite(dist[idx(to)])) throw new Error(`no walkable path ${from} -> ${to}`);
  const out = [];
  for (let at = idx(to); at !== -1; at = prev[at]) out.push([at % W, Math.floor(at / W)]);
  return out.reverse();
}

const cellMilli = ([cx, cy]) => [cx * 1000 + 500, cy * 1000 + 500];
const out = [
  "# Lane routes: the 6.88g creep paths of both sides (player 0 Sentinel, player 1 Scourge; 1 top, 2 mid, 3 bot), written by scenarios/moba/tools/gen-lane-routes-688g.mjs.",
  "# 6.88g sends each wave unit from its melee spawn rect to the lane waypoint unit (top gR, mid JR, bot hR), then to the enemy base target (FR for the Sentinel, GR for the Scourge), and the pathfinder chooses the way.",
  "# Points are the shortest walkable path on this grid between those same points, one every 40 cells, then the waypoint cell and the base cell. Lane creeps then march on the nearest enemy building.",
];
for (const side of [0, 1]) {
  for (let lane = 0; lane < 3; lane++) {
    const start = snap(toMilli(centre(SPAWN_RECT[side][lane])));
    const way = snap(toMilli(WAYPOINT[lane]));
    const base = snap(toMilli(BASE_TARGET[side]));
    const a = shortest(start, way);
    const b = shortest(way, base);
    const full = [...a, ...b.slice(1)];
    const wayAt = a.length - 1;
    const route = [];
    for (let i = SAMPLE_CELLS; i < wayAt - SAMPLE_CELLS / 2; i += SAMPLE_CELLS) route.push(full[i]);
    route.push(way);
    for (let i = wayAt + SAMPLE_CELLS; i < full.length - SAMPLE_CELLS / 2; i += SAMPLE_CELLS) route.push(full[i]);
    route.push(base);
    const flat = route.flatMap((p) => cellMilli(p));
    out.push(`route(${side}, ${lane + 1}, [${flat.join(", ")}])`);
  }
}
const text = out.join("\n") + "\n";
if (process.argv.includes("--check")) {
  if (emitSplit(path.dirname(outPath), "routes.osd", text, true).length) {
    console.error("routes.osd differs from the generator output");
    process.exit(1);
  }
  console.log("routes.osd matches");
} else {
  emitSplit(path.dirname(outPath), "routes.osd", text, false);
  console.log(`wrote ${outPath}`);
}

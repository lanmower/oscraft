// Placement re-anchoring for the 6.88 scenario. Shared by gen-w3-map.mjs (--reanchor).
import fs from "node:fs";
import path from "node:path";

const GRID = 327;
// Re-anchors the placements of an already generated scenario that sit on blocked cells. A placement moves to the
// centre of the nearest open cell (Euclidean in cells, ties by row then column). Usage: node scripts/gen-w3-map.mjs --reanchor <scenario dir>; prints one TSV row per moved line.
export function reanchor(dir) {
  const osd = fs.readFileSync(path.join(dir, "map.osd"), "utf8");
  const grid = new Uint8Array(GRID * GRID).fill(1);
  for (const l of osd.split("\n")) {
    const m = l.match(/^paint\((\d+), (\d+), (\d+), (\d+), (\d+)\)$/);
    if (!m) continue;
    const [x0, y0, x1, y1, v] = m.slice(1).map(Number);
    for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) grid[y * GRID + x] = v;
  }
  const isOpen = (cx, cy) => cx >= 0 && cy >= 0 && cx < GRID && cy < GRID && grid[cy * GRID + cx] === 0;
  const nearestOpen = (cx, cy, taken = new Set()) => {
    let best = null;
    for (let y = 0; y < GRID; y++) for (let x = 0; x < GRID; x++) {
      if (!isOpen(x, y) || taken.has(x + "," + y)) continue;
      const d = (x - cx) ** 2 + (y - cy) ** 2;
      if (!best || d < best.d) best = { x, y, d };
    }
    return best;
  };
  const rows = [];
  // A camp (the placements after one "# camp" line) moves as a group: every member takes the shift of the camp
  // anchor, so units keep their spacing. A member whose shifted cell is blocked moves to its own nearest open cell.
  const placement = (line) => {
    const m = line.match(/^(spawn\("([^"]+)", (-?\d+), )(-?\d+), (-?\d+)\)$/);
    if (m) return { kind: "spawn", unit: m[2], player: m[3], x: Number(m[4]), y: Number(m[5]), fmt: (x, y) => `spawn("${m[2]}", ${m[3]}, ${x}, ${y})` };
    const d = line.match(/^doodad\((\d+), "([^"]+)", (-?\d+), (-?\d+)(.*)\)$/);
    if (d) return { kind: "doodad", unit: d[2], player: d[1], x: Number(d[3]), y: Number(d[4]), fmt: (x, y) => `doodad(${d[1]}, "${d[2]}", ${x}, ${y}${d[5]})` };
    const st = line.match(/^start\((\d+), (-?\d+), (-?\d+)(.*)\)$/);
    if (st) return { kind: "start", unit: "start", player: st[1], x: Number(st[2]), y: Number(st[3]), fmt: (x, y) => `start(${st[1]}, ${x}, ${y}${st[4]})` };
    return null;
  };
  const cell = (p) => [Math.floor(p.x / 1000), Math.floor(p.y / 1000)];
  for (const name of ["map.osd", "neutral.osd", "waves.osd"]) {
    const file = path.join(dir, name);
    const lines = fs.readFileSync(file, "utf8").split("\n");
    const groups = [];
    const camped = lines.some((l) => l.startsWith("# camp"));
    let current = null;
    lines.forEach((line, i) => {
      if (line.startsWith("# camp")) { current = []; groups.push(current); return; }
      const p = placement(line);
      if (!p) return;
      if (!camped) { groups.push([{ i, p, line }]); return; }
      if (!current) { current = []; groups.push(current); }
      current.push({ i, p, line });
    });
    const out = [...lines];
    for (const group of groups) {
      const anchor = group.find((m) => !isOpen(...cell(m.p))) ;
      let shift = [0, 0];
      if (anchor) {
        const [ax, ay] = cell(anchor.p);
        const b = nearestOpen(ax, ay);
        shift = [b.x - ax, b.y - ay];
      }
      const taken = new Set(group.filter((m) => isOpen(...cell(m.p))).map((m) => cell(m.p).join(",")));
      for (const m of group) {
        const [cx, cy] = cell(m.p);
        if (isOpen(cx, cy)) continue;
        let nx = cx + shift[0], ny = cy + shift[1];
        let rule = "camp-shift";
        if (!isOpen(nx, ny) || taken.has(nx + "," + ny)) {
          const b = nearestOpen(nx, ny, taken);
          nx = b.x; ny = b.y; rule = "nearest-open";
        }
        taken.add(nx + "," + ny);
        const ox = m.p.x, oy = m.p.y;
        const x = nx * 1000 + 500, y = ny * 1000 + 500;
        rows.push([name, m.p.kind === "spawn" ? m.p.unit : m.p.unit, m.p.player, ox, oy, x, y, rule].join("\t"));
        out[m.i] = m.p.fmt(x, y);
      }
    }
    fs.writeFileSync(file, out.join("\n"));
  }
  return rows;
}

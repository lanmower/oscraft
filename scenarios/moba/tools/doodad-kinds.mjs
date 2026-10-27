export const MAPPED_ID_BASE = 10000;
export const LANE_CELL_MILLI = 0;
export const BASE_CELL_MILLI = 8000;

export function parseRoutes(routesText) {
  return routesText.split("\n").filter((l) => l.startsWith("route(")).map((l) => {
    const a = l.match(/\[(.*)\]/)[1].split(",").map(Number);
    const pts = [];
    for (let i = 0; i + 1 < a.length; i += 2) pts.push([a[i], a[i + 1]]);
    return pts;
  });
}

export function parseBases(mapOsdText) {
  return mapOsdText.split("\n").filter((l) => l.startsWith('spawn("base"')).map((l) => l.match(/, (-?\d+), (-?\d+)\)/).slice(1).map(Number));
}

export function parseTypeTable(typesText) {
  const [header, ...rows] = typesText.trim().split("\n").map((l) => l.split("\t"));
  const col = Object.fromEntries(header.map((h, i) => [h, i]));
  const kinds = new Map();
  for (const r of rows) if (r[col.tree_rule] !== "tree" && r[col.shipped_kind]) kinds.set(r[col.type], r[col.shipped_kind]);
  return kinds;
}

function segDist(px, py, [ax, ay], [bx, by]) {
  const dx = bx - ax, dy = by - ay, L = dx * dx + dy * dy;
  const t = L ? Math.max(0, Math.min(1, ((px - ax) * dx + (py - ay) * dy) / L)) : 0;
  return Math.hypot(px - ax - t * dx, py - ay - t * dy);
}

export function mappedDoodadLines(records, kinds, { bases, routes }) {
  const lines = [];
  let excludedLane = 0, excludedBase = 0;
  records.forEach((r, k) => {
    const mapped = kinds.get(r.type);
    if (!mapped) return;
    const parts = String(mapped).split("|").map((x) => x.trim()).filter(Boolean);
    const kind = parts.length > 1 ? parts[k % parts.length] : parts[0];
    const onLane = routes.some((pts) => pts.some((p, i) => i + 1 < pts.length && segDist(r.x, r.y, p, pts[i + 1]) <= LANE_CELL_MILLI));
    if (onLane) { excludedLane++; return; }
    if (bases.some(([bx, by]) => Math.hypot(r.x - bx, r.y - by) <= BASE_CELL_MILLI)) { excludedBase++; return; }
    const s = Number(r.scale);
    const suffix = Number.isFinite(s) && s > 0 && Math.round(s) !== 1000 ? `, scale=${Math.round(s)}` : "";
    lines.push(`doodad(${MAPPED_ID_BASE + k}, "${kind}", ${r.x}, ${r.y}${suffix})`);
  });
  return { lines, excludedLane, excludedBase };
}

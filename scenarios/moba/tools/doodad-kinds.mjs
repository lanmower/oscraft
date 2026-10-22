// Placement of the doodad types that have no tree rule: the shipped kind of each type comes from the type table
// (scenarios/moba/doodad-types-688g.tsv, column shipped_kind, written by the inference in docs/DOODAD_TYPES_688G.md).
// A record is not placed when it sits on a lane route (within LANE_CELL_MILLI of a route segment) or inside a base
// footprint (within BASE_CELL_MILLI of a base spawn). Tree records are placed by gen-w3-map.mjs, not here.
export const MAPPED_ID_BASE = 10000;
// A mapped doodad is placed where the reference map actually put it. The lane margin used to drop any record
// within two cells of a route, which cost 144 placements (trees, shrubs, flowers) and left the lanes barer than
// the original. Doodads are scenery and do not block movement, so nothing is excluded for being near a lane.
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

// records: { type, x, y } in milli. Returns the doodad lines and the counts of excluded records.
export function mappedDoodadLines(records, kinds, { bases, routes }) {
  const lines = [];
  let excludedLane = 0, excludedBase = 0;
  records.forEach((r, k) => {
    const kind = kinds.get(r.type);
    if (!kind) return;
    const onLane = routes.some((pts) => pts.some((p, i) => i + 1 < pts.length && segDist(r.x, r.y, p, pts[i + 1]) <= LANE_CELL_MILLI));
    if (onLane) { excludedLane++; return; }
    if (bases.some(([bx, by]) => Math.hypot(r.x - bx, r.y - by) <= BASE_CELL_MILLI)) { excludedBase++; return; }
    lines.push(`doodad(${MAPPED_ID_BASE + k}, "${kind}", ${r.x}, ${r.y})`);
  });
  return { lines, excludedLane, excludedBase };
}

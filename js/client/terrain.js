// Terrain mesh and doodad placements read from the map image (map.bin). Layout: rust/crates/data/FORMAT.md.
// Pure functions with no DOM, so the page and the CI node witness share one code path.

const HEADER_LEN = 16;
const SECTION_ENTRY_LEN = 16;
const GRID_HEADER_LEN = 16;
const DODA_RECORD = 20;
// Ground texture repeats: one 128px tile covers four cells, so the shipped ground patch stays readable at unit scale.
export const GROUND_UV_PER_CELL = 0.25;

function sectionOf(bytes, tag) {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const count = view.getUint32(8, true);
  for (let i = 0; i < count; i++) {
    const at = HEADER_LEN + i * SECTION_ENTRY_LEN;
    if (String.fromCharCode(...bytes.subarray(at, at + 4)) === tag) {
      return { offset: view.getUint32(at + 4, true), count: view.getUint32(at + 8, true), view };
    }
  }
  throw new Error(`map image has no ${tag} section`);
}

export function readGrid(bytes) {
  const { offset, view } = sectionOf(bytes, "GRID");
  const width = view.getUint32(offset, true);
  const height = view.getUint32(offset + 4, true);
  const cellMilli = view.getUint32(offset + 8, true);
  const heightStep = view.getUint32(offset + 12, true);
  const n = width * height;
  const walk = offset + GRID_HEADER_LEN;
  return {
    width,
    height,
    cellMilli,
    heightStep,
    flags: bytes.subarray(walk, walk + n),
    levels: bytes.subarray(walk + n, walk + 2 * n),
    tiles: bytes.subarray(walk + 2 * n, walk + 3 * n),
  };
}

export function readDoodads(bytes) {
  const { offset, count, view } = sectionOf(bytes, "DODA");
  const placements = [];
  for (let i = 0; i < count; i++) {
    const at = offset + i * DODA_RECORD;
    placements.push({
      id: view.getUint32(at, true),
      kind: view.getUint16(at + 4, true),
      x: view.getUint32(at + 8, true),
      y: view.getUint32(at + 12, true),
      rotation: view.getUint32(at + 16, true),
    });
  }
  return placements;
}

// One vertex per grid corner (heights averaged over the cells that touch it) and two triangles per cell.
// Positions are in milli-units: x and z from the cell size, y from the height level times the height step.
// Display-only height field, in cell units per cell. A cell beside a drop under a full cliff level is a slope cell: it relaxes
// toward its neighbours, so a ridge of slope cells (a crest of several levels above its surroundings) draws as a straight
// incline between the surfaces on either side. Cells beside a full cliff level keep their own height, so cliff walls still
// meet the surface where a full drop exists. The sim never reads this field (walkability is the BLOCKED flag, heights are
// read from the map levels), so only the drawn surface changes.
const DISPLAY_RELAX_MAX_PASSES = 5000;
const DISPLAY_RELAX_TOLERANCE = 1e-6;
export function displayHeights(grid) {
  const { width: w, height: h, cellMilli, heightStep, levels } = grid;
  const n = w * h;
  const shown = new Float64Array(n);
  for (let i = 0; i < n; i++) shown[i] = (levels[i] * heightStep) / cellMilli;
  const fixed = new Uint8Array(n);
  const relax = new Uint8Array(n);
  const neighbours = (i) => {
    const x = i % w, y = (i - x) / w;
    const out = [];
    if (x > 0) out.push(i - 1);
    if (x < w - 1) out.push(i + 1);
    if (y > 0) out.push(i - w);
    if (y < h - 1) out.push(i + w);
    return out;
  };
  for (let i = 0; i < n; i++) if (isWaterCell(grid, i)) fixed[i] = 1;
  // Every impassable cell (BLOCKED or cliff, as tiles.js cellKind reads it) keeps its committed height. Walkable cells beside
  // one are not pinned, so the slopes next to a barrier still smooth.
  for (let i = 0; i < n; i++) if (isImpassableCell(grid, i)) fixed[i] = 1;
  // No adjacency pinning: walkable cells beside a cliff or water relax like every other slope cell, so the drop fades
  // across several cells on both sides (one ramp rule; a cliff's own committed height is the only sheer part).
  // A walkable slope cell relaxes even beside a pinned one: the pinned cells keep their committed heights (cliffs stay sheer,
  // water stays level), and the walkable pathways around them smooth out instead of stepping like hills.
  for (let i = 0; i < n; i++) {
    if (fixed[i]) continue;
    const list = neighbours(i);
    if (list.some((j) => Math.abs(shown[i] - shown[j]) > 1e-9)) relax[i] = 1;
  }
  const active = [];
  for (let i = 0; i < n; i++) if (relax[i]) active.push(i);
  const nbr = active.map(neighbours);
  for (let pass = 0; pass < DISPLAY_RELAX_MAX_PASSES; pass++) {
    let change = 0;
    for (let k = 0; k < active.length; k++) {
      const i = active[k];
      const list = nbr[k];
      let sum = 0;
      for (const j of list) sum += shown[j];
      const next = sum / list.length;
      change = Math.max(change, Math.abs(next - shown[i]));
      shown[i] = next;
    }
    if (change < DISPLAY_RELAX_TOLERANCE) break;
  }
  return { shown, fixed };
}

// Water cell as tiles.js cellKind reads it: no cliff bit, and the water tile id (TILE_IDS.water in tiles.js, set from the
// W3E water flag by scripts/convert-688g-map.mjs).
const CLIFF_BIT = 2, BLOCKED_BIT = 1, WATER_TILE_ID = 255;
function isWaterCell(grid, i) {
  return (grid.flags[i] & CLIFF_BIT) === 0 && grid.tiles[i] === WATER_TILE_ID;
}

// Impassable cell: the BLOCKED or CLIFF flag is set, or the cell is water (cellKind is "cliff" or "water" for these).
function isImpassableCell(grid, i) {
  return (grid.flags[i] & (BLOCKED_BIT | CLIFF_BIT)) !== 0 || isWaterCell(grid, i);
}

export function buildTerrain(bytes) {
  hot.set("terrain.buildTerrain", [bytes]);
  const grid = readGrid(bytes);
  const { width: w, height: h, cellMilli, heightStep, levels } = grid;
  const { shown, fixed } = displayHeights(grid);
  const cols = w + 1;
  const positions = new Float32Array(cols * (h + 1) * 3);
  const uvs = new Float32Array(cols * (h + 1) * 2);
  for (let cy = 0; cy <= h; cy++) {
    for (let cx = 0; cx <= w; cx++) {
      // One vertex per grid corner: the average of the display heights of its cells. Impassable cells hold their
      // committed height in shown, so barrier corners follow them; walkable slopes carry the smoothing.
      let sum = 0;
      let touching = 0;
      for (let dy = -1; dy <= 0; dy++) {
        for (let dx = -1; dx <= 0; dx++) {
          const x = cx + dx;
          const y = cy + dy;
          if (x >= 0 && y >= 0 && x < w && y < h) {
            sum += shown[y * w + x];
            touching++;
          }
        }
      }
      const v = (cy * cols + cx) * 3;
      positions[v] = cx * cellMilli;
      positions[v + 1] = (sum / touching) * cellMilli;
      positions[v + 2] = cy * cellMilli;
      uvs[(cy * cols + cx) * 2] = cx * GROUND_UV_PER_CELL;
      uvs[(cy * cols + cx) * 2 + 1] = cy * GROUND_UV_PER_CELL;
    }
  }
  // Each cell is split along its steeper diagonal: the corners that differ most in height are joined, so the two
  // triangles follow the slope and no crease runs across it. Both splits wind the same way (up-facing).
  const indices = new Uint32Array(w * h * 6);
  let t = 0;
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const a = y * cols + x;
      const c = a + cols;
      const steepMain = Math.abs(positions[a * 3 + 1] - positions[(c + 1) * 3 + 1]);
      const steepAnti = Math.abs(positions[(a + 1) * 3 + 1] - positions[c * 3 + 1]);
      if (steepMain > steepAnti) indices.set([a, c, c + 1, a, c + 1, a + 1], t);
      else indices.set([a, c, a + 1, a + 1, c, c + 1], t);
      t += 6;
    }
  }
  return { width: w, height: h, cellMilli, positions, uvs, normals: vertexNormals(positions, indices), indices, tiles: grid.tiles, flags: grid.flags };
}

// Per-vertex normals: the normalised sum of the face normals of every triangle that uses the vertex.
export function vertexNormals(positions, indices) {
  const normals = new Float32Array(positions.length);
  for (let t = 0; t < indices.length; t += 3) {
    const a = indices[t] * 3;
    const b = indices[t + 1] * 3;
    const c = indices[t + 2] * 3;
    const ux = positions[b] - positions[a], uy = positions[b + 1] - positions[a + 1], uz = positions[b + 2] - positions[a + 2];
    const vx = positions[c] - positions[a], vy = positions[c + 1] - positions[a + 1], vz = positions[c + 2] - positions[a + 2];
    const nx = uy * vz - uz * vy, ny = uz * vx - ux * vz, nz = ux * vy - uy * vx;
    for (const v of [a, b, c]) {
      normals[v] += nx;
      normals[v + 1] += ny;
      normals[v + 2] += nz;
    }
  }
  for (let v = 0; v < normals.length; v += 3) {
    const len = Math.hypot(normals[v], normals[v + 1], normals[v + 2]) || 1;
    normals[v] /= len;
    normals[v + 1] /= len;
    normals[v + 2] /= len;
  }
  return normals;
}

// Hot-update hook (see hmr.js): re-runs buildTerrain on the bytes it last received, so display heights and the mesh
// are rebuilt from the map bytes play.js already holds. The returned terrain replaces play.js's mesh.
const hot = (globalThis.__oscraftHot ??= new Map());
export function hmrReload() {
  const args = hot.get("terrain.buildTerrain");
  return args ? buildTerrain(...args) : null;
}

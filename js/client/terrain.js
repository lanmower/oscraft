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
export function buildTerrain(bytes) {
  const grid = readGrid(bytes);
  const { width: w, height: h, cellMilli, heightStep, levels } = grid;
  const cols = w + 1;
  const positions = new Float32Array(cols * (h + 1) * 3);
  const uvs = new Float32Array(cols * (h + 1) * 2);
  for (let cy = 0; cy <= h; cy++) {
    for (let cx = 0; cx <= w; cx++) {
      let sum = 0;
      let touching = 0;
      for (let dy = -1; dy <= 0; dy++) {
        for (let dx = -1; dx <= 0; dx++) {
          const x = cx + dx;
          const y = cy + dy;
          if (x >= 0 && y >= 0 && x < w && y < h) {
            sum += levels[y * w + x];
            touching++;
          }
        }
      }
      const v = (cy * cols + cx) * 3;
      positions[v] = cx * cellMilli;
      positions[v + 1] = (sum / touching) * heightStep;
      positions[v + 2] = cy * cellMilli;
      uvs[(cy * cols + cx) * 2] = cx * GROUND_UV_PER_CELL;
      uvs[(cy * cols + cx) * 2 + 1] = cy * GROUND_UV_PER_CELL;
    }
  }
  limitSlopes(positions, cols, h + 1, cellMilli);
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

// Render-only slope limit: no two corners may differ by more than MAX_SLOPE cells of height per cell of run (side or
// diagonal), so a full cliff level (two cells of rise) spreads over two cells of slope and reads as a ramp, not a hill.
// The sim's pathing never reads heights (walkability is the BLOCKED flag), so only the drawn surface changes. Each
// pass moves the two corners of an over-steep pair toward each other by half the excess; passes stop when none is left.
export const MAX_SLOPE = 1;
const SLOPE_PASSES = 64;
const NEIGHBOURS = [[1, 0, 1], [0, 1, 1], [1, 1, Math.SQRT2], [1, -1, Math.SQRT2]];
export function limitSlopes(positions, cols, rows, cellMilli) {
  for (let pass = 0; pass < SLOPE_PASSES; pass++) {
    let worst = 0;
    for (let cy = 0; cy < rows; cy++) {
      for (let cx = 0; cx < cols; cx++) {
        const a = cy * cols + cx;
        for (const [dx, dy, run] of NEIGHBOURS) {
          const nx = cx + dx, ny = cy + dy;
          if (nx < 0 || ny < 0 || nx >= cols || ny >= rows) continue;
          const b = ny * cols + nx;
          const ya = positions[a * 3 + 1], yb = positions[b * 3 + 1];
          const limit = MAX_SLOPE * cellMilli * run;
          const excess = Math.abs(ya - yb) - limit;
          if (excess <= 0) continue;
          worst = Math.max(worst, excess);
          const shift = excess / 2;
          if (ya > yb) {
            positions[a * 3 + 1] = ya - shift;
            positions[b * 3 + 1] = yb + shift;
          } else {
            positions[a * 3 + 1] = ya + shift;
            positions[b * 3 + 1] = yb - shift;
          }
        }
      }
    }
    if (worst < 1) break;
  }
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

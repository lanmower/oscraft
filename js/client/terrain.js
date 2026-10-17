// Terrain mesh and doodad placements read from the map image (map.bin). Layout: rust/crates/data/FORMAT.md.
// Pure functions with no DOM, so the page and the CI node witness share one code path.

const HEADER_LEN = 16;
const SECTION_ENTRY_LEN = 16;
const GRID_HEADER_LEN = 16;
const DODA_RECORD = 20;

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
    }
  }
  const indices = new Uint32Array(w * h * 6);
  let t = 0;
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const a = y * cols + x;
      const c = a + cols;
      indices.set([a, c, a + 1, a + 1, c, c + 1], t);
      t += 6;
    }
  }
  return { width: w, height: h, cellMilli, positions, indices, tiles: grid.tiles, flags: grid.flags };
}

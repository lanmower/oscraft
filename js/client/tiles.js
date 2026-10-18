// Terrain tiles from the CC0 pack: each grid cell picks one tile model by its kind, and the cells of one kind are baked
// into one mesh so the page draws one call per kind. Pure: no DOM and no fetch, so the Node CI witness shares this path.
import { parseGlb } from "./glb.js";
import { GROUND_UV_PER_CELL, readGrid, vertexNormals } from "./terrain.js";

// Cell kinds and the tile model that draws each, by file name under models/ (see NOTICE.md).
export const TILE_MODELS = Object.freeze({
  grass: "tile-low",
  dirt: "ground",
  lane: "ground_pathTile",
  water: "ground_riverTile",
  cliff: "tile-high",
});
// Fallback colour for a tile model with no base colour texture (the lane and river tiles ship untextured).
export const TILE_TINTS = Object.freeze({ grass: [1, 1, 1], dirt: [0.5, 0.38, 0.24], lane: [0.78, 0.68, 0.48], water: [0.2, 0.45, 0.7], cliff: [0.6, 0.58, 0.55] });
// Author-defined GRID tile ids that select a flat kind; every other unblocked cell is grass.
export const TILE_IDS = Object.freeze({ lane: 2, dirt: 3 });
const CLIFF_BIT = 2;
const BLOCKED_BIT = 1;

// A cell's kind from its walkability bits and tile id: cliff first, then blocked water, then the tile id.
export function cellKind(flags, tile) {
  if (flags & CLIFF_BIT) return "cliff";
  if (flags & BLOCKED_BIT) return "water";
  if (tile === TILE_IDS.lane) return "lane";
  if (tile === TILE_IDS.dirt) return "dirt";
  return "grass";
}

// A GLB centred on x and z, its larger footprint side scaled to `size`, and stood on y = 0.
export function fitModel(glb, size) {
  const p = glb.positions;
  let minX = Infinity, maxX = -Infinity, minY = Infinity, minZ = Infinity, maxZ = -Infinity;
  for (let v = 0; v < p.length; v += 3) {
    minX = Math.min(minX, p[v]);
    maxX = Math.max(maxX, p[v]);
    minY = Math.min(minY, p[v + 1]);
    minZ = Math.min(minZ, p[v + 2]);
    maxZ = Math.max(maxZ, p[v + 2]);
  }
  const scale = size / (Math.max(maxX - minX, maxZ - minZ) || 1);
  const cx = (minX + maxX) / 2;
  const cz = (minZ + maxZ) / 2;
  const positions = new Float32Array(p.length);
  for (let v = 0; v < p.length; v += 3) {
    positions[v] = (p[v] - cx) * scale;
    positions[v + 1] = (p[v + 1] - minY) * scale;
    positions[v + 2] = (p[v + 2] - cz) * scale;
  }
  const fit = Float32Array.of(scale, 0, 0, 0, 0, scale, 0, 0, 0, 0, scale, 0, -cx * scale, -minY * scale, -cz * scale, 1);
  return { positions, normals: glb.normals || vertexNormals(positions, glb.indices), uvs: glb.texcoords, indices: glb.indices, baseColorTexture: glb.baseColorTexture, fit };
}

// Kinds drawn with the repeating ground texture: their cells split by position modulo the texture period, so each
// split shares one instanced draw and still lands on the right part of the texture.
export const GROUND_KINDS = Object.freeze(["grass", "dirt"]);
const GROUND_PERIOD = Math.round(1 / GROUND_UV_PER_CELL);

function groundVariantUvs(model, ox, oy) {
  const uvs = new Float32Array((model.positions.length / 3) * 2);
  for (let v = 0; v < model.positions.length / 3; v++) {
    uvs[v * 2] = (ox + 0.5 + model.positions[v * 3]) * GROUND_UV_PER_CELL;
    uvs[v * 2 + 1] = (oy + 0.5 + model.positions[v * 3 + 2]) * GROUND_UV_PER_CELL;
  }
  return uvs;
}

// Groups every cell by kind (and by texture phase for ground kinds) into instance batches: one translation matrix per
// cell on the cell's height level, so memory and draw calls scale with the kind count, not the cell count.
// Returns null when a kind the map uses has no model, so the caller keeps the heightfield as the fallback.
export function bakeTiles(bytes, models) {
  const grid = readGrid(bytes);
  const { width: w, height: h, cellMilli, heightStep, levels, flags, tiles } = grid;
  const kindAt = new Uint8Array(w * h);
  const names = [];
  for (let i = 0; i < w * h; i++) {
    const kind = cellKind(flags[i], tiles[i]);
    if (!models[kind]) return null;
    let id = names.indexOf(kind);
    if (id < 0) id = names.push(kind) - 1;
    kindAt[i] = id;
  }
  const phases = (kind) => (GROUND_KINDS.includes(kind) ? GROUND_PERIOD * GROUND_PERIOD : 1);
  const groups = new Map();
  for (const kind of names) {
    groups.set(kind, Array.from({ length: phases(kind) }, () => ({ cells: [] })));
  }
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const i = y * w + x;
      const kind = names[kindAt[i]];
      const list = groups.get(kind);
      const phase = list.length === 1 ? 0 : (y % GROUND_PERIOD) * GROUND_PERIOD + (x % GROUND_PERIOD);
      list[phase].cells.push(i);
    }
  }
  const meshes = new Map();
  let cells = 0;
  for (const [kind, list] of groups) {
    const model = models[kind];
    const batches = [];
    list.forEach((group, phase) => {
      if (group.cells.length === 0) return;
      const matrices = new Float32Array(group.cells.length * 16);
      group.cells.forEach((i, n) => {
        const o = n * 16;
        matrices[o] = 1;
        matrices[o + 5] = 1;
        matrices[o + 10] = 1;
        matrices[o + 12] = (i % w) + 0.5;
        matrices[o + 13] = (levels[i] * heightStep) / cellMilli;
        matrices[o + 14] = Math.floor(i / w) + 0.5;
        matrices[o + 15] = 1;
      });
      const uvs = list.length === 1 ? model.uvs : groundVariantUvs(model, phase % GROUND_PERIOD, Math.floor(phase / GROUND_PERIOD));
      batches.push({ uvs, batch: { matrices, count: group.cells.length, dirty: true } });
      cells += group.cells.length;
    });
    meshes.set(kind, { model, batches, cells: batches.reduce((n, b) => n + b.batch.count, 0) });
  }
  return { kinds: meshes, cells, width: w, height: h };
}

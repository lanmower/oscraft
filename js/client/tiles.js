// Terrain tiles from the CC0 pack: each grid cell picks one tile model by its kind, and the cells of one kind are baked
// into one mesh so the page draws one call per kind. Pure: no DOM and no fetch, so the Node CI witness shares this path.
import { parseGlb } from "./glb.js";
import { GROUND_UV_PER_CELL, readGrid, vertexNormals } from "./terrain.js";

// Cell kinds and the tile models that draw each, by file name under models/ (see NOTICE.md). A cell picks one of its
// kind's variants by cellVariant, so the map gets shape variety while every client draws the same tile per cell.
// Grass and dirt draw the ground texture over the model's footprint. Every other kind draws its tint, because the pack's
// colormap atlas maps the cliff and road UVs onto palette stripes (white for the cliff tiles). ground_riverTile ships
// without a texture and no other water tile exists, so water has one model.
export const TILE_MODELS = Object.freeze({
  grass: Object.freeze(["tile-low", "patch-grass", "patch-grass-large"]),
  dirt: Object.freeze(["ground", "patch-dirt", "dirt"]),
  lane: Object.freeze(["road-straight", "road-intersection-path", "road-crossroad-line"]),
  water: Object.freeze(["ground_riverTile"]),
  cliff: Object.freeze(["tile-high", "tile-slant"]),
});
// Tint for every kind drawn without a texture (all but grass and dirt, which take the ground texture).
export const TILE_TINTS = Object.freeze({ grass: [0.36, 0.5, 0.24], dirt: [0.5, 0.38, 0.24], lane: [0.78, 0.68, 0.48], water: [0.2, 0.45, 0.7], cliff: [0.6, 0.58, 0.55] });
// Author-defined GRID tile ids that select a flat kind; every other unblocked cell is grass.
export const TILE_IDS = Object.freeze({ lane: 2, dirt: 3, water: 4 });
export const VOID_REACH = 2;
const CLIFF_BIT = 2;
const BLOCKED_BIT = 1;

// A cell's kind from its walkability bits and tile id: cliff bit first, then a blocked cell is water only when its tile
// id says so and rock otherwise, then the flat tile ids.
export function cellKind(flags, tile) {
  if (flags & CLIFF_BIT) return "cliff";
  if (flags & BLOCKED_BIT) return tile === TILE_IDS.water ? "water" : "cliff";
  if (tile === TILE_IDS.lane) return "lane";
  if (tile === TILE_IDS.dirt) return "dirt";
  return "grass";
}

// Cells with no walkable cell within `reach` cells (Chebyshev): the unreachable void around and between playfields.
export function voidMask(grid, reach = VOID_REACH) {
  const { width: w, height: h, flags } = grid;
  const open = (i) => (flags[i] & (BLOCKED_BIT | CLIFF_BIT)) === 0;
  const rows = new Uint8Array(w * h);
  for (let y = 0; y < h; y++) {
    let last = -Infinity;
    const base = y * w;
    for (let x = 0; x < w; x++) {
      if (open(base + x)) last = x;
      if (x - last <= reach) rows[base + x] = 1;
    }
    last = Infinity;
    for (let x = w - 1; x >= 0; x--) {
      if (open(base + x)) last = x;
      if (last - x <= reach) rows[base + x] = 1;
    }
  }
  const near = new Uint8Array(w * h);
  for (let x = 0; x < w; x++) {
    let last = -Infinity;
    for (let y = 0; y < h; y++) {
      if (rows[y * w + x]) last = y;
      if (y - last <= reach) near[y * w + x] = 1;
    }
    last = Infinity;
    for (let y = h - 1; y >= 0; y--) {
      if (rows[y * w + x]) last = y;
      if (last - y <= reach) near[y * w + x] = 1;
    }
  }
  return near.map((v) => 1 - v);
}

// Cell-space bounding box [x0, z0, x1, z1] of the walkable cells, or the whole grid when none is open.
export function walkableBounds(grid) {
  const { width: w, height: h, flags } = grid;
  let x0 = w, z0 = h, x1 = 0, z1 = 0;
  for (let i = 0; i < w * h; i++) {
    if (flags[i] & (BLOCKED_BIT | CLIFF_BIT)) continue;
    const x = i % w;
    const z = (i - x) / w;
    if (x < x0) x0 = x;
    if (x > x1) x1 = x;
    if (z < z0) z0 = z;
    if (z > z1) z1 = z;
  }
  return x1 < x0 ? [0, 0, w, h] : [x0, z0, x1 + 1, z1 + 1];
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

// Deterministic variant index for a cell: an integer hash of its coordinates, so every client picks the same variant.
export function cellVariant(x, y, count) {
  let h = Math.imul(x, 0x9e3779b1) ^ Math.imul(y, 0x85ebca77);
  h = Math.imul(h ^ (h >>> 15), 0x2c1b3c6d);
  h ^= h >>> 12;
  return (h >>> 0) % count;
}

// Groups every cell by kind and variant (and by texture phase for ground kinds) into instance batches: one translation
// matrix per cell on the cell's height level, so memory and draw calls scale with the kind and variant count, not the
// cell count. models maps each kind to its fitted variant models. Returns null when a kind the map uses has no model,
// so the caller keeps the heightfield as the fallback.
export function bakeTiles(bytes, models) {
  const grid = readGrid(bytes);
  const { width: w, height: h, cellMilli, heightStep, levels, flags, tiles } = grid;
  const empty = voidMask(grid);
  const groups = new Map();
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const i = y * w + x;
      if (empty[i]) continue;
      const kind = cellKind(flags[i], tiles[i]);
      const variants = models[kind];
      if (!variants?.length) return null;
      const v = cellVariant(x, y, variants.length);
      const key = `${kind}/${v}`;
      let group = groups.get(key);
      if (!group) {
        const phaseCount = GROUND_KINDS.includes(kind) ? GROUND_PERIOD * GROUND_PERIOD : 1;
        group = { kind, model: variants[v], phases: Array.from({ length: phaseCount }, () => []) };
        groups.set(key, group);
      }
      const phase = group.phases.length === 1 ? 0 : (y % GROUND_PERIOD) * GROUND_PERIOD + (x % GROUND_PERIOD);
      group.phases[phase].push(i);
    }
  }
  const meshes = new Map();
  let cells = 0;
  for (const [key, { kind, model, phases }] of groups) {
    const batches = [];
    let count = 0;
    phases.forEach((cellList, phase) => {
      if (cellList.length === 0) return;
      const matrices = new Float32Array(cellList.length * 16);
      cellList.forEach((i, n) => {
        const o = n * 16;
        matrices[o] = 1;
        matrices[o + 5] = 1;
        matrices[o + 10] = 1;
        matrices[o + 12] = (i % w) + 0.5;
        matrices[o + 13] = (levels[i] * heightStep) / cellMilli;
        matrices[o + 14] = Math.floor(i / w) + 0.5;
        matrices[o + 15] = 1;
      });
      const uvs = phases.length === 1 ? model.uvs : groundVariantUvs(model, phase % GROUND_PERIOD, Math.floor(phase / GROUND_PERIOD));
      batches.push({ uvs, batch: { matrices, count: cellList.length, dirty: true } });
      count += cellList.length;
    });
    meshes.set(key, { kind, model, batches, cells: count });
    cells += count;
  }
  return { kinds: meshes, cells, width: w, height: h };
}

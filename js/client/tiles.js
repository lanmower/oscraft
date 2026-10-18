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

// World-plane UVs for a baked mesh: the ground texture repeats on the same grid as the heightfield. The CC0 tile
// models' own UVs all sample one palette texel, so the ground tiles take their UVs from position instead.
export function planarUvs(positions) {
  const uvs = new Float32Array((positions.length / 3) * 2);
  for (let v = 0; v < positions.length / 3; v++) {
    uvs[v * 2] = positions[v * 3] * GROUND_UV_PER_CELL;
    uvs[v * 2 + 1] = positions[v * 3 + 2] * GROUND_UV_PER_CELL;
  }
  return uvs;
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

// Bakes every cell into its kind's mesh: the tile model placed at the cell centre, on the cell's height level.
// Returns null when a kind the map uses has no model, so the caller keeps the heightfield as the fallback.
export function bakeTiles(bytes, models) {
  const grid = readGrid(bytes);
  const { width: w, height: h, cellMilli, heightStep, levels, flags, tiles } = grid;
  const perKind = new Map();
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const i = y * w + x;
      const kind = cellKind(flags[i], tiles[i]);
      if (!models[kind]) return null;
      if (!perKind.has(kind)) perKind.set(kind, []);
      perKind.get(kind).push({ x: x + 0.5, y: (levels[i] * heightStep) / cellMilli, z: y + 0.5 });
    }
  }
  const meshes = new Map();
  let cells = 0;
  for (const [kind, placements] of perKind) {
    const model = models[kind];
    const vertexCount = model.positions.length / 3;
    const positions = new Float32Array(placements.length * model.positions.length);
    const normals = new Float32Array(positions.length);
    const uvs = new Float32Array(placements.length * model.uvs.length);
    const indices = new Uint32Array(placements.length * model.indices.length);
    placements.forEach((at, n) => {
      const vBase = n * vertexCount;
      for (let v = 0; v < vertexCount; v++) {
        const p = model.positions;
        const o = (vBase + v) * 3;
        positions[o] = p[v * 3] + at.x;
        positions[o + 1] = p[v * 3 + 1] + at.y;
        positions[o + 2] = p[v * 3 + 2] + at.z;
        normals[o] = model.normals[v * 3];
        normals[o + 1] = model.normals[v * 3 + 1];
        normals[o + 2] = model.normals[v * 3 + 2];
      }
      uvs.set(model.uvs, vBase * 2);
      for (let k = 0; k < model.indices.length; k++) indices[n * model.indices.length + k] = model.indices[k] + vBase;
    });
    cells += placements.length;
    meshes.set(kind, { positions, normals, uvs, indices, cells: placements.length, model });
  }
  return { kinds: meshes, cells, width: w, height: h };
}

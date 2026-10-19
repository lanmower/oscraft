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
  cliff: Object.freeze(["tile-high"]),
  ramp: Object.freeze(["tile-slant"]),
});
// Tint for every kind drawn without a texture (the editor draws all kinds with it; play draws a textured kind in white).
export const TILE_TINTS = Object.freeze({ grass: [1, 1, 1], dirt: [0.62, 0.5, 0.3], lane: [0.78, 0.68, 0.48], water: [0.2, 0.45, 0.7], cliff: [0.6, 0.58, 0.55] });
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

// The tile a cell draws. A blocked cell without the cliff bit is an obstacle standing on open ground (a tree or prop), so it
// draws the ground of its tile id unless it stands above a neighbour, where its drop is a cliff.
export function tileKind(grid, i) {
  const { width: w, height: h, flags, tiles, levels } = grid;
  const kind = cellKind(flags[i], tiles[i]);
  if (kind !== "cliff" || (flags[i] & CLIFF_BIT)) return kind;
  const x = i % w, y = (i - x) / w;
  let lowest = levels[i];
  if (x > 0) lowest = Math.min(lowest, levels[i - 1]);
  if (x < w - 1) lowest = Math.min(lowest, levels[i + 1]);
  if (y > 0) lowest = Math.min(lowest, levels[i - w]);
  if (y < h - 1) lowest = Math.min(lowest, levels[i + w]);
  return lowest < levels[i] ? kind : cellKind(0, tiles[i]);
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

// Material per cell kind: the named texture a kind's batches bind, or null for the model's tint. Adding a surface is one
// entry here; a kind mapped to "ground" is drawn with the repeating ground texture.
export const TILE_MATERIALS = Object.freeze({ grass: "ground", dirt: "ground", lane: "lane", water: null, cliff: "cliff" });
// Texture file per material name (CC0, see NOTICE.md). A material's kinds share one texture; the ground material repeats
// over the map, while the others stretch once over each tile through the model's own UVs.
export const TILE_TEXTURES = Object.freeze({ ground: "textures/ground.png", lane: "textures/lane.jpg", cliff: "textures/rock.jpg" });
// Kinds drawn with the repeating ground texture: their cells split by position modulo the texture period, so each
// split shares one instanced draw and still lands on the right part of the texture.
export const GROUND_KINDS = Object.freeze(Object.keys(TILE_MATERIALS).filter((kind) => TILE_MATERIALS[kind] === "ground"));
const GROUND_PERIOD = Math.round(1 / GROUND_UV_PER_CELL);

function groundVariantUvs(model, ox, oy) {
  const uvs = new Float32Array((model.positions.length / 3) * 2);
  for (let v = 0; v < model.positions.length / 3; v++) {
    uvs[v * 2] = (ox + 0.5 + model.positions[v * 3]) * GROUND_UV_PER_CELL;
    uvs[v * 2 + 1] = (oy + 0.5 + model.positions[v * 3 + 2]) * GROUND_UV_PER_CELL;
  }
  return uvs;
}

// Planar texture coordinates for a model's vertices: u runs along x + z and v up y, one texture repeat per two units. Cliff
// faces take these instead of the model's own atlas UVs, which sample the pack's dark palette stripes.
export function planarUvs(positions) {
  const uvs = new Float32Array((positions.length / 3) * 2);
  for (let v = 0; v < positions.length / 3; v++) {
    uvs[v * 2] = (positions[v * 3] + positions[v * 3 + 2]) * 0.5;
    uvs[v * 2 + 1] = positions[v * 3 + 1] * 0.5;
  }
  return uvs;
}

// Planar texture coordinates on the ground plane, in the ground texture's repeat: a ramp's walking surface takes the ground
// layer, and its UVs follow the surface's real extent rather than the model's atlas.
export function planarGroundUvs(positions) {
  const uvs = new Float32Array((positions.length / 3) * 2);
  for (let v = 0; v < positions.length / 3; v++) {
    uvs[v * 2] = positions[v * 3] * GROUND_UV_PER_CELL;
    uvs[v * 2 + 1] = positions[v * 3 + 2] * GROUND_UV_PER_CELL;
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

function modelTop(model) {
  let top = 0;
  for (let v = 1; v < model.positions.length; v += 3) top = Math.max(top, model.positions[v]);
  return top;
}

// Appends one instance transform: rotation about y, then per-axis scale, then translation (column-major 4x4).
function placeInstance(out, x, y, z, rot, sx, sy, sz) {
  const c = Math.cos(rot), s = Math.sin(rot);
  out.push(c * sx, 0, -s * sx, 0, 0, sy, 0, 0, s * sz, 0, c * sz, 0, x, y, z, 1);
}

// Top height of a model's low end: the highest surface at its smallest x, where a ramp meets the lower cell.
function lowEdgeHeight(model) {
  const p = model.positions;
  let minX = Infinity;
  for (let v = 0; v < p.length; v += 3) minX = Math.min(minX, p[v]);
  let top = 0;
  for (let v = 0; v < p.length; v += 3) if (p[v] - minX < 1e-4) top = Math.max(top, p[v + 1]);
  return top;
}

// A ramp joins a drop of up to one cell, or one height step when a map's step is taller than a cell (the 6.88 layers of
// the generated map are smoothed, so a one-layer step is one cell of slope). Taller drops, and drops beside water, get walls.
const RAMP_MAX_DROP_CELLS = 1;

// Edge pieces from the height data, one batch per key so each key is one draw. A boundary between two cells of equal
// height gets nothing. A drop of at most RAMP_MAX_STEPS between cells that are not water gets a tile-slant ramp (the slant
// rises along its local +x) covering the lower cell, whatever the kinds of the two cells. A steeper drop, or any drop
// beside water, gets a tile-high wall from the lower surface up to the higher one (a cliff cell's own tile adds its
// height). Corners are covered by the edge pieces of the two cells that meet there. `models.cliff` is [wall] and
// `models.ramp` is [ramp], as TILE_MODELS lists them.
export function bakeEdges(grid, models, empty) {
  const { width: w, height: h, cellMilli, heightStep, levels, flags, tiles } = grid;
  const [wall] = models.cliff ?? [];
  const [ramp] = models.ramp ?? [];
  const wallTop = wall ? modelTop(wall) : 0;
  const rampTop = ramp ? modelTop(ramp) : 0;
  const rampLift = ramp ? lowEdgeHeight(ramp) : 0;
  const edges = new Map();
  const put = (key, kind, material, model, instance) => {
    let edge = edges.get(key);
    if (!edge) edges.set(key, (edge = { kind, material, model, instances: [] }));
    edge.instances.push(instance);
  };
  const heightOf = (i) => (levels[i] * heightStep) / cellMilli;
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const i = y * w + x;
      if (empty[i]) continue;
      for (const [dx, dz] of [[1, 0], [0, 1]]) {
        const nx = x + dx, nz = y + dz;
        if (nx >= w || nz >= h) continue;
        const j = nz * w + nx;
        if (empty[j]) continue;
        const ki = tileKind(grid, i);
        const kj = tileKind(grid, j);
        const hi = heightOf(i), hj = heightOf(j);
        const dh = Math.abs(hi - hj);
        const rot = dx ? -Math.PI / 2 : 0;
        const bx = dx ? x + 1 : x + 0.5;
        const bz = dx ? y + 0.5 : y + 1;
        if (dh === 0) continue;
        const low = hi <= hj ? i : j;
        const high = low === i ? j : i;
        const kLow = low === i ? ki : kj;
        const kHigh = high === i ? ki : kj;
        const rampMaxDrop = Math.max(RAMP_MAX_DROP_CELLS, heightStep / cellMilli);
        const gradual = dh <= rampMaxDrop + 1e-9 && kLow !== "water" && kHigh !== "water";
        if (!gradual) {
          if (!wall) continue;
          const base = heightOf(low) + (kLow === "cliff" ? wallTop : 0);
          const top = heightOf(high) + (kHigh === "cliff" ? wallTop : 0);
          if (top > base) put("wall", "cliff", TILE_MATERIALS.cliff, wall, [bx, base, bz, rot, 1, (top - base) / wallTop, 1]);
          continue;
        }
        if (!ramp) continue;
        const dirX = high === j ? dx : -dx;
        const dirZ = high === j ? dz : -dz;
        // A ramp rises its full drop over one cell, which is steep when a step is taller than a cell is wide. When the cell
        // beyond the low side is no higher than the low cell, the ramp runs over that cell too, so the same drop spans two.
        const lx = low % w, lz = (low - lx) / w;
        const fx = lx - dirX, fz = lz - dirZ;
        const far = fx >= 0 && fz >= 0 && fx < w && fz < h ? fz * w + fx : -1;
        const span = dh > 1 + 1e-9 && far >= 0 && !empty[far] && levels[far] <= levels[low] ? 2 : 1;
        const sy = dh / (rampTop - rampLift);
        put(`ramp:${kLow}`, kLow, TILE_MATERIALS[kLow] ?? TILE_MATERIALS.cliff, ramp, [bx - 0.5 * span * dirX, heightOf(low) - rampLift * sy, bz - 0.5 * span * dirZ, Math.atan2(-dirZ, dirX), span, sy, 1]);
      }
    }
  }
  return new Map([...edges].map(([key, { kind, material, model, instances }]) => {
    const matrices = [];
    for (const [x, y, z, r, sx, sy, sz] of instances) placeInstance(matrices, x, y, z, r, sx, sy, sz);
    return [key, { kind, material, model, matrices: Float32Array.from(matrices), count: instances.length }];
  }));
}

// Splat layers: the ground is one draw whose fragment shader blends four layers by a per-cell control map, one RGBA
// texel per cell, each channel the weight of one layer. A cell kind sets its channel; the fourth (rock) channel also
// takes water and cliff cells. SPLAT_TEXTURES gives each layer [albedo, height] files (CC0, see NOTICE.md).
export const SPLAT_LAYERS = Object.freeze(["grass", "dirt", "lane", "rock"]);
const SPLAT_CHANNEL = Object.freeze({ grass: 0, dirt: 1, lane: 2, cliff: 3, water: 3 });
export const SPLAT_TEXTURES = Object.freeze({
  grass: Object.freeze(["textures/grass.jpg", "textures/grass_disp.jpg"]),
  dirt: Object.freeze(["textures/dirt.jpg", "textures/dirt_disp.jpg"]),
  lane: Object.freeze(["textures/lane.jpg", "textures/lane_disp.jpg"]),
  rock: Object.freeze(["textures/rock.jpg", "textures/rock_disp.jpg"]),
});
// Cell kinds whose ground is splatted (a channel below the rock channel): their tile batches are not baked.
export function isSplatKind(kind) {
  return SPLAT_CHANNEL[kind] !== undefined && SPLAT_CHANNEL[kind] < SPLAT_LAYERS.indexOf("rock");
}
// Control map for a grid: { data: Uint8Array RGBA per cell, width, height }, each cell's channel at full weight.
export function controlMap(bytes) {
  const grid = readGrid(bytes);
  const { width, height, flags, tiles } = grid;
  const data = new Uint8Array(width * height * 4);
  for (let i = 0; i < width * height; i++) data[i * 4 + SPLAT_CHANNEL[tileKind(grid, i)]] = 255;
  return { data, width, height };
}
// Rewrites one cell texel to a kind (the creep effect) and returns that texel for a sub-region upload.
export function setSplatTexel(control, index, kind) {
  const o = index * 4;
  control.data.fill(0, o, o + 4);
  control.data[o + SPLAT_CHANNEL[kind]] = 255;
  return control.data.subarray(o, o + 4);
}

// Groups every cell by kind and variant (and by texture phase for ground kinds) into instance batches: one translation
// matrix per cell on the cell's height level, so memory and draw calls scale with the kind and variant count, not the
// cell count. models maps each kind to its fitted variant models. Returns null when a kind the map uses has no model,
// so the caller keeps the heightfield as the fallback.
export function bakeTiles(bytes, models, { splat = false } = {}) {
  const grid = readGrid(bytes);
  const { width: w, height: h, cellMilli, heightStep, levels, flags, tiles } = grid;
  const empty = voidMask(grid);
  const groups = new Map();
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const i = y * w + x;
      if (empty[i]) continue;
      const kind = tileKind(grid, i);
      if (splat && isSplatKind(kind)) continue;
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
    meshes.set(key, { kind, material: TILE_MATERIALS[kind], model, batches, cells: count });
    cells += count;
  }
  for (const [key, edge] of bakeEdges(grid, models, empty)) {
    const batch = { matrices: edge.matrices, count: edge.count, dirty: true };
    meshes.set(key, { kind: edge.kind, material: edge.material, model: edge.model, batches: [{ uvs: edge.model.uvs, batch }], cells: edge.count, planar: key.startsWith("ramp:") });
  }
  return { kinds: meshes, cells, width: w, height: h };
}

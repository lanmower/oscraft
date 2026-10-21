// Terrain tiles from the CC0 pack: each grid cell picks one tile model by its kind, and the cells of one kind are baked
// into one mesh so the page draws one call per kind. Pure: no DOM and no fetch, so the Node CI witness shares this path.
import { parseGlb } from "./glb.js";
import { displayHeights, GROUND_UV_PER_CELL, readGrid, vertexNormals } from "./terrain.js";

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
export const TILE_TINTS = Object.freeze({ grass: [1, 1, 1], dirt: [1.92, 1.36, 1.35], lane: [0.78, 0.68, 0.48], water: [0.2, 0.45, 0.7], cliff: [0.6, 0.58, 0.55] });
// Author-defined GRID tile ids that select a flat kind; every other unblocked cell is grass.
export const TILE_IDS = Object.freeze({ lane: 2, dirt: 3, water: 255 });
export const VOID_REACH = 2;
const CLIFF_BIT = 2;
const BLOCKED_BIT = 1;
// A ramp cell (the 6.88 W3E ramp flag, painted by paint_ramp): a walkable slope, so no cliff wall stands on its edges.
const RAMP_BIT = 4;
export function isRampCell(flags, i) {
  return (flags[i] & RAMP_BIT) !== 0;
}

// A cell's kind from its walkability bits and tile id: cliff bit first, then the water tile id (255, set from the W3E water
// flag by scripts/convert-688g-map.mjs), then any blocked cell is a cliff, then the flat tile ids.
export function cellKind(flags, tile) {
  if (flags & CLIFF_BIT) return "cliff";
  if (tile === TILE_IDS.water) return "water";
  if (flags & BLOCKED_BIT) return "cliff";
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
export const TILE_MATERIALS = Object.freeze({ grass: "ground", dirt: "ground", lane: "lane", water: "water", cliff: "cliff" });
// Texture file per material name (CC0, see NOTICE.md). A material's kinds share one texture; the ground and water materials
// repeat over the map in world space (GROUND_KINDS), while the others stretch once over each tile through the model's UVs.
export const TILE_TEXTURES = Object.freeze({ ground: "textures/ground.jpg", lane: "textures/lane.jpg", cliff: "textures/cliff.jpg", water: "textures/water.jpg" });
// Kinds drawn with a repeating texture (ground and water): their cells split by position modulo the texture period, so each
// split shares one instanced draw and still lands on the right part of the texture. A river is one continuous surface.
export const GROUND_KINDS = Object.freeze(Object.keys(TILE_MATERIALS).filter((kind) => ["ground", "water"].includes(TILE_MATERIALS[kind])));
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
// One 6.88 cliff level is 128 z, which the generated heights draw as 2 cells of height. A drop of a full cliff level or more is
// a wall; any smaller drop is the mesh slope (the interpolated corners), with no wall.
const CLIFF_LEVEL_CELLS = 2;

// The drop between two side-adjacent cells: its height, the lower and higher cell, their kinds, and whether a ramp joins
// them. A map whose height step is a cell or taller takes one level as its cliff level (legacy maps: a drop of one level is
// a ramp). A fine-stepped map (the generated 6.88 heights) takes a full cliff level: smaller drops are slope. A drop beside
// water is never a ramp.
function edgeDrop(grid, i, j) {
  const { cellMilli, heightStep, levels } = grid;
  const heightOf = (k) => (levels[k] * heightStep) / cellMilli;
  const dh = Math.abs(heightOf(i) - heightOf(j));
  const low = heightOf(i) <= heightOf(j) ? i : j;
  const high = low === i ? j : i;
  const kLow = tileKind(grid, low);
  const kHigh = tileKind(grid, high);
  const level = heightStep / cellMilli;
  const rampMaxDrop = level >= RAMP_MAX_DROP_CELLS ? level + 1e-9 : CLIFF_LEVEL_CELLS - 1e-6;
  const gradual = dh <= rampMaxDrop + 1e-9 && kLow !== "water" && kHigh !== "water";
  return { dh, low, high, kLow, kHigh, gradual };
}

// The lower cell of every gradual drop between side-adjacent cells: the cells a ramp covers, by the test bakeEdges uses.
export function rampLowCells(grid) {
  const { width: w, height: h } = grid;
  const cells = [];
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const i = y * w + x;
      for (const j of [x + 1 < w ? i + 1 : -1, y + 1 < h ? i + w : -1]) {
        if (j < 0) continue;
        const drop = edgeDrop(grid, i, j);
        if (drop.dh > 0 && drop.gradual) cells.push(drop.low);
      }
    }
  }
  return cells;
}

// Cliff strips: a drop that is a wall becomes one vertical quad on its cell edge, from the lower surface up to the higher
// one, instead of a tile-high slab per edge. Each edge is stored as 8 numbers: the edge's two ends in x and z, its base
// and top heights, and the unit normal that faces the lower side.
const EDGE_STRIDE = 8;
const STRIP_QUADS_MAX = 16384; // 4 vertices per quad, so a chunk's indices fit in 16 bits
function cliffStrips(edges) {
  const total = edges.length / EDGE_STRIDE;
  const chunks = [];
  for (let start = 0; start < total; start += STRIP_QUADS_MAX) {
    const n = Math.min(STRIP_QUADS_MAX, total - start);
    const positions = new Float32Array(n * 12);
    const normals = new Float32Array(n * 12);
    const uvs = new Float32Array(n * 8);
    const indices = new Uint16Array(n * 6);
    for (let q = 0; q < n; q++) {
      const o = (start + q) * EDGE_STRIDE;
      const [ax, az, bx, bz, base, top, nx, nz] = edges.subarray(o, o + EDGE_STRIDE);
      const corners = [[ax, base, az], [bx, base, bz], [bx, top, bz], [ax, top, az]];
      // The quad faces the lower side: its counter-clockwise front side is the one the normal points out of.
      const facing = ((bz - az) * -nx + (bx - ax) * nz) * (top - base);
      const order = facing < 0 ? [0, 2, 1, 0, 3, 2] : [0, 1, 2, 0, 2, 3];
      const first = q * 4;
      corners.forEach(([x, y, z], k) => {
        const v = (q * 4 + k) * 3;
        positions[v] = x;
        positions[v + 1] = y;
        positions[v + 2] = z;
        normals[v] = nx;
        normals[v + 1] = 0;
        normals[v + 2] = nz;
        uvs[(q * 4 + k) * 2] = (x + z) * 0.5;
        uvs[(q * 4 + k) * 2 + 1] = y * 0.5;
      });
      for (let k = 0; k < 6; k++) indices[q * 6 + k] = first + order[k];
    }
    chunks.push({ positions, normals, uvs, indices, quads: n });
  }
  return chunks;
}

// Edge pieces from the height data, one batch per key so each key is one draw. A boundary between two cells of equal
// height gets nothing. A drop of at most RAMP_MAX_STEPS between cells that are not water gets a tile-slant ramp (the slant
// rises along its local +x) covering the lower cell, whatever the kinds of the two cells. A steeper drop, or any drop
// beside water, is a cliff wall: a vertical strip from the lower surface up to the higher one (a cliff cell's own tile adds
// its height), collected into `walls` as EDGE_STRIDE numbers per edge. Corners are covered by the edge pieces of the two
// cells that meet there. `models.cliff` is [wall] and `models.ramp` is [ramp], as TILE_MODELS lists them; the wall model
// still sizes the cliff cells' own tiles.
export function bakeEdges(grid, models, empty, { ramps = true } = {}) {
  const { width: w, height: h, cellMilli, heightStep, levels, flags, tiles } = grid;
  const [wall] = models.cliff ?? [];
  const [ramp] = models.ramp ?? [];
  const wallTop = wall ? modelTop(wall) : 0;
  const rampTop = ramp ? modelTop(ramp) : 0;
  const rampLift = ramp ? lowEdgeHeight(ramp) : 0;
  const edges = new Map();
  const walls = [];
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
        const bx = dx ? x + 1 : x + 0.5;
        const bz = dx ? y + 0.5 : y + 1;
        const { dh, low, high, kLow, kHigh, gradual } = edgeDrop(grid, i, j);
        if (dh === 0) continue;
        const rampSide = isRampCell(flags, i) || isRampCell(flags, j);
        if (!gradual) {
          if (!wall || rampSide) continue;
          const base = heightOf(low) + (kLow === "cliff" ? wallTop : 0);
          const top = heightOf(high) + (kHigh === "cliff" ? wallTop : 0);
          if (top > base) {
            // The edge runs along z for an x-neighbour and along x for a z-neighbour; the normal points to the lower cell.
            const sgn = low === i ? -1 : 1;
            // The edge runs from its start corner (ax, az) to the far corner (x + 1, y + 1) of the cell.
            const ax = dx ? x + 1 : x, az = dx ? y : y + 1;
            walls.push(ax, az, x + 1, y + 1, base, top, sgn * dx, sgn * dz);
          }
          continue;
        }
        if (!ramp || !ramps) continue;
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
  const pieces = new Map([...edges].map(([key, { kind, material, model, instances }]) => {
    const matrices = [];
    for (const [x, y, z, r, sx, sy, sz] of instances) placeInstance(matrices, x, y, z, r, sx, sy, sz);
    return [key, { kind, material, model, matrices: Float32Array.from(matrices), count: instances.length }];
  }));
  return { pieces, walls: Float64Array.from(walls) };
}

// Splat layers: the ground is one draw whose fragment shader blends four layers by a per-cell control map, one RGBA
// texel per cell, each channel the weight of one layer. A cell kind sets its channel; the fourth (rock) channel takes
// cliff cells. SPLAT_TEXTURES gives each layer [albedo, height] files (CC0, see NOTICE.md).
export const SPLAT_LAYERS = Object.freeze(["grass", "dirt", "lane", "rock"]);
// Water cells take the dirt layer as the riverbed the water is drawn over (the shader shows it through the shallows).
const SPLAT_CHANNEL = Object.freeze({ grass: 0, dirt: 1, lane: 2, cliff: 3, water: 1 });
// The water surface is procedural (render3d.js): the water map is one byte per cell, 255 on water cells.
export const SPLAT_TEXTURES = Object.freeze({
  grass: Object.freeze(["textures/grass.jpg", "textures/grass_disp.jpg"]),
  dirt: Object.freeze(["textures/grass_b_cc0_256.jpg", "textures/grass_b_cc0_256_disp.jpg"]),
  lane: Object.freeze(["textures/lane_earth_cc0_256.jpg", "textures/lane_disp.jpg"]),
  rock: Object.freeze(["textures/rock.jpg", "textures/rock_disp.jpg"]),
});
// Cell kinds whose ground is splatted (a channel below the rock channel): their tile batches are not baked.
export function isSplatKind(kind) {
  return kind === "water" || (SPLAT_CHANNEL[kind] !== undefined && SPLAT_CHANNEL[kind] < SPLAT_LAYERS.indexOf("rock"));
}
// A cell is at a full cliff level when one of its side edges is a wall: a drop of a full cliff level, or any drop beside water.
// Slopes (smaller drops) are not walls.
function isCliffLevelCell(grid, i) {
  const { width: w, height: h } = grid;
  const x = i % w, y = (i - x) / w;
  for (const j of [x > 0 ? i - 1 : -1, x < w - 1 ? i + 1 : -1, y > 0 ? i - w : -1, y < h - 1 ? i + w : -1]) {
    if (j < 0) continue;
    const { dh, kLow, kHigh, gradual } = edgeDrop(grid, i, j);
    if (dh > 0 && !gradual && (dh >= CLIFF_LEVEL_CELLS - 1e-6 || kLow === "water" || kHigh === "water")) return true;
  }
  return false;
}
// The kind a cell's surface layer takes in the splat: a cliff-kind cell on a slope (no wall edge) is ground of its tile id,
// so the cliff layer shows only at a full cliff level or on water.
function splatKind(grid, i) {
  const kind = tileKind(grid, i);
  if (kind !== "cliff" || isCliffLevelCell(grid, i)) return kind;
  return cellKind(0, grid.tiles[i]);
}

// Dirt cells in 8-connected groups smaller than this are scattered paint, not a trail: they draw as grass (client only).
export const DIRT_MIN_GROUP = 10;
// Dirt kind cells in groups of at least DIRT_MIN_GROUP (8-connected) stay dirt; the rest become grass. Returns a Uint8Array
// over the cells: 1 where the cell is dirt and kept. Only the splat's dirt channel reads this; the sim is unchanged.
function keptDirtCells(grid, kinds) {
  const { width: w, height: h } = grid;
  const keep = new Uint8Array(w * h);
  const seen = new Uint8Array(w * h);
  const group = [];
  for (let s = 0; s < w * h; s++) {
    if (kinds[s] !== "dirt" || seen[s]) continue;
    seen[s] = 1;
    group.length = 0;
    group.push(s);
    for (let k = 0; k < group.length; k++) {
      const i = group[k], x = i % w, y = (i - x) / w;
      for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) {
        const nx = x + dx, ny = y + dy;
        if (nx < 0 || ny < 0 || nx >= w || ny >= h) continue;
        const j = ny * w + nx;
        if (!seen[j] && kinds[j] === "dirt") { seen[j] = 1; group.push(j); }
      }
    }
    if (group.length >= DIRT_MIN_GROUP) for (const i of group) keep[i] = 1;
  }
  return keep;
}

// Control map for a grid: { data: Uint8Array RGBA per cell, width, height }, each cell's channel at full weight.
export function controlMap(bytes) {
  const grid = readGrid(bytes);
  const { width, height } = grid;
  const data = new Uint8Array(width * height * 4);
  const kinds = new Array(width * height);
  for (let i = 0; i < width * height; i++) kinds[i] = splatKind(grid, i);
  const keptDirt = keptDirtCells(grid, kinds);
  for (let i = 0; i < width * height; i++) {
    let kind = kinds[i];
    if (kind === "dirt" && !keptDirt[i]) kind = "grass";
    const ch = SPLAT_CHANNEL[kind];
    if (ch !== undefined) data[i * 4 + ch] = 255;
  }
  // A ramp's lower cell is half its ground layer and half rock, so a ramp reads apart from the flat ground of its kind.
  const rock = SPLAT_LAYERS.indexOf("rock");
  for (const i of rampLowCells(grid)) {
    const ch = SPLAT_CHANNEL[splatKind(grid, i)];
    if (ch === undefined || ch >= rock) continue;
    data[i * 4 + ch] = 128;
    data[i * 4 + rock] = 127;
  }
  return { data, width, height };
}
// The water surface: one flat quad per water cell, raised from the riverbed toward the bank (60% of the drop to the lowest
// land neighbour, or a small lift where no land touches it). Positions are in cell units, like the terrain after scaling.
// The splat shader draws the water on these quads, so the surface is what the eye sees over the bed.
export function waterSurface(bytes) {
  const grid = readGrid(bytes);
  const { width: w, height: h } = grid;
  const { shown } = displayHeights(grid);
  const isWater = (k) => tileKind(grid, k) === "water";
  const positions = [];
  const normals = [];
  const uvs = [];
  const indices = [];
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const i = y * w + x;
      if (!isWater(i)) continue;
      let bank = Infinity;
      if (x > 0 && !isWater(i - 1)) bank = Math.min(bank, shown[i - 1]);
      if (x < w - 1 && !isWater(i + 1)) bank = Math.min(bank, shown[i + 1]);
      if (y > 0 && !isWater(i - w)) bank = Math.min(bank, shown[i - w]);
      if (y < h - 1 && !isWater(i + w)) bank = Math.min(bank, shown[i + w]);
      const lift = bank === Infinity ? 0.2 : Math.max(0, bank - shown[i]);
      const level = shown[i] + 0.6 * lift;
      const base = positions.length / 3;
      for (const [cx, cy] of [[x, y], [x + 1, y], [x + 1, y + 1], [x, y + 1]]) {
        positions.push(cx, level, cy);
        normals.push(0, 1, 0);
        uvs.push(cx, cy);
      }
      indices.push(base, base + 2, base + 1, base, base + 3, base + 2);
    }
  }
  return { positions: new Float32Array(positions), normals: new Float32Array(normals), uvs: new Float32Array(uvs), indices: new Uint32Array(indices) };
}
// Water map for a grid: { data: Uint8Array one byte per cell, width, height }, 255 on water cells only. A water cell is a
// blocked cell painted with water tile id 4 in the map (scenarios/moba/map.osd); nothing else takes water, so nearest
// sampling keeps land and cliff cells dry.
export function waterMap(bytes) {
  const grid = readGrid(bytes);
  const { width, height } = grid;
  const data = new Uint8Array(width * height);
  const isWater = (x, y) => x >= 0 && y >= 0 && x < width && y < height && tileKind(grid, y * width + x) === "water";
  // The outer ring of a river is painted as water tiles too: only a water cell whose eight neighbours are water is wet
  // (deep water). The shore ring stays dry, so its ground shows instead of the water.
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      if (!isWater(x, y)) continue;
      let interior = true;
      for (let dy = -1; dy <= 1 && interior; dy++) for (let dx = -1; dx <= 1; dx++) if (!isWater(x + dx, y + dy)) { interior = false; break; }
      if (interior) data[y * width + x] = 255;
    }
  }
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
export function bakeTiles(bytes, models, { splat = false, ramps = true, cliffCells = true, water = true } = {}) {
  hot.set("tiles.bakeTiles", [bytes, models, { splat, ramps, cliffCells, water }]);
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
      if (!cliffCells && kind === "cliff") continue;
      // Water cells draw as flat tinted squares (the water tile). With the splat floor on, the floor shows them instead.
      if (!water && kind === "water") continue;
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
  const edges = bakeEdges(grid, models, empty, { ramps });
  for (const [key, edge] of edges.pieces) {
    const batch = { matrices: edge.matrices, count: edge.count, dirty: true };
    meshes.set(key, { kind: edge.kind, material: edge.material, model: edge.model, batches: [{ uvs: edge.model.uvs, batch }], cells: edge.count, planar: key.startsWith("ramp:") });
  }
  // Cliff walls: one identity-placed mesh per strip chunk, so a wall costs its quad and no per-cell slab.
  let quads = 0;
  cliffStrips(edges.walls).forEach((strip, c) => {
    const identity = Float32Array.of(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1);
    const model = { positions: strip.positions, normals: strip.normals, indices: strip.indices, uvs: strip.uvs, baseColorTexture: null, glbUrl: undefined };
    meshes.set(`strip/${c}`, { kind: "cliff", material: TILE_MATERIALS.cliff, model, batches: [{ uvs: strip.uvs, batch: { matrices: identity, count: 1, dirty: true } }], cells: strip.quads, strip: true });
    quads += strip.quads;
  });
  // Edge walls are no longer tile-high slabs: the count of slabs is zero by construction.
  return { kinds: meshes, cells, strips: quads, slabs: 0, width: w, height: h };
}

// Hot-update hook (see hmr.js): re-runs bakeTiles on the bytes, models and options of its last call, so tile kinds and
// batches are rebuilt from the map bytes play.js already holds. The returned batches replace play.js's tile meshes.
const hot = (globalThis.__oscraftHot ??= new Map());
export function hmrReload() {
  const args = hot.get("tiles.bakeTiles");
  return args ? bakeTiles(...args) : null;
}

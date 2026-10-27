import { parseGlb } from "./glb.js";
import { displayHeights, GROUND_UV_PER_CELL, readGrid, vertexNormals } from "./terrain.js";

export const TILE_MODELS = Object.freeze({
  grass: Object.freeze(["tile-low", "patch-grass", "patch-grass-large"]),
  dirt: Object.freeze(["ground", "patch-dirt", "dirt"]),
  lane: Object.freeze(["road-straight", "road-intersection-path", "road-crossroad-line"]),
  water: Object.freeze(["ground_riverTile"]),
  cliff: Object.freeze(["tile-high"]),
  ramp: Object.freeze(["tile-slant"]),
});
export const TILE_TINTS = Object.freeze({
  grass: [1, 1, 1], dirt: [1.92, 1.36, 1.35], lane: [0.78, 0.68, 0.48], water: [0.2, 0.45, 0.7], cliff: [0.6, 0.58, 0.55],
  meadow: [0.94, 1.05, 0.9], litter: [1.08, 1, 0.7], vine: [0.74, 0.95, 0.78],
  cracked: [0.88, 0.86, 0.82], track: [0.88, 0.82, 0.7], cobble: [0.8, 0.79, 0.76],
});
export const TILE_IDS = Object.freeze({ lane: 2, dirt: 3, water: 255, meadow: 4, litter: 5, vine: 6, cracked: 7, track: 8, cobble: 9 });
export const TILESET_TILE = Object.freeze({ Cgrs: 4, Clvg: 5, Zvin: 6, Crck: 7, Ndrd: 8, Zbks: 9, Cdrt: 2, Cdrd: 2, Cvin: 3 });
export const VOID_REACH = 2;
const CLIFF_BIT = 2;
const BLOCKED_BIT = 1;
const RAMP_BIT = 4;
export function isRampCell(flags, i) {
  return (flags[i] & RAMP_BIT) !== 0;
}

export function cellKind(flags, tile) {
  if (flags & CLIFF_BIT) return "cliff";
  if (tile === TILE_IDS.water) return "water";
  if (flags & BLOCKED_BIT) return "cliff";
  if (tile === TILE_IDS.lane) return "lane";
  if (tile === TILE_IDS.dirt) return "dirt";
  if (tile === TILE_IDS.meadow) return "meadow";
  if (tile === TILE_IDS.litter) return "litter";
  if (tile === TILE_IDS.vine) return "vine";
  if (tile === TILE_IDS.cracked) return "cracked";
  if (tile === TILE_IDS.track) return "track";
  if (tile === TILE_IDS.cobble) return "cobble";
  return "grass";
}

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

export const TILE_MATERIALS = Object.freeze({ grass: "ground", dirt: "ground", lane: "path_road", water: "water", cliff: "cliff" });
export const TILE_TEXTURES = Object.freeze({ ground: "ground", path_road: "path_road", cliff: "cliff", water: "water" });
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

export function planarUvs(positions) {
  const uvs = new Float32Array((positions.length / 3) * 2);
  for (let v = 0; v < positions.length / 3; v++) {
    uvs[v * 2] = (positions[v * 3] + positions[v * 3 + 2]) * 0.5;
    uvs[v * 2 + 1] = positions[v * 3 + 1] * 0.5;
  }
  return uvs;
}

export function planarGroundUvs(positions) {
  const uvs = new Float32Array((positions.length / 3) * 2);
  for (let v = 0; v < positions.length / 3; v++) {
    uvs[v * 2] = positions[v * 3] * GROUND_UV_PER_CELL;
    uvs[v * 2 + 1] = positions[v * 3 + 2] * GROUND_UV_PER_CELL;
  }
  return uvs;
}

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

function placeInstance(out, x, y, z, rot, sx, sy, sz) {
  const c = Math.cos(rot), s = Math.sin(rot);
  out.push(c * sx, 0, -s * sx, 0, 0, sy, 0, 0, s * sz, 0, c * sz, 0, x, y, z, 1);
}

function lowEdgeHeight(model) {
  const p = model.positions;
  let minX = Infinity;
  for (let v = 0; v < p.length; v += 3) minX = Math.min(minX, p[v]);
  let top = 0;
  for (let v = 0; v < p.length; v += 3) if (p[v] - minX < 1e-4) top = Math.max(top, p[v + 1]);
  return top;
}

const RAMP_MAX_DROP_CELLS = 1;
const CLIFF_LEVEL_CELLS = 2;

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

const EDGE_STRIDE = 8;
const STRIP_QUADS_MAX = 16384;
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
            const sgn = low === i ? -1 : 1;
            const ax = dx ? x + 1 : x, az = dx ? y : y + 1;
            walls.push(ax, az, x + 1, y + 1, base, top, sgn * dx, sgn * dz);
          }
          continue;
        }
        if (!ramp || !ramps) continue;
        const dirX = high === j ? dx : -dx;
        const dirZ = high === j ? dz : -dz;
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

export const SPLAT_LAYERS = Object.freeze(["grass", "dirt", "lane", "rock"]);
const SPLAT_CHANNEL = Object.freeze({ grass: 0, dirt: 1, lane: 2, cliff: 3, water: 1, meadow: 0, litter: 0, vine: 0, cobble: 1, track: 2, cracked: 3 });
export const CRACKED_ROCK_SHARE = 0.42;
export const SPLAT_TEXTURES = Object.freeze({
  grass: Object.freeze(["textures.grass", "textures.grassDisp"]),
  dirt: Object.freeze(["textures.dirt", "textures.dirtDisp"]),
  lane: Object.freeze(["textures.path_earth", "textures.path_earth_disp"]),
  rock: Object.freeze(["textures.rock", "textures.rockDisp"]),
});
export function isSplatKind(kind) {
  return kind === "water" || kind === "cracked" || (SPLAT_CHANNEL[kind] !== undefined && SPLAT_CHANNEL[kind] < SPLAT_LAYERS.indexOf("rock"));
}
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
function splatKind(grid, i) {
  const kind = tileKind(grid, i);
  if (kind !== "cliff" || isCliffLevelCell(grid, i)) return kind;
  return cellKind(0, grid.tiles[i]);
}

export const DIRT_MIN_GROUP = 10;
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

const RAMP_SPAN = 4;
const WATER_SHARE_LO = 0.25;
function easeStep(t) {
  const c = t < 0 ? 0 : t > 1 ? 1 : t;
  return c * c * (3 - 2 * c);
}
function boxMean(grid, values) {
  const { width: w, height: h } = grid;
  const rows = new Float64Array(w * h);
  for (let y = 0; y < h; y++) {
    const base = y * w;
    let run = 0;
    for (let x = 0; x <= RAMP_SPAN && x < w; x++) run += values[base + x];
    for (let x = 0; x < w; x++) {
      rows[base + x] = run;
      const add = x + RAMP_SPAN + 1;
      const drop = x - RAMP_SPAN;
      if (add < w) run += values[base + add];
      if (drop >= 0) run -= values[base + drop];
    }
  }
  const out = new Float32Array(w * h);
  for (let x = 0; x < w; x++) {
    const cx = Math.min(w - 1, x + RAMP_SPAN) - Math.max(0, x - RAMP_SPAN) + 1;
    for (let y = 0; y < h; y++) {
      const y0 = Math.max(0, y - RAMP_SPAN);
      const y1 = Math.min(h - 1, y + RAMP_SPAN);
      let sum = 0;
      for (let yy = y0; yy <= y1; yy++) sum += rows[yy * w + x];
      out[y * w + x] = sum / (cx * (y1 - y0 + 1));
    }
  }
  return out;
}

const WATER_ROCK_WET_LO = 0.9;
export function controlMap(bytes) {
  const grid = readGrid(bytes);
  const { width, height } = grid;
  const data = new Uint8Array(width * height * 4);
  const kinds = new Array(width * height);
  for (let i = 0; i < width * height; i++) kinds[i] = splatKind(grid, i);
  const keptDirt = keptDirtCells(grid, kinds);
  const variant = new Uint8Array(width * height);
  for (let i = 0; i < width * height; i++) {
    let kind = kinds[i];
    if (kind === "dirt" && !keptDirt[i]) kind = "grass";
    if (kind === "litter" || kind === "vine") variant[i] = 255;
    const ch = SPLAT_CHANNEL[kind];
    if (ch === undefined) continue;
    if (kind === "cracked") data[i * 4 + SPLAT_CHANNEL.grass] = 255;
    data[i * 4 + ch] = kind === "cracked" ? Math.round(255 * CRACKED_ROCK_SHARE) : 255;
  }
  const ground = SPLAT_LAYERS.indexOf("rock");
  const groundShare = [];
  for (let c = 0; c < ground; c++) {
    const one = new Float32Array(width * height);
    for (let i = 0; i < one.length; i++) one[i] = data[i * 4 + c] / 255;
    groundShare.push(boxMean(grid, boxMean(grid, one)));
  }
  for (let i = 0; i < width * height; i++) {
    const o = i * 4;
    const budget = 255 - data[o + ground];
    let total = 0;
    for (let c = 0; c < ground; c++) total += groundShare[c][i];
    if (total <= 0 || budget <= 0) continue;
    for (let c = 0; c < ground; c++) data[o + c] = Math.round((budget * groundShare[c][i]) / total);
  }
  const rock = SPLAT_LAYERS.indexOf("rock");
  const walls = new Float32Array(width * height);
  for (let i = 0; i < walls.length; i++) if (isCliffLevelCell(grid, i)) walls[i] = 1;
  const rockShare = boxMean(grid, boxMean(grid, walls));
  const wet = new Float32Array(width * height);
  for (let i = 0; i < wet.length; i++) if (kinds[i] === "water") wet[i] = 1;
  const wetShare = boxMean(grid, wet);
  for (let i = 0; i < walls.length; i++) {
    const r = kinds[i] === "water"
      ? easeStep((wetShare[i] - WATER_ROCK_WET_LO) / (1 - WATER_ROCK_WET_LO))
      : easeStep(2 * rockShare[i]);
    if (r <= 0) continue;
    const o = i * 4;
    for (let c = 0; c < rock; c++) data[o + c] = Math.round(data[o + c] * (1 - r));
    data[o + rock] = Math.round(255 * r + data[o + rock] * (1 - r));
  }
  for (const i of rampLowCells(grid)) {
    const ch = SPLAT_CHANNEL[splatKind(grid, i)];
    if (ch === undefined || ch >= rock) continue;
    data[i * 4 + ch] = 128;
    data[i * 4 + rock] = 127;
  }
  return { data, width, height, variant: { data: variant, width, height } };
}
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
export function waterMap(bytes) {
  const grid = readGrid(bytes);
  const { width, height } = grid;
  const wet = new Float32Array(width * height);
  for (let i = 0; i < wet.length; i++) if (tileKind(grid, i) === "water") wet[i] = 1;
  const share = boxMean(grid, wet);
  const data = new Uint8Array(width * height);
  for (let i = 0; i < data.length; i++) data[i] = Math.round(255 * easeStep((share[i] - WATER_SHARE_LO) / (1 - WATER_SHARE_LO)));
  return { data, width, height };
}
export function setSplatTexel(control, index, kind) {
  const o = index * 4;
  control.data.fill(0, o, o + 4);
  control.data[o + SPLAT_CHANNEL[kind]] = 255;
  return control.data.subarray(o, o + 4);
}

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
  let quads = 0;
  cliffStrips(edges.walls).forEach((strip, c) => {
    const identity = Float32Array.of(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1);
    const model = { positions: strip.positions, normals: strip.normals, indices: strip.indices, uvs: strip.uvs, baseColorTexture: null, glbUrl: undefined };
    meshes.set(`strip/${c}`, { kind: "cliff", material: TILE_MATERIALS.cliff, model, batches: [{ uvs: strip.uvs, batch: { matrices: identity, count: 1, dirty: true } }], cells: strip.quads, strip: true });
    quads += strip.quads;
  });
  return { kinds: meshes, cells, strips: quads, slabs: 0, width: w, height: h };
}

const hot = (globalThis.__oscraftHot ??= new Map());
export function hmrReload() {
  const args = hot.get("tiles.bakeTiles");
  return args ? bakeTiles(...args) : null;
}

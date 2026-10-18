// Map editor: paints cell kinds and heights, places doodads and unit spawns, marks regions, and exports map.osd text.
// The model and exportOsd are pure so the Node witness (editor-check.mjs) shares them with the page.
import { readDoodads, readGrid } from "./terrain.js";
import { cellKind, TILE_IDS, TILE_TINTS } from "./tiles.js";
import { createCamera, createRenderer, eyeOf, multiply, orbitCamera, panCamera, scaling, translation, viewProjection, zoomCamera } from "./render3d.js";

export const DOODAD_KINDS = Object.freeze(["rock", "tree", "crystal", "debris", "base", "shop", "fountain", "lane", "camp"]);
export const CELL_KINDS = Object.freeze({
  grass: { cell: 0, tile: 0 },
  dirt: { cell: 0, tile: TILE_IDS.dirt },
  lane: { cell: 0, tile: TILE_IDS.lane },
  water: { cell: 1, tile: 0 },
  cliff: { cell: 2, tile: 0 },
});
const MAX_LEVEL = 255;

export function newModel(width, height, cellMilli = 1000) {
  const n = width * height;
  return { width, height, cellMilli, heightStep: 1000, cell: new Uint8Array(n), tile: new Uint8Array(n), level: new Uint8Array(n), doodads: [], spawns: [], regions: [] };
}

export function modelFromMap(bytes) {
  const g = readGrid(bytes);
  const m = newModel(g.width, g.height, g.cellMilli);
  m.heightStep = g.heightStep;
  m.cell.set(g.flags);
  m.tile.set(g.tiles);
  m.level.set(g.levels);
  m.doodads = readDoodads(bytes).map((d) => ({ id: d.id, kind: DOODAD_KINDS[d.kind] ?? DOODAD_KINDS[0], x: d.x, y: d.y, rotation: d.rotation }));
  return m;
}

export function kindAt(m, x, y) {
  const i = y * m.width + x;
  return cellKind(m.cell[i], m.tile[i]);
}

function inside(m, x, y) {
  return x >= 0 && y >= 0 && x < m.width && y < m.height;
}

// Applies f(i) to the cells within `radius` of (x, y); radius 0 is one cell.
function brush(m, x, y, radius, f) {
  for (let dy = -radius; dy <= radius; dy++) {
    for (let dx = -radius; dx <= radius; dx++) {
      if (dx * dx + dy * dy <= radius * radius + radius && inside(m, x + dx, y + dy)) f((y + dy) * m.width + x + dx);
    }
  }
}

export function paintKind(m, x, y, kind, radius = 0) {
  const def = CELL_KINDS[kind];
  brush(m, x, y, radius, (i) => { m.cell[i] = def.cell; m.tile[i] = def.tile; });
}

export function paintLevel(m, x, y, level, radius = 0) {
  const v = Math.max(0, Math.min(MAX_LEVEL, level | 0));
  brush(m, x, y, radius, (i) => { m.level[i] = v; });
}

function nextDoodadId(m) {
  return m.doodads.reduce((top, d) => Math.max(top, d.id), 0) + 1;
}

export function placeDoodad(m, kind, xMilli, yMilli) {
  const d = { id: nextDoodadId(m), kind, x: Math.round(xMilli), y: Math.round(yMilli), rotation: 0 };
  m.doodads.push(d);
  return d;
}

export function placeSpawn(m, unit, player, xMilli, yMilli) {
  const s = { unit, player: player | 0, x: Math.round(xMilli), y: Math.round(yMilli) };
  m.spawns.push(s);
  return s;
}

export function addRegion(m, name, x0, y0, x1, y1) {
  const r = { name, x0: Math.min(x0, x1), y0: Math.min(y0, y1), x1: Math.max(x0, x1) + 1, y1: Math.max(y0, y1) + 1 };
  m.regions.push(r);
  return r;
}

// Deletes the doodad or spawn nearest the point within `reach` milli-units; returns what was removed or null.
export function deleteNear(m, xMilli, yMilli, reach) {
  let best = null;
  const consider = (list, kind) => list.forEach((item, index) => {
    const d = Math.hypot(item.x - xMilli, item.y - yMilli);
    if (d <= reach && (!best || d < best.d)) best = { d, list, index, kind };
  });
  consider(m.doodads, "doodad");
  consider(m.spawns, "spawn");
  if (!best) return null;
  return { kind: best.kind, item: best.list.splice(best.index, 1)[0] };
}

// Rectangles covering every cell whose value is not `skip`: row runs of equal value, merged down while they line up.
export function rects(values, width, height, skip) {
  const out = [];
  let open = new Map();
  for (let y = 0; y < height; y++) {
    const next = new Map();
    let x = 0;
    while (x < width) {
      const v = values[y * width + x];
      if (v === skip) { x++; continue; }
      let e = x + 1;
      while (e < width && values[y * width + e] === v) e++;
      const key = `${x},${e},${v}`;
      const r = open.get(key);
      if (r) { r.y1 = y + 1; next.set(key, r); } else {
        const fresh = { x0: x, y0: y, x1: e, y1: y + 1, v };
        out.push(fresh);
        next.set(key, fresh);
      }
      x = e;
    }
    open = next;
  }
  return out;
}

export function exportOsd(m) {
  const lines = [`grid(${m.width}, ${m.height}, cell_milli=${m.cellMilli}, fill=0)`];
  const rect = (fn, r) => `${fn}(${r.x0}, ${r.y0}, ${r.x1}, ${r.y1}, ${r.v})`;
  for (const r of rects(m.cell, m.width, m.height, 0)) lines.push(rect("paint", r));
  for (const r of rects(m.tile, m.width, m.height, 0)) lines.push(rect("paint_tile", r));
  lines.push(`height_step(${m.heightStep})`);
  for (const r of rects(m.level, m.width, m.height, 0)) lines.push(rect("paint_height", r));
  for (const d of m.doodads) lines.push(`doodad(${d.id}, "${d.kind}", ${d.x}, ${d.y}${d.rotation ? `, rotation=${d.rotation}` : ""})`);
  for (const s of m.spawns) lines.push(`spawn("${s.unit}", ${s.player}, ${s.x}, ${s.y})`);
  for (const r of m.regions) lines.push(`# region ${r.name} ${r.x0} ${r.y0} ${r.x1} ${r.y1}`);
  return lines.join("\n") + "\n";
}

// Pure world helpers for drawing: cell units on x and z, height in cells.
function levelHeight(m, i) {
  return (m.level[i] * m.heightStep) / m.cellMilli;
}

export function kindMeshes(m) {
  const per = new Map();
  for (let y = 0; y < m.height; y++) {
    for (let x = 0; x < m.width; x++) {
      const kind = kindAt(m, x, y);
      if (!per.has(kind)) per.set(kind, []);
      per.get(kind).push(x, y);
    }
  }
  const meshes = new Map();
  for (const [kind, cells] of per) {
    const count = cells.length / 2;
    const positions = new Float32Array(count * 12);
    const normals = new Float32Array(count * 12);
    const indices = new Uint32Array(count * 6);
    for (let c = 0; c < count; c++) {
      const x = cells[c * 2];
      const y = cells[c * 2 + 1];
      const h = levelHeight(m, y * m.width + x);
      positions.set([x, h, y, x + 1, h, y, x, h, y + 1, x + 1, h, y + 1], c * 12);
      for (let v = 0; v < 4; v++) normals[c * 12 + v * 3 + 1] = 1;
      const b = c * 4;
      indices.set([b, b + 2, b + 1, b + 1, b + 2, b + 3], c * 6);
    }
    meshes.set(kind, { positions, normals, uvs: new Float32Array(count * 8), indices, color: TILE_TINTS[kind] });
  }
  return meshes;
}

function cubeMesh(color) {
  const p = [];
  const idx = [];
  const n = [];
  const faces = [[[0, 0, 1], [1, 0, 0], [0, 1, 0]], [[0, 0, -1], [-1, 0, 0], [0, 1, 0]], [[1, 0, 0], [0, 0, -1], [0, 1, 0]], [[-1, 0, 0], [0, 0, 1], [0, 1, 0]], [[0, 1, 0], [1, 0, 0], [0, 0, -1]], [[0, -1, 0], [1, 0, 0], [0, 0, 1]]];
  for (const [nrm, u, v] of faces) {
    const base = p.length / 3;
    for (const [a, b] of [[-1, -1], [1, -1], [-1, 1], [1, 1]]) {
      for (let k = 0; k < 3; k++) p.push(0.5 * nrm[k] + 0.5 * a * u[k] + 0.5 * b * v[k] + (k === 1 ? 0.5 : 0));
      n.push(...nrm);
    }
    idx.push(base, base + 1, base + 2, base + 1, base + 3, base + 2);
  }
  return { positions: Float32Array.from(p), normals: Float32Array.from(n), uvs: new Float32Array(p.length / 3 * 2), indices: Uint32Array.from(idx), color };
}

function invert(m) {
  const a = Float64Array.from(m);
  const inv = Float64Array.from([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]);
  for (let c = 0; c < 4; c++) {
    let p = c;
    for (let r = c + 1; r < 4; r++) if (Math.abs(a[c * 4 + r]) > Math.abs(a[c * 4 + p])) p = r;
    for (let k = 0; k < 4; k++) {
      [a[k * 4 + c], a[k * 4 + p]] = [a[k * 4 + p], a[k * 4 + c]];
      [inv[k * 4 + c], inv[k * 4 + p]] = [inv[k * 4 + p], inv[k * 4 + c]];
    }
    const d = a[c * 4 + c] || 1e-12;
    for (let k = 0; k < 4; k++) { a[k * 4 + c] /= d; inv[k * 4 + c] /= d; }
    for (let r = 0; r < 4; r++) {
      if (r === c) continue;
      const f = a[c * 4 + r];
      for (let k = 0; k < 4; k++) { a[k * 4 + r] -= f * a[k * 4 + c]; inv[k * 4 + r] -= f * inv[k * 4 + c]; }
    }
  }
  return inv;
}

// The ground point (cell units) under a pixel: unprojects the pixel to a ray and meets it with y = 0.
export function pickGround(cam, aspect, px, py, w, h) {
  const inv = invert(viewProjection(cam, aspect));
  const ndc = (z) => {
    const v = [(px / w) * 2 - 1, 1 - (py / h) * 2, z, 1];
    const o = [0, 0, 0, 0];
    for (let r = 0; r < 4; r++) for (let k = 0; k < 4; k++) o[r] += inv[k * 4 + r] * v[k];
    return [o[0] / o[3], o[1] / o[3], o[2] / o[3]];
  };
  const a = eyeOf(cam);
  const b = ndc(1);
  const dir = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
  if (Math.abs(dir[1]) < 1e-9) return null;
  const t = -a[1] / dir[1];
  if (t < 0) return null;
  return { x: a[0] + dir[0] * t, z: a[2] + dir[2] * t };
}

function startPage() {
  const $ = (id) => document.getElementById(id);
  const canvas = $("view");
  const gl = canvas.getContext("webgl2", { antialias: true });
  if (!gl) { $("status").textContent = "WebGL2 is required"; return; }
  const renderer = createRenderer(gl);
  const cam = createCamera({ distance: 50, pitch: 1.0 });
  let model = newModel(32, 32);
  let meshes = new Map();
  let dirty = true;
  let regionStart = null;
  const markers = { doodad: cubeMesh([0.85, 0.7, 0.2]), region: cubeMesh([0.8, 0.2, 0.8]), spawn: [0, 1, 2, 3].map((p) => cubeMesh([[0.9, 0.2, 0.2], [0.2, 0.4, 0.9], [0.2, 0.8, 0.3], [0.9, 0.8, 0.2]][p])), neutral: cubeMesh([0.7, 0.7, 0.7]) };

  const fill = (select, items) => { select.replaceChildren(...items.map((v) => new Option(v, v))); };
  fill($("kind"), Object.keys(CELL_KINDS));
  fill($("doodad-kind"), DOODAD_KINDS);

  const load = (bytes) => {
    model = modelFromMap(bytes);
    cam.target = [model.width / 2, 0, model.height / 2];
    cam.distance = Math.max(model.width, model.height) * 1.1;
    cam.maxDistance = cam.distance * 4;
    dirty = true;
    $("status").textContent = `${model.width}x${model.height} cells, ${model.doodads.length} doodads`;
  };
  const rebuild = () => { meshes = kindMeshes(model); dirty = false; };
  const status = (text) => { $("status").textContent = text; };

  const apply = (x, z) => {
    const cx = Math.floor(x);
    const cy = Math.floor(z);
    const tool = $("tool").value;
    const radius = Number($("radius").value);
    const xm = x * model.cellMilli;
    const ym = z * model.cellMilli;
    if (tool === "paint" && inside(model, cx, cy)) { paintKind(model, cx, cy, $("kind").value, radius); dirty = true; }
    else if (tool === "height" && inside(model, cx, cy)) { paintLevel(model, cx, cy, Number($("level").value), radius); dirty = true; }
    else if (tool === "doodad") placeDoodad(model, $("doodad-kind").value, xm, ym);
    else if (tool === "spawn") placeSpawn(model, $("unit").value, Number($("player").value), xm, ym);
    else if (tool === "delete") { const gone = deleteNear(model, xm, ym, model.cellMilli); if (gone) status(`deleted ${gone.kind}`); }
    else if (tool === "region" && inside(model, cx, cy)) {
      if (!regionStart) { regionStart = [cx, cy]; status("region: click the opposite corner"); }
      else { const r = addRegion(model, $("region-name").value || "region", regionStart[0], regionStart[1], cx, cy); regionStart = null; status(`region ${r.name} ${r.x0},${r.y0}..${r.x1},${r.y1}`); }
    }
  };

  const aspect = () => canvas.width / Math.max(1, canvas.height);
  const at = (e) => { const r = canvas.getBoundingClientRect(); return pickGround(cam, aspect(), e.clientX - r.left, e.clientY - r.top, r.width, r.height); };
  let drag = null;
  canvas.addEventListener("contextmenu", (e) => e.preventDefault());
  canvas.addEventListener("pointerdown", (e) => {
    canvas.setPointerCapture(e.pointerId);
    drag = { button: e.button, x: e.clientX, y: e.clientY };
    if (e.button === 0) { const p = at(e); if (p) apply(p.x, p.z); }
  });
  canvas.addEventListener("pointermove", (e) => {
    if (!drag) return;
    const dx = e.clientX - drag.x;
    const dy = e.clientY - drag.y;
    drag.x = e.clientX;
    drag.y = e.clientY;
    if (drag.button === 2) orbitCamera(cam, dx, dy);
    else if (drag.button === 1 || e.shiftKey) panCamera(cam, dx, dy, canvas.height);
    else if (drag.button === 0 && ["paint", "height"].includes($("tool").value)) { const p = at(e); if (p) apply(p.x, p.z); }
  });
  canvas.addEventListener("pointerup", () => { drag = null; });
  canvas.addEventListener("wheel", (e) => { e.preventDefault(); zoomCamera(cam, e.deltaY); }, { passive: false });

  $("open").addEventListener("change", async (e) => {
    const f = e.target.files[0];
    if (f) load(new Uint8Array(await f.arrayBuffer()));
  });
  $("export").addEventListener("click", () => {
    const text = exportOsd(model);
    $("osd").value = text;
    const a = document.createElement("a");
    a.href = URL.createObjectURL(new Blob([text], { type: "text/plain" }));
    a.download = "map.osd";
    a.click();
    status(`exported ${text.split("\n").length - 1} lines`);
  });
  $("new").addEventListener("click", () => {
    model = newModel(Number($("new-w").value), Number($("new-h").value));
    cam.target = [model.width / 2, 0, model.height / 2];
    dirty = true;
  });

  const drawBox = (mesh, xCell, zCell, size, height) => {
    const gx = Math.floor(xCell);
    const gy = Math.floor(zCell);
    const base = inside(model, gx, gy) ? levelHeight(model, gy * model.width + gx) : 0;
    renderer.drawMesh(mesh, multiply(translation(xCell - size / 2, base, zCell - size / 2), scaling(size, height, size)));
  };

  const frame = () => {
    const w = Math.floor(canvas.clientWidth * devicePixelRatio);
    const h = Math.floor(canvas.clientHeight * devicePixelRatio);
    if (canvas.width !== w || canvas.height !== h) { canvas.width = w; canvas.height = h; }
    if (dirty) rebuild();
    renderer.setCamera({ ...cam, aspect: aspect() });
    renderer.beginFrame();
    for (const mesh of meshes.values()) renderer.drawMesh(mesh, null);
    for (const d of model.doodads) drawBox(markers.doodad, d.x / model.cellMilli, d.y / model.cellMilli, 0.6, 0.8);
    for (const s of model.spawns) drawBox(s.player >= 0 && s.player < 4 ? markers.spawn[s.player] : markers.neutral, s.x / model.cellMilli, s.y / model.cellMilli, 0.5, 1.4);
    for (const r of model.regions) {
      for (const [x, z] of [[r.x0, r.y0], [r.x1, r.y0], [r.x0, r.y1], [r.x1, r.y1]]) drawBox(markers.region, x, z, 0.3, 2);
    }
    requestAnimationFrame(frame);
  };

  fetch("map.bin").then((r) => (r.ok ? r.arrayBuffer() : Promise.reject(new Error(r.status)))).then((b) => load(new Uint8Array(b))).catch(() => status("map.bin not loaded: open a file or create a new map"));
  requestAnimationFrame(frame);
}

if (typeof document !== "undefined") startPage();

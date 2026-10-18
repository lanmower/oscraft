// Net client: WebGL2 map and unit view. Map and units come from the same assets as the browser
// page (map.bin and the wasm sim pkg). Joins a wireweave DataSession room named by the URL hash.
import { createDataSession, createFSM, NostrAuth, RelayPool } from "wireweave";
import * as NostrTools from "nostr-tools";
import * as xstate from "xstate";
import { encodeFrame, decodeFrame, FRAME } from "./wire/frame.js";
import { iceServersFrom } from "./ice.js";
import { applyFrame, createStateEncoder } from "./delta.js";
import { AnimationPlayer, jointMatrices } from "./anim.js";
import { parseGlb, parseGlbAnimations } from "./glb.js";
import { createHud } from "./hud.js";
import { createHud5 } from "./hud5.js";
import { createCamera, createRenderer as createRenderer3d, eyeOf, MAX_JOINTS, multiply, rotationY, translation, viewProjection } from "./render3d.js";
import { buildTerrain, readDoodads, readGrid, vertexNormals } from "./terrain.js";
import { bakeTiles, fitModel, planarUvs, TILE_MODELS, TILE_TINTS } from "./tiles.js";
import { loadBaseColorImage, uploadTexture } from "./texture.js";
import { createLobby } from "./lobby.js";
import { createFogView } from "./fogview.js";
import { createShop } from "./ui_shop.js";
import { createSkills } from "./ui_skills.js";
import { createInventory } from "./ui_inventory.js";
import { createFx } from "./fx.js";
import { createModUi } from "./modui.js";

// Map size comes from the loaded map image (loadSim); these are the fallbacks before a map is read.
let W = 32;
let H = 24;
const CAMERA_DISTANCE_PER_TILE = 40 / 32;
const CAMERA_MAX_PER_TILE = 120 / 32;
const BACKGROUND = [0.114, 0.145, 0.173];
const MAP_COLOR = [0.2, 0.28, 0.22];
const PLAYER_COLORS = [[0.227, 0.482, 0.835], [0.835, 0.329, 0.227]];
const SELECT_COLOR = [0.95, 0.85, 0.3];
const UNIT_HALF = 0.9;
const DOODAD_HALF = 0.5;
const PICK_RADIUS = 1.2;
const FLAT_PX_PER_UNIT = 20;
const DRAG_SLOP_PX = 4;
const PAN_PX_PER_S = 500;
let orbitOrigin = [W / 2, 0, H / 2];
let cameraDistance = CAMERA_DISTANCE_PER_TILE * Math.max(W, H);
let cameraMaxDistance = CAMERA_MAX_PER_TILE * Math.max(W, H);
const DOODAD_KINDS = ["rock", "tree", "crystal", "debris", "base", "shop", "fountain", "lane", "camp"];
const EDGE_PX = 16;
const RECONNECT_BASE_MS = 1000;
const RECONNECT_CAP_MS = 16000;
const RECONNECT_MAX_ATTEMPTS = 6;
const HOST_TICK_MS = 100;
const WIRE_SCALE = 1000;
const MAX_PLAYERS = 256;

const params = new URLSearchParams(location.search);
const relayUrls = params.get("relay") ? params.get("relay").split(",") : null;
const namespace = params.get("ns") || "oscraft";
const statusEl = document.getElementById("status");
const canvas = document.getElementById("view");
const room = location.hash.slice(1) || "lobby";
const pkgUrl = new URL(document.body.dataset.pkg, location.href).href;
// The #moba page loads the moba scenario image, the default page loads the duel map image.
const mapUrl = new URL(location.hash === "#moba" ? "moba.bin" : document.body.dataset.map, location.href).href;

// Flat fallback view (no WebGL2): a fixed top-down frame centred on the map.
const flatView = { cx: W / 2, cz: H / 2 };
let mapGrid = null;
// 3D view state: the renderer and its orbit camera exist only when WebGL2 is available.
let renderer3d = null;
let mapBytes = null;
let terrainMesh = null;
let tileMeshes = null;
let terrainTiles = null;
let unitMeshes = [];
let doodadMeshes = [];
const drawCounts = { terrain: 0, doodad: 0, unit: 0, quad: 0, animated: 0 };
const MOVE_EPS = 1e-4;
const MAX_FRAME_DT_S = 0.1;
// Per-unit clip players keyed by unit id and type, with the last seen position (walking vs standing still).
const animators = new Map();
let lastDrawMs = null;
const peers = new Set();
let netState = null;
let casts = 0;
let attacks = 0;
let localGame = null;
let hud = null;
let hud5 = null;
let fogview = null;
const heroPanels = [];
let fx = null;
let selectedId = -1;
let reconnectAttempt = 0;
let reconnectTimer = 0;
let drag = null;
let hosting = false;
let orders = 0;
let pointer = null;
let lastFrameMs = 0;
const keysDown = new Set();
const hostPlayers = new Map();

function setStatus(extra) {
  const mode = netState ? "net" : "local";
  statusEl.textContent = `room ${room}\npeers ${peers.size} (${mode})\n${extra || ""}`;
}

async function loadSim() {
  const mod = await import(/* @vite-ignore */ pkgUrl);
  await mod.default();
  const image = new Uint8Array(await (await fetch(mapUrl)).arrayBuffer());
  mapBytes = image;
  setMapSize(readGrid(image));
  const game = new mod.Game(image);
  // The moba preview runs without fog so every enemy is a valid target; other pages keep the sim's fog.
  game.set_fog(location.hash !== "#moba");
  return game;
}

// The map's grid sets the playfield: orbit origin, default and maximum camera distance scale with its larger side.
function setMapSize(grid) {
  mapGrid = grid;
  W = grid.width;
  H = grid.height;
  flatView.cx = W / 2;
  flatView.cz = H / 2;
  orbitOrigin = [W / 2, 0, H / 2];
  cameraDistance = CAMERA_DISTANCE_PER_TILE * Math.max(W, H);
  cameraMaxDistance = CAMERA_MAX_PER_TILE * Math.max(W, H);
}

// Skin, node tree and clips for a skinned GLB, with its rest-pose geometry (the joint matrices carry the fit).
// Null when the model has no clips or its skin has more joints than the shader palette.
function skinnedRig(bytes, glb, fit) {
  const { clips, skins, nodes } = parseGlbAnimations(bytes);
  const skin = skins[0];
  if (!skin || clips.length === 0 || skin.joints.length > MAX_JOINTS) return null;
  return { clips, skin, nodes, fit, positions: glb.positions, normals: glb.normals || vertexNormals(glb.positions, glb.indices), joints: glb.joints, weights: glb.weights };
}

// A flat ground square with its normal up: the fallback mesh for a unit with no model.
function quadMesh(half) {
  return {
    positions: new Float32Array([-half, 0, -half, half, 0, -half, half, 0, half, -half, 0, half]),
    normals: new Float32Array([0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0]),
    indices: new Uint16Array([0, 1, 2, 0, 2, 3]),
  };
}

// Loads one GLB by name and returns its fitted model, or null when the file is missing or unreadable.
async function fetchModel(name, size) {
  if (!name) return null;
  try {
    const url = new URL(`models/${name}.glb`, location.href);
    const bytes = await (await fetch(url)).arrayBuffer();
    const glb = parseGlb(bytes);
    const fitted = fitModel(glb, size);
    const animation = glb.joints && glb.weights ? skinnedRig(bytes, glb, fitted.fit) : null;
    return { ...fitted, glbUrl: url.href, animation };
  } catch {
    return null;
  }
}

// One fitted model per unit type, indexed like the wasm unit_types export. A type with no model, or a model
// that fails to load, is null and draws as a ground quad.
async function loadModels(game) {
  if (typeof game.unit_model_names !== "function") return [];
  return Promise.all(game.unit_model_names().map((name) => fetchModel(name, 2 * UNIT_HALF)));
}

// Doodads: one fitted model per kind code (DODA kind), read from doodads.json which maps kind name to GLB path.
// The shipped asset catalogue (assets.json) names each doodad's GLB; doodads.json is the fallback when it is missing.
async function loadDoodadModels() {
  const [map, catalogue] = await Promise.all([
    fetch(new URL("doodads.json", location.href)).then((r) => r.json()),
    fetch(new URL("assets.json", location.href)).then((r) => r.json()).catch(() => ({ items: [] })),
  ]);
  return Promise.all(DOODAD_KINDS.map((kind) => {
    const file = catalogue.items.find((item) => item.id === kind)?.path ?? map[kind];
    return fetchModel(file ? file.replace(/^(js\/client\/)?models\//, "").replace(/\.glb$/, "") : null, 2 * DOODAD_HALF);
  }));
}

// Uploads a fitted model as a drawable mesh. Its base colour texture is uploaded when the model has one.
// Units pass animated: their skinned rig replaces the fitted geometry, which the joint matrices carry instead.
async function toMesh(gl, fitted, fallbackColor, { animated = false } = {}) {
  if (!fitted) return null;
  const rig = animated ? fitted.animation : null;
  const mesh = rig
    ? { positions: rig.positions, normals: rig.normals, uvs: fitted.uvs, indices: fitted.indices, joints: rig.joints, weights: rig.weights, animation: rig, color: fallbackColor, height: maxY(fitted.positions) }
    : { positions: fitted.positions, normals: fitted.normals, uvs: fitted.uvs, indices: fitted.indices, color: fallbackColor, height: maxY(fitted.positions) };
  if (fitted.baseColorTexture) {
    try {
      const image = await loadBaseColorImage(fitted.baseColorTexture, fitted.glbUrl);
      if (image) mesh.texture = uploadTexture(gl, image);
    } catch {
      // A texture that fails to load leaves the model drawn in its fallback colour.
    }
  }
  return mesh;
}

function maxY(positions) {
  let top = 0;
  for (let v = 1; v < positions.length; v += 3) top = Math.max(top, positions[v]);
  return top;
}

// Ground height under a tile position; the terrain is flat at 0 until its mesh is built.
function groundAt(x, z) {
  return terrainTiles ? groundY(terrainTiles, x, z) : 0;
}

// Adds `top`, the world height of the unit's model top, so its health bar sits above the model.
function anchored(u) {
  const model = unitMeshes[u.type];
  return { ...u, top: groundAt(u.x, u.z) + (model ? model.height : 0) };
}

function currentUnits() {
  if (netState) return netState.units.map(anchored);
  if (!localGame) return [];
  const flat = localGame.units();
  const ids = localGame.unit_ids();
  // Older wasm builds have no unit_types or unit_hps export; their units draw as quads without health bars.
  const types = typeof localGame.unit_types === "function" ? localGame.unit_types() : null;
  const hps = typeof localGame.unit_hps === "function" ? localGame.unit_hps() : null;
  // Older wasm builds lack unit_attacking; their units never report an attack clip.
  const attacking = typeof localGame.unit_attacking === "function" ? localGame.unit_attacking() : null;
  const out = [];
  for (let i = 0; i < ids.length; i++) {
    out.push(anchored({ id: ids[i], x: flat[3 * i], z: flat[3 * i + 1], player: flat[3 * i + 2], type: types ? types[i] : -1, hp: hps ? hps[i] : -1, attacking: attacking ? attacking[i] === 1 : false }));
  }
  return fogview ? fogview.filter(out) : out;
}

// Draw counts per mesh kind for the current frame, published on the body for the browser probe.
function countDraw(kind, mesh, transform, joints = null) {
  drawCounts[kind] += 1;
  if (joints) renderer3d.drawSkinned(mesh, transform, joints);
  else renderer3d.drawMesh(mesh, transform);
}

// Unit meshes are cached per (type, colour key): a textured model keeps its texture whatever the colour.
const unitMeshCache = new Map();
function unitMeshFor(type, color) {
  const key = `${type}:${color.join(",")}`;
  let mesh = unitMeshCache.get(key);
  if (!mesh) {
    const base = type >= 0 && unitMeshes[type] ? unitMeshes[type] : null;
    mesh = base ? { ...base, color } : { ...quadMesh(UNIT_HALF), color };
    unitMeshCache.set(key, mesh);
  }
  return mesh;
}

// Joint palette for a unit this frame: its clip (walk when it moved, attack when it reports attacking, else idle)
// advanced by dt seconds. A unit with no matching clip holds the rest pose.
function jointsFor(u, mesh, dt) {
  const key = `${u.id}:${u.type}`;
  let entry = animators.get(key);
  if (!entry) {
    entry = { player: new AnimationPlayer(mesh.animation.clips), x: u.x, z: u.z, seen: false };
    animators.set(key, entry);
  }
  const moving = Math.hypot(u.x - entry.x, u.z - entry.z) > MOVE_EPS;
  entry.x = u.x;
  entry.z = u.z;
  entry.seen = true;
  entry.player.playState({ attacking: Boolean(u.attacking), moving });
  entry.player.update(dt);
  const { nodes, skin, fit } = mesh.animation;
  return jointMatrices(nodes, skin, entry.player.pose() ?? new Map(), fit);
}

function drawUnits3d(units, dt) {
  for (const entry of animators.values()) entry.seen = false;
  for (const u of units) {
    const color = u.id === selectedId ? SELECT_COLOR : PLAYER_COLORS[u.player] || PLAYER_COLORS[0];
    const model = unitMeshes[u.type];
    const mesh = unitMeshFor(u.type, color);
    const transform = translation(u.x, groundAt(u.x, u.z), u.z);
    if (mesh.animation) {
      countDraw("unit", mesh, transform, jointsFor(u, mesh, dt));
      drawCounts.animated += 1;
    } else {
      countDraw(model ? "unit" : "quad", mesh, transform);
    }
  }
  for (const [key, entry] of animators) if (!entry.seen) animators.delete(key);
}

function drawWorld3d() {
  drawCounts.terrain = 0;
  drawCounts.doodad = 0;
  drawCounts.unit = 0;
  drawCounts.quad = 0;
  drawCounts.animated = 0;
  const now = performance.now();
  const dt = lastDrawMs === null ? 0 : Math.min((now - lastDrawMs) / 1000, MAX_FRAME_DT_S);
  lastDrawMs = now;
  renderer3d.beginFrame(BACKGROUND);
  for (const mesh of tileMeshes ?? (terrainMesh ? [terrainMesh] : [])) countDraw("terrain", mesh, null);
  for (const d of doodadMeshes) countDraw("doodad", d.mesh, d.transform);
  const units = currentUnits();
  drawUnits3d(units, dt);
  if (fx) fx.frame(units, dt);
}

// Flat fallback without WebGL2: each unit is a 2D square at its screen position on a 2D canvas.
function drawFlat(ctx, units) {
  ctx.fillStyle = `rgb(${MAP_COLOR.map((c) => Math.round(c * 255)).join(",")})`;
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  for (const u of units) {
    const color = u.id === selectedId ? SELECT_COLOR : PLAYER_COLORS[u.player] || PLAYER_COLORS[0];
    const p = worldToScreen(u.x, u.z);
    const half = UNIT_HALF * FLAT_PX_PER_UNIT;
    ctx.fillStyle = `rgb(${color.map((c) => Math.round(c * 255)).join(",")})`;
    ctx.fillRect(p.x * window.devicePixelRatio - half, p.y * window.devicePixelRatio - half, 2 * half, 2 * half);
    drawCounts.quad += 1;
  }
}

// Arrow keys and the pointer at the window edge pan the orbit camera; the speed is in pixels per second, so
// the world distance per second scales with the camera distance. A drag pans by itself, so edge scroll waits.
function panByKeys(dtS) {
  if (!renderer3d) return;
  let dx = 0;
  let dz = 0;
  if (keysDown.has("ArrowLeft")) dx -= 1;
  if (keysDown.has("ArrowRight")) dx += 1;
  if (keysDown.has("ArrowUp")) dz -= 1;
  if (keysDown.has("ArrowDown")) dz += 1;
  if (pointer && !drag) {
    if (pointer.x < EDGE_PX) dx -= 1;
    else if (pointer.x > window.innerWidth - EDGE_PX) dx += 1;
    if (pointer.y < EDGE_PX) dz -= 1;
    else if (pointer.y > window.innerHeight - EDGE_PX) dz += 1;
  }
  if (dx === 0 && dz === 0) return;
  // pan() moves the world with the pixel delta (a drag), so a key that moves the view right passes a negative delta.
  renderer3d.pan(-dx * PAN_PX_PER_S * dtS, -dz * PAN_PX_PER_S * dtS);
}

function resize() {
  const dpr = window.devicePixelRatio || 1;
  canvas.width = Math.floor(window.innerWidth * dpr);
  canvas.height = Math.floor(window.innerHeight * dpr);
}

// Ground point (x, z) under a client pixel. In 3D this intersects the view ray with the ground plane y = 0;
// the flat fallback maps the pixel through its fixed top-down frame.
function screenToWorld(clientX, clientY) {
  if (!renderer3d) {
    const ppu = FLAT_PX_PER_UNIT;
    return { x: flatView.cx + (clientX - window.innerWidth / 2) / ppu, z: flatView.cz + (clientY - window.innerHeight / 2) / ppu };
  }
  const cam = renderer3d.camera;
  const eye = eyeOf(cam);
  const fwd = normalize(sub(cam.target, eye));
  const right = normalize(cross(fwd, [0, 1, 0]));
  const up = cross(right, fwd);
  const t = Math.tan(cam.fovY / 2);
  const aspect = window.innerWidth / Math.max(1, window.innerHeight);
  const ndcX = (2 * clientX) / window.innerWidth - 1;
  const ndcY = 1 - (2 * clientY) / window.innerHeight;
  const dir = [0, 1, 2].map((i) => fwd[i] + right[i] * ndcX * t * aspect + up[i] * ndcY * t);
  // Intersect the view ray with the terrain surface: start at y = 0 and re-intersect at the ground height found.
  let y = 0;
  let point = null;
  for (let i = 0; i < 4; i++) {
    const s = (y - eye[1]) / dir[1];
    point = { x: eye[0] + dir[0] * s, z: eye[2] + dir[2] * s };
    y = groundAt(point.x, point.z);
  }
  return point;
}

function sub(a, b) { return [a[0] - b[0], a[1] - b[1], a[2] - b[2]]; }
function cross(a, b) { return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]; }
function normalize(v) { const n = Math.hypot(v[0], v[1], v[2]) || 1; return [v[0] / n, v[1] / n, v[2] / n]; }

function pick(world) {
  let best = null;
  let bestD = PICK_RADIUS * PICK_RADIUS;
  for (const u of currentUnits()) {
    const d = (u.x - world.x) ** 2 + (u.z - world.z) ** 2;
    if (d <= bestD) { bestD = d; best = u; }
  }
  return best;
}

// Client pixel of the world point (x, y, z), y defaulting to the ground: the inverse of screenToWorld. Points behind the camera map far off screen.
function worldToScreen(x, z, y = 0) {
  if (!renderer3d) {
    return { x: window.innerWidth / 2 + (x - flatView.cx) * FLAT_PX_PER_UNIT, y: window.innerHeight / 2 + (z - flatView.cz) * FLAT_PX_PER_UNIT };
  }
  const vp = viewProjection(renderer3d.camera, window.innerWidth / Math.max(1, window.innerHeight));
  const clip = [0, 1, 2, 3].map((r) => vp[r] * x + vp[4 + r] * y + vp[8 + r] * z + vp[12 + r]);
  if (clip[3] <= 0) return { x: -1e6, y: -1e6 };
  return {
    x: ((clip[0] / clip[3] + 1) / 2) * window.innerWidth,
    y: ((1 - clip[1] / clip[3]) / 2) * window.innerHeight,
  };
}

// The commands createHud routes input through. Each runs on the host sim, or sends the frame to the host
// when this page is a guest; each returns whether the sim (or the host's send) accepted it.
const commands = {
  order(unitId, x, z) {
    const ok = netState ? (session.broadcast(sendOrder(unitId, x, z)), true) : localGame.order_move(unitId, x, z);
    if (ok) orders += 1;
    setStatus(`order move unit ${unitId} to ${x.toFixed(2)},${z.toFixed(2)}${netState ? "" : " (local)"}`);
    return ok;
  },
  attack(unitId, targetId) {
    const ok = netState ? (session.broadcast(sendAttack(unitId, targetId)), true) : localGame.attack(unitId, targetId);
    if (ok) attacks += 1;
    return ok;
  },
  cast(unitId, slot, target, x, z) {
    const ok = netState ? (session.broadcast(sendCast(unitId, slot, target, x, z)), true) : localGame.cast(unitId, slot, target, x, z);
    if (ok) casts += 1;
    if (ok && fx) fx.cast(unitId, slot, target, x, z);
    return ok;
  },
};

function bindInput() {
  canvas.addEventListener("pointerdown", (e) => {
    drag = { x: e.clientX, y: e.clientY, moved: false };
    canvas.setPointerCapture(e.pointerId);
  });
  canvas.addEventListener("pointermove", (e) => {
    if (!drag) return;
    const dx = e.clientX - drag.x;
    const dy = e.clientY - drag.y;
    if (Math.hypot(dx, dy) > DRAG_SLOP_PX) drag.moved = true;
    if (!drag.moved) return;
    if (renderer3d) {
      // A drag pans the ground under the pointer; shift or the right button orbits instead.
      if (e.shiftKey || e.buttons === 2) renderer3d.orbit(dx, dy);
      else renderer3d.pan(dx, dy);
    }
    drag.x = e.clientX;
    drag.y = e.clientY;
  });
  canvas.addEventListener("pointerup", (e) => {
    const wasDrag = drag && drag.moved;
    drag = null;
    if (!wasDrag && hud) hud.onPointerUp(e);
  });
  canvas.addEventListener("contextmenu", (e) => e.preventDefault());
  window.addEventListener("keydown", (e) => {
    if (e.target instanceof HTMLInputElement) return;
    if (e.key.startsWith("Arrow")) {
      keysDown.add(e.key);
      e.preventDefault();
      return;
    }
    if (hud) hud.onKey(e.key, pointer);
  });
  window.addEventListener("keyup", (e) => keysDown.delete(e.key));
  window.addEventListener("blur", () => keysDown.clear());
  window.addEventListener("pointermove", (e) => { pointer = { x: e.clientX, y: e.clientY }; });
  document.documentElement.addEventListener("pointerleave", () => { pointer = null; });
  canvas.addEventListener("wheel", (e) => {
    e.preventDefault();
    if (renderer3d) renderer3d.zoom(e.deltaY);
  }, { passive: false });
  window.addEventListener("resize", resize);
}

// Browser host: this page owns the sim (localGame). Each peer is assigned a player id when its channel opens
// (the lowest id no other peer holds); an order is applied only when the unit's player is the sender's.
// Every HOST_TICK_MS the host ticks the sim and broadcasts that tick's state through the delta encoder (delta.js).
function assignPlayer(players, peerPubkey) {
  if (players.has(peerPubkey)) return players.get(peerPubkey);
  const taken = new Set(players.values());
  for (let p = 0; p < MAX_PLAYERS; p++) {
    if (!taken.has(p)) {
      players.set(peerPubkey, p);
      return p;
    }
  }
  return null;
}

// A peer's move, cast or attack runs on the host only for a unit the peer's player owns. Accepted casts and
// attacks are counted on the host page (data-remote-casts, data-remote-attacks) for the CI witness.
function applyPeerCommand(peerPubkey, command) {
  const player = hostPlayers.get(peerPubkey);
  const unit = currentUnits().find((u) => u.id === command.unitId);
  if (player === undefined || !unit || unit.player !== player) {
    setStatus(`order refused not_owner peer=${String(peerPubkey).slice(0, 8)} unit=${command.unitId}`);
    document.body.dataset.lastRefusal = `not_owner unit=${command.unitId}`;
    return false;
  }
  if (command.type === FRAME.ORDER) {
    localGame.order_move(command.unitId, command.x, command.y);
    return true;
  }
  if (command.type === FRAME.CAST) {
    const ok = localGame.cast(command.unitId, command.slot, command.target, command.x, command.y);
    if (!ok) document.body.dataset.lastRefusal = `cast_refused unit=${command.unitId}`;
    if (ok) countRemote("remoteCasts");
    return ok;
  }
  const ok = localGame.attack(command.unitId, command.targetId);
  if (!ok) document.body.dataset.lastRefusal = `attack_refused unit=${command.unitId}`;
  if (ok) countRemote("remoteAttacks");
  return ok;
}

function countRemote(name) {
  document.body.dataset[name] = String(Number(document.body.dataset[name] || 0) + 1);
}

const SNAPSHOT_EVERY_TICKS = 50;

function snapshotKey() {
  return `oscraft:snapshot:${namespace}:${room}`;
}

function saveSnapshot(tick) {
  if (typeof localGame.save_state !== "function") return;
  const bytes = localGame.save_state();
  let binary = "";
  for (let i = 0; i < bytes.length; i += 0x8000) binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  const record = { tick, hash: localGame.state_hash().toString(), bytes: btoa(binary) };
  localStorage.setItem(snapshotKey(), JSON.stringify(record));
}

// Restores the room from its last snapshot. Returns the restored tick, or -1 after an explicit refusal line.
function restoreSnapshot() {
  const raw = localStorage.getItem(snapshotKey());
  if (raw === null) {
    setStatus("snapshot: none stored, starting fresh");
    return 0;
  }
  try {
    const record = JSON.parse(raw);
    const bytes = Uint8Array.from(atob(record.bytes), (c) => c.charCodeAt(0));
    localGame.load_state(bytes);
    const hash = localGame.state_hash().toString();
    if (hash !== record.hash) throw new Error(`hash ${hash} does not match stored ${record.hash}`);
    document.body.dataset.restoredTick = String(record.tick);
    document.body.dataset.restoredHash = hash;
    setStatus(`snapshot: restored tick=${record.tick} state_hash=${hash}`);
    return record.tick;
  } catch (err) {
    setStatus(`snapshot refused: ${err.message || err}`);
    document.body.dataset.snapshotRefused = String(err.message || err);
    return -1;
  }
}

function startHost() {
  hosting = true;
  const encoder = createStateEncoder();
  const restoredAt = restoreSnapshot();
  let hostTick = restoredAt > 0 ? restoredAt : 0;
  setInterval(() => {
    localGame.tick();
    hostTick += 1;
    if (hostTick % SNAPSHOT_EVERY_TICKS === 0) saveSnapshot(hostTick);
    const hash = Number(localGame.state_hash() & 0xffffffffn) >>> 0;
    const units = currentUnits().map((u) => ({ id: u.id, kind: Math.max(0, u.type), player: u.player, x: Math.round(u.x * WIRE_SCALE), y: Math.round(u.z * WIRE_SCALE), hp: Math.max(0, u.hp) }));
    if (window.__ciSent) recordTick(window.__ciSent, hostTick, units);
    const frame = encoder.encode(hostTick, hash, units);
    const bytes = encodeFrame(frame);
    if (window.__ciFrames && window.__ciFrames.length < 200) window.__ciFrames.push({ type: frame.type, bytes: bytes.byteLength });
    session.broadcast(bytes);
  }, HOST_TICK_MS);
  setStatus("hosting");
}

function scheduleReconnect() {
  if (reconnectAttempt >= RECONNECT_MAX_ATTEMPTS) {
    setStatus("reconnect gave up; reload to retry");
    return;
  }
  const delay = Math.min(RECONNECT_CAP_MS, RECONNECT_BASE_MS * 2 ** reconnectAttempt);
  reconnectAttempt += 1;
  setStatus(`reconnecting in ${delay} ms (attempt ${reconnectAttempt})`);
  clearTimeout(reconnectTimer);
  reconnectTimer = setTimeout(() => joinRoom(), delay);
}

async function joinRoom() {
  try {
    await session.connect(room);
    setStatus("joined");
  } catch (err) {
    setStatus(`join failed: ${err && err.message ? err.message : err}`);
    scheduleReconnect();
  }
}

function sendOrder(unitId, x, z) {
  return encodeFrame({ type: FRAME.ORDER, unitId, x: Math.round(x), y: Math.round(z) });
}

function sendCast(unitId, slot, target, x, z) {
  return encodeFrame({ type: FRAME.CAST, unitId, slot, target, x: Math.round(x), y: Math.round(z) });
}

function sendAttack(unitId, targetId) {
  return encodeFrame({ type: FRAME.ATTACK, unitId, targetId });
}

const fsm = createFSM(xstate);
const auth = new NostrAuth({ nostrTools: NostrTools });
auth.loadFromStorage() || auth.generateKey();
createLobby({ playerId: auth.pubkey || "local", onStart: () => setStatus("lobby: match starting") });
const relayPool = new RelayPool({
  verifyEvent: NostrTools.verifyEvent,
  ...(relayUrls ? { relays: relayUrls, fallbackRelays: [], autoRotate: false } : {}),
});
relayPool.connect();

// Room announcements over the relay. Each page publishes its room as a parameterized replaceable
// event (kind 30078, tag t=oscraft-room, ns and room tags) every 15 s and lists the rooms other
// pages announce. An announcement expires after three missed announcements (45 s), so a closed page
// drops off the list. Lists feed the "oscraft:rooms" window event that rooms.js renders as a snapshot.
const ROOM_KIND = 30078;
const ROOM_TAG = "oscraft-room";
const ROOM_ANNOUNCE_S = 15;
const ROOM_TTL_S = 3 * ROOM_ANNOUNCE_S;
const ROOM_PRUNE_MS = 5000;
const announcedRooms = new Map();
let shownRooms = "";

function announceRoom() {
  if (!auth.pubkey) return;
  auth.sign({
    kind: ROOM_KIND,
    created_at: Math.floor(Date.now() / 1000),
    tags: [["d", `oscraft-room:${namespace}:${room}`], ["t", ROOM_TAG], ["ns", namespace], ["room", room]],
    content: "",
  }).then((signed) => relayPool.publish(signed)).catch(() => {});
}

function showRooms() {
  const names = [...announcedRooms.keys()].sort();
  const key = names.join(",");
  if (key === shownRooms) return;
  shownRooms = key;
  window.dispatchEvent(new CustomEvent("oscraft:rooms", { detail: { rooms: names } }));
}

function pruneRooms() {
  const nowS = Math.floor(Date.now() / 1000);
  for (const [name, createdAt] of announcedRooms) {
    if (nowS - createdAt > ROOM_TTL_S) announcedRooms.delete(name);
  }
  showRooms();
}

relayPool.subscribe("oscraft-rooms", [{ kinds: [ROOM_KIND], "#t": [ROOM_TAG] }], (event) => {
  const tagValue = (name) => (event.tags.find((t) => t[0] === name) || [])[1];
  if (tagValue("ns") !== namespace || typeof tagValue("room") !== "string") return;
  if (Math.floor(Date.now() / 1000) - event.created_at > ROOM_TTL_S) return;
  announcedRooms.set(tagValue("room"), event.created_at);
  pruneRooms();
});
announceRoom();
setInterval(announceRoom, ROOM_ANNOUNCE_S * 1000);
setInterval(pruneRooms, ROOM_PRUNE_MS);
// ?ice=<JSON list of stun entries> replaces the default STUN list. Relay test mode (?relay=) keeps no ICE servers unless ?ice is given.
const iceConfig = (() => {
  try {
    return iceServersFrom(params.get("ice"));
  } catch (err) {
    setStatus(`ice config refused: ${err.message}`);
    return iceServersFrom(null);
  }
})();
const ciPeerConnections = [];
const createPeerConnection = (config) => {
  const pc = new RTCPeerConnection(config);
  if (params.get("ci") === "1") ciPeerConnections.push(pc);
  return pc;
};
const session = createDataSession({ fsm, xstate, relayPool, auth, namespace, createPeerConnection, iceServers: relayUrls && !params.has("ice") ? [] : iceConfig.servers });

session.addEventListener("peer-open", (e) => {
  peers.add(e.detail.peerPubkey);
  if (hosting) assignPlayer(hostPlayers, e.detail.peerPubkey);
  reconnectAttempt = 0;
  setStatus(`peer-open ${String(e.detail.peerPubkey).slice(0, 8)}`);
});
session.addEventListener("peer-close", (e) => {
  peers.delete(e.detail.peerPubkey);
  setStatus(`peer-close ${String(e.detail.peerPubkey).slice(0, 8)}`);
  if (peers.size === 0) {
    netTick = -1;
    scheduleReconnect();
  }
});
const netUnits = new Map();
let netTick = -1;
session.addEventListener("data", (e) => {
  let frame = null;
  try { frame = decodeFrame(new Uint8Array(e.detail.data)); } catch { return; }
  if (frame.type === FRAME.ORDER || frame.type === FRAME.CAST || frame.type === FRAME.ATTACK) {
    if (hosting) applyPeerCommand(e.detail.peerPubkey, frame);
    return;
  }
  if (hosting) return;
  if (frame.tick <= netTick) return;
  if (!applyFrame(netUnits, frame)) return;
  netTick = frame.tick;
  netState = { tick: frame.tick, units: [...netUnits.values()].map((u) => ({ id: u.id, x: u.x / WIRE_SCALE, z: u.y / WIRE_SCALE, player: u.player, hp: u.hp, type: u.kind })) };
  if (window.__ciRecv) recordTick(window.__ciRecv, frame.tick, [...netUnits.values()]);
});

// CI witness of wireweave agreement: each side keeps the unit positions it sent or applied for the last host ticks.
const CI_TICK_LOG = 300;
function recordTick(log, tick, units) {
  log.set(tick, units.map((u) => `${u.id}:${u.x}:${u.y}`).sort().join(","));
  if (log.size > CI_TICK_LOG) log.delete(log.keys().next().value);
}

// Page state for headless checks. The HUD (gold, winner, health bars, minimap) refreshes from the same frame.
function publishWitness() {
  document.body.dataset.units = String(currentUnits().length);
  document.body.dataset.ticks = String(localGame ? localGame.ticks() : netState ? netState.tick : 0);
  document.body.dataset.mode = renderer3d ? "3d" : "flat";
  document.body.dataset.center = renderer3d ? renderer3d.camera.target.map((v) => v.toFixed(2)).join(",") : `${flatView.cx},${flatView.cz}`;
  document.body.dataset.distance = renderer3d ? renderer3d.camera.distance.toFixed(3) : "";
  document.body.dataset.drawTerrain = String(drawCounts.terrain);
  document.body.dataset.terrainMode = tileMeshes ? "tiles" : "heightfield";
  document.body.dataset.drawDoodad = String(drawCounts.doodad);
  document.body.dataset.drawUnit = String(drawCounts.unit);
  document.body.dataset.drawQuad = String(drawCounts.quad);
  document.body.dataset.drawAnimated = String(drawCounts.animated);
  document.body.dataset.orders = String(orders);
  document.body.dataset.casts = String(casts);
  document.body.dataset.attacks = String(attacks);
  if (hud) hud.update();
  if (hud5) hud5.update();
  for (const panel of heroPanels) panel.update();
}

function createBarCanvas() {
  const bars = document.createElement("canvas");
  bars.style.cssText = "position:fixed;left:0;top:0;pointer-events:none";
  document.body.appendChild(bars);
  return bars;
}

// Terrain positions in tile units: the map's cell size is the unit of the sim's coordinates.
function terrainInTiles(terrain) {
  const s = 1 / terrain.cellMilli;
  return { ...terrain, positions: terrain.positions.map((v) => v * s) };
}

// Height of the terrain at a tile position: the nearest grid corner's height.
function groundY(terrain, x, z) {
  const cols = terrain.width + 1;
  const cx = Math.min(terrain.width, Math.max(0, Math.round(x)));
  const cy = Math.min(terrain.height, Math.max(0, Math.round(z)));
  return terrain.positions[(cy * cols + cx) * 3 + 1];
}

const DOODAD_TINT = [0.55, 0.5, 0.42];

// Builds the 3D world from the staged assets: the terrain mesh from map.bin, doodads at their placements with
// their GLB models, and unit models from the sim's model names. Each unit type keeps a model or null (quad).
async function build3d(gl, game) {
  const terrain = terrainInTiles(buildTerrain(mapBytes));
  terrainTiles = terrain;
  terrainMesh = { positions: terrain.positions, normals: terrain.normals, uvs: terrain.uvs, indices: terrain.indices, color: MAP_COLOR };
  const groundImage = await loadBaseColorImage({ uri: "textures/ground.png" }, location.href).catch(() => null);
  const groundTexture = groundImage ? uploadTexture(gl, groundImage) : null;
  terrainMesh.texture = groundTexture;
  tileMeshes = await loadTileMeshes(gl, groundTexture).catch(() => null);
  const doodadByKind = await Promise.all((await loadDoodadModels().catch(() => [])).map((f) => toMesh(gl, f, DOODAD_TINT)));
  const s = 1 / terrain.cellMilli;
  doodadMeshes = readDoodads(mapBytes).flatMap((p) => {
    const mesh = doodadByKind[p.kind];
    if (!mesh) return [];
    const x = p.x * s;
    const z = p.y * s;
    const angle = (p.rotation / 1000) * 2 * Math.PI;
    return [{ mesh, transform: multiply(translation(x, groundY(terrain, x, z), z), rotationY(angle)) }];
  });
  unitMeshes = await Promise.all((await loadModels(game)).map((f) => toMesh(gl, f, [1, 1, 1], { animated: true })));
}

// Terrain drawn from the CC0 tile models: one fitted model per kind, cells baked per kind. Any tile that fails to load
// returns null, so the heightfield mesh stays the drawn terrain.
async function loadTileMeshes(gl, groundTexture) {
  const names = Object.entries(TILE_MODELS);
  const fitted = await Promise.all(names.map(([, name]) => fetchModel(name, 1)));
  if (fitted.some((model) => !model)) return null;
  const models = Object.fromEntries(names.map(([kind], i) => [kind, fitted[i]]));
  const baked = bakeTiles(mapBytes, models);
  if (!baked) return null;
  return Promise.all([...baked.kinds].map(async ([kind, mesh]) => {
    const model = models[kind];
    const textured = groundTexture && (kind === "grass" || kind === "dirt");
    const drawn = await toMesh(gl, { ...mesh, uvs: textured ? planarUvs(mesh.positions) : mesh.uvs, baseColorTexture: textured ? null : model.baseColorTexture, glbUrl: model.glbUrl }, TILE_TINTS[kind]);
    if (drawn && textured) drawn.texture = groundTexture;
    return drawn;
  }));
}

async function main() {
  resize();
  bindInput();
  const gl = canvas.getContext("webgl2", { antialias: true });
  const flatCtx = gl ? null : canvas.getContext("2d");
  localGame = await loadSim();
  if (gl) {
    renderer3d = createRenderer3d(gl);
    renderer3d.setCamera({ target: orbitOrigin, distance: cameraDistance, maxDistance: cameraMaxDistance, pitch: 0.9 });
    try {
      await build3d(gl, localGame);
    } catch (err) {
      // Missing or broken assets leave the scene with what loaded; units without a model draw as quads.
      document.body.dataset.assetError = err && err.message ? err.message : String(err);
    }
  }
  hud = createHud({
    game: localGame,
    hudEl: document.getElementById("hud"),
    winEl: document.getElementById("win"),
    minimapEl: document.getElementById("minimap"),
    barCanvas: createBarCanvas(),
    mapSize: { w: W, h: H },
    grid: mapGrid,
    units: currentUnits,
    selected: () => selectedId,
    select: (unitId) => { selectedId = unitId; },
    localPlayer: () => 0,
    commands,
    pick,
    toWorld: screenToWorld,
    toScreen: worldToScreen,
    playerColor: (player) => PLAYER_COLORS[player] || PLAYER_COLORS[0],
  });
  if (renderer3d) fx = createFx({ renderer: renderer3d, game: () => localGame, toScreen: worldToScreen, groundAt, playerColor: (player) => PLAYER_COLORS[player] || PLAYER_COLORS[0], selected: () => selectedId });
  hud5 = createHud5({ game: localGame, units: currentUnits, selected: () => selectedId, localPlayer: () => 0 });
  const eventsBase = params.get("events");
  const modUi = createModUi({
    root: document.body,
    renderer: () => renderer3d,
    localPlayer: () => 0,
    defaultDistance: () => cameraDistance,
    baseUrl: eventsBase === null ? undefined : eventsBase.replace(/\/$/, ""),
  });
  window.__modUi = modUi;
  const heroUi = { game: localGame, selected: () => selectedId, localPlayer: () => 0 };
  if (typeof localGame.buy === "function") heroPanels.push(createShop(heroUi), createSkills(heroUi), createInventory(heroUi));
  fogview = createFogView({ game: localGame, localPlayer: () => 0, toScreen: worldToScreen, groundAt });
  if (params.get("host") === "1") startHost();
  joinRoom();

  function frame(now) {
    const dtS = lastFrameMs ? Math.min(0.1, (now - lastFrameMs) / 1000) : 0;
    lastFrameMs = now;
    panByKeys(dtS);
    modUi.step(dtS);
    if (!netState && localGame && !hosting) localGame.tick();
    if (renderer3d) {
      drawWorld3d();
      fogview.draw();
    } else if (flatCtx) {
      drawCounts.quad = 0;
      drawFlat(flatCtx, currentUnits());
    }
    publishWitness();
    requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);
  setStatus("local preview");
  document.body.dataset.ready = "1";
}

if (params.get("ci") === "1") {
  window.__ciPeers = () => peers.size;
  window.__ciFrames = [];
  window.__ciUnits = () => currentUnits().map((u) => ({ id: u.id, x: u.x, z: u.z, player: u.player, type: u.type, modeled: !!unitMeshes[u.type] }));
  window.__ciOrder = (unitId, x, z) => session.broadcast(sendOrder(unitId, x, z));
  window.__ciCast = (unitId, slot, target, x, z) => session.broadcast(sendCast(unitId, slot, target, x, z));
  window.__ciLearn = (unitId, slot) => (localGame ? localGame.skill_up(unitId, slot) : false);
  window.__ciHostile = (a, b) => localGame.hostile(a, b);
  window.__ciAttack = (unitId, targetId) => session.broadcast(sendAttack(unitId, targetId));
  window.__ciSent = new Map();
  window.__ciRecv = new Map();
  window.__ciSentTicks = () => [...window.__ciSent];
  window.__ciRecvTicks = () => [...window.__ciRecv];
  window.__ciSelected = () => selectedId;
  // Candidate types of every local candidate and of each succeeded, selected pair, one entry per peer connection.
  window.__ciCandidates = async () => {
    const localTypes = new Set();
    const pairs = [];
    for (const pc of ciPeerConnections) {
      const stats = await pc.getStats();
      stats.forEach((s) => { if (s.type === "local-candidate") localTypes.add(s.candidateType); });
      stats.forEach((s) => {
        if (s.type !== "candidate-pair" || s.state !== "succeeded" || !(s.nominated || s.selected)) return;
        pairs.push({
          local: stats.get(s.localCandidateId)?.candidateType || "?",
          remote: stats.get(s.remoteCandidateId)?.candidateType || "?",
        });
      });
    }
    return { localTypes: [...localTypes].sort(), pairs };
  };
  window.__ciSelect = (unitId) => { selectedId = unitId; };
  window.__ciToScreen = (x, z) => worldToScreen(x, z, groundAt(x, z));
  window.__ciKillSide = (player) => {
    for (const u of currentUnits()) if (u.player === player) localGame.hurt(u.id, 10000);
  };
  window.__ciDebug = () => [...session.peers.values()].map((p) => ({ ice: p.pc?.iceConnectionState, conn: p.pc?.connectionState, remoteMdns: (p.pc?.remoteDescription?.sdp || '').split('.local').length - 1 }));
}

main().catch((err) => {
  const message = err && err.message ? err.message : String(err);
  document.body.dataset.error = message;
  setStatus(`failed: ${message}`);
});
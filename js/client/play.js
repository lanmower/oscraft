// Net client: WebGL2 map and unit view. Map and units come from the same assets as the browser
// page (map.bin and the wasm sim pkg). Joins a wireweave DataSession room named by the URL hash.
// There is no fallback path: a missing or failed asset is shown on the page (see recordAssetError), never drawn as something else.
import { createDataSession, createFSM, NostrAuth, RelayPool } from "wireweave";
import * as NostrTools from "nostr-tools";
import * as xstate from "xstate";
import { encodeFrame, decodeFrame, frameTypeName, FRAME, MAX_PLAYERS, SPECTATOR } from "./wire/frame.js";
import { iceServersFrom } from "./ice.js";
import { assetJson, assetRel, assetUrl, manifest, manifestOf, packBytes, packJson, packText, packUrl, readPackFile, stashPack, UI } from "./pack_load.js";
import { applyFrame, createSeating, createStateEncoder, startFrame } from "./delta.js";
import { AnimationPlayer, cycleSpeed, jointMatrices, retargetLibrary, skinFloor } from "./anim.js";
import { parseGlb, parseGlbAnimations } from "./glb.js";
import { resolveGlbExtensions } from "./glb_resolve.js";
import { createHud } from "./hud.js";
import { createWidgets } from "./widgets.js";
import { createCamera, createRenderer as createRenderer3d, eyeOf, MAX_JOINTS, meshReach, multiply, rotationY, scaling, translation, viewProjection } from "./render3d.js";
import { buildTerrain, GROUND_UV_PER_CELL, readDoodads, readGrid, vertexNormals } from "./terrain.js";
import { controlMap, fitModel, planarGroundUvs, rampLowCells, SPLAT_LAYERS, SPLAT_TEXTURES, setSplatTexel, walkableBounds, waterMap, waterSurface } from "./tiles.js";
import { loadBaseColorImage, uploadTexture } from "./texture.js";
import { SWAP_DEFER_TICKS, SWAP_GATE_TICKS, SWAP_LEAD_TICKS, packSwapKind, swapGate, swapIsNew } from "./swap_protocol.js";
import { createFogView } from "./fogview.js";
import { installDebugApi } from "./debug_api.js";
import { createFx } from "./fx.js";
import { createModUi } from "./modui.js";
import { Pack } from "./pack.js";
import { createPackEditor } from "./pack_editor.js";
import { changeFrames, chunkFrames, createPackReceiver, packStateFrames } from "./pack_sync.js";

// Map size comes from the loaded map image (loadSim); these are the fallbacks before a map is read.
let W = 32;
let H = 24;
const CAMERA_DISTANCE_PER_TILE = 40 / 32;
const CAMERA_MAX_PER_TILE = 1.6;
const CAMERA_MAX_FLOOR = 120;
const CAMERA_PLAY_DISTANCE = 70;
// The start view: the pack's view event names the seat's unit, and the page centres the camera on it once it is in the sim.
let pendingStartCentre = null;
const CAMERA_FAR_PER_MAX = 2;
const BACKGROUND = [0.114, 0.145, 0.173];
const MAP_COLOR = [0.2, 0.28, 0.22];
const PLAYER_COLORS = [[0.227, 0.482, 0.835], [0.835, 0.329, 0.227]];
const SELECT_COLOR = [0.95, 0.85, 0.3];
const UNIT_HALF = 0.9;
const CONTROLLED_DRAW_SCALE = UI.controlledDrawScale ?? 1;
const DOODAD_HALF = 0.5;
const PICK_RADIUS = 1.2;
const DRAG_SLOP_PX = 4;
const PAN_PX_PER_S = 500;
let orbitOrigin = [W / 2, 0, H / 2];
let cameraBounds = null;
let cameraDistance = Math.min(CAMERA_PLAY_DISTANCE, CAMERA_DISTANCE_PER_TILE * Math.max(W, H));
let cameraMaxDistance = Math.max(CAMERA_MAX_FLOOR, CAMERA_MAX_PER_TILE * Math.max(W, H));
const DOODAD_KINDS = UI.doodadKinds ?? [];
const EDGE_PX = 16;
const RECONNECT_BASE_MS = 1000;
const RECONNECT_CAP_MS = 16000;
const RECONNECT_MAX_ATTEMPTS = 6;
const SIM_TICKS_PER_SECOND = 16;
const SIM_TICK_S = 1 / SIM_TICKS_PER_SECOND;
const SIM_TICK_MS = 1000 * SIM_TICK_S;
const HOST_POLL_MS = 10;
const WIRE_SCALE = 1000;
const ROSTER_EVERY_TICKS = 10;
const U16_MAX = 0xffff;

const params = new URLSearchParams(location.search);
const relayUrls = params.get("relay") ? params.get("relay").split(",") : null;
const namespace = params.get("ns") || "oscraft";
const statusEl = document.getElementById("status");
const canvas = document.getElementById("view");
const room = location.hash.slice(1) || "lobby";
const pkgUrl = new URL(document.body.dataset.pkg, location.href).href;
// The map image and rules come from the pack (pack_load.js); mapUrl is set when the sim loads.
let mapUrl = null;

let mapGrid = null;
// The WebGL2 renderer and its orbit camera; created once the WebGL2 context exists.
let renderer3d = null;
let mapBytes = null;
let terrainMesh = null;
let waterMesh = null;
// Splat ground: the control map, the four layers and the GPU handles. Built before the match starts; a layer that fails is an error.
let splat = null;
const SPLAT_BAND = 0.4;
// Sun shadows: one setting. ?shadows=0 turns the shadow map off (no shadow pass, no sampling).
const SHADOW_ON = params.get("shadows") !== "0";
const SHADOW_SIZE = 2048;
// Ground lighting: ambient, diffuse and albedo gain. The gain lifts the owner's darker albedo to lit terrain.
const SPLAT_SHADE = [0.42, 0.68, 1.3];
// Per-layer albedo tint, in SPLAT_LAYERS order. The sand (dirt) layer takes the dirt tint so it sits with the grass family;
// the other layers keep their own colour.
const SPLAT_TINTS = SPLAT_LAYERS.map((name) => (name === "dirt" ? UI.colours.terrain.dirt : [1, 1, 1]));
// Water tint: multiplies the procedural water colour (1 leaves it as shaded).
const SPLAT_WATER_TINT = [1, 1, 1];
let terrainTiles = null;
let unitMeshes = [];
let arrowMesh = null;
const drawCounts = { terrain: 0, splat: 0, doodad: 0, unit: 0, animated: 0 };
// Every asset that failed to load or build, by asset name (and the unit types with no model). Shown on the status line
// and in body.dataset.assetError; a failed required asset (a controlled unit's model, the splat ground, the arrow) stops the start.
const assetErrors = new Map();
// Unit instances drawn into the sun's shadow map this frame (the dynamic casters), published for the browser probe.
let shadowUnitCount = 0;
// Scratch matrices for the per-unit transform and the screen projection, reused every frame.
const unitTransform = new Float32Array(16);
const worldVp = new Float32Array(16);
// Doodads grouped per mesh into static instance batches, and the per-frame batches of unskinned units per mesh.
let doodadBatches = [];
const unitBatches = new Map();
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
let widgets = null;
let packStartMounted = false;
let fogview = null;
let fx = null;
let selectedId = -1;
let reconnectAttempt = 0;
let reconnectTimer = 0;
let drag = null;
let hosting = false;
let orders = 0;
let pointer = null;
let lastFrameMs = 0;
let localTickAcc = 0;
const keysDown = new Set();
const seating = createSeating();
const autoSeats = [];
let seatsExplicit = false;
let encoder = createStateEncoder();
let myPlayer = 0;
let lastAppliedTick = 0;
// A module reload waiting for its swap tick: every peer applies it at the same tick the host announced (FRAME.SWAP).
// { tick, hash, file, url, value, image, error, ready, deferrals, readyPeers (host: peers that sent SWAP_READY), decided }.
let pendingSwap = null;
// The last swap this page applied, { tick, hash }: a SWAP at or before its tick is ignored, so no swap applies twice.
let lastApplied = null;
// A peer's image fetch for the announced hash, reused when the host re-announces the same swap at a later tick.
let peerFetch = null;
// The pubkey of the host, learnt from its ROSTER (only the host sends one); only this peer may announce a swap.
let hostKey = null;
// The last swap announcements this page refused, kept for window.oscraft.swapRejections().
const swapRejections = [];
let rosterView = null;
let spawnedSeats = [];
const localPlayerId = () => myPlayer;
const isGuest = () => netTick >= 0;

function setStatus(extra) {
  const mode = netState ? "net" : "local";
  const errors = assetErrors.size ? `\nasset errors:\n${[...assetErrors].map(([name, message]) => `${name}: ${message}`).join("\n")}` : "";
  statusEl.textContent = `room ${room}\npeers ${peers.size} (${mode})\n${extra || ""}${errors}`;
}

// A failed asset is named on the page: console error, the status line, and body.dataset.assetError. Each name is recorded once.
function recordAssetError(name, err) {
  if (assetErrors.has(name)) return;
  const message = err && err.message ? err.message : String(err);
  assetErrors.set(name, message);
  console.error(`[asset] ${name}: ${message}`);
  document.body.dataset.assetError = [...assetErrors].map(([n, m]) => `${n}: ${m}`).join("; ");
  setStatus("assets failed to load");
}

// A start that cannot run (no WebGL2, a required asset) shows its reason on the page and does not continue.
function failStart(err) {
  const message = err && err.message ? err.message : String(err);
  console.error(`[start] ${message}`);
  document.body.dataset.startError = message;
  setStatus(`start failed: ${message}`);
}

// A mod error comes back as text from the sim's load_mods and tick, for the page to log.
// The mod errors this page has logged, newest last (kept for oscraft.debug.modErrors()).
const MOD_ERROR_LOG_MAX = 50;
const modErrorLog = [];
function logModError(text) {
  if (!text) return;
  console.error(`[mods] ${text}`);
  modErrorLog.push({ tick: localGame ? localGame.ticks() : null, text: String(text) });
  if (modErrorLog.length > MOD_ERROR_LOG_MAX) modErrorLog.shift();
}

// Input goes to the pack. The page that runs the sim calls it directly; a joined page sends the same event to the host
// as an ACTION frame, and the host hands it to the pack with the sender's seat (applyPeerAction).
function sendInput(name, args) {
  if (netState) {
    if (session) session.broadcast(encodeFrame({ type: FRAME.ACTION, name, body: JSON.stringify(args) }));
    return;
  }
  if (localGame) logModError(localGame.dispatch_action(name, JSON.stringify(args)));
}

// A ground event: the local sim's pointer export, or the pack's pointer action for a joined seat.
function forwardPointer(kind, e) {
  if (!renderer3d) return;
  const at = screenToWorld(e.clientX, e.clientY);
  if (!at) return;
  if (netState) sendInput("pointer", { kind, x: at.x, y: at.z, button: e.button });
  else if (localGame) logModError(localGame.pointer(kind, at.x, at.z, e.button));
}

// A key event: the page names the key (lowercase, "space" for the bar), the modifiers and the ground under the pointer.
function forwardKey(phase, e) {
  if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLSelectElement) return;
  const at = pointer && renderer3d ? screenToWorld(pointer.x, pointer.y) : null;
  sendInput("key", {
    key: e.key === " " ? "space" : e.key.toLowerCase(),
    phase,
    shift: e.shiftKey,
    ctrl: e.ctrlKey,
    alt: e.altKey,
    meta: e.metaKey,
    world: at ? { x: at.x, z: at.z } : { x: 0, z: 0 },
  });
}

// The host hands a joined seat's input to the pack: the seat comes from the session, never from the frame.
function applyPeerAction(peerPubkey, frame) {
  const player = seating.seat(peerPubkey).you;
  if (!localGame || player === SPECTATOR) return;
  let args = null;
  try { args = JSON.parse(frame.body); } catch { return; }
  if (!args || typeof args !== "object") return;
  args.player = player;
  logModError(localGame.dispatch_action(frame.name, JSON.stringify(args)));
}

// The last reply the pack gave a debug action (debug_actions), read by oscraft.debug.actions() after its dispatch.
let debugReply = null;

// UI events from the pack. The host's page renders its own and sends every event to the joined pages; each page
// shows the events of its own seat (or of every seat, when the event names none).
function routeUi(ev) {
  const data = ev.data || {};
  if (ev.kind === "debug") { debugReply = data; return; }
  if (ev.kind === "widget") {
    if (data.player !== undefined && data.player !== myPlayer) return;
    if (widgets) widgets.apply(data);
    // The pack-start bar sits in the lobby panel: it is mounted once that panel is up.
    if (data.id === "lobby" && !data.remove && !packStartMounted) { packStartMounted = true; mountPackStart(); }
    return;
  }
  if (ev.kind === "lobby") {
    // The lobby's start comes from the page that runs the sim: it starts the match from the seats the pack gives.
    if (data.player === myPlayer && data.started && !netState) onLobbyStart({ slots: data.slots });
    return;
  }
  if (ev.kind === "feedback" || ev.kind === "action" || ev.kind === "view") {
    if (data.player !== undefined && data.player !== myPlayer) return;
    if (ev.kind === "feedback") {
      if (data.kind === "selection") selectedId = data.ids[0] ?? -1;
      // The pack's panels follow the selection: the page that runs the sim tells the pack what is selected.
      if (data.kind === "selection" && localGame && !netState) logModError(localGame.dispatch_action("selection", JSON.stringify({ player: data.player, ids: data.ids })));
      if (data.kind === "refusal") document.body.dataset.lastRefusal = `cast_dropped ${data.reason}`;
      window.dispatchEvent(new CustomEvent("pack:feedback", { detail: data }));
    } else if (ev.kind === "action") {
      window.dispatchEvent(new CustomEvent("pack:action", { detail: data }));
    } else if (data.centreOn !== undefined) {
      pendingStartCentre = { id: data.centreOn, distance: data.distance };
    } else if (fogview) {
      fogview.setEnabled(data.fog === true);
    }
    return;
  }
  if (window.__modUi) window.__modUi.push(ev);
}

// Each frame: the host takes the pack's UI events, renders them and sends them on; a seat change re-sends the setup.
let setupSeat = null;
function pumpUi() {
  if (!localGame || netState) return;
  if (setupSeat !== myPlayer) {
    setupSeat = myPlayer;
    logModError(localGame.dispatch_action("setup", JSON.stringify({ ui: UI, player: myPlayer, pubkey: auth.pubkey || "local" })));
  }
  for (const ev of JSON.parse(localGame.drain_ui())) {
    routeUi(ev);
    if (peers.size > 0) session.broadcast(encodeFrame({ type: FRAME.UI, body: JSON.stringify(ev) }));
  }
}

async function loadSim() {
  const mod = await import(/* @vite-ignore */ pkgUrl);
  await mod.default();
  mapUrl = packUrl(manifest.image);
  const image = new Uint8Array(await (await fetch(mapUrl)).arrayBuffer());
  mapBytes = image;
  setMapSize(readGrid(image));
  const game = new mod.Game(image);
  // The pack's Lua mods run in the sim, in manifest order; a mod that fails to load keeps the sim and is logged.
  const mods = await Promise.all((manifest.mods ?? []).map(async (name) => ({ name, text: await packText(name) })));
  logModError(game.load_mods(JSON.stringify(mods)));
  // The pack's controls read the UI data (bind table, refusal text, timings) and the local seat once, here and on a seat change.
  logModError(game.dispatch_action("setup", JSON.stringify({ ui: UI, player: myPlayer, pubkey: auth.pubkey || "local" })));
  // Fog of war is on for every page: enemies outside the local team's vision stay hidden.
  game.set_fog(true);
  return game;
}

// The map's grid sets the playfield: orbit origin, default and maximum camera distance scale with its larger side.
function setMapSize(grid) {
  mapGrid = grid;
  W = grid.width;
  H = grid.height;
  const [x0, z0, x1, z1] = walkableBounds(grid);
  cameraBounds = [x0, z0, x1, z1];
  orbitOrigin = [(x0 + x1) / 2, 0, (z0 + z1) / 2];
  cameraDistance = Math.min(CAMERA_PLAY_DISTANCE, CAMERA_DISTANCE_PER_TILE * Math.max(W, H));
  cameraMaxDistance = Math.max(CAMERA_MAX_FLOOR, CAMERA_MAX_PER_TILE * Math.max(W, H));
}

// Skin, node tree and clips for a skinned GLB, with its rest-pose geometry (the joint matrices carry the fit).
// Null when the model has no clips or its skin has more joints than the shader palette.
// The fit is lowered so the rig's lowest skinned vertex stands on y = 0 in its idle, walk and run poses (the fit's own
// floor is the raw first mesh, which the bind pose does not match).
function skinnedRig(animations, glb, fit) {
  const { clips, skins, nodes } = animations;
  const skin = skins[0];
  if (!skin || clips.length === 0 || skin.joints.length > MAX_JOINTS) return null;
  const grounding = clips.filter((clip) => /^(idle|walk|run)/.test(clip.name));
  const grounded = Float32Array.from(fit);
  grounded[13] -= skinFloor(nodes, skin, fit, glb.positions, glb.joints, glb.weights, grounding.length ? grounding : clips);
  return { clips, skin, nodes, fit: grounded, positions: glb.positions, normals: glb.normals || vertexNormals(glb.positions, glb.indices), joints: glb.joints, weights: glb.weights };
}

// Humanoid clip library (anim/library.json, extracted from the spoint animation set). Shared by every rig.
let animLibrary = null;
function humanoidLibrary() {
  if (!animLibrary) animLibrary = assetJson("anim").catch((error) => { animLibrary = null; throw error; });
  return animLibrary;
}

// A rig without embedded clips is animated from the humanoid library through its bone map (anim/maps/<model>.json).
// A rig with no map, or a map that fails to load, is an error for that model: it is never drawn in its rest pose.
async function libraryClips(name, nodes) {
  const [map, library] = await Promise.all([packJson(assetRel("animMaps", `${name}.json`)), humanoidLibrary()]);
  return retargetLibrary(library, map, nodes);
}
// Rig names that take their clips from the library (see parsedModel): the ones a hot reload re-retargets.
const libraryRigs = new Set();

// Each model name is fetched and parsed once: unit types and doodad kinds that name the same GLB share one parse,
// and every fit reads from it. A failed load is dropped from the cache so a later call may retry it.
const parsedModels = new Map();
function parsedModel(name) {
  let entry = parsedModels.get(name);
  if (!entry) {
    entry = (async () => {
      const url = new URL(assetUrl("models", `${name}.glb`), location.href);
      const res = await fetch(url);
      if (!res.ok) throw new Error(`models/${name}.glb HTTP ${res.status}`);
      const bytes = await resolveGlbExtensions(await res.arrayBuffer());
      const glb = parseGlb(bytes);
      const animations = glb.joints && glb.weights ? parseGlbAnimations(bytes) : null;
      if (animations && animations.clips.length === 0) {
        animations.clips = await libraryClips(name, animations.nodes);
        libraryRigs.add(name);
      }
      return { url: url.href, glb, animations };
    })();
    parsedModels.set(name, entry);
    entry.catch(() => parsedModels.delete(name));
  }
  return entry;
}

// Loads one GLB by name and returns its fitted model. A missing or unreadable file, or a rig with no clips or too many
// joints, throws with the asset name: nothing is drawn in its place.
async function fetchModel(name, size) {
  try {
    const { url, glb, animations } = await parsedModel(name);
    const fitted = fitModel(glb, size);
    const animation = animations ? skinnedRig(animations, glb, fitted.fit) : null;
    if (animations && !animation) throw new Error("rig has no clips or more joints than the shader palette");
    return { ...fitted, glbUrl: url, animation };
  } catch (err) {
    throw new Error(`model ${name}: ${err && err.message ? err.message : err}`);
  }
}

// One fitted model per unit type, indexed like the wasm unit_types export. A type with no model entry, or a model that
// fails to load, is null: the unit type is then reported on the page once and not drawn. A controlled unit that has no model, or
// whose model fails, throws, so the match does not start. A model with its own size in doodad_sizes.json (a large unit,
// at controlled-unit heights) is fitted to it; every other unit is fitted to a controlled-unit footprint (UNIT_HALF, the design rule that
// scripts/doodad-sizes.mjs also states).
async function loadModels(game) {
  if (typeof game.unit_model_names !== "function") throw new Error("the wasm build has no unit_model_names export");
  const sizes = await assetJson("doodadSizes");
  // Controlled units (player-controlled unit types) are drawn at CONTROLLED_DRAW_SCALE times their fitted size: the owner's intended size is twice the fit.
  const controlledFlags = typeof game.unit_flags === "function" ? game.unit_flags() : [];
  const typeNames = typeof game.unit_type_names === "function" ? game.unit_type_names() : [];
  return Promise.all(game.unit_model_names().map(async (name, type) => {
    const controlled = controlledFlags[type] === 1;
    if (!name) {
      if (controlled) throw new Error(`unit ${typeNames[type] ?? type}: controlled unit has no model`);
      recordAssetError(`unit ${typeNames[type] ?? type}`, new Error("has no model"));
      return null;
    }
    const size = (sizes[name] ?? 2 * UNIT_HALF) * (controlled ? CONTROLLED_DRAW_SCALE : 1);
    try {
      return await fetchModel(name, size);
    } catch (err) {
      if (controlled) throw err;
      recordAssetError(name, err);
      return null;
    }
  }));
}

// Doodads: one fitted model per kind code (DODA kind). The asset catalogue (assets.json) names a kind's GLB when it has
// an entry; doodads.json maps every kind to its GLB path. A placed kind whose model or footprint is missing is recorded
// on the page and its placements are not drawn.
async function loadDoodadModels(usedKinds) {
  const [catalogue, paths, footprints] = await Promise.all([assetJson("catalogue"), assetJson("doodads"), assetJson("doodadSizes")]);
  const toStem = (file) => file.replace(/^(js\/client\/)?models\//, "").replace(/\.glb$/, "");
  const modelName = (id) => {
    const file = catalogue.items.find((item) => item.id === id && item.kind === "model")?.path;
    return file ? toStem(file) : null;
  };
  // Each model is fitted to its footprint in cells from doodad_sizes.json (scripts/doodad-sizes.mjs).
  const sizeOf = (name) => footprints[name];
  // Only kinds the map places are fetched; an unplaced kind keeps an empty entry, so indices still match the kind codes.
  return Promise.all(DOODAD_KINDS.map(async (kind, code) => {
    if (usedKinds && !usedKinds.has(code)) return { high: null, low: null };
    const name = modelName(kind) ?? (paths[kind] ? toStem(paths[kind]) : null);
    if (!name) {
      recordAssetError(`doodad ${kind}`, new Error("no model path in assets.json or doodads.json"));
      return { high: null, low: null };
    }
    const lowName = modelName(`${kind}-low`);
    const load = (model) => {
      if (sizeOf(model) === undefined) {
        recordAssetError(model, new Error("no footprint in doodad_sizes.json"));
        return null;
      }
      return modelOrRecord(model, fetchModel(model, sizeOf(model)));
    };
    const [high, low] = await Promise.all([load(name), lowName ? load(lowName) : null]);
    return { high, low };
  }));
}

// Records a model that fails and resolves to null, so only that asset is missing from the page.
function modelOrRecord(name, promise) {
  return promise.catch((err) => {
    recordAssetError(name, err);
    return null;
  });
}

// Uploads a fitted model as a drawable mesh. Its base colour texture is uploaded when the model has one; a texture
// that does not load throws. Units pass animated: their skinned rig replaces the fitted geometry, which the joint matrices carry instead.
async function toMesh(gl, fitted, fallbackColor, { animated = false } = {}) {
  if (!fitted) return null;
  const rig = animated ? fitted.animation : null;
  const mesh = rig
    ? { positions: rig.positions, normals: rig.normals, uvs: fitted.uvs, indices: fitted.indices, joints: rig.joints, weights: rig.weights, animation: rig, color: fallbackColor, height: maxY(fitted.positions) }
    : { positions: fitted.positions, normals: fitted.normals, uvs: fitted.uvs, indices: fitted.indices, color: fallbackColor, height: maxY(fitted.positions) };
  if (fitted.baseColorTexture) {
    const image = await loadBaseColorImage(fitted.baseColorTexture, fitted.glbUrl);
    if (!image) throw new Error(`base colour texture of ${fitted.glbUrl} did not load`);
    mesh.texture = uploadTexture(gl, image);
  }
  return mesh;
}

function maxY(positions) {
  let top = 0;
  for (let v = 1; v < positions.length; v += 3) top = Math.max(top, positions[v]);
  return top;
}

// The ranged attack's projectile: the arrow model, fitted to the footprint doodad_sizes.json gives it. Required: a
// failed arrow stops the start rather than drawing the projectile as something else.
async function loadArrowMesh(gl) {
  const footprints = await assetJson("doodadSizes");
  if (footprints.arrow === undefined) throw new Error("model arrow: no footprint in doodad_sizes.json");
  return toMesh(gl, await fetchModel("arrow", footprints.arrow), [1, 1, 1]);
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
  const out = simUnits();
  return fogview ? fogview.filter(out) : out;
}

// Every living unit of the local sim, unfiltered by fog: the host's view for seats, stats and command ownership.
function simUnits() {
  if (!localGame) return [];
  const flat = localGame.units();
  const ids = localGame.unit_ids();
  // Older wasm builds have no unit_types or unit_hps export: their type is unknown (-1), so no model is drawn for them and the page reports it.
  const types = typeof localGame.unit_types === "function" ? localGame.unit_types() : null;
  const hps = typeof localGame.unit_hps === "function" ? localGame.unit_hps() : null;
  // Older wasm builds lack unit_attacking; their units never report an attack clip.
  const attacking = typeof localGame.unit_attacking === "function" ? localGame.unit_attacking() : null;
  const out = [];
  for (let i = 0; i < ids.length; i++) {
    out.push(anchored({ id: ids[i], x: flat[3 * i], z: flat[3 * i + 1], player: flat[3 * i + 2], type: types ? types[i] : -1, hp: hps ? hps[i] : -1, attacking: attacking ? attacking[i] === 1 : false }));
  }
  return out;
}

// Draw counts per mesh kind for the current frame, published on the body for the browser probe.
function countDraw(kind, mesh, transform, joints = null) {
  drawCounts[kind] += 1;
  if (joints) renderer3d.drawSkinned(mesh, transform, joints);
  else renderer3d.drawMesh(mesh, transform);
}

// Unit meshes are cached per (type, colour key): a textured model keeps its texture whatever the colour. Callers pass
// only a type that has a model (collectUnitDraws reports the rest).
const unitMeshCache = new Map();
function unitMeshFor(type, color) {
  const key = `${type}:${color.join(",")}`;
  let mesh = unitMeshCache.get(key);
  if (!mesh) {
    mesh = { ...unitMeshes[type], color };
    unitMeshCache.set(key, mesh);
  }
  return mesh;
}

// A unit type with no model is reported once (console and status line) and its units are not drawn.
function reportMissingModel(type) {
  const names = localGame && typeof localGame.unit_type_names === "function" ? localGame.unit_type_names() : [];
  recordAssetError(`unit ${names[type] ?? type}`, new Error(`type ${type} has no model`));
}

// Joint palette for a unit this frame: its clip (walk when it moved, attack when it reports attacking, else idle)
// advanced by dt seconds. A unit with no matching clip holds the rest pose.
const EMPTY_POSE = new Map();
// Turn rate of a rigged unit toward its heading, in radians per second scale (the step is eased by dt).
const TURN_RATE = 10;
// Walk and run clip rate bounds, and the ease (per second) of the draw-side speed the rate follows.
const RATE_MIN = 0.25;
const RATE_MAX = 4;
const SPEED_EASE = 8;
function wrapAngle(a) {
  return ((a + Math.PI) % (2 * Math.PI) + 2 * Math.PI) % (2 * Math.PI) - Math.PI;
}
function jointsFor(u, mesh, dt) {
  const key = `${u.id}:${u.type}`;
  let entry = animators.get(key);
  if (!entry) {
    const { skin } = mesh.animation;
    entry = { player: new AnimationPlayer(mesh.animation.clips), x: u.x, z: u.z, seen: false, heading: 0, yaw: 0, speed: 0, cycle: {}, joints: new Float32Array(16 * skin.joints.length) };
    animators.set(key, entry);
  }
  const { nodes, skin, fit } = mesh.animation;
  // Heading comes from the draw-side step since the last frame; an attacking unit keeps its heading (no target is exposed).
  const dx = u.x - entry.x;
  const dz = u.z - entry.z;
  const step = Math.hypot(dx, dz);
  const moving = step > MOVE_EPS;
  if (moving) entry.heading = Math.atan2(dx, dz);
  const turn = wrapAngle(entry.heading - entry.yaw);
  entry.yaw += turn * Math.min(1, dt * TURN_RATE);
  // Draw-side speed, eased so one frame's jitter does not change the clip's rate.
  entry.speed += ((dt > 0 ? step / dt : 0) - entry.speed) * Math.min(1, dt * SPEED_EASE);
  entry.x = u.x;
  entry.z = u.z;
  entry.seen = true;
  entry.player.playState({ attacking: Boolean(u.attacking), moving });
  // A walk or run cycle plays at the unit's speed over the gait's own speed, so its feet keep pace with the ground.
  const clip = entry.player.clip;
  if (clip && entry.cycle[clip.name] === undefined) entry.cycle[clip.name] = cycleSpeed(nodes, fit, clip);
  const cycle = clip ? entry.cycle[clip.name] : 0;
  entry.player.rate = moving && !u.attacking && cycle > 0 ? Math.min(RATE_MAX, Math.max(RATE_MIN, entry.speed / cycle)) : 1;
  entry.player.update(dt);
  return jointMatrices(nodes, skin, entry.player.pose() ?? EMPTY_POSE, fit, entry.joints);
}

// Groups { mesh, lowMesh?, transform } placements into one static instance batch per mesh. lowMesh is the mesh's
// lower-poly model (shared by every placement of that mesh) for the renderer's level-of-detail path.
function batchByMesh(placed) {
  const groups = new Map();
  for (const p of placed) {
    let list = groups.get(p.mesh);
    if (!list) groups.set(p.mesh, (list = { lowMesh: p.lowMesh ?? null, transforms: [] }));
    list.transforms.push(p.transform);
  }
  return [...groups].map(([mesh, { lowMesh, transforms }]) => {
    const matrices = new Float32Array(transforms.length * 16);
    transforms.forEach((t, i) => matrices.set(t, i * 16));
    return { mesh, lowMesh, batch: { matrices, count: transforms.length, dirty: true } };
  });
}

// Appends a unit transform to its mesh's per-frame batch; flushUnitBatches draws one call per mesh.
function queueUnit(mesh, transform) {
  let entry = unitBatches.get(mesh);
  if (!entry) unitBatches.set(mesh, (entry = { matrices: new Float32Array(16 * 64), count: 0, dirty: true }));
  if ((entry.count + 1) * 16 > entry.matrices.length) {
    const grown = new Float32Array(entry.matrices.length * 2);
    grown.set(entry.matrices);
    entry.matrices = grown;
  }
  entry.matrices.set(transform, entry.count * 16);
  entry.count += 1;
}

function flushUnitBatches() {
  for (const [mesh, entry] of unitBatches) {
    entry.dirty = true;
    if (entry.count > 0) renderer3d.drawInstanced(mesh, entry);
    entry.count = 0;
  }
}

// Collects this frame's visible unit draws once: each animator advances here, and the list is replayed by the shadow
// pass and the lit pass. The transform is copied because the scratch matrix is reused by the next unit.
function collectUnitDraws(units, dt) {
  const list = [];
  const clips = [];
  const yaws = [];
  for (const entry of animators.values()) entry.seen = false;
  for (const raw of units) {
    const u = drawnPosition(raw);
    if (!unitMeshes[u.type]) {
      reportMissingModel(u.type);
      continue;
    }
    const color = u.id === selectedId ? SELECT_COLOR : PLAYER_COLORS[u.player] || PLAYER_COLORS[0];
    const mesh = unitMeshFor(u.type, color);
    if (!renderer3d.isVisible(u.x, u.z, meshReach(mesh))) {
      // Off screen: no draw and no clip advance, but its animator survives so the clip resumes where it stopped.
      const kept = animators.get(`${u.id}:${u.type}`);
      if (kept) kept.seen = true;
      continue;
    }
    const transform = new Float32Array(translation(u.x, groundAt(u.x, u.z), u.z, unitTransform));
    if (mesh.animation) {
      const joints = jointsFor(u, mesh, dt);
      const entry = animators.get(`${u.id}:${u.type}`);
      const facing = new Float32Array(multiply(translation(u.x, groundAt(u.x, u.z), u.z), rotationY(entry.yaw)));
      list.push({ kind: "unit", mesh, transform: facing, joints });
      clips.push(`${u.id}:${entry.player.clip?.name ?? ""}:${entry.player.rate.toFixed(2)}`);
      yaws.push(`${u.id}:${entry.yaw.toFixed(2)}`);
    } else list.push({ kind: "unit", mesh, transform, joints: null });
  }
  for (const [key, entry] of animators) if (!entry.seen) animators.delete(key);
  // The clip each rigged unit plays this frame, for the browser probe.
  document.body.dataset.animClips = clips.join(",");
  document.body.dataset.tickAlpha = tickShare().toFixed(3);
  document.body.dataset.animYaw = yaws.join(",");
  return list;
}

// Draw-only interpolation between sim ticks. Each unit glides from its position at the tick before to its position at
// the latest tick, over one tick of wall time, so the sim's positions (and everything that reads them) stay unchanged.
const tickClock = new Map();
let tickStampMs = null;
function advanceTickClock(units) {
  const next = new Map();
  for (const u of units) {
    const old = tickClock.get(u.id);
    next.set(u.id, { px: old ? old.x : u.x, pz: old ? old.z : u.z, x: u.x, z: u.z });
  }
  tickClock.clear();
  for (const [id, v] of next) tickClock.set(id, v);
  tickStampMs = performance.now();
}

// The unit with its draw position: between its previous and latest tick positions by the share of a tick elapsed.
function tickShare() {
  return tickStampMs === null ? 0 : Math.min(1, Math.max(0, (performance.now() - tickStampMs) / SIM_TICK_MS));
}

// The unit as drawn this frame: model and health bar both read this, so they move together between ticks. The top height
// follows the drawn ground, so a bar stays over its model up a slope.
function drawnPosition(u) {
  const c = tickClock.get(u.id);
  if (!c || tickStampMs === null) return u;
  const a = tickShare();
  return anchored({ ...u, x: c.px + (c.x - c.px) * a, z: c.pz + (c.z - c.pz) * a });
}

// The units the HUD's health bars and feedback read: the same drawn positions the models use this frame.
function shownUnits() {
  return currentUnits().map(drawnPosition);
}

// Draws a collected unit list. The shadow pass (shadow = true) re-issues the same meshes with the light's matrix and
// returns the number of unit instances it cast; the lit pass counts per category as before.
function emitUnitDraws(list, shadow) {
  let casters = 0;
  for (const d of list) {
    if (shadow) casters += 1;
    if (d.joints) {
      if (shadow) renderer3d.drawSkinned(d.mesh, d.transform, d.joints);
      else {
        countDraw(d.kind, d.mesh, d.transform, d.joints);
        drawCounts.animated += 1;
      }
    } else {
      if (!shadow) drawCounts[d.kind] += 1;
      queueUnit(d.mesh, d.transform);
    }
  }
  flushUnitBatches();
  return casters;
}

// Creep: sets one cell's ground kind (grass, dirt or path) by rewriting its control texel. Only that texel is uploaded;
// the terrain mesh and the tile batches are not rebuilt. Index is y * width + x in map cells.
window.__setCellKind = (index, kind) => {
  if (!splat || !renderer3d) return false;
  renderer3d.updateSplatCell(splat, index, setSplatTexel(splat.control, index, kind));
  return true;
};

// Terrain and doodads: the casters of the sun's shadow map as well as the lit draws.
function drawGround3d() {
  // The water is drawn on the terrain itself (the splat shader's water blend), so the water quads are not drawn on top of
  // it: at the banks they overlapped the terrain and showed the same ground twice.
  renderer3d.drawSplat(terrainMesh, splat);
  drawCounts.splat += 1;
  drawCounts.terrain += 1;
  for (const d of doodadBatches) {
    drawCounts.doodad += renderer3d.drawInstancedLod(d.mesh, d.batch, d.lowMesh);
  }
}

function drawWorld3d() {
  drawCounts.terrain = 0;
  drawCounts.splat = 0;
  drawCounts.doodad = 0;
  drawCounts.unit = 0;
  drawCounts.animated = 0;
  const now = performance.now();
  const dt = lastDrawMs === null ? 0 : Math.min((now - lastDrawMs) / 1000, MAX_FRAME_DT_S);
  lastDrawMs = now;
  renderer3d.beginFrame(BACKGROUND);
  const units = currentUnits();
  const unitDraws = collectUnitDraws(units, dt);
  // Static casters (terrain, doodads, buildings, towers) are drawn into the cached layer only when it is dirty; the
  // dynamic casters (units) are drawn into the working map every frame, on top of that layer.
  const shadow = renderer3d.beginShadowPass ? renderer3d.beginShadowPass() : null;
  shadowUnitCount = 0;
  if (shadow) {
    const counted = { ...drawCounts };
    if (shadow.staticDirty) drawGround3d();
    renderer3d.beginShadowUnits();
    shadowUnitCount = emitUnitDraws(unitDraws, true);
    renderer3d.endShadowPass();
    Object.assign(drawCounts, counted);
  }
  drawGround3d();
  emitUnitDraws(unitDraws, false);
  // window.__fxOff is the CI probe switch that measures the frame cost of the effects.
  if (fx && window.__fxOff !== true) fx.frame(units, dt);
  renderer3d.endFrame();
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

// Ground point (x, z) under a client pixel: the view ray intersected with the terrain surface.
function screenToWorld(clientX, clientY) {
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
  const clip = worldClip(x, z, y);
  if (clip[3] <= 0) return { x: -1e6, y: -1e6 };
  return {
    x: ((clip[0] / clip[3] + 1) / 2) * window.innerWidth,
    y: ((1 - clip[1] / clip[3]) / 2) * window.innerHeight,
  };
}

// Homogeneous clip coordinates [X, Y, Z, W] of the world point; W <= 0 is behind the camera plane. The fog overlay cuts cells at W so none is projected through the camera.
function worldClip(x, z, y = 0) {
  const vp = viewProjection(renderer3d.camera, window.innerWidth / Math.max(1, window.innerHeight), worldVp);
  return [0, 1, 2, 3].map((r) => vp[r] * x + vp[4 + r] * y + vp[8 + r] * z + vp[12 + r]);
}

// The commands createHud routes input through. Each runs on the host sim, or sends the frame to the host
// when this page is a guest; each returns whether the sim (or the host's send) accepted it.
// A command the local sim refused leaves the sim's refusal name on the page (body.dataset.lastRefusal) for the CI witness.
// An accepted command clears the page's last refusal, so lastRefusal always names the most recent command's outcome.
function noteLocalRefusal(ok, what) {
  if (ok) delete document.body.dataset.lastRefusal;
  else document.body.dataset.lastRefusal = `${what} ${localGame.last_refusal()}`;
  return ok;
}

const commands = {
  order(unitId, x, z) {
    const ok = netState ? (session.broadcast(sendOrder(unitId, x, z)), true) : noteLocalRefusal(localGame.order_move(unitId, x, z), `order_refused unit=${unitId}`);
    if (ok) { orders += 1; publishCommandCounts(); }
    setStatus(`order move unit ${unitId} to ${x.toFixed(2)},${z.toFixed(2)}${netState ? "" : " (local)"}`);
    return ok;
  },
  attackMove(unitId, x, z) {
    const ok = netState ? (session.broadcast(sendAttackMove(unitId, x, z)), true) : noteLocalRefusal(localGame.attack_move(unitId, x, z), `attack_move_refused unit=${unitId}`);
    if (ok) { orders += 1; publishCommandCounts(); }
    setStatus(`attack-move unit ${unitId} to ${x.toFixed(2)},${z.toFixed(2)}${netState ? "" : " (local)"}`);
    return ok;
  },
  attack(unitId, targetId) {
    const ok = netState ? (session.broadcast(sendAttack(unitId, targetId)), true) : noteLocalRefusal(localGame.attack(unitId, targetId), `attack_refused unit=${unitId}`);
    if (ok) { attacks += 1; publishCommandCounts(); }
    return ok;
  },
  cast(unitId, slot, target, x, z) {
    const ok = netState ? (session.broadcast(sendCast(unitId, slot, target, x, z)), true) : noteLocalRefusal(localGame.cast(unitId, slot, target, x, z), `cast_refused unit=${unitId} slot=${slot}`);
    if (ok) { casts += 1; publishCommandCounts(); }
    return ok;
  },
  hold(unitId) {
    const ok = netState ? (session.broadcast(sendHold(unitId)), true) : noteLocalRefusal(localGame.hold(unitId), `hold_refused unit=${unitId}`);
    if (ok) { orders += 1; publishCommandCounts(); }
    return ok;
  },
  useItem(unitId, slot, target, x, z) {
    const ok = netState ? (session.broadcast(sendUseItem(unitId, slot, target, x, z)), true) : noteLocalRefusal(localGame.use_item(unitId, slot, target, Math.round(x), Math.round(z)), `item_refused unit=${unitId} slot=${slot}`);
    if (ok) { casts += 1; publishCommandCounts(); }
    return ok;
  },
  // Debug overrides of a unit's target class and attack-move hold (-1 clears; see the wasm Game's set_class_override).
  // They are not orders, so they do not count toward the order or cast counters.
  classOverride(unitId, cls) {
    return netState ? (session.broadcast(sendClassOverride(unitId, cls)), true) : localGame.set_class_override(unitId, cls);
  },
  holdOverride(unitId, hold) {
    return netState ? (session.broadcast(sendHoldOverride(unitId, hold)), true) : localGame.set_hold_override(unitId, hold);
  },
};

function bindInput() {
  canvas.addEventListener("pointerdown", (e) => {
    // A left press is selection (the pack decides), so only the other buttons drag the view.
    drag = e.button === 0 ? null : { x: e.clientX, y: e.clientY, moved: false };
    canvas.setPointerCapture(e.pointerId);
    forwardPointer("down", e);
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
    forwardPointer("up", e);
    if (e.button === 2) document.body.dataset.rightUp = `${wasDrag ? "drag" : "click"} hud=${!!hud} local=${myPlayer}`;
  });
  canvas.addEventListener("contextmenu", (e) => e.preventDefault());
  window.addEventListener("keydown", (e) => {
    if (e.target instanceof HTMLInputElement) return;
    if (e.key.startsWith("Arrow")) {
      keysDown.add(e.key);
      e.preventDefault();
      return;
    }
    // Plain keys (letters, digits, space, Tab, Escape) are the pack's; browser shortcuts with Ctrl or Meta keep their default.
    if (e.key.length === 1 || e.key === " " || e.key === "Tab" || e.key === "Escape") {
      if (!e.ctrlKey && !e.metaKey) e.preventDefault();
    }
    forwardKey("down", e);
  });
  window.addEventListener("keyup", (e) => {
    keysDown.delete(e.key);
    forwardKey("up", e);
  });
  window.addEventListener("blur", () => keysDown.clear());
  window.addEventListener("pointermove", (e) => { pointer = { x: e.clientX, y: e.clientY }; });
  document.documentElement.addEventListener("pointerleave", () => { pointer = null; });
  canvas.addEventListener("wheel", (e) => {
    e.preventDefault();
    if (renderer3d) renderer3d.zoom(e.deltaY);
  }, { passive: false });
  window.addEventListener("resize", resize);
}

// Browser host: this page owns the sim (localGame). Seats come from the lobby START frame (seating.start); before a
// START a joining peer takes the lowest free slot. A seat is kept by its key, so a reconnecting peer gets it back.
// Every SIM_TICK_MS the host ticks the sim and broadcasts that tick's state through the delta encoder (delta.js).
function autoSeat(peerPubkey) {
  if (seatsExplicit || seating.seat(peerPubkey).you !== SPECTATOR) return;
  const taken = new Set(autoSeats.map((s) => s.slot));
  for (let slot = 0; slot < MAX_PLAYERS; slot++) {
    if (!taken.has(slot)) {
      autoSeats.push({ slot, owner: peerPubkey, pick: "" });
      seating.start(autoSeats);
      return;
    }
  }
}

function sendTo(peerPubkey, bytes) {
  if (typeof session.send === "function") session.send(peerPubkey, bytes);
  else session.broadcast(bytes);
}

function sendRoster(peerPubkey) {
  sendTo(peerPubkey, encodeFrame(seating.roster(peerPubkey)));
}

// A player-controlled unit type (the map's data flag, unit_flags) is a controlled unit; the test is by type.
let playerControlledTypes = null;
function isControlledUnit(id) {
  const u = simUnits().find((v) => v.id === id);
  if (!u || typeof localGame.unit_flags !== "function") return false;
  playerControlledTypes ??= localGame.unit_flags();
  return playerControlledTypes[u.type] === 1;
}

// Records each START seat's pick. The sim's Lua spawn rule places the picked unit; the page does not spawn it.
function recordSeatPicks(seats) {
  spawnedSeats = seats.map(({ slot, pick }) => ({ slot, pick }));
}

// Applies a START (the lobby's own or a peer's): seats the players, lets the sim spawn the picked units and, on the host, tells every peer.
function applyStart(frame) {
  seating.start(frame.seats);
  seatsExplicit = true;
  const mine = seating.seat(auth.pubkey);
  if (mine.you !== SPECTATOR) myPlayer = mine.you;
  if (!isGuest()) recordSeatPicks(frame.seats);
  document.body.dataset.seated = String(seating.size());
  if (hosting) {
    encoder.keyframe();
    for (const key of peers) sendRoster(key);
  }
}

function endLobby() {
  document.getElementById("ui-modal")?.remove();
}

// Pack editing. The host edits the pack in the editor; a saved change goes to every peer as PACK_FILE frames, and a peer
// stores it in its own copy. Only the host sends pack files: a PACK_FILE from anyone else is ignored.
let pack = new Pack();
let packReceiver = createPackReceiver(pack);
const packBlobs = new Map(); // hash -> { name, bytes } of the large files sent by reference
let packEditor = null;

function openPackEditor() {
  if (packEditor) packEditor.dispose();
  const root = document.createElement("div");
  document.body.append(root);
  packEditor = createPackEditor({
    root,
    pack,
    canEdit: hosting,
    onSave: publishPackChanges,
    onLoad: (loaded) => { pack = loaded; packReceiver = createPackReceiver(pack); },
  });
  packEditor.open();
}

// A saved change goes two ways. Its files go to the peers as PACK_FILE frames; then the host's own sim takes the change at a
// swap tick, the same tick the peers take it: a Lua file reloads the mod set, a data file or the manifest swaps the map
// image (both through the image swap: SWAP, the readiness gate and applyDueSwap). An asset applies at once.
function publishPackChanges(changes) {
  if (!hosting) throw new Error("only the host may save the pack");
  for (const change of changes) for (const frame of changeFrames(change, packBlobs)) session.broadcast(encodeFrame(frame));
  for (const change of changes) if (!change.deleted && packKindOf(change.path) === null) applyPackAsset(change.path);
  queuePackSwaps(changes);
  setStatus(`pack: sent ${changes.length} change(s) to peers`);
}

// The pack's manifest as the pack now holds it (the page's own when the pack holds none). A bad manifest throws.
function currentManifest() {
  const raw = pack.get("manifest.json");
  if (raw === undefined) return manifest;
  return JSON.parse(typeof raw === "string" ? raw : new TextDecoder().decode(raw));
}

// The swap kind of a pack file (see packSwapKind): the map image the manifest names is the data swap's image.
function packKindOf(file) {
  let image = manifest.image;
  try { image = currentManifest().image ?? image; } catch { /* a bad manifest keeps the page's image name */ }
  return packSwapKind(file, image);
}

// A pack file's text: the pack's copy, or the page's own when the pack does not hold it.
async function packFileText(name) {
  const held = pack.get(name);
  if (held === undefined) return packText(name);
  return typeof held === "string" ? held : new TextDecoder().decode(held);
}

// The bytes a swap of `kind` applies, the same on the host and on every peer: the Lua mod set as load_mods takes it
// (lua), or the map image named by the manifest (data). A peer checks these bytes against the announced hash.
async function packSwapBytes(kind) {
  if (kind === "lua") {
    const mods = await Promise.all((currentManifest().mods ?? []).map(async (name) => ({ name, text: await packFileText(name) })));
    return new TextEncoder().encode(JSON.stringify(mods));
  }
  const name = currentManifest().image;
  const held = pack.get(name);
  if (held !== undefined) return typeof held === "string" ? new TextEncoder().encode(held) : held.slice();
  return new Uint8Array(await packBytes(name));
}

// Pack swaps waiting for their turn: one swap is pending at a time, so the kinds one save changes are announced in turn.
const packSwapQueue = [];

function queuePackSwaps(changes) {
  const mods = currentManifest().mods ?? [];
  const queued = new Set();
  for (const change of changes) {
    const kind = packKindOf(change.path);
    if (kind === null || queued.has(kind)) continue;
    if (kind === "lua" && !mods.includes(change.path)) continue; // a Lua file the mods do not load changes nothing
    queued.add(kind);
    packSwapQueue.push({ kind, file: change.path });
  }
}

// Host: announces the next queued pack swap when none is pending or being announced. Called once per sim tick.
function drainPackSwapQueue() {
  if (pendingSwap || swapFetching || !localGame || packSwapQueue.length === 0) return;
  const next = packSwapQueue.shift();
  announcePackSwap(next.kind, next.file);
}

// Host half of a pack swap: builds the bytes from the host's own pack, then announces the swap tick exactly as an image swap.
async function announcePackSwap(kind, file) {
  swapFetching = true;
  try {
    const bytes = await packSwapBytes(kind);
    const hash = fnv1a32(bytes);
    const tick = localGame.ticks() + SWAP_LEAD_TICKS;
    session.broadcast(encodeFrame({ type: FRAME.SWAP, tick, hash, file }));
    pendingSwap = { tick, hash, file, kind, url: null, value: null, image: bytes, error: null, ready: Promise.resolve(bytes), deferrals: 0, readyPeers: new Set(), decided: false };
    console.log(`[pack] ${file}: ${kind} swap at tick ${tick} (hash ${hash.toString(16)})`);
  } catch (err) {
    console.error(`[pack] ${file}: swap not announced, not applied: ${err.message}`);
    lastSwapResult = { tick: localGame.ticks(), file, ok: false, carried: null, refused: null, error: err.message, readiness: "failed", deferred: false, deferrals: 0 };
  } finally {
    swapFetching = false;
  }
}

// A pack asset applies at once: a model that is loaded is parsed again on its next use (by its base name), and a texture is
// read again when it next loads. Anything holding the old bytes is the page's own cache to drop.
function applyPackAsset(path) {
  const base = path.replace(/^.*\//, "").replace(/\.glb$/i, "");
  if (/\.glb$/i.test(path)) parsedModels.delete(base);
  console.log(`[pack] asset ${path} applied at once`);
  window.dispatchEvent(new CustomEvent("oscraft:pack-asset", { detail: { path } }));
}

// Start from a loaded pack: the lobby reads a pack file (pack_load.js), checks its manifest, stashes it and loads this page
// again on the stash, so the match runs that pack. `hostRoom` makes the new page the room's host (?host=1); otherwise the
// game is local to this page until the room is hosted.
function startFromLoadedPack(hostRoom) {
  const picker = document.createElement("input");
  picker.type = "file";
  picker.accept = ".zip,.tar.gz,.tgz";
  picker.addEventListener("change", async () => {
    const file = picker.files[0];
    if (!file) return;
    try {
      setStatus(`pack ${file.name}: reading`);
      const { files, skipped } = await readPackFile(file);
      const loaded = manifestOf(files);
      await stashPack(files);
      const next = new URL(location.href);
      next.searchParams.set("stashed", "1");
      if (hostRoom) next.searchParams.set("host", "1");
      else next.searchParams.delete("host");
      console.log(`[pack] ${loaded.name} ${loaded.version ?? ""}: ${files.size} files${skipped.length ? `, skipped ${skipped.join(", ")}` : ""}`);
      location.href = next.href;
    } catch (err) {
      setStatus(`pack not started: ${err.message}`);
    }
  });
  picker.click();
}

// The lobby's "start from loaded pack" buttons: a local game and a hosted one. The lobby redraws its children on every
// change, so the bar is put back into the lobby section after each redraw (and hides with it).
function mountPackStart() {
  const lobbyEl = document.getElementById("ui-modal");
  if (!lobbyEl) return;
  const bar = document.createElement("div");
  bar.className = "pack-start";
  for (const [label, hostRoom] of [["start from loaded pack", false], ["host from loaded pack", true]]) {
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = label;
    button.addEventListener("click", () => startFromLoadedPack(hostRoom));
    bar.append(button);
  }
  const put = () => { if (bar.parentNode !== lobbyEl) lobbyEl.append(bar); };
  new MutationObserver(put).observe(lobbyEl, { childList: true });
  put();
}

function handlePackFrame(sender, frame) {
  if (frame.type === FRAME.PACK_WANT) {
    const blob = hosting ? packBlobs.get(frame.hash) : undefined;
    if (!blob) return;
    for (const f of chunkFrames(blob.name, blob.bytes, frame.hash)) sendTo(sender, encodeFrame(f));
    return;
  }
  if (hosting || sender !== hostKey) return;
  const out = packReceiver.accept(frame);
  if (out.kind === "want") { sendTo(hostKey, encodeFrame({ type: FRAME.PACK_WANT, hash: out.hash })); return; }
  if (out.kind === "refused") console.warn(`[pack] refused: ${out.reason}`);
  if (out.kind === "applied") {
    console.log(`[pack] ${out.path} ${out.deleted ? "deleted" : "updated"}`);
    if (!out.deleted && packKindOf(out.path) === null) applyPackAsset(out.path);
  }
}

function onLobbyStart(state) {
  const frame = startFrame(state);
  setStatus("lobby: match starting");
  if (!isGuest()) applyStart(frame);
  try { session.broadcast(encodeFrame(frame)); } catch { /* the frame is refused when a seat key is oversize */ }
}

// A peer's move, cast or attack runs on the host only for a unit the peer's player owns. Accepted casts and
// attacks are counted on the host page (data-remote-casts, data-remote-attacks) for the CI witness.
function applyPeerCommand(peerPubkey, command) {
  const player = seating.seat(peerPubkey).you;
  const unit = simUnits().find((u) => u.id === command.unitId);
  if (player === SPECTATOR || !unit || unit.player !== player) {
    setStatus(`order refused not_owner peer=${String(peerPubkey).slice(0, 8)} unit=${command.unitId}`);
    document.body.dataset.lastRefusal = `not_owner unit=${command.unitId}`;
    return false;
  }
  if (command.type === FRAME.ORDER) {
    const ok = localGame.order_move(command.unitId, command.x, command.y);
    if (!ok) document.body.dataset.lastRefusal = `order_refused unit=${command.unitId} x=${command.x} z=${command.y}`;
    if (ok) countRemote("remoteOrders");
    return ok;
  }
  if (command.type === FRAME.CAST) {
    const ok = localGame.cast(command.unitId, command.slot, command.target, command.x, command.y);
    if (!ok) document.body.dataset.lastRefusal = `cast_refused unit=${command.unitId}`;
    if (ok) countRemote("remoteCasts");
    return ok;
  }
  if (command.type === FRAME.ATTACK_MOVE) {
    const ok = localGame.attack_move(command.unitId, command.x, command.y);
    if (!ok) document.body.dataset.lastRefusal = `attack_move_refused unit=${command.unitId} x=${command.x} z=${command.y}`;
    if (ok) countRemote("remoteOrders");
    return ok;
  }
  if (command.type === FRAME.HOLD) {
    const ok = localGame.hold(command.unitId);
    if (!ok) document.body.dataset.lastRefusal = `hold_refused unit=${command.unitId}`;
    if (ok) countRemote("remoteOrders");
    return ok;
  }
  if (command.type === FRAME.USE_ITEM) {
    const ok = localGame.use_item(command.unitId, command.slot, command.target, command.x, command.y);
    if (!ok) document.body.dataset.lastRefusal = `item_refused unit=${command.unitId} slot=${command.slot}`;
    if (ok) countRemote("remoteCasts");
    return ok;
  }
  if (command.type === FRAME.CLASS_OVERRIDE) {
    return localGame.set_class_override(command.unitId, command.class === 255 ? -1 : command.class);
  }
  if (command.type === FRAME.HOLD_OVERRIDE) {
    return localGame.set_hold_override(command.unitId, command.hold === 255 ? -1 : command.hold);
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
  const record = { tick, hash: localGame.state_hash().toString(), bytes: btoa(binary), seats: seating.export() };
  try {
    localStorage.setItem(snapshotKey(), JSON.stringify(record));
    delete document.body.dataset.snapshotSkipped;
  } catch (e) {
    // The match state can outgrow the origin quota; the room then runs without a restore point instead of throwing per save.
    if (e && e.name === "QuotaExceededError") { localStorage.removeItem(snapshotKey()); document.body.dataset.snapshotSkipped = String(tick); return; }
    throw e;
  }
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
    if (Array.isArray(record.seats) && record.seats.length > 0) {
      seating.restore(record.seats);
      seatsExplicit = true;
    }
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
  encoder = createStateEncoder();
  const restoredAt = restoreSnapshot();
  let hostTick = restoredAt > 0 ? restoredAt : 0;
  const hostStep = () => {
    logModError(localGame.tick());
    drainPackSwapQueue();
    hostSwapGate();
    applyDueSwap();
    advanceTickClock(simUnits());
    hostTick += 1;
    if (hostTick % SNAPSHOT_EVERY_TICKS === 0) saveSnapshot(hostTick);
    const hash = Number(localGame.state_hash() & 0xffffffffn) >>> 0;
    const units = currentUnits().map((u) => ({ id: u.id, kind: Math.max(0, u.type), player: u.player, x: Math.round(u.x * WIRE_SCALE), y: Math.round(u.z * WIRE_SCALE), hp: Math.max(0, u.hp) }));
    if (window.__ciSent) recordTick(window.__ciSent, hostTick, units);
    const frame = encoder.encode(hostTick, hash, units);
    const bytes = encodeFrame(frame);
    if (window.__ciFrames && window.__ciFrames.length < 200) window.__ciFrames.push({ type: frame.type, bytes: bytes.byteLength });
    session.broadcast(bytes);
    if (hostTick % ROSTER_EVERY_TICKS === 0) {
      for (const key of peers) sendRoster(key);
    }
  };
  let hostAcc = 0;
  let hostLast = performance.now();
  setInterval(() => {
    const now = performance.now();
    hostAcc += Math.min(now - hostLast, MAX_FRAME_DT_S * 1000);
    hostLast = now;
    while (hostAcc >= SIM_TICK_MS) {
      hostAcc -= SIM_TICK_MS;
      hostStep();
    }
  }, HOST_POLL_MS);
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

function sendAttackMove(unitId, x, z) {
  return encodeFrame({ type: FRAME.ATTACK_MOVE, unitId, x: Math.round(x), y: Math.round(z) });
}

function sendAttack(unitId, targetId) {
  return encodeFrame({ type: FRAME.ATTACK, unitId, targetId });
}

function sendHold(unitId) {
  return encodeFrame({ type: FRAME.HOLD, unitId });
}

// The wire carries 255 for a cleared override; the sim-facing value is -1.
function sendClassOverride(unitId, cls) {
  return encodeFrame({ type: FRAME.CLASS_OVERRIDE, unitId, class: cls < 0 ? 255 : cls });
}

function sendHoldOverride(unitId, hold) {
  return encodeFrame({ type: FRAME.HOLD_OVERRIDE, unitId, hold: hold < 0 ? 255 : hold });
}

function sendUseItem(unitId, slot, target, x, z) {
  return encodeFrame({ type: FRAME.USE_ITEM, unitId, slot, target, x: Math.round(x), y: Math.round(z) });
}

const fsm = createFSM(xstate);
const auth = new NostrAuth({ nostrTools: NostrTools });
auth.loadFromStorage() || auth.generateKey();
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
    tags: [["d", `oscraft-room:${namespace}:${room}`], ["t", ROOM_TAG], ["ns", namespace], ["room", room], ["players", String(Math.min(MAX_PLAYERS, hosting ? seating.size() + 1 : peers.size + 1))]],
    content: "",
  }).then((signed) => relayPool.publish(signed)).catch(() => {});
}

function showRooms() {
  const names = [...announcedRooms].map(([name, a]) => ({ name, players: a.players })).sort((a, b) => a.name.localeCompare(b.name));
  const key = names.map((n) => `${n.name}:${n.players}`).join(",");
  if (key === shownRooms) return;
  shownRooms = key;
  window.dispatchEvent(new CustomEvent("oscraft:rooms", { detail: { rooms: names } }));
}

function pruneRooms() {
  const nowS = Math.floor(Date.now() / 1000);
  for (const [name, entry] of announcedRooms) {
    if (nowS - entry.createdAt > ROOM_TTL_S) announcedRooms.delete(name);
  }
  showRooms();
}

relayPool.subscribe("oscraft-rooms", [{ kinds: [ROOM_KIND], "#t": [ROOM_TAG] }], (event) => {
  const tagValue = (name) => (event.tags.find((t) => t[0] === name) || [])[1];
  if (tagValue("ns") !== namespace || typeof tagValue("room") !== "string") return;
  if (Math.floor(Date.now() / 1000) - event.created_at > ROOM_TTL_S) return;
  announcedRooms.set(tagValue("room"), { createdAt: event.created_at, players: Number(tagValue("players")) || 1 });
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
  if (hosting) {
    const key = e.detail.peerPubkey;
    seating.join(key);
    autoSeat(key);
    encoder.keyframe();
    sendRoster(key);
    // A joining peer takes the host's pack as it stands, so its copy matches the bytes the swaps are checked against.
    for (const frame of packStateFrames(pack, packBlobs)) sendTo(key, encodeFrame(frame));
  } else {
    try { sendTo(e.detail.peerPubkey, encodeFrame({ type: FRAME.JOIN, lastTick: lastAppliedTick })); } catch { /* the host asks again on the next join */ }
  }
  reconnectAttempt = 0;
  setStatus(`peer-open ${String(e.detail.peerPubkey).slice(0, 8)}`);
});
session.addEventListener("peer-close", (e) => {
  peers.delete(e.detail.peerPubkey);
  if (hosting) seating.leave(e.detail.peerPubkey);
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
  const bytes = new Uint8Array(e.detail.data);
  try { frame = decodeFrame(bytes); } catch (err) {
    console.error(`[wire] frame decode failed type=${frameTypeName(bytes[0]) ?? bytes[0]} from ${String(e.detail.peerPubkey).slice(0, 8)}: ${err.message}`);
    return;
  }
  if (frame.type === FRAME.ACTION) {
    if (hosting) applyPeerAction(e.detail.peerPubkey, frame);
    return;
  }
  if (frame.type === FRAME.UI) {
    if (!hosting) routeUi(JSON.parse(frame.body));
    return;
  }
  if (frame.type === FRAME.ORDER || frame.type === FRAME.CAST || frame.type === FRAME.ATTACK || frame.type === FRAME.HOLD || frame.type === FRAME.USE_ITEM || frame.type === FRAME.ATTACK_MOVE || frame.type === FRAME.CLASS_OVERRIDE || frame.type === FRAME.HOLD_OVERRIDE) {
    if (hosting) applyPeerCommand(e.detail.peerPubkey, frame);
    return;
  }
  if (frame.type === FRAME.JOIN) {
    if (hosting) {
      seating.join(e.detail.peerPubkey);
      encoder.keyframe();
      sendRoster(e.detail.peerPubkey);
    }
    return;
  }
  if (frame.type === FRAME.PACK_FILE || frame.type === FRAME.PACK_WANT) {
    handlePackFrame(e.detail.peerPubkey, frame);
    return;
  }
  if (frame.type === FRAME.SWAP) {
    // A peer records the host's swap tick and fetches the new image now; once verified it reports SWAP_READY to the host.
    // The module reload itself is applied by the host's announced value, here or on reload.
    const refusal = swapRefusal(e.detail.peerPubkey, frame.file);
    if (refusal) { refuseSwap(refusal, e.detail.peerPubkey, frame.file); return; }
    if (!swapIsNew(lastApplied ? lastApplied.tick : null, frame.tick)) { refuseSwap(`tick ${frame.tick} is not after the applied tick ${lastApplied.tick}`, e.detail.peerPubkey, frame.file); return; }
    const kind = packKindOf(frame.file);
    const swap = { tick: frame.tick, hash: frame.hash, file: frame.file, kind, url: new URL(`./${frame.file}`, import.meta.url).href, value: null, image: null, error: null, ready: null, deferrals: 0 };
    pendingSwap = swap;
    prefetchSwapImage(swap, kind === null ? fetchMapImage : () => packSwapBytes(kind));
    return;
  }
  if (frame.type === FRAME.SWAP_READY) {
    if (hosting) noteSwapReady(frame, e.detail.peerPubkey);
    return;
  }
  if (frame.type === FRAME.SWAP_NOT_READY) {
    // A peer did not have the image at the swap tick, so the swap failed there; the host reports it and does not re-announce.
    if (hosting) failSwapNotReady(frame, e.detail.peerPubkey);
    return;
  }
  if (frame.type === FRAME.SWAP_CANCEL) {
    // The host could not commit the swap: a peer drops the pending swap with this tick and hash.
    if (!hosting && e.detail.peerPubkey === hostKey && pendingSwap && pendingSwap.tick === frame.tick && pendingSwap.hash === frame.hash) {
      console.log(`[swap] ${pendingSwap.file}: cancelled by the host at tick ${frame.tick}`);
      pendingSwap = null;
    }
    return;
  }
  if (frame.type === FRAME.START) {
    const sender = e.detail.peerPubkey;
    if (hosting && !frame.seats.some((s) => s.owner === sender)) return;
    if (hosting) {
      applyStart(frame);
      for (const key of peers) if (key !== sender) sendTo(key, encodeFrame(frame));
    } else {
      seatsExplicit = true;
    }
    endLobby();
    return;
  }
  if (frame.type === FRAME.ROSTER) {
    if (hosting) return;
    hostKey = e.detail.peerPubkey;
    rosterView = frame;
    if (frame.you !== SPECTATOR) myPlayer = frame.you;
    document.body.dataset.you = String(frame.you);
    document.body.dataset.players = String(frame.players.length);
    return;
  }
  if (hosting) return;
  if (frame.type !== FRAME.STATE && frame.type !== FRAME.DELTA) return;
  if (frame.tick <= netTick) return;
  if (!applyFrame(netUnits, frame)) return;
  netTick = frame.tick;
  lastAppliedTick = frame.tick;
  netState = { tick: frame.tick, units: [...netUnits.values()].map((u) => ({ id: u.id, x: u.x / WIRE_SCALE, z: u.y / WIRE_SCALE, player: u.player, hp: u.hp, type: u.kind })) };
  advanceTickClock(netState.units);
  applyDueSwap();
  if (window.__ciRecv) recordTick(window.__ciRecv, frame.tick, [...netUnits.values()]);
});

// CI witness of wireweave agreement: each side keeps the unit positions it sent or applied for the last host ticks.
const CI_TICK_LOG = 300;
function recordTick(log, tick, units) {
  log.set(tick, units.map((u) => `${u.id}:${u.x}:${u.y}`).sort().join(","));
  if (log.size > CI_TICK_LOG) log.delete(log.keys().next().value);
}

// Accepted commands are published when they are counted, not on the next animation frame: a CI frame can take longer than the
// probe's wait for the witness, so a frame-only publish reads an accepted command as dropped.
function publishCommandCounts() {
  document.body.dataset.orders = String(orders);
  document.body.dataset.casts = String(casts);
  document.body.dataset.attacks = String(attacks);
}

// Page state for headless checks. The HUD (gold, winner, health bars, minimap) refreshes from the same frame.
function publishWitness() {
  document.body.dataset.units = String(currentUnits().length);
  document.body.dataset.ticks = String(localGame ? localGame.ticks() : netState ? netState.tick : 0);
  document.body.dataset.mode = "3d";
  document.body.dataset.center = renderer3d.camera.target.map((v) => v.toFixed(2)).join(",");
  document.body.dataset.distance = renderer3d.camera.distance.toFixed(3);
  document.body.dataset.drawTerrain = String(drawCounts.terrain);
  // No quad placeholder exists any more: units with no model are not drawn, so the quad count is always 0.
  document.body.dataset.drawQuad = "0";
  document.body.dataset.drawSplat = String(drawCounts.splat);
  document.body.dataset.splatGround = splat ? "1" : "0";
  document.body.dataset.shadow = SHADOW_ON && renderer3d && renderer3d.setShadows ? "1" : "0";
  document.body.dataset.shadowUnits = String(shadowUnitCount);
  document.body.dataset.texelUploads = String(renderer3d.stats.texelUploads || 0);
  document.body.dataset.terrainMode = "heightfield";
  document.body.dataset.drawDoodad = String(drawCounts.doodad);
  document.body.dataset.drawUnit = String(drawCounts.unit);
  document.body.dataset.drawAnimated = String(drawCounts.animated);
  document.body.dataset.drawCalls = String(renderer3d ? renderer3d.stats.drawCalls : 0);
  document.body.dataset.drawInstances = String(renderer3d ? renderer3d.stats.instances : 0);
  document.body.dataset.drawCulled = String(renderer3d ? renderer3d.stats.culled : 0);
  document.body.dataset.drawCpuMs = renderer3d ? renderer3d.stats.frameCpuMs.toFixed(3) : "0";
  publishCommandCounts();
  pumpUi();
  if (hud) hud.update();
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

// Deterministic per-placement variety: a hash of the placement's own position (and a salt per property) gives a value in
// [0, 1), so the map looks the same on every run and every placement differs from its neighbours.
function unitHash(n) {
  let h = n | 0;
  h = Math.imul(h ^ (h >>> 16), 0x45d9f3b);
  h = Math.imul(h ^ (h >>> 16), 0x45d9f3b);
  return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
}
function placementHash(x, y, salt) {
  return unitHash(Math.imul(x | 0, 0x9e3779b1) ^ Math.imul(y | 0, 0x85ebca77) ^ Math.imul(salt, 0xc2b2ae3d));
}
// Tints multiply a doodad's colour (its texture, or its fallback colour when the model has no texture); index 0 is the plain model.
const DOODAD_TINTS = [[1, 1, 1], [0.88, 0.92, 0.98], [1.08, 1.02, 0.9], [0.94, 1.05, 0.96]];
const doodadVariants = new Map();
function doodadVariant(base, kind, v) {
  if (v === 0) return base;
  const key = kind + ":" + v;
  if (!doodadVariants.has(key)) doodadVariants.set(key, { ...base, tint: DOODAD_TINTS[v] });
  return doodadVariants.get(key);
}

// Builds the 3D world from the staged assets: the terrain mesh from map.bin, doodads at their placements with
// their GLB models, and unit models from the sim's model names. Each unit type keeps a model, or null when it has none.
async function build3d(gl, game) {
  const terrain = terrainInTiles(buildTerrain(mapBytes));
  terrainTiles = terrain;
  terrainMesh = terrainMeshOf(terrain);
  waterMesh = waterMeshOf(mapBytes);
  // Splat layers, doodad and unit models are independent downloads: they start together, so the wait is the slowest
  // chain rather than the sum of the stages.
  const placements = readDoodads(mapBytes);
  const usedKinds = new Set(placements.map((p) => p.kind));
  const splatReady = buildSplat(gl);
  const doodadModelsReady = loadDoodadModels(usedKinds);
  const unitFitsReady = loadModels(game);
  // The splat ground is the default floor: a layer that does not build stops the start (buildSplat throws).
  splat = await splatReady;
  const doodadByKind = await Promise.all((await doodadModelsReady).map(async ({ high, low }) => ({
    base: high ? await modelOrRecord(high.glbUrl, toMesh(gl, high, DOODAD_TINT)) : null,
    lowBase: low ? await modelOrRecord(low.glbUrl, toMesh(gl, low, DOODAD_TINT)) : null,
  })));
  const s = 1 / terrain.cellMilli;
  const placed = placements.flatMap((p) => {
    const kind = doodadByKind[p.kind];
    if (!kind?.base) return [];
    const x = p.x * s;
    const z = p.y * s;
    const angle = (p.rotation / 1000) * 2 * Math.PI + placementHash(p.x, p.y, 1) * 2 * Math.PI;
    const k = 0.85 + 0.3 * placementHash(p.x, p.y, 2);
    const variant = Math.floor(placementHash(p.x, p.y, 3) * DOODAD_TINTS.length);
    const mesh = doodadVariant(kind.base, p.kind, variant);
    const lowMesh = kind.lowBase ? doodadVariant(kind.lowBase, `${p.kind}-low`, variant) : null;
    return [{ mesh, lowMesh, transform: multiply(multiply(translation(x, groundY(terrain, x, z), z), rotationY(angle)), scaling(k, k, k)) }];
  });
  doodadBatches = batchByMesh(placed);
  if (renderer3d.setShadows) renderer3d.setShadows({ enabled: SHADOW_ON, size: SHADOW_SIZE, center: [terrain.width / 2, 0, terrain.height / 2], halfExtent: Math.max(terrain.width, terrain.height) / 2 + 4 });
  // A controlled unit's mesh failure stops the start; any other unit's failure is recorded and that type is not drawn.
  const controlledFlags = typeof game.unit_flags === "function" ? game.unit_flags() : [];
  unitMeshes = await Promise.all((await unitFitsReady).map((f, type) => {
    if (!f) return null;
    const mesh = toMesh(gl, f, [1, 1, 1], { animated: true });
    return controlledFlags[type] === 1 ? mesh : modelOrRecord(f.glbUrl, mesh);
  }));
  arrowMesh = await loadArrowMesh(gl);
}

// Splat ground: the control map from the grid and each layer's albedo and height textures, in SPLAT_LAYERS order.
// A layer texture that does not load throws, naming the layer: the splat ground is required.
async function buildSplat(gl) {
  const layers = await Promise.all(SPLAT_LAYERS.map(async (name) => {
    const [albedo, height] = await Promise.all(SPLAT_TEXTURES[name].map(async (key) => {
      const uri = assetUrl(key);
      const image = await loadBaseColorImage({ uri }, location.href);
      if (!image) throw new Error(`splat layer ${name} texture ${uri} did not load`);
      return image;
    }));
    return { albedo: uploadTexture(gl, albedo), height: uploadTexture(gl, height) };
  }));
  const maps = splatMaps(mapBytes);
  return {
    ...maps, layers, uvScale: GROUND_UV_PER_CELL, band: SPLAT_BAND, shade: SPLAT_SHADE, tints: SPLAT_TINTS,
    waterTint: SPLAT_WATER_TINT, gpu: null,
  };
}

// The terrain mesh and the water quads, as the splat pass draws them. Shared by the first build and a hot reload.
function terrainMeshOf(terrain) {
  return { positions: terrain.positions, normals: terrain.normals, uvs: planarGroundUvs(terrain.positions), indices: terrain.indices, color: MAP_COLOR };
}
function waterMeshOf(bytes, surface = waterSurface) {
  const water = surface(bytes);
  return water.indices.length ? { ...water, color: MAP_COLOR } : null;
}
// The splat's control map and water map (the textures the splat pass samples), from the map bytes. A hot reload passes
// the fresh tiles module as `tiles`.
function splatMaps(bytes, tiles = { controlMap, waterMap }) {
  const control = tiles.controlMap(bytes);
  return { control, width: control.width, height: control.height, waterMap: tiles.waterMap(bytes) };
}

// Hot reload (hmr.js dispatches oscraft:hmr with the module url and its hook's fresh value). Each branch rebuilds only
// the GPU data that module feeds; with no hook firing, nothing runs and the default path is unchanged.
window.addEventListener("oscraft:hmr", (e) => {
  const { url, value } = e.detail || {};
  if (!value) return;
  const file = url.slice(url.lastIndexOf("/") + 1);
  if (hosting && localGame) {
    announceSwap(file, url, value);
    return;
  }
  hmrApply(file, url, value).catch((err) => console.error(`[hmr] ${file}: rebuild failed: ${err.message}`));
});
// True while the host fetches the new image for an announcement; a second hot reload is refused until it is sent.
let swapFetching = false;
// The last data swap this page applied or failed, for window.oscraft.lastSwap() and getState().
let lastSwapResult = null;

// The page's map image as the sim loads it, fetched from the same origin path with no cache, so a rebuilt image is read.
async function fetchMapImage() {
  const res = await fetch(mapUrl, { cache: "no-store" });
  if (!res.ok) throw new Error(`${mapUrl} HTTP ${res.status}`);
  return new Uint8Array(await res.arrayBuffer());
}

// 32-bit FNV-1a of the bytes: the image's identity in a SWAP frame.
function fnv1a32(bytes) {
  let h = 0x811c9dc5;
  for (let i = 0; i < bytes.length; i += 1) h = Math.imul(h ^ bytes[i], 0x01000193) >>> 0;
  return h >>> 0;
}

// Host half of a data swap: fetches the new image and hashes it, then announces the swap tick, so every peer swaps at
// the tick the host does. A failed fetch announces nothing and the match keeps its data.
async function announceSwap(file, url, value) {
  if (pendingSwap || swapFetching) { refuseSwap(`swap already pending for ${pendingSwap ? pendingSwap.file : file} at tick ${pendingSwap ? pendingSwap.tick : "?"}`, null, file, "[hmr]"); return; }
  swapFetching = true;
  try {
    const bytes = await fetchMapImage();
    const hash = fnv1a32(bytes);
    const tick = localGame.ticks() + SWAP_LEAD_TICKS;
    // Broadcast first: the host only keeps the swap if its peers were sent the announcement.
    session.broadcast(encodeFrame({ type: FRAME.SWAP, tick, hash, file }));
    pendingSwap = { tick, hash, file, url, value, image: bytes, error: null, ready: Promise.resolve(bytes), deferrals: 0, readyPeers: new Set(), decided: false };
    console.log(`[hmr] ${file}: swap at tick ${tick} (image hash ${hash.toString(16)})`);
  } catch (err) {
    console.error(`[hmr] ${file}: swap not announced, not applied: ${err.message}`);
    lastSwapResult = { tick: localGame ? localGame.ticks() : 0, file, ok: false, carried: null, refused: null, error: err.message, readiness: "failed", deferred: false };
  } finally {
    swapFetching = false;
  }
}

// Applies the data half of a swap on this peer at its tick: the Game takes the new image and keeps the live units.
// A missing or wrong image, or a refused load, is logged and the old Game stays in service. Called once per swap tick
// (applyDueSwap guards the tick), so the Game's swap_data never runs twice for one swap.
function applyDataSwap(due) {
  const tick = swapClock();
  const fail = (error) => {
    lastSwapResult = { tick, file: due.file, ok: false, carried: null, refused: null, error, readiness: due.error ? "failed" : "in-flight", deferred: false, deferrals: due.deferrals };
    console.error(`[swap] ${due.file} at tick ${tick}: data swap failed, previous data kept: ${error}`);
  };
  if (due.error) return fail(due.error);
  if (!due.image) return fail("image not loaded on this peer by the swap tick");
  try {
    const [carried, refused] = localGame.swap_data(due.image);
    lastSwapResult = { tick, file: due.file, ok: true, carried, refused, error: null, readiness: "ready", deferred: false, deferrals: due.deferrals };
    console.log(`[swap] ${due.file} at tick ${tick}: carried=${carried} refused=${refused}`);
  } catch (err) {
    fail(err && err.message ? err.message : String(err));
  }
}

// A swap name is a plain module file name: letters, digits, _ . - and a .js or .mjs ending, with no slash, query or "..".
const SWAP_FILE = /^[A-Za-z0-9_.-]+\.(m?js|lua|osd|osdt|json|tsv|txt)$/;

// Why a SWAP announcement must be ignored, or null when it may be applied: only the host announces one, and the name must be a plain module file.
function swapRefusal(sender, file) {
  if (hosting) return "this page is the host";
  if (hostKey === null || sender !== hostKey) return `sender ${String(sender).slice(0, 8)} is not the host`;
  if (typeof file !== "string" || !SWAP_FILE.test(file) || file.includes("..")) return `file name ${JSON.stringify(file)} is not a plain module, Lua or data file name`;
  return null;
}

// Logs a refused swap to the console and keeps it on window.oscraft (swapRejections()).
function refuseSwap(reason, sender, file, tag = "[swap]") {
  const entry = { reason, sender: sender === null ? null : String(sender).slice(0, 8), file: String(file).slice(0, 80), at: Date.now() };
  swapRejections.push(entry);
  if (swapRejections.length > 20) swapRejections.shift();
  console.warn(`${tag} swap refused: ${reason} (file=${JSON.stringify(entry.file)}${entry.sender ? ` sender=${entry.sender}` : ""})`);
}

// A peer's half of a swap: re-import the module under a fresh query and dispatch its hook's value, as hmr.js does for
// a changed file. A module with no hook, or a failed import, is logged and the previous code stays in service.
async function peerReloadModule(file, url) {
  const bumped = new URL(url);
  bumped.search = `?v=${Date.now()}`;
  let mod;
  try {
    mod = await import(bumped.href);
  } catch (err) {
    console.error(`[hmr] ${file} failed to re-import on this peer: ${err.message}`);
    return;
  }
  if (typeof mod.hmrReload !== "function") {
    console.log(`[hmr] ${file}: swap applied on this peer (no hook; a page reload applies it)`);
    return;
  }
  const value = mod.hmrReload(mod);
  window.dispatchEvent(new CustomEvent("oscraft:hmr", { detail: { url, value } }));
}

// A peer's verified fetch of a swap image: the record's image is set once the bytes match the announced hash, its error otherwise.
function prefetchSwapImage(swap, source = fetchMapImage) {
  if (!peerFetch || peerFetch.hash !== swap.hash) {
    const promise = source().then((bytes) => {
      const got = fnv1a32(bytes);
      if (got !== swap.hash) throw new Error(`image hash ${got.toString(16)} is not the host's ${swap.hash.toString(16)}`);
      return bytes;
    });
    peerFetch = { hash: swap.hash, promise };
  }
  swap.ready = peerFetch.promise;
  swap.ready.then((bytes) => {
    swap.image = bytes;
    // Verified: report readiness for the swap as it now stands (a re-announce replaces this record and reports again).
    if (pendingSwap !== swap || hostKey === null) return;
    try { sendTo(hostKey, encodeFrame({ type: FRAME.SWAP_READY, tick: swap.tick, hash: swap.hash })); } catch (err) { console.warn(`[swap] ${swap.file}: ready report not sent: ${err.message}`); }
  }, (err) => { swap.error = err.message; });
}

// Host: a peer reported that it holds the verified image for the pending swap. Counted at the gate (hostSwapGate).
function noteSwapReady(report, sender) {
  const p = pendingSwap;
  if (!p || !peers.has(sender) || report.tick !== p.tick || report.hash !== p.hash) return;
  p.readyPeers.add(sender);
}

// Host: decides the pending swap at its gate tick, SWAP_GATE_TICKS before it (see swap_protocol.js). It commits the swap
// once every connected peer is ready, re-announces it SWAP_DEFER_TICKS later while it may still move, and cancels it
// after SWAP_MAX_DEFERS (swap_protocol.js) moves. Called once per sim tick, before applyDueSwap.
function hostSwapGate() {
  const p = pendingSwap;
  if (!hosting || !p || p.decided || !localGame) return;
  const action = swapGate(localGame.ticks(), p, peers);
  if (action === "wait") return;
  if (action === "commit") { p.decided = true; return; }
  if (action === "defer") {
    const tick = p.tick + SWAP_DEFER_TICKS;
    pendingSwap = { ...p, tick, deferrals: p.deferrals + 1, error: null, readyPeers: new Set(), decided: false };
    session.broadcast(encodeFrame({ type: FRAME.SWAP, tick, hash: p.hash, file: p.file }));
    lastSwapResult = { tick: localGame.ticks(), file: p.file, ok: false, carried: null, refused: null, error: "a peer was not ready at the gate; re-announced", readiness: "in-flight", deferred: true, deferrals: pendingSwap.deferrals };
    console.log(`[swap] ${p.file}: re-announced from tick ${p.tick} to ${tick} (a peer not ready by tick ${p.tick - SWAP_GATE_TICKS})`);
    return;
  }
  // cancel: no peer applies it, and the host keeps its data.
  pendingSwap = null;
  session.broadcast(encodeFrame({ type: FRAME.SWAP_CANCEL, tick: p.tick, hash: p.hash }));
  lastSwapResult = { tick: localGame.ticks(), file: p.file, ok: false, carried: null, refused: null, error: `a peer was still not ready after ${p.deferrals} re-announces`, readiness: "failed", deferred: true, deferrals: p.deferrals };
  console.warn(`[swap] ${p.file}: cancelled at tick ${p.tick}, a peer was never ready (${p.deferrals} re-announces)`);
}

// Host: a peer had no verified image at the swap tick, which the gate should have prevented. The swap fails for everyone:
// a pending swap is cancelled before its tick, an applied one is only reported. No re-announce.
function failSwapNotReady(report, sender) {
  const short = String(sender).slice(0, 8);
  const p = pendingSwap && pendingSwap.tick === report.tick && pendingSwap.hash === report.hash ? pendingSwap : null;
  const applied = lastApplied && lastApplied.tick === report.tick && lastApplied.hash === report.hash ? lastApplied : null;
  if (!p && !applied) return;
  const file = p ? p.file : applied.file;
  if (p) {
    pendingSwap = null;
    session.broadcast(encodeFrame({ type: FRAME.SWAP_CANCEL, tick: report.tick, hash: report.hash }));
  }
  lastSwapResult = { tick: report.tick, file, ok: false, carried: null, refused: null, error: `peer ${short} was not ready at the swap tick${applied ? " (this host had already applied it)" : ""}`, readiness: "failed", deferred: false, deferrals: p ? p.deferrals : 0 };
  console.error(`[swap] ${file} at tick ${report.tick}: peer ${short} was not ready; swap failed`);
}

// The tick a pending swap is due against: a guest's sim is the host's frame stream (netTick), a host or local page runs its own.
function swapClock() {
  if (netState) return netTick;
  return localGame ? localGame.ticks() : -1;
}

// Applies a pending swap once the sim reaches its tick. A peer without the verified image keeps its old data, logs it and
// reports SWAP_NOT_READY. A swap tick this page already applied is never applied again.
function applyDueSwap() {
  if (!pendingSwap || !localGame || swapClock() < pendingSwap.tick) return;
  const due = pendingSwap;
  pendingSwap = null;
  if (lastApplied && !swapIsNew(lastApplied.tick, due.tick)) {
    console.warn(`[swap] ${due.file} at tick ${due.tick}: already applied at tick ${lastApplied.tick}, ignored`);
    return;
  }
  if (!due.image) {
    // Not ready at the tick (the host's gate should have prevented it): keep the old Game, log it, report it to the host.
    lastSwapResult = { tick: localGame.ticks(), file: due.file, ok: false, carried: null, refused: null, error: due.error || "image not ready by the swap tick", readiness: due.error ? "failed" : "in-flight", deferred: false, deferrals: due.deferrals };
    console.warn(`[swap] ${due.file} at tick ${due.tick}: image not ready, swap not applied here; reported to the host`);
    if (!hosting && hostKey !== null) {
      try { sendTo(hostKey, encodeFrame({ type: FRAME.SWAP_NOT_READY, tick: due.tick, hash: due.hash })); } catch (err) { console.warn(`[swap] not-ready report not sent: ${err.message}`); }
    }
    return;
  }
  lastApplied = { tick: due.tick, hash: due.hash, file: due.file };
  if (due.kind === "lua") return applyLuaSwap(due);
  applyDataSwap(due);
  if (due.kind === "data") return;
  if (!due.value) {
    // A peer re-imports the named module at the swap tick, as the host did, and runs its hook.
    peerReloadModule(due.file, due.url).catch((err) => console.error(`[hmr] ${due.file}: peer reload failed: ${err.message}`));
    return;
  }
  hmrApply(due.file, due.url, due.value).catch((err) => console.error(`[hmr] ${due.file}: swap failed: ${err.message}`));
}

// A Lua swap at its tick: the pack's mod set loads again on this peer (load_mods). A set that fails to build keeps the
// last good set, and the error is logged and reported as the swap's result.
function applyLuaSwap(due) {
  const tick = localGame.ticks();
  const error = localGame.load_mods(new TextDecoder().decode(due.image)) || "";
  logModError(error);
  lastSwapResult = { tick, file: due.file, ok: !error, carried: null, refused: null, error: error || null, readiness: error ? "failed" : "ready", deferred: false, deferrals: due.deferrals };
  console.log(`[swap] ${due.file} at tick ${tick}: ${error ? `mods kept, reload failed: ${error}` : "mods reloaded"}`);
}

async function hmrApply(file, url, value) {
  if (file === "terrain.js") return hmrTerrain(value);
  if (file === "tiles.js") return hmrTiles(url);
  if (file === "anim.js") return hmrAnim();
  if (file === "hud.js") {
    hud = value;
    return;
  }
}

// terrain.js: the value is the rebuilt terrain (map units). The mesh and the splat control map are rebuilt from it.
function hmrTerrain(raw) {
  const terrain = terrainInTiles(raw);
  terrainTiles = terrain;
  terrainMesh = terrainMeshOf(terrain);
  if (splat) Object.assign(splat, splatMaps(mapBytes), { gpu: null });
}

// tiles.js: the bake's value is not held by play.js (no tile batches here); the splat maps and the water surface are
// rebuilt from the fresh module, which the hook has already re-evaluated under a new query.
async function hmrTiles(url) {
  const fresh = await import(`${url}?v=${Date.now()}`);
  waterMesh = waterMeshOf(mapBytes, fresh.waterSurface);
  if (splat) Object.assign(splat, splatMaps(mapBytes, fresh), { gpu: null });
}

// anim.js: re-runs the retarget for every library-clipped rig, then points the unit meshes at the new clips and drops
// the players that read the old ones (they are recreated on the next frame).
async function hmrAnim() {
  if (!localGame) return;
  const clipsByName = new Map();
  for (const name of libraryRigs) {
    const parsed = await parsedModels.get(name);
    parsed.animations.clips = await libraryClips(name, parsed.animations.nodes);
    clipsByName.set(name, parsed.animations.clips);
  }
  const names = localGame.unit_model_names();
  unitMeshes.forEach((mesh, type) => {
    const clips = clipsByName.get(names[type]);
    if (mesh && mesh.animation && clips) mesh.animation.clips = clips;
  });
  animators.clear();
}

const WORLD_STALL_MS = 15000;

async function main() {
  resize();
  bindInput();
  const gl = canvas.getContext("webgl2", { antialias: true });
  if (!gl) throw new Error("WebGL2 is required: this browser gave no WebGL2 context for the game canvas");
  localGame = await loadSim();
  renderer3d = createRenderer3d(gl);
  renderer3d.setCamera({ target: orbitOrigin, distance: cameraDistance, maxDistance: cameraMaxDistance, far: cameraMaxDistance * CAMERA_FAR_PER_MAX, bounds: cameraBounds, pitch: 0.9 });
  // A download that never settles would otherwise leave the status on its initial "connecting" text.
  setStatus("loading world assets");
  const stalled = setTimeout(() => setStatus(`loading world assets: still pending after ${WORLD_STALL_MS / 1000} s, check the network`), WORLD_STALL_MS);
  try {
    await build3d(gl, localGame);
  } finally {
    clearTimeout(stalled);
  }
  hud = createHud({
    game: localGame,
    minimapEl: document.getElementById("minimap"),
    barCanvas: createBarCanvas(),
    mapSize: { w: W, h: H },
    grid: mapGrid,
    units: shownUnits,
    selected: () => selectedId,
    localPlayer: localPlayerId,
    send: sendInput,
    toWorld: screenToWorld,
    toScreen: worldToScreen,
    playerColor: (player) => PLAYER_COLORS[player] || PLAYER_COLORS[0],
    isControlled: isControlledUnit,
  });
  if (renderer3d) fx = createFx({ renderer: renderer3d, game: () => localGame, toScreen: worldToScreen, groundAt, playerColor: (player) => PLAYER_COLORS[player] || PLAYER_COLORS[0], selected: () => selectedId, arrow: () => arrowMesh });
  widgets = createWidgets({ send: (call, args) => sendInput("ui", { call, args }) });
  const eventsBase = params.get("events");
  const modUi = createModUi({
    root: document.body,
    renderer: () => renderer3d,
    localPlayer: localPlayerId,
    defaultDistance: () => cameraDistance,
    baseUrl: eventsBase === null ? undefined : eventsBase.replace(/\/$/, ""),
    onBind: (bind) => sendInput("bind", bind),
  });
  window.__modUi = modUi;
  fogview = createFogView({ game: localGame, localPlayer: localPlayerId, toClip: worldClip, groundAt });
  window.__fogView = fogview;
  const packButton = document.createElement("button");
  packButton.type = "button";
  packButton.id = "pack-open";
  packButton.textContent = "pack";
  packButton.addEventListener("click", openPackEditor);
  document.getElementById("ui-left").append(packButton);
  if (params.get("host") === "1") startHost();
  joinRoom();

  function frame(now) {
    const dtS = lastFrameMs ? Math.min(0.1, (now - lastFrameMs) / 1000) : 0;
    lastFrameMs = now;
    panByKeys(dtS);
    modUi.step(dtS);
    if (pendingStartCentre !== null && renderer3d) {
      const own = simUnits().find((u) => u.id === pendingStartCentre.id);
      if (own) {
        renderer3d.setCamera({ target: [own.x, groundAt(own.x, own.z), own.z], distance: Math.min(pendingStartCentre.distance, cameraMaxDistance) });
        pendingStartCentre = null;
      }
    }
    if (!netState && localGame && !hosting) {
      localTickAcc += dtS;
      while (localTickAcc >= SIM_TICK_S) {
        logModError(localGame.tick());
        applyDueSwap();
        advanceTickClock(simUnits());
        localTickAcc -= SIM_TICK_S;
      }
    }
    drawWorld3d();
    fogview.draw();
    publishWitness();
    requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);
  setStatus("local preview");
  document.body.dataset.ready = "1";
}

// Live debug API (window.oscraft) on every load, with no per-frame work; see debug_api.js for each call's effect.
// Shadows are tracked here because the boot-time ?shadows switch seeds them and publishWitness keeps the page's shadow mark.
let shadowsEnabled = SHADOW_ON;
installDebugApi({
  renderer: () => renderer3d,
  rendererKind: () => "webgl",
  game: () => localGame,
  localPlayer: () => myPlayer,
  netTick: () => (netState ? netState.tick : 0),
  units: currentUnits,
  lastSwap: () => (lastSwapResult ? { ...lastSwapResult } : null),
  debugActions: () => {
    if (!localGame || netState) return null;
    logModError(localGame.dispatch_action("debug_actions", "null"));
    pumpUi();
    return debugReply && Array.isArray(debugReply.actions) ? [...debugReply.actions] : null;
  },
  effects: () => (fx ? fx.recentEffects() : null),
  modErrors: () => modErrorLog.map((e) => ({ ...e })),
  swap: () => ({
    applied: lastSwapResult ? { ...lastSwapResult } : null,
    pending: pendingSwap ? { tick: pendingSwap.tick, file: pendingSwap.file, hash: pendingSwap.hash, deferrals: pendingSwap.deferrals, ready: Boolean(pendingSwap.image) } : null,
    rejections: swapRejections.slice(-10),
  }),
  run: (name, args = null) => {
    if (!localGame && !netState) return false;
    sendInput(name, args);
    return true;
  },
  isControlled: (id) => localGame !== null && isControlledUnit(id),
  selected: () => (hud ? hud.input.selection() : selectedId === -1 ? [] : [selectedId]),
  select: (ids) => { if (hud) hud.input.setSelection(ids); else selectedId = ids[0] ?? -1; },
  pick: (world) => pick(world),
  order: (id, x, z) => commands.order(id, x, z),
  attack: (id, targetId) => commands.attack(id, targetId),
  overrides: (id) => (localGame ? { classOverride: localGame.class_override(id), holdOverride: localGame.hold_override(id) } : null),
  setClassOverride: (id, cls) => commands.classOverride(id, cls),
  setAttackHold: (id, hold) => commands.holdOverride(id, hold),
  drawCounts: () => ({ ...drawCounts }),
  terrainMode: () => "heightfield",
  shadowsOn: () => shadowsEnabled,
  setShadows: (on) => {
    if (!renderer3d || !renderer3d.setShadows) return false;
    renderer3d.setShadows({ enabled: on, size: SHADOW_SIZE, center: [W / 2, 0, H / 2], halfExtent: Math.max(W, H) / 2 + 4 });
    shadowsEnabled = on;
    return true;
  },
  fogOn: () => (fogview ? fogview.enabled() : false),
  setFog: (on) => {
    if (!fogview) return false;
    fogview.setEnabled(on);
    return true;
  },
});

if (params.get("ci") === "1") {
  window.__ciPeers = () => peers.size;
  window.__ciFrames = [];
  window.__ciUnits = () => currentUnits().map((u) => ({ id: u.id, x: u.x, z: u.z, player: u.player, type: u.type, modeled: !!unitMeshes[u.type] }));
  // Read-only: every living unit of the local sim, fogged or not, for probes that walk toward a foe until it is in sight.
  window.__ciRoster = () => simUnits().map((u) => ({ id: u.id, x: u.x, z: u.z, player: u.player, type: u.type }));
  window.__ciOrder = (unitId, x, z) => session.broadcast(sendOrder(unitId, x, z));
  // Local order through the sim's own command path (the local match has no peers to broadcast to).
  window.__ciMove = (unitId, x, z) => commands.order(unitId, x, z);
  window.__ciAttack = (unitId, targetId) => commands.attack(unitId, targetId);
  window.__ciCast = (unitId, slot, target, x, z) => session.broadcast(sendCast(unitId, slot, target, x, z));
  window.__ciLearn = (unitId, slot) => (localGame ? localGame.skill_up(unitId, slot) : false);
  // The first ability slot of a unit whose cast waits for a click (target unit or point), or -1: the probe learns and casts that one.
  window.__ciItemIndex = (name) => JSON.parse(localGame.item_catalog_json()).findIndex((e) => e.name === name);
  window.__ciGiveItem = (unitId, index, count) => localGame.give_item(unitId, index, count);
  window.__ciInventory = (unitId) => JSON.parse(localGame.inventory_json(unitId)).slots;
  window.__ciTargetedSlot =(unitId) => { const list = JSON.parse(localGame.abilities_json(unitId)); const t = list.find((a) => (a.target === "unit" || a.target === "point") && a.slot < 4); return t ? t.slot : -1; };
  window.__ciHostile = (a, b) => localGame.hostile(a, b);
  window.__ciIsControlled = (unitId) => isControlledUnit(unitId);
  window.__ciAttack = (unitId, targetId) => session.broadcast(sendAttack(unitId, targetId));
  // Local cast for the probe: the sim cast and the effect the page draws for it, without a network session.
  window.__ciLocalCast = (unitId, slot, target, x, z) => localGame.cast(unitId, slot, target, x, z);
  window.__ciLocalAttack = (unitId, targetId) => localGame.attack(unitId, targetId);
  window.__ciFxCounts = () => (fx ? fx.counts() : null);
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
  // Selects through the input layer, the same path a left click takes, so a right click orders this unit.
  window.__ciSelect = (unitId) => { if (hud) hud.input.setSelection([unitId]); else selectedId = unitId; };
  window.__ciInputSelection = () => (hud ? hud.input.selection() : []);
  window.__ciLocalPlayer = () => myPlayer;
  // Moves the 3D camera target onto a ground point, so a probe can act on units that the default view does not show.
  window.__ciFocus = (x, z) => { if (renderer3d) renderer3d.setCamera({ target: [x, groundAt(x, z), z] }); };
  window.__ciToScreen = (x, z) => worldToScreen(x, z, groundAt(x, z));
  // Every unit of the side, fogged or not: the sim's own roster, so a side is killed whole whatever the local vision.
  window.__ciKillSide = (player) => {
    for (const u of simUnits()) if (u.player === player) localGame.hurt(u.id, 10000);
  };
  // Read-only: per START seat, the picked kind name and the catalog name of the unit the sim actually spawned for it.
  window.__ciControlledNames = () => {
    const names = typeof localGame.unit_type_names === "function" ? localGame.unit_type_names() : [];
    return spawnedSeats.map(({ slot, pick }) => {
      // The seat's controlled unit is the unit of the picked kind; the first controlled unit of the slot may be a map-placed unit of another type.
      const kind = names.indexOf(pick);
      const u = simUnits().find((v) => v.player === slot && isControlledUnit(v.id) && v.type === kind) || null;
      return { slot, picked: pick, spawned: u ? names[u.type] ?? null : null };
    });
  };
  // Read-only: the first ramp cell's centre in tile units, and the effects' counts (ranged attack projectiles in flight).
  window.__ciRampCell = () => {
    const grid = readGrid(mapBytes);
    const [cell] = rampLowCells(grid);
    return cell === undefined ? null : { x: (cell % grid.width) + 0.5, z: Math.floor(cell / grid.width) + 0.5 };
  };
  window.__ciFx = () => (fx ? fx.counts() : null);
  // Debug API for probes and screenshots: move the 3D camera onto a ground point (cell units), or set its distance.
  // Merged into the debug API installed above, so getState and the other probes stay on window.oscraft.
  window.oscraft = Object.assign(window.oscraft || {}, {
    setCamera: (opts = {}) => (renderer3d ? renderer3d.setCamera(opts) : null),
    centreOn: (x, z) => (renderer3d ? renderer3d.setCamera({ target: [x, groundAt(x, z), z] }) : null),
    fx: () => (fx ? fx.counts() : null),
    swapRejections: () => swapRejections.slice(),
    lastSwap: () => (lastSwapResult ? { ...lastSwapResult } : null),
  });
  window.__ciDebug = () => [...session.peers.values()].map((p) => ({ ice: p.pc?.iceConnectionState, conn: p.pc?.connectionState, remoteMdns: (p.pc?.remoteDescription?.sdp || '').split('.local').length - 1 }));
}

main().catch((err) => {
  const message = err && err.message ? err.message : String(err);
  document.body.dataset.error = message;
  failStart(err);
});
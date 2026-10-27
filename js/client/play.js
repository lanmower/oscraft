import { createDataSession } from "wireweave/data.js";
import { createFSM } from "wireweave/fsm.js";
import { NostrAuth } from "wireweave/auth.js";
import { RelayPool } from "wireweave/relay-pool.js";
import * as NostrTools from "nostr-tools";
import * as xstate from "xstate";
import { createAmbientFx, AMBIENT_KINDS } from "./ambient_fx.js";
import { encodeFrame, decodeFrame, frameTypeName, FRAME, MAX_PLAYERS, SPECTATOR } from "./wire/frame.js";
import { iceServersFrom } from "./ice.js";
import { assetJson, assetRel, assetUrl, fetchRetrying, manifest, manifestOf, moduleClosure, modulePaths, packBytes, packJson, packText, packUrl, readPackFile, stashPack, UI } from "./pack_load.js";
import { applyFrame, createSeating, createStateEncoder, startFrame } from "./delta.js";
import { AnimationPlayer, cycleSpeed, jointMatrices, retargetLibrary, skinFloor } from "./anim.js";
import { parseGlb, parseGlbAnimations } from "./glb.js";
import { resolveGlbExtensions } from "./glb_resolve.js";
import { createHud } from "./hud.js";
import { createMinimap } from "./ui_minimap.js";
import { createWidgets, setImageSrcResolver, setPreviewRig } from "./widgets.js";
import { createCamera, createRenderer as createRenderer3d, eyeOf, MAX_JOINTS, meshReach, multiply, rotationY, scaling, translation, viewProjection } from "./render3d.js";
import { buildTerrain, GROUND_UV_PER_CELL, readProps, readGrid, vertexNormals } from "./terrain.js";
import { controlMap, fitModel, planarGroundUvs, rampLowCells, SPLAT_LAYERS, SPLAT_TEXTURES, setSplatTexel, TILE_TINTS, walkableBounds, waterMap, waterSurface } from "./tiles.js";
import { loadBaseColorImage, uploadTexture } from "./texture.js";
import { loadBasis } from "./decoders.js";

// The texture transcoder is on the splat layer's critical path (every KTX2 sibling
// texture waits on it); memoized, so warming it here while the page's other modules
// and the wasm image download trade off compile and network time pays for itself.
const basisWarm = loadBasis();
import { SWAP_DEFER_TICKS, SWAP_GATE_TICKS, SWAP_LEAD_TICKS, packSwapKind, swapGate, swapIsNew } from "./swap_protocol.js";
import { createFogView } from "./fogview.js";
import { packCameraRules } from "./camera_rules.js";
import { createZoomKeys } from "./zoom_keys.js";
import { createDraftClock } from "./draft_clock.js";
import { createSfx, createVoices } from "./audio.js";
import { installDebugApi } from "./debug_api.js";
import { createFx } from "./fx.js";
import { createChat } from "./chat.js";
import { createModUi, setMatchUnderWay } from "./modui.js";
import { Pack } from "./pack.js";
import { createPackEditor } from "./pack_editor.js";
import { changeFrames, chunkFrames, createPackReceiver, packStateFrames } from "./pack_sync.js";
import { createCover, createLoadGate, PHASE } from "./load_gate.js";
import { runWarmup } from "./warmup.js";

let W = 32;
let H = 24;
const CAMERA_DISTANCE_PER_TILE = 40 / 32;
const CAMERA_MAX_PER_TILE = 1.6;
const CAMERA_MAX_FLOOR = 120;
const CAMERA_PLAY_DISTANCE = 70;
const CAMERA_START_DISTANCE = 40;
let pendingStartCentre = false;
const CAMERA_FAR_PER_MAX = 2;
const BACKGROUND = [0.114, 0.145, 0.173];
const MAP_COLOR = [0.2, 0.28, 0.22];
const PLAYER_COLORS = [[0.227, 0.482, 0.835], [0.835, 0.329, 0.227]];
const SELECT_COLOR = [0.95, 0.85, 0.3];
const seatColours = new Map();
const playerColor = (player) => seatColours.get(player) || PLAYER_COLORS[player] || PLAYER_COLORS[0];
const rgbFloats = (rgb) => [((rgb >> 16) & 255) / 255, ((rgb >> 8) & 255) / 255, (rgb & 255) / 255];
const UNIT_HALF = 0.9;
let HERO_DRAW_SCALE = UI.heroDrawScale ?? 1;
const PROP_HALF = 0.5;
const PICK_RADIUS = 1.2;
const DRAG_SLOP_PX = 4;
const PAN_PX_PER_S = 500;
let orbitOrigin = [W / 2, 0, H / 2];
let cameraBounds = null;
let cameraYawLocked = false;
let zoomKeys = null;
let cameraDistance = Math.min(CAMERA_PLAY_DISTANCE, CAMERA_DISTANCE_PER_TILE * Math.max(W, H));
let cameraMaxDistance = Math.max(CAMERA_MAX_FLOOR, CAMERA_MAX_PER_TILE * Math.max(W, H));
let PROP_KINDS = UI.propKinds ?? [];
const EDGE_PX = 16;
const RECONNECT_BASE_MS = 1000;
const RECONNECT_CAP_MS = 30000;
const RECONNECT_GIVE_UP_MS = 30 * 60 * 1000;
const SIM_TICKS_PER_SECOND = 16;
const SIM_TICK_S = 1 / SIM_TICKS_PER_SECOND;
const SIM_TICK_MS = 1000 * SIM_TICK_S;
const HOST_POLL_MS = 10;
const WIRE_SCALE = 1000;
const ROSTER_EVERY_TICKS = 10;
const U16_MAX = 0xffff;

const params = new URLSearchParams(location.search);
const noMods = params.get("nomods") === "1";
const relayUrls = params.get("relay") ? params.get("relay").split(",") : null;
const namespace = params.get("ns") || "oscraft";
const statusEl = document.getElementById("status");
const canvas = document.getElementById("view");
const room = location.hash.slice(1) || "lobby";
const pkgUrl = new URL(document.body.dataset.pkg, location.href).href;
let mapUrl = null;

let mapGrid = null;
let renderer3d = null;
let mapBytes = null;
let terrainMesh = null;
let waterMesh = null;
let splat = null;
const SPLAT_BAND = 0.4;
const SHADOW_ON = params.get("shadows") !== "0";
const SHADOW_SIZE = 2048;
const AMBIENT_OFF_KEY = "oscraft.ambientOff";
function readStoredFlag(key) {
  try { return localStorage.getItem(key) === "1"; } catch { return false; }
}
function writeStoredFlag(key, on) {
  try { localStorage.setItem(key, on ? "1" : "0"); } catch { }
}
let ambientOff = params.get("ambient") === "0" || readStoredFlag(AMBIENT_OFF_KEY);
function toggleAmbient() {
  ambientOff = !ambientOff;
  writeStoredFlag(AMBIENT_OFF_KEY, ambientOff);
  if (ambientFx) ambientFx.setEnabled(!ambientOff);
  document.body.dataset.ambient = ambientOff ? "off" : "on";
}
function toggleMute() {
  document.body.dataset.muted = voices && voices.toggleMute() ? "1" : "0";
}
const SPLAT_SHADE = [0.42, 0.68, 1.3];
const SPLAT_TINTS = SPLAT_LAYERS.map((name) => (name === "dirt" ? TILE_TINTS.dirt : [1, 1, 1]));
const SPLAT_WATER_TINT = [1, 1, 1];
let terrainTiles = null;
let unitMeshes = [];
let arrowMesh = null;
const drawCounts = { terrain: 0, splat: 0, prop: 0, unit: 0, animated: 0 };
const assetErrors = new Map();
let shadowUnitCount = 0;
const unitTransform = new Float32Array(16);
const worldVp = new Float32Array(16);
let propBatches = [];
let placedProps = [];
let ambientFx = null;
let voices = null;
let ambientSeeded = false;
let propByKindState = [];
const propModelNames = [];
const unitBatches = new Map();
const MOVE_EPS = 1e-4;
const MAX_FRAME_DT_S = 0.1;
const animators = new Map();
let lastDrawMs = null;
const peers = new Set();
let netState = null;
let casts = 0;
let attacks = 0;
let localGame = null;
const liveGame = new Proxy({}, {
  get(_, name) {
    const game = localGame;
    if (!game) return undefined;
    const value = game[name];
    return typeof value === "function" ? value.bind(game) : value;
  },
});
let wasmModule = null;
let rulesText = "";
let lastGoodSnapshot = null;
let hud = null;
let widgets = null;
let packStartMounted = false;
let menuHeld = true;
const heldWidgets = new Map();
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
let spectating = false;
let lastAppliedTick = 0;
let pendingSwap = null;
let lastApplied = null;
let peerFetch = null;
let hostKey = null;
const swapRejections = [];
let rosterView = null;
let spawnedSeats = [];
const localPlayerId = () => myPlayer;
const uiPlayerId = () => (spectating ? SPECTATOR : myPlayer);
const isGuest = () => netTick >= 0;
const gate = createLoadGate();
const cover = createCover(document.body);
const uiBarEl = document.getElementById("ui-bar");

const FATAL_STATUS = /^(game stopped|start failed|assets failed|reconnect gave up)/;
const STATUS_LOG_MAX = 200;
const statusLog = [];
const fatalLines = new Map();
let statusLine = "";
const fatalBox = document.createElement("div");
fatalBox.id = "fatal";
fatalBox.setAttribute("role", "alert");
fatalBox.hidden = true;
fatalBox.style.cssText = "padding:5px 8px;background:#3a1d1a;border:1px solid #ff7a6b;border-radius:4px;color:#ffd6cf;font-size:var(--t-label);white-space:pre-line";
const fatalText = document.createElement("div");
const fatalDismiss = document.createElement("button");
fatalDismiss.type = "button";
fatalDismiss.textContent = "dismiss";
fatalDismiss.style.cssText = "margin-top:4px;font:var(--t-label) var(--font)";
fatalDismiss.addEventListener("click", () => { fatalLines.clear(); renderFatal(); });
fatalBox.append(fatalText, fatalDismiss);
statusEl.insertAdjacentElement("afterend", fatalBox);

function renderFatal() {
  fatalText.textContent = [...fatalLines.values()].join("\n");
  fatalBox.hidden = fatalLines.size === 0;
}
function renderStatus() {
  const mode = netState ? "net" : "local";
  statusEl.textContent = `room ${room} · ${peers.size} ${peers.size === 1 ? "peer" : "peers"} · ${mode}${statusLine ? `\n${statusLine}` : ""}`;
}
function telemetry(text, level = "log") {
  const line = String(text);
  statusLog.push({ at: Date.now(), line });
  if (statusLog.length > STATUS_LOG_MAX) statusLog.shift();
  console[level](`[play] ${line}`);
}
function setStatus(text) {
  const line = String(text ?? "");
  if (FATAL_STATUS.test(line)) { pinFatal(line); return; }
  telemetry(line);
  statusLine = line;
  renderStatus();
}
function pinFatal(line, key = line) {
  telemetry(line, "error");
  fatalLines.set(key, line);
  renderFatal();
}

function recordAssetError(name, err) {
  if (assetErrors.has(name)) return;
  const message = err && err.message ? err.message : String(err);
  assetErrors.set(name, message);
  console.error(`[asset] ${name}: ${message}`);
  document.body.dataset.assetError = [...assetErrors].map(([n, m]) => `${n}: ${m}`).join("; ");
  pinFatal(`assets failed to load: ${name}: ${message}`, `asset ${name}`);
}

function failStart(err) {
  const message = err && err.message ? err.message : String(err);
  document.body.dataset.startError = message;
  pinFatal(`start failed: ${message}`);
  bootDone(message);
}

const MOD_ERROR_LOG_MAX = 50;
const modErrorLog = [];
function logModError(text) {
  if (!text) return;
  console.error(`[mods] ${text}`);
  modErrorLog.push({ tick: localGame ? localGame.ticks() : null, text: String(text) });
  if (modErrorLog.length > MOD_ERROR_LOG_MAX) modErrorLog.shift();
}

function sendInput(name, args) {
  if (gate.loading) return;
  if (netState || (!hosting && peers.size > 0)) {
    if (session) session.broadcast(encodeFrame({ type: FRAME.ACTION, name, body: JSON.stringify(args) }));
    return;
  }
  if (localGame) logModError(localGame.dispatch_action(name, JSON.stringify(args)));
}

function reportSeat(name, key, player) {
  if (localGame) logModError(localGame.dispatch_action(name, JSON.stringify({ pubkey: key, player })));
}

function forwardPointer(kind, e) {
  if (!renderer3d || gate.loading) return;
  const at = screenToWorld(e.clientX, e.clientY);
  if (!at) return;
  if (netState) sendInput("pointer", { kind, x: at.x, y: at.z, button: e.button });
  else if (localGame) logModError(localGame.pointer(kind, at.x, at.z, e.button));
}

function numpadKey(e) {
  const pad = /^Numpad(\d)$/.exec(e.code || "");
  return pad ? `num${pad[1]}` : null;
}

function forwardKey(phase, e) {
  if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLSelectElement) return;
  const at = pointer && renderer3d ? screenToWorld(pointer.x, pointer.y) : null;
  sendInput("key", {
    key: numpadKey(e) ?? (e.key === " " ? "space" : e.key.toLowerCase()),
    phase,
    shift: e.shiftKey,
    ctrl: e.ctrlKey,
    alt: e.altKey,
    meta: e.metaKey,
    ...(at ? { world: { x: at.x, z: at.z } } : {}),
  });
}

function applyPeerAction(peerPubkey, frame) {
  const player = seating.seat(peerPubkey).you;
  if (!localGame) return;
  let args = null;
  try { args = JSON.parse(frame.body); } catch { return; }
  if (!args || typeof args !== "object") return;
  args.player = player;
  args.pubkey = peerPubkey;
  logModError(localGame.dispatch_action(frame.name, JSON.stringify(args)));
}

let debugReply = null;
let draftClock = null;

function routeUi(ev) {
  const data = ev.data || {};
  if (ev.kind === "debug") { debugReply = data; return; }
  if (ev.kind === "load") {
    if (data.phase === PHASE.LIVE) goLive("host");
    else if (!hosting) gate.applyRemote(data);
    return;
  }
  if (ev.kind === "widget") {
    if (data.player !== undefined && data.player !== uiPlayerId()) return;
    if (menuHeld) heldWidgets.set(data.id, data);
    else applyWidget(data);
    return;
  }
  if (ev.kind === "lobby") {
    if (!data.started) { preloadPickedModels(data.slots); return; }
    if (data.player === myPlayer && !netState) onLobbyStart({ slots: data.slots });
    return;
  }
  if (ev.kind === "feedback" || ev.kind === "action" || ev.kind === "view") {
    if (data.player !== undefined && data.player !== uiPlayerId()) return;
    if (ev.kind === "feedback") {
      if (data.kind === "selection") selectedId = data.ids[0] ?? -1;
      if (data.kind === "selection" && localGame && !netState) logModError(localGame.dispatch_action("selection", JSON.stringify({ player: data.player, ids: data.ids })));
      if (data.kind === "refusal") document.body.dataset.lastRefusal = `cast_dropped ${data.reason}`;
      window.dispatchEvent(new CustomEvent("moba:feedback", { detail: data }));
    } else if (ev.kind === "action") {
      window.dispatchEvent(new CustomEvent("moba:action", { detail: data }));
    } else if (fogview) {
      fogview.setEnabled(data.fog === true);
    }
    return;
  }
  if (ev.kind === "seat" && Number.isInteger(ev.colour)) seatColours.set(ev.seat, rgbFloats(ev.colour));
  if (ev.kind === "panel" && ev.id === "draft") draftClock?.setActive(ev.remove !== true);
  if (window.__modUi) window.__modUi.push(ev);
}

const LOCAL_OWNER_ID = "local-" + Math.random().toString(16).slice(2) + "-" + Date.now().toString(36);
let setupSeat = null;
function pumpUi() {
  if (!localGame || netState) return;
  if (setupSeat !== myPlayer) {
    setupSeat = myPlayer;
    logModError(localGame.dispatch_action("setup", JSON.stringify({ ui: UI, player: myPlayer, pubkey: auth.pubkey || LOCAL_OWNER_ID })));
  }
  for (const ev of JSON.parse(localGame.drain_ui())) {
    routeUi(ev);
    if (peers.size > 0) session.broadcast(encodeFrame({ type: FRAME.UI, body: JSON.stringify(ev) }));
  }
}

function newGame() {
  const game = new wasmModule.Game(mapBytes);
  logModError(game.set_attr_rules(rulesText));
  return game;
}

function finishGame(game, modsJson) {
  logModError(game.load_mods(modsJson));
  logModError(game.dispatch_action("setup", JSON.stringify({ ui: UI, player: myPlayer, pubkey: auth.pubkey || LOCAL_OWNER_ID })));
  game.set_fog(true);
  return game;
}

async function loadSim() {
  mapUrl = packUrl(manifest.image);
  const imageFetch = fetch(mapUrl).then((res) => res.arrayBuffer());
  const rulesFetch = packText(manifest.rules);
  const modsFetch = noMods ? Promise.resolve([]) : moduleClosure(manifest, packText);
  const mod = await import(pkgUrl);
  await mod.default();
  wasmModule = mod;
  const image = new Uint8Array(await imageFetch);
  mapBytes = image;
  setMapSize(readGrid(image));
  rulesText = await rulesFetch;
  const mods = await modsFetch;
  const game = finishGame(newGame(), JSON.stringify(mods));
  lastGoodSnapshot = captureSnapshot(game);
  if (noMods) document.body.dataset.nomods = "1";
  return game;
}

function captureSnapshot(game) {
  return { tick: game.ticks(), bytes: game.save_state() };
}

function sameBytes(a, b) {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false;
  return true;
}

const RESTORE_POINT_EVERY_TICKS = 16;
const MAX_RESTARTS_IN_A_ROW = 3;
let restartsInARow = 0;

function recoverGame(err, where) {
  const reason = err && err.message ? err.message : String(err);
  const snapshot = lastGoodSnapshot;
  localGame = null;
  restartsInARow += 1;
  if (restartsInARow > MAX_RESTARTS_IN_A_ROW) {
    logModError(`${where} failed (${reason}) again after ${MAX_RESTARTS_IN_A_ROW} restarts with no good tick between; the game stops`);
    setStatus(`game stopped: ${MAX_RESTARTS_IN_A_ROW} restarts in a row failed (${reason})`);
    return;
  }
  logModError(`${where} failed (${reason}); restarting ${snapshot ? `from the snapshot at tick ${snapshot.tick}` : "with no snapshot"} with the pack's mods disabled`);
  try {
    if (!snapshot) throw new Error("no snapshot to restore");
    const game = newGame();
    game.load_state(snapshot.bytes);
    if (!sameBytes(game.save_state(), snapshot.bytes)) throw new Error("the restored sim does not save back to the snapshot's bytes");
    finishGame(game, "[]");
    localGame = game;
    lastGoodSnapshot = captureSnapshot(game);
    if (hosting) encoder.keyframe();
    document.body.dataset.modsDisabledAt = String(snapshot.tick);
    telemetry(`mods disabled: restarted at tick=${snapshot.tick} after a failed ${where}`);
    setStatus("mods disabled after a failed restart");
  } catch (restoreErr) {
    const message = restoreErr && restoreErr.message ? restoreErr.message : String(restoreErr);
    logModError(`restart failed: ${message}`);
    if (!noMods) {
      const next = new URL(location.href);
      next.searchParams.set("nomods", "1");
      location.replace(next.href);
      return;
    }
    setStatus(`game stopped: the restart failed (${message})`);
  }
}

function decided() {
  return typeof localGame?.winner === "function" && localGame.winner() >= 0;
}

function stepSim(where) {
  try {
    logModError(localGame.tick());
    if (localGame.ticks() % RESTORE_POINT_EVERY_TICKS === 0) lastGoodSnapshot = captureSnapshot(localGame);
    restartsInARow = 0;
  } catch (err) {
    recoverGame(err, where);
    return false;
  }
  return true;
}

function setMapSize(grid) {
  mapGrid = grid;
  W = grid.width;
  H = grid.height;
  const [x0, z0, x1, z1] = walkableBounds(grid);
  cameraBounds = [x0, z0, x1, z1];
  orbitOrigin = [(x0 + x1) / 2, 0, (z0 + z1) / 2];
  cameraDistance = Math.min(CAMERA_PLAY_DISTANCE, CAMERA_DISTANCE_PER_TILE * Math.max(W, H));
  cameraMaxDistance = Math.max(CAMERA_MAX_FLOOR, CAMERA_MAX_PER_TILE * Math.max(W, H));
  const rules = packCameraRules(UI);
  if (rules) { cameraDistance = rules.distance; cameraMaxDistance = rules.max; cameraYawLocked = rules.rotationLock; }
}

function skinnedRig(animations, glb, fit) {
  const { clips, skins, nodes } = animations;
  const skin = skins[0];
  if (!skin || clips.length === 0 || skin.joints.length > MAX_JOINTS) return null;
  const grounding = clips.filter((clip) => /^(idle|walk|run)/.test(clip.name));
  const grounded = Float32Array.from(fit);
  grounded[13] -= skinFloor(nodes, skin, fit, glb.positions, glb.joints, glb.weights, grounding.length ? grounding : clips);
  return { clips, skin, nodes, fit: grounded, positions: glb.positions, normals: glb.normals || vertexNormals(glb.positions, glb.indices), joints: glb.joints, weights: glb.weights };
}

let animLibrary = null;
function humanoidLibrary() {
  if (!animLibrary) animLibrary = assetJson("anim").catch((error) => { animLibrary = null; throw error; });
  return animLibrary;
}

async function libraryClips(name, nodes) {
  const [map, library] = await Promise.all([humanoidBoneMap(name, nodes), humanoidLibrary()]);
  return retargetLibrary(library, map, nodes);
}

async function humanoidBoneMap(name, nodes) {
  const map = await packJson(assetRel("animMaps", "humanoid.json"));
  const names = new Set(nodes.map((node) => node.name).filter(Boolean));
  const resolved = Object.values(map.bones).filter((bone) => names.has(bone)).length;
  if (!names.has(map.bones.hips) || resolved < Object.keys(map.bones).length * 0.8) {
    throw new Error(`rig ${name} has bones that are not humanoid (${resolved}/${Object.keys(map.bones).length} found)`);
  }
  return map;
}
const libraryRigs = new Set();

const parsedModels = new Map();
function parsedModel(name) {
  let entry = parsedModels.get(name);
  if (!entry) {
    entry = (async () => {
      const url = new URL(assetUrl("models", `${name}.glb`), location.href);
      const res = await fetchRetrying(url);
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

let propSizesReady = null;
const hotAssets = new Map();
const assetNow = (key) => (hotAssets.has(key) ? Promise.resolve(hotAssets.get(key)) : assetJson(key));
const propSizes = () => (propSizesReady ??= assetNow("propSizes"));

async function buildUnitMesh(type) {
  const names = localGame.unit_model_names();
  const name = names[type];
  const hero = (typeof localGame.unit_flags === "function" ? localGame.unit_flags() : [])[type] === 1;
  const typeNames = typeof localGame.unit_type_names === "function" ? localGame.unit_type_names() : [];
  if (!name) {
    if (!hero) recordAssetError(`unit ${typeNames[type] ?? type}`, new Error("has no model"));
    return null;
  }
  const sizes = await propSizes();
  const size = (sizes[name] ?? 2 * UNIT_HALF) * (hero ? HERO_DRAW_SCALE : 1);
  const fitted = hero ? await fetchModel(name, size) : await modelOrRecord(name, fetchModel(name, size));
  if (!fitted) return null;
  const mesh = toMesh(renderGl, fitted, [1, 1, 1], { animated: true });
  return hero ? await mesh : await modelOrRecord(fitted.glbUrl, mesh);
}

const UNIT_MODEL_CONCURRENCY = 4;
const unitModelJobs = new Map();
let unitModelQueue = [];
let unitModelRunning = 0;

function requestUnitModel(type) {
  let job = unitModelJobs.get(type);
  if (!job) {
    job = buildUnitMesh(type).then(
      (mesh) => {
        setUnitMesh(type, mesh);
        return mesh;
      },
      (err) => {
        setUnitMesh(type, null);
        throw err;
      },
    );
    unitModelJobs.set(type, job);
  }
  return job;
}

function setUnitMesh(type, mesh) {
  unitMeshes[type] = mesh;
  for (const key of [...unitMeshCache.keys()]) if (key.startsWith(`${type}:`)) unitMeshCache.delete(key);
}

function unitModelPump() {
  while (unitModelRunning < UNIT_MODEL_CONCURRENCY && unitModelQueue.length > 0) {
    const type = unitModelQueue.shift();
    unitModelRunning += 1;
    requestUnitModel(type)
      .catch((err) => recordAssetError(`unit ${type}`, err))
      .finally(() => {
        unitModelRunning -= 1;
        unitModelPump();
      });
  }
}

function queueUnitModel(type, ahead = false) {
  if (!renderGl || type < 0 || type === undefined) return;
  if (unitMeshes[type] !== undefined || unitModelJobs.has(type) || unitModelQueue.includes(type)) return;
  if (ahead) unitModelQueue.unshift(type);
  else unitModelQueue.push(type);
  unitModelPump();
}

let deferredUntilDrawn = null;
function deferUntilDrawn(work) {
  deferredUntilDrawn = work;
}
let firstFrameUp = false;
const awaitingFirstFrame = [];
function runDeferredUntilDrawn() {
  firstFrameUp = true;
  for (const name of awaitingFirstFrame.splice(0)) preloadModel(name, 1);
  const work = deferredUntilDrawn;
  deferredUntilDrawn = null;
  if (work) work();
}

function queueUnitModelBacklog() {
  const names = localGame.unit_model_names();
  for (let type = 0; type < names.length; type += 1) if (names[type]) queueUnitModel(type);
}

async function loadPropModels(usedKinds) {
  const [paths, footprints] = await Promise.all([assetNow("props"), assetNow("propSizes")]);
  const toStem = (file) => file.replace(/^(js\/client\/)?models\//, "").replace(/\.glb$/, "");
  const modelName = (id) => (paths[id] ? toStem(paths[id]) : null);
  const sizeOf = (name) => footprints[name];
  return PROP_KINDS.map(async (kind, code) => {
    if (usedKinds && !usedKinds.has(code)) return { high: null, low: null };
    const name = modelName(kind) ?? (paths[kind] ? toStem(paths[kind]) : null);
    if (!name) {
      recordAssetError(`prop ${kind}`, new Error("no model path in props.json"));
      return { high: null, low: null };
    }
    const lowName = modelName(`${kind}-low`);
    propModelNames[code] = { high: name, low: lowName ?? null };
    const load = (model) => {
      if (sizeOf(model) === undefined) {
        recordAssetError(model, new Error("no footprint in prop_sizes.json"));
        return null;
      }
      return modelOrRecord(model, fetchModel(model, sizeOf(model)));
    };
    const [high, low] = await Promise.all([load(name), lowName ? load(lowName) : null]);
    return { high, low };
  });
}

function modelOrRecord(name, promise) {
  return promise.catch((err) => {
    recordAssetError(name, err);
    return null;
  });
}

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

function restJoints(mesh) {
  if (!mesh.animation) return null;
  const { nodes, skin, fit, clips } = mesh.animation;
  const player = new AnimationPlayer(clips);
  player.playState({});
  return jointMatrices(nodes, skin, player.pose() ?? EMPTY_POSE, fit, new Float32Array(16 * skin.joints.length));
}

async function previewRig(model, gl) {
  const fitted = await fetchModel(model, 2);
  const mesh = await toMesh(gl, fitted, [1, 1, 1], { animated: true });
  return { mesh, joints: restJoints(mesh), positions: fitted.positions };
}

function maxY(positions) {
  let top = 0;
  for (let v = 1; v < positions.length; v += 3) top = Math.max(top, positions[v]);
  return top;
}

async function loadArrowMesh(gl) {
  const footprints = await assetNow("propSizes");
  if (footprints.arrow === undefined) throw new Error("model arrow: no footprint in prop_sizes.json");
  return toMesh(gl, await fetchModel("arrow", footprints.arrow), [1, 1, 1]);
}

function groundAt(x, z) {
  return terrainTiles ? groundY(terrainTiles, x, z) : 0;
}

function anchored(u) {
  const model = unitMeshes[u.type];
  return { ...u, top: groundAt(u.x, u.z) + (model ? model.height : 0) };
}

function currentUnits() {
  if (netState) return netState.units.map(anchored);
  const out = simUnits();
  return fogview && localGame ? fogview.filter(out) : out;
}

function simUnits() {
  if (!localGame) return [];
  const flat = localGame.units();
  const ids = localGame.unit_ids();
  const types = typeof localGame.unit_types === "function" ? localGame.unit_types() : null;
  const hps = typeof localGame.unit_hps === "function" ? localGame.unit_hps() : null;
  const attacking = typeof localGame.unit_attacking === "function" ? localGame.unit_attacking() : null;
  const periods = typeof localGame.unit_periods === "function" ? localGame.unit_periods() : null;
  const strikes = typeof localGame.unit_strike_target === "function" ? localGame.unit_strike_target() : null;
  const headings = typeof localGame.unit_headings === "function" ? localGame.unit_headings() : null;
  const scales = typeof localGame.unit_scales === "function" ? localGame.unit_scales() : null;
  const out = [];
  for (let i = 0; i < ids.length; i++) {
    out.push(anchored({ id: ids[i], simHeading: headings ? headings[i] : -1, x: flat[3 * i], z: flat[3 * i + 1], player: flat[3 * i + 2], type: types ? types[i] : -1, hp: hps ? hps[i] : -1, attacking: attacking ? attacking[i] === 1 : false, period: periods ? periods[i] : 0, strikeAt: strikes ? strikes[i] : -1, scale: scales ? scales[i] / 100 : 1 }));
  }
  return out;
}

function countDraw(kind, mesh, transform, joints = null) {
  drawCounts[kind] += 1;
  if (joints) renderer3d.drawSkinned(mesh, transform, joints);
  else renderer3d.drawMesh(mesh, transform);
}

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

function reportMissingModel(type) {
  const names = localGame && typeof localGame.unit_type_names === "function" ? localGame.unit_type_names() : [];
  recordAssetError(`unit ${names[type] ?? type}`, new Error(`type ${type} has no model`));
}

const EMPTY_POSE = new Map();
const TURN_RATE = 10;
const RATE_MIN = 0.25;
const RATE_MAX = 4;
const SPEED_EASE = 8;
function wrapAngle(a) {
  return ((a + Math.PI) % (2 * Math.PI) + 2 * Math.PI) % (2 * Math.PI) - Math.PI;
}
function jointsFor(u, mesh, dt, target = null) {
  const key = `${u.id}:${u.type}`;
  let entry = animators.get(key);
  if (!entry) {
    const { skin } = mesh.animation;
    entry = { player: new AnimationPlayer(mesh.animation.clips), x: u.x, z: u.z, seen: false, heading: 0, yaw: 0, speed: 0, cycle: {}, joints: new Float32Array(16 * skin.joints.length) };
    animators.set(key, entry);
  }
  const { nodes, skin, fit } = mesh.animation;
  const dx = u.x - entry.x;
  const dz = u.z - entry.z;
  const step = Math.hypot(dx, dz);
  const moving = step > MOVE_EPS;
  if (u.simHeading >= 0) entry.heading = Math.PI / 2 - (u.simHeading / 65536) * 2 * Math.PI;
  else if (target) entry.heading = Math.atan2(target.x - u.x, target.z - u.z);
  else if (moving) entry.heading = Math.atan2(dx, dz);
  const turn = wrapAngle(entry.heading - entry.yaw);
  entry.yaw += turn * Math.min(1, dt * TURN_RATE);
  entry.speed += ((dt > 0 ? step / dt : 0) - entry.speed) * Math.min(1, dt * SPEED_EASE);
  entry.x = u.x;
  entry.z = u.z;
  entry.seen = true;
  entry.player.playState({ attacking: Boolean(u.attacking), moving });
  const clip = entry.player.clip;
  if (clip && entry.cycle[clip.name] === undefined) entry.cycle[clip.name] = cycleSpeed(nodes, fit, clip);
  const cycle = clip ? entry.cycle[clip.name] : 0;
  const attackSeconds = u.attacking && u.period > 0 ? u.period * SIM_TICK_S : 0;
  entry.player.rate = moving && !u.attacking && cycle > 0
    ? Math.min(RATE_MAX, Math.max(RATE_MIN, entry.speed / cycle))
    : attackSeconds > 0
      ? Math.min(RATE_MAX, Math.max(RATE_MIN, clip.duration / attackSeconds))
      : 1;
  entry.player.update(dt);
  return jointMatrices(nodes, skin, entry.player.pose() ?? EMPTY_POSE, fit, entry.joints);
}

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

function collectUnitDraws(units, dt) {
  const list = [];
  const clips = [];
  const yaws = [];
  for (const entry of animators.values()) entry.seen = false;
  const drawnAt = new Map(units.map((raw) => { const p = drawnPosition(raw); return [p.id, p]; }));
  for (const raw of units) {
    const u = drawnPosition(raw);
    if (unitMeshes[u.type] === undefined) {
      queueUnitModel(u.type, true);
      continue;
    }
    if (!unitMeshes[u.type]) {
      reportMissingModel(u.type);
      continue;
    }
    const color = u.id === selectedId ? SELECT_COLOR : playerColor(u.player);
    const mesh = unitMeshFor(u.type, color);
    if (!renderer3d.isVisible(u.x, u.z, meshReach(mesh))) {
      const kept = animators.get(`${u.id}:${u.type}`);
      if (kept) kept.seen = true;
      continue;
    }
    const transform = new Float32Array(multiply(translation(u.x, groundAt(u.x, u.z), u.z, unitTransform), scaling(u.scale, u.scale, u.scale)));
    if (mesh.animation) {
      const target = u.attacking && u.strikeAt >= 0 ? drawnAt.get(u.strikeAt) ?? null : null;
      const joints = jointsFor(u, mesh, dt, target);
      const entry = animators.get(`${u.id}:${u.type}`);
      const facing = new Float32Array(multiply(multiply(translation(u.x, groundAt(u.x, u.z), u.z), rotationY(entry.yaw)), scaling(u.scale, u.scale, u.scale)));
      list.push({ kind: "unit", mesh, transform: facing, joints });
      clips.push(`${u.id}:${entry.player.clip?.name ?? ""}:${entry.player.rate.toFixed(2)}`);
      yaws.push(`${u.id}:${entry.yaw.toFixed(2)}`);
    } else list.push({ kind: "unit", mesh, transform, joints: null });
  }
  for (const [key, entry] of animators) if (!entry.seen) animators.delete(key);
  document.body.dataset.animClips = clips.join(",");
  document.body.dataset.tickAlpha = tickShare().toFixed(3);
  document.body.dataset.animYaw = yaws.join(",");
  return list;
}

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

function tickShare() {
  return tickStampMs === null ? 0 : Math.min(1, Math.max(0, (performance.now() - tickStampMs) / SIM_TICK_MS));
}

function drawnPosition(u) {
  const c = tickClock.get(u.id);
  if (!c || tickStampMs === null) return u;
  const a = tickShare();
  return anchored({ ...u, x: c.px + (c.x - c.px) * a, z: c.pz + (c.z - c.pz) * a });
}

function shownUnits() {
  return currentUnits().map(drawnPosition);
}

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

window.__setCellKind = (index, kind) => {
  if (!splat || !renderer3d) return false;
  renderer3d.updateSplatCell(splat, index, setSplatTexel(splat.control, index, kind));
  return true;
};

function drawGround3d() {
  renderer3d.drawSplat(terrainMesh, splat);
  drawCounts.splat += 1;
  drawCounts.terrain += 1;
  for (const d of propBatches) {
    drawCounts.prop += renderer3d.drawInstancedLod(d.mesh, d.batch, d.lowMesh);
  }
}

let deadPropKey = "";
function syncDeadProps() {
  if (!localGame || typeof localGame.dead_props !== "function" || placedProps.length === 0) return;
  const dead = localGame.dead_props();
  const key = dead.join(",");
  if (key === deadPropKey) return;
  deadPropKey = key;
  const gone = new Set(dead);
  propBatches = batchByMesh(placedProps.filter((d) => !gone.has(d.id)));
}

function drawWorld3d() {
  syncDeadProps();
  drawCounts.terrain = 0;
  drawCounts.splat = 0;
  drawCounts.prop = 0;
  drawCounts.unit = 0;
  drawCounts.animated = 0;
  const now = performance.now();
  const dt = lastDrawMs === null ? 0 : Math.min((now - lastDrawMs) / 1000, MAX_FRAME_DT_S);
  lastDrawMs = now;
  renderer3d.beginFrame(BACKGROUND);
  const units = currentUnits();
  const unitDraws = collectUnitDraws(units, dt);
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
  if (fx && window.__fxOff !== true) fx.frame(units, dt);
  renderer3d.endFrame();
}

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
  renderer3d.pan(-dx * PAN_PX_PER_S * dtS, -dz * PAN_PX_PER_S * dtS);
}

function resize() {
  const dpr = window.devicePixelRatio || 1;
  canvas.width = Math.floor(window.innerWidth * dpr);
  canvas.height = Math.floor(window.innerHeight * dpr);
}

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

function worldToScreen(x, z, y = 0) {
  const clip = worldClip(x, z, y === 0 ? groundAt(x, z) : y);
  if (clip[3] <= 0) return { x: -1e6, y: -1e6 };
  return {
    x: ((clip[0] / clip[3] + 1) / 2) * window.innerWidth,
    y: ((1 - clip[1] / clip[3]) / 2) * window.innerHeight,
  };
}

function worldClip(x, z, y = 0) {
  const vp = viewProjection(renderer3d.camera, window.innerWidth / Math.max(1, window.innerHeight), worldVp);
  return [0, 1, 2, 3].map((r) => vp[r] * x + vp[4 + r] * y + vp[8 + r] * z + vp[12 + r]);
}

function noteLocalRefusal(ok, what) {
  if (ok) delete document.body.dataset.lastRefusal;
  else document.body.dataset.lastRefusal = `${what} ${localGame.last_refusal()}`;
  return ok;
}

const commands = {
  order(unitId, x, z) {
    const ok = netState ? (session.broadcast(sendOrder(unitId, x, z)), true) : noteLocalRefusal(localGame.order_move(unitId, x, z), `order_refused unit=${unitId}`);
    if (ok) { orders += 1; publishCommandCounts(); }
    telemetry(`order move unit ${unitId} to ${x.toFixed(2)},${z.toFixed(2)}${netState ? "" : " (local)"}`);
    return ok;
  },
  attackMove(unitId, x, z) {
    const ok = netState ? (session.broadcast(sendAttackMove(unitId, x, z)), true) : noteLocalRefusal(localGame.attack_move(unitId, x, z), `attack_move_refused unit=${unitId}`);
    if (ok) { orders += 1; publishCommandCounts(); }
    telemetry(`attack-move unit ${unitId} to ${x.toFixed(2)},${z.toFixed(2)}${netState ? "" : " (local)"}`);
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
  classOverride(unitId, cls) {
    return netState ? (session.broadcast(sendClassOverride(unitId, cls)), true) : localGame.set_class_override(unitId, cls);
  },
  holdOverride(unitId, hold) {
    return netState ? (session.broadcast(sendHoldOverride(unitId, hold)), true) : localGame.set_hold_override(unitId, hold);
  },
};

function bindInput() {
  canvas.addEventListener("pointerdown", (e) => {
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
      if (e.shiftKey || e.buttons === 2) renderer3d.orbit(dx, dy);
      else renderer3d.pan(dx, dy);
    }
    drag.x = e.clientX;
    drag.y = e.clientY;
  });
  canvas.addEventListener("pointerup", (e) => {
    const wasDrag = drag && drag.moved;
    drag = null;
    if (!wasDrag) forwardPointer("up", e);
    if (e.button === 2) document.body.dataset.rightUp = `${wasDrag ? "drag" : "click"} hud=${!!hud} local=${myPlayer}`;
  });
  canvas.addEventListener("contextmenu", (e) => e.preventDefault());
  window.addEventListener("keydown", (e) => {
    if (e.target instanceof HTMLInputElement) return;
    if (zoomKeys?.onKeyDown(e)) {
      e.preventDefault();
      return;
    }
    if (e.key.startsWith("Arrow")) {
      keysDown.add(e.key);
      e.preventDefault();
      return;
    }
    if (e.key.length === 1 || e.key === " " || e.key === "Tab" || e.key === "Escape") {
      if (!e.ctrlKey && !e.metaKey) e.preventDefault();
    }
    if (!e.repeat && !e.ctrlKey && !e.metaKey && !e.altKey) {
      if (e.key === "M") toggleMute();
      else if (e.key === "g") toggleAmbient();
    }
    forwardKey("down", e);
  });
  window.addEventListener("keyup", (e) => {
    keysDown.delete(e.key);
    forwardKey("up", e);
  });
  window.addEventListener("blur", () => {
    keysDown.clear();
    forwardKey("up", { key: "Shift", shiftKey: false, ctrlKey: false, altKey: false, metaKey: false });
  });
  window.addEventListener("pointermove", (e) => { pointer = { x: e.clientX, y: e.clientY }; });
  document.documentElement.addEventListener("pointerleave", () => { pointer = null; });
  canvas.addEventListener("wheel", (e) => {
    e.preventDefault();
    if (renderer3d) renderer3d.zoom(e.deltaY, e.deltaMode);
  }, { passive: false });
  window.addEventListener("resize", resize);
}

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
  if (typeof session.send === "function") return session.send(peerPubkey, bytes);
  session.broadcast(bytes);
  return true;
}

function sendJoin(hostKey, attempt = 0) {
  if (sendTo(hostKey, encodeFrame({ type: FRAME.JOIN, lastTick: lastAppliedTick })) !== false) return;
  console.warn(`[net] JOIN to ${String(hostKey).slice(0, 8)} not sent (attempt ${attempt + 1})`);
  if (attempt < JOIN_RETRIES) setTimeout(() => { if (peers.has(hostKey)) sendJoin(hostKey, attempt + 1); }, JOIN_RETRY_MS * (attempt + 1));
}

function askForKeyframe(hostKey) {
  awaitingKeyframe = true;
  if (Date.now() - keyframeAskedAt < KEYFRAME_ASK_MS) return;
  keyframeAskedAt = Date.now();
  console.warn(`[net] delta from ${String(hostKey).slice(0, 8)} has no baseline; asking for a keyframe`);
  sendJoin(hostKey);
}

function sendRoster(peerPubkey) {
  sendTo(peerPubkey, encodeFrame(seating.roster(peerPubkey)));
}

function heroOf(slot) {
  return simUnits().find((u) => u.player === slot && isHeroUnit(u.id)) || null;
}

let playerControlledTypes = null;
function isHeroUnit(id) {
  const u = simUnits().find((v) => v.id === id);
  if (!u || typeof localGame.unit_flags !== "function") return false;
  playerControlledTypes ??= localGame.unit_flags();
  return playerControlledTypes[u.type] === 1;
}

const BUILDING_BIT = 2048;
function isBuildingUnit(id) {
  if (!localGame || typeof localGame.unit_filter_bits !== "function") return false;
  const ids = localGame.unit_ids();
  const bits = localGame.unit_filter_bits();
  const at = ids.indexOf(id);
  return at >= 0 && (bits[at] & BUILDING_BIT) !== 0;
}

function installBuildingMinimap() {
  const args = globalThis.__oscraftHot?.get("ui-minimap");
  if (!args) return;
  const minimap = createMinimap({ ...args, isBuilding: (id) => isBuildingUnit(id) });
  window.dispatchEvent(new CustomEvent("oscraft:hmr", { detail: { url: "js/client/ui_minimap.js", value: minimap } }));
}

function recordSeatPicks(seats) {
  spawnedSeats = seats.map(({ slot, pick, owner }) => ({ slot, pick, owner }));
  if (!localGame) return;
  for (const { slot, pick, owner } of spawnedSeats) {
    if (!pick) continue;
    const bot = String(owner || "").startsWith("bot:");
    logModError(localGame.dispatch_action("seat_pick", JSON.stringify({ player: slot, hero: pick, bot })));
  }
}

function applyStart(frame) {
  seating.start(frame.seats);
  seatsExplicit = true;
  const mine = seating.seat(auth.pubkey);
  spectating = mine.you === SPECTATOR;
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
  setMatchUnderWay(true);
}

let pack = new Pack();
let packReceiver = createPackReceiver(pack);
const packBlobs = new Map();
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

function publishPackChanges(changes) {
  if (!hosting) throw new Error("only the host may save the pack");
  for (const change of changes) for (const frame of changeFrames(change, packBlobs)) session.broadcast(encodeFrame(frame));
  for (const change of changes) if (!change.deleted && packKindOf(change.path) === null) applyPackAsset(change.path);
  queuePackSwaps(changes);
  telemetry(`pack: sent ${changes.length} change(s) to peers`);
}

function currentManifest() {
  const raw = pack.get("manifest.json");
  if (raw === undefined) return manifest;
  return JSON.parse(typeof raw === "string" ? raw : new TextDecoder().decode(raw));
}

function packKindOf(file) {
  let image = manifest.image;
  try { image = currentManifest().image ?? image; } catch { }
  return packSwapKind(file, image);
}

async function packFileText(name) {
  const held = pack.get(name);
  if (held === undefined) return packText(name);
  return typeof held === "string" ? held : new TextDecoder().decode(held);
}

async function packSwapBytes(kind) {
  if (kind === "rules") return new TextEncoder().encode(await packFileText(currentManifest().rules));
  if (kind === "lua") {
    const mods = await moduleClosure(currentManifest(), packFileText);
    return new TextEncoder().encode(JSON.stringify(mods));
  }
  const name = currentManifest().image;
  const held = pack.get(name);
  if (held !== undefined) return typeof held === "string" ? new TextEncoder().encode(held) : held.slice();
  return new Uint8Array(await packBytes(name));
}

const packSwapQueue = [];

function queuePackSwaps(changes) {
  const mods = modulePaths(currentManifest());
  const queued = new Set();
  for (const change of changes) {
    const kind = packKindOf(change.path);
    if (kind === null || queued.has(kind)) continue;
    if (kind === "lua" && !mods.includes(change.path)) continue;
    queued.add(kind);
    packSwapQueue.push({ kind, file: change.path });
  }
}

function drainPackSwapQueue() {
  if (pendingSwap || swapFetching || !localGame || packSwapQueue.length === 0) return;
  const next = packSwapQueue.shift();
  announcePackSwap(next.kind, next.file);
}

async function announcePackSwap(kind, file, override = null) {
  swapFetching = true;
  try {
    const bytes = override ?? (await packSwapBytes(kind));
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

function applyPackAsset(path) {
  const base = path.replace(/^.*\//, "").replace(/\.glb$/i, "");
  if (/\.glb$/i.test(path)) parsedModels.delete(base);
  console.log(`[pack] asset ${path} applied at once`);
  window.dispatchEvent(new CustomEvent("oscraft:pack-asset", { detail: { path } }));
}

function startFromLoadedPack(hostRoom) {
  const picker = document.createElement("input");
  picker.type = "file";
  picker.accept = ".zip,.tar.gz,.tgz";
  picker.addEventListener("change", async () => {
    const file = picker.files[0];
    if (!file) return;
    try {
      telemetry(`pack ${file.name}: reading`);
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
      telemetry(`pack not started: ${err.message}`);
    }
  });
  picker.click();
}

function applyWidget(data) {
  if (widgets) widgets.apply(data);
  if (data.id === "lobby" && !data.remove && !packStartMounted) { packStartMounted = true; mountPackStart(); }
}

function releaseWidgets() {
  menuHeld = false;
  for (const data of heldWidgets.values()) applyWidget(data);
  heldWidgets.clear();
}

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

const preloadedModels = new Set();
function preloadPickedModels(slots) {
  for (const s of slots ?? []) {
    if (!s.model || preloadedModels.has(s.model)) continue;
    preloadedModels.add(s.model);
    if (firstFrameUp) preloadModel(s.model, 1);
    else awaitingFirstFrame.push(s.model);
  }
}

function preloadModel(name, retriesLeft) {
  parsedModel(name).then(
    () => window.dispatchEvent(new CustomEvent("oscraft:model-loaded", { detail: { model: name } })),
    (err) => {
      if (retriesLeft > 0) return preloadModel(name, retriesLeft - 1);
      console.warn(`[lobby] preload ${name}: ${err.message}`);
    },
  );
}

function onLobbyStart(state) {
  const frame = startFrame(state);
  setStatus("match starting");
  if (!isGuest()) applyStart(frame);
  endLobby();
  pendingStartCentre = true;
  loadStartFrame = frame;
  try { session.broadcast(encodeFrame(frame)); } catch { }
  beginLoadGate(frame.seats);
}

const SCENERY_SETTLE_MS = 12000;
const GATE_POLL_MS = 50;
const GATE_TICK_MS = 500;
const WARM_HERO_COLORS = [PLAYER_COLORS[0], PLAYER_COLORS[1], SELECT_COLOR];
let loadRun = 0;
let loadTimer = 0;
let loadStartFrame = null;
let plainCanvas = false;
let worldBuiltPromise = Promise.resolve();
let sceneryReady = Promise.resolve();
const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const isPeerPage = () => !hosting && peers.size > 0;
let worldBuiltForLobby = false;
function announceWorldReady() {
  if (!worldBuiltForLobby || isPeerPage()) return;
  sendInput("ui", { call: "world_ready", args: {} });
}
const gateCtx = {
  localOwner: (owner) => owner.startsWith("bot:") || owner === (auth.pubkey || LOCAL_OWNER_ID),
  connected: (owner) => peers.has(owner),
};

function syncPhase() {
  document.body.dataset.phase = gate.phase;
  uiBarEl.hidden = !gate.live;
  if (gate.loading) cover.show();
  else cover.hide();
}

function heroTypeOf(pick) {
  if (!pick || typeof localGame?.unit_type_names !== "function") return -1;
  return localGame.unit_type_names().indexOf(pick);
}

function seatHeroTypes() {
  const types = new Set();
  for (const seat of gate.seats()) {
    const type = heroTypeOf(seat.pick);
    if (type >= 0) types.add(type);
  }
  return [...types];
}

function heroesInSim() {
  return gate.seats().every((seat) => !seat.pick || heroOf(seat.slot) !== null);
}

function paintCover() {
  const rows = gate.rows(gateCtx);
  cover.render(rows, `Loading match: ${rows.filter((r) => r.ready).length} of ${rows.length} heroes ready`);
}

function broadcastLoad() {
  if (peers.size === 0) return;
  session.broadcast(encodeFrame({ type: FRAME.UI, body: JSON.stringify({ kind: "load", data: { phase: PHASE.LOADING, rows: gate.rows(gateCtx) } }) }));
}

function beginLoadGate(seats) {
  if (!gate.begin(seats)) return;
  syncPhase();
  paintCover();
  clearInterval(loadTimer);
  loadTimer = setInterval(() => {
    paintCover();
    if (isPeerPage()) { if (gate.local.stage === "ready") reportReady(); return; }
    broadcastLoad();
    maybeOpenGate();
  }, GATE_TICK_MS);
  runLoadGate();
}

async function runLoadGate() {
  const run = ++loadRun;
  const stale = () => run !== loadRun || !gate.loading;
  const t0 = performance.now();
  try {
    await worldBuiltPromise;
    if (!isPeerPage()) {
      while (!stale() && !heroesInSim()) {
        gate.setLocal("waiting", "no hero unit in the sim yet");
        await delay(GATE_POLL_MS);
      }
    }
    gate.setLocal("loading");
    await Promise.all(seatHeroTypes().map((type) => requestUnitModel(type)));
    const heroesMs = performance.now() - t0;
    await settleScenery();
    if (stale()) return;
    gate.setLocal("warming");
    await warmGpu(stale);
    if (stale()) return;
    gate.setLocal("ready");
    document.body.dataset.loadReadyMs = String(Math.round(performance.now() - t0));
    console.log(`[gate] ready: heroes ${Math.round(heroesMs)} ms, total ${Math.round(performance.now() - t0)} ms`);
  } catch (err) {
    gate.setLocal("failed", err && err.message ? err.message : String(err));
    console.error(`[gate] not ready: ${gate.local.detail}`);
    return;
  }
  if (isPeerPage()) reportReady();
  else maybeOpenGate();
}

async function settleScenery() {
  while (!firstFrameUp) await delay(GATE_POLL_MS);
  const deadline = performance.now() + SCENERY_SETTLE_MS;
  await Promise.race([sceneryReady, delay(SCENERY_SETTLE_MS)]);
  while (performance.now() < deadline && (unitModelRunning > 0 || unitModelQueue.length > 0)) await delay(100);
}

function warmScene() {
  const instanced = [];
  for (const d of propBatches) {
    instanced.push(d.mesh);
    if (d.lowMesh) instanced.push(d.lowMesh);
  }
  const heroTypes = new Set(seatHeroTypes());
  const flags = typeof localGame.unit_flags === "function" ? localGame.unit_flags() : [];
  const units = [];
  for (let type = 0; type < unitMeshes.length; type += 1) {
    const base = unitMeshes[type];
    if (!base) continue;
    let joints = null;
    if (base.animation) {
      const { nodes, skin, fit, clips } = base.animation;
      joints = jointMatrices(nodes, skin, EMPTY_POSE, fit, new Float32Array(16 * skin.joints.length));
      if (heroTypes.has(type)) for (const clip of clips) cycleSpeed(nodes, fit, clip);
    }
    for (const color of flags[type] === 1 ? WARM_HERO_COLORS : PLAYER_COLORS) units.push({ mesh: unitMeshFor(type, color), joints });
  }
  return {
    terrain: terrainMesh && splat ? () => renderer3d.drawSplat(terrainMesh, splat) : null,
    instanced,
    units,
    fx: fx ? (at) => fx.warm(at) : null,
  };
}

async function warmGpu(stale) {
  const scene = warmScene();
  const result = await runWarmup(scene, {
    renderer: renderer3d,
    background: BACKGROUND,
    cancelled: stale,
    onProgress: (done, total) => { document.body.dataset.warmProgress = `${done}/${total}`; },
  });
  document.body.dataset.warmMs = String(Math.round(result.ms));
  console.log(`[gate] warm-up: terrain ${scene.terrain ? 1 : 0}, prop meshes ${scene.instanced.length}, unit meshes ${scene.units.length}, effects ${scene.fx ? "yes" : "no"}, ${result.frames} frames, ${Math.round(result.ms)} ms${result.truncated ? " (budget reached)" : ""}`);
  return result;
}

function reportReady() {
  const frame = encodeFrame({ type: FRAME.ACTION, name: "load_ready", body: "{}" });
  if (hostKey !== null) sendTo(hostKey, frame);
  else session.broadcast(frame);
}

function maybeOpenGate() {
  if (isPeerPage() || !gate.canOpen(gateCtx)) return;
  goLive("every seat ready");
  if (peers.size > 0) session.broadcast(encodeFrame({ type: FRAME.UI, body: JSON.stringify({ kind: "load", data: { phase: PHASE.LIVE, rows: [] } }) }));
}

function goLive(why) {
  const waitedMs = gate.beganAt > 0 ? Math.round(performance.now() - gate.beganAt) : 0;
  if (!gate.open()) return;
  clearInterval(loadTimer);
  loadRun += 1;
  syncPhase();
  document.body.dataset.liveAfterMs = String(waitedMs);
  console.log(`[gate] live (${why}) after ${waitedMs} ms`);
}
syncPhase();

function applyPeerCommand(peerPubkey, command) {
  const player = seating.seat(peerPubkey).you;
  const unit = simUnits().find((u) => u.id === command.unitId);
  if (player === SPECTATOR || !unit || unit.player !== player) {
    telemetry(`order refused not_owner peer=${String(peerPubkey).slice(0, 8)} unit=${command.unitId}`);
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

const SNAPSHOT_EVERY_TICKS = 600;
const SNAPSHOT_MAX_CHARS = 2 * 1024 * 1024;

function snapshotKey() {
  return `oscraft:snapshot:${namespace}:${room}`;
}

function saveSnapshot(tick) {
  if (typeof localGame.save_state !== "function") return;
  const bytes = localGame.save_state();
  let binary = "";
  for (let i = 0; i < bytes.length; i += 0x8000) binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  const record = { tick, hash: localGame.state_hash().toString(), bytes: btoa(binary), seats: seating.export() };
  const text = JSON.stringify(record);
  if (text.length > SNAPSHOT_MAX_CHARS) {
    document.body.dataset.snapshotTooLarge = String(text.length);
    return;
  }
  try {
    localStorage.setItem(snapshotKey(), text);
    delete document.body.dataset.snapshotSkipped;
  } catch (e) {
    if (e && e.name === "QuotaExceededError") { localStorage.removeItem(snapshotKey()); document.body.dataset.snapshotSkipped = String(tick); return; }
    throw e;
  }
}

function restoreSnapshot() {
  const raw = localStorage.getItem(snapshotKey());
  if (raw === null) {
    telemetry("snapshot: none stored, starting fresh");
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
    telemetry(`snapshot: restored tick=${record.tick} state_hash=${hash}`);
    return record.tick;
  } catch (err) {
    telemetry(`snapshot refused: ${err.message || err}`);
    document.body.dataset.snapshotRefused = String(err.message || err);
    return -1;
  }
}

function startHost() {
  hosting = true;
  announceWorldReady();
  encoder = createStateEncoder();
  const restoredAt = restoreSnapshot();
  lastGoodSnapshot = captureSnapshot(localGame);
  if (restoredAt > 0) goLive("restored room");
  let hostTick = restoredAt > 0 ? restoredAt : 0;
  const hostStep = () => {
    if (!localGame || decided() || !gate.live) return;
    if (!stepSim("host tick")) {
      if (localGame) hostTick = localGame.ticks();
      return;
    }
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

let reconnectSince = 0;
function scheduleReconnect() {
  const now = performance.now();
  if (reconnectAttempt === 0) reconnectSince = now;
  if (now - reconnectSince > RECONNECT_GIVE_UP_MS) {
    setStatus(`reconnect gave up after ${Math.round(RECONNECT_GIVE_UP_MS / 60000)} min; reload to retry`);
    return;
  }
  const delay = Math.round(Math.random() * Math.min(RECONNECT_CAP_MS, RECONNECT_BASE_MS * 2 ** Math.min(reconnectAttempt, 16)));
  reconnectAttempt += 1;
  telemetry(`reconnecting in ${delay} ms (attempt ${reconnectAttempt})`);
  setStatus("reconnecting");
  clearTimeout(reconnectTimer);
  reconnectTimer = setTimeout(() => joinRoom(), delay);
}

function retryReconnectNow() {
  if (reconnectAttempt === 0 || peers.size > 0 || (document.visibilityState && document.visibilityState === "hidden")) return;
  clearTimeout(reconnectTimer);
  reconnectTimer = setTimeout(() => joinRoom(), 0);
}
window.addEventListener("online", retryReconnectNow);
document.addEventListener("visibilitychange", retryReconnectNow);

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

const RELAY_RETRY_MS = 5000;
const RELAY_RETRY_MAX_MS = 120000;
const RELAY_RETRIES = 6;
const relayCount = relayPool.urls.length;
const establishedRelays = new Set();
const relayRetries = new Map();
const relayRetryTimers = new Map();
function dropRelay(url, reason) {
  if (!relayPool.urls.includes(url)) return;
  relayPool.urls = relayPool.urls.filter((u) => u !== url);
  const relay = relayPool.relays.get(url);
  relayPool.relays.delete(url);
  if (relay) {
    clearTimeout(relay._connectTimer);
    if (relay.ws) {
      relay.ws.onopen = relay.ws.onmessage = relay.ws.onerror = relay.ws.onclose = null;
      try { relay.ws.close(); } catch {}
    }
  }
  clearTimeout(relayPool._reconnectTimers.get(url));
  relayPool._reconnectTimers.delete(url);
  console.warn(`relay ${url} dropped from the pool: ${reason}`);
  scheduleRelayRetry(url);
}
function scheduleRelayRetry(url) {
  const tries = relayRetries.get(url) ?? 0;
  if (tries >= RELAY_RETRIES) {
    telemetry(`relay ${url} gave up after ${tries} retries`, "warn");
    return;
  }
  const delay = Math.min(RELAY_RETRY_MS * 2 ** tries, RELAY_RETRY_MAX_MS);
  relayRetries.set(url, tries + 1);
  telemetry(`relay ${url} retry ${tries + 1} of ${RELAY_RETRIES} in ${delay / 1000} s`);
  relayRetryTimers.set(url, setTimeout(() => retryRelay(url), delay));
}
function retryRelay(url) {
  relayRetryTimers.delete(url);
  if (establishedRelays.has(url) || relayPool.urls.includes(url)) return;
  relayPool.urls.push(url);
  relayPool._open(url);
}
relayPool.addEventListener("relay-status", ({ detail: { url, status } }) => {
  if (status === "connected") {
    establishedRelays.add(url);
    relayRetries.delete(url);
    clearTimeout(relayRetryTimers.get(url));
    relayRetryTimers.delete(url);
  } else if ((status === "error" || status === "closed") && !establishedRelays.has(url)) {
    setTimeout(() => dropRelay(url, "its handshake failed"), 0);
  }
});
setTimeout(() => {
  const alive = relayPool.status().filter((r) => r.status === "connected").length;
  document.body.dataset.relaysAlive = String(alive);
  telemetry(`relays: ${alive} of ${relayCount} connected after load; ${relayRetryTimers.size} retrying`);
}, 15000);

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
const whenIdle = globalThis.requestIdleCallback ?? ((f) => setTimeout(f, 0));
whenIdle(announceRoom);
setInterval(announceRoom, ROOM_ANNOUNCE_S * 1000);
setInterval(pruneRooms, ROOM_PRUNE_MS);
const iceConfig = (() => {
  try {
    return iceServersFrom(params.get("ice"));
  } catch (err) {
    telemetry(`ice config refused: ${err.message}`);
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
    reportSeat("peer_back", key, seating.seat(key).you);
    encoder.keyframe();
    sendRoster(key);
    if (gate.loading && loadStartFrame) sendTo(key, encodeFrame(loadStartFrame));
    for (const frame of packStateFrames(pack, packBlobs)) sendTo(key, encodeFrame(frame));
  } else {
    sendJoin(e.detail.peerPubkey);
  }
  reconnectAttempt = 0;
  telemetry(`peer-open ${String(e.detail.peerPubkey).slice(0, 8)}`);
});
session.addEventListener("peer-close", (e) => {
  peers.delete(e.detail.peerPubkey);
  if (hosting) {
    const seat = seating.seat(e.detail.peerPubkey).you;
    seating.leave(e.detail.peerPubkey);
    reportSeat("peer_left", e.detail.peerPubkey, seat);
  }
  telemetry(`peer-close ${String(e.detail.peerPubkey).slice(0, 8)}`);
  if (peers.size === 0) {
    netTick = -1;
    awaitingKeyframe = true;
    scheduleReconnect();
  }
});
const netUnits = new Map();
let netTick = -1;
const JOIN_RETRIES = 4;
const JOIN_RETRY_MS = 250;
const KEYFRAME_ASK_MS = 1000;
let keyframeAskedAt = 0;
let awaitingKeyframe = false;
session.addEventListener("data", (e) => {
  let frame = null;
  const bytes = new Uint8Array(e.detail.data);
  try { frame = decodeFrame(bytes); } catch (err) {
    console.error(`[wire] frame decode failed type=${frameTypeName(bytes[0]) ?? bytes[0]} from ${String(e.detail.peerPubkey).slice(0, 8)}: ${err.message}`);
    return;
  }
  if (frame.type === FRAME.ACTION) {
    if (frame.name === "load_ready") {
      if (hosting) { gate.notePeer(e.detail.peerPubkey); maybeOpenGate(); }
      return;
    }
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
    const refusal = swapRefusal(e.detail.peerPubkey, frame.file);
    if (refusal) { refuseSwap(refusal, e.detail.peerPubkey, frame.file); return; }
    if (!swapIsNew(lastApplied ? lastApplied.tick : null, frame.tick)) { refuseSwap(`tick ${frame.tick} is not after the applied tick ${lastApplied.tick}`, e.detail.peerPubkey, frame.file); return; }
    const kind = packKindOf(frame.file);
    const swap = { tick: frame.tick, hash: frame.hash, file: frame.file, kind, url: new URL(`./${frame.file}`, import.meta.url).href, value: null, image: null, error: null, ready: null, deferrals: 0 };
    const reannounce = pendingSwap !== null && pendingSwap.file === frame.file && pendingSwap.hash === frame.hash && frame.tick > pendingSwap.tick;
    if (pendingSwap && !reannounce) {
      refuseSwap(`swap already pending for ${pendingSwap.file} at tick ${pendingSwap.tick}, so ${frame.file} at tick ${frame.tick} is refused`, e.detail.peerPubkey, frame.file);
      if (!hosting && hostKey !== null) {
        try { sendTo(hostKey, encodeFrame({ type: FRAME.SWAP_NOT_READY, tick: frame.tick, hash: frame.hash })); }
        catch (err) { console.warn(`[swap] not-ready report not sent: ${err.message}`); }
      }
      return;
    }
    pendingSwap = swap;
    prefetchSwapImage(swap, kind === null ? fetchMapImage : () => packSwapBytes(kind));
    return;
  }
  if (frame.type === FRAME.SWAP_READY) {
    if (hosting) noteSwapReady(frame, e.detail.peerPubkey);
    return;
  }
  if (frame.type === FRAME.SWAP_NOT_READY) {
    if (hosting) failSwapNotReady(frame, e.detail.peerPubkey);
    return;
  }
  if (frame.type === FRAME.SWAP_CANCEL) {
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
    beginLoadGate(frame.seats);
    return;
  }
  if (frame.type === FRAME.ROSTER) {
    if (hosting) return;
    hostKey = e.detail.peerPubkey;
    rosterView = frame;
    spectating = frame.you === SPECTATOR;
    if (frame.you !== SPECTATOR) myPlayer = frame.you;
    document.body.dataset.you = String(frame.you);
    document.body.dataset.players = String(frame.players.length);
    return;
  }
  if (hosting) return;
  if (frame.type !== FRAME.STATE && frame.type !== FRAME.DELTA) return;
  if (frame.tick <= netTick && frame.type !== FRAME.STATE) return;
  if ((frame.type === FRAME.DELTA && awaitingKeyframe) || !applyFrame(netUnits, frame)) {
    askForKeyframe(e.detail.peerPubkey);
    return;
  }
  awaitingKeyframe = false;
  netTick = frame.tick;
  lastAppliedTick = frame.tick;
  if (!gate.live) goLive("state");
  netState = { tick: frame.tick, units: [...netUnits.values()].map((u) => ({ id: u.id, x: u.x / WIRE_SCALE, z: u.y / WIRE_SCALE, player: u.player, hp: u.hp, type: u.kind })) };
  advanceTickClock(netState.units);
  applyDueSwap();
  if (window.__ciRecv) recordTick(window.__ciRecv, frame.tick, [...netUnits.values()]);
});

const CI_TICK_LOG = 300;
function recordTick(log, tick, units) {
  log.set(tick, units.map((u) => `${u.id}:${u.x}:${u.y}`).sort().join(","));
  if (log.size > CI_TICK_LOG) log.delete(log.keys().next().value);
}

function publishCommandCounts() {
  document.body.dataset.orders = String(orders);
  document.body.dataset.casts = String(casts);
  document.body.dataset.attacks = String(attacks);
}

function publishWitness() {
  document.body.dataset.units = String(currentUnits().length);
  document.body.dataset.ticks = String(localGame ? localGame.ticks() : netState ? netState.tick : 0);
  document.body.dataset.mode = "3d";
  document.body.dataset.center = renderer3d.camera.target.map((v) => v.toFixed(2)).join(",");
  document.body.dataset.distance = renderer3d.camera.distance.toFixed(3);
  document.body.dataset.drawTerrain = String(drawCounts.terrain);
  document.body.dataset.drawQuad = "0";
  document.body.dataset.drawSplat = String(drawCounts.splat);
  document.body.dataset.splatGround = splat ? "1" : "0";
  document.body.dataset.shadow = SHADOW_ON && renderer3d && renderer3d.setShadows ? "1" : "0";
  document.body.dataset.shadowUnits = String(shadowUnitCount);
  document.body.dataset.texelUploads = String(renderer3d.stats.texelUploads || 0);
  document.body.dataset.terrainMode = "heightfield";
  document.body.dataset.drawProp = String(drawCounts.prop);
  document.body.dataset.drawUnit = String(drawCounts.unit);
  document.body.dataset.drawAnimated = String(drawCounts.animated);
  document.body.dataset.drawCalls = String(renderer3d ? renderer3d.stats.drawCalls : 0);
  document.body.dataset.drawInstances = String(renderer3d ? renderer3d.stats.instances : 0);
  document.body.dataset.drawCulled = String(renderer3d ? renderer3d.stats.culled : 0);
  document.body.dataset.drawCpuMs = renderer3d ? renderer3d.stats.frameCpuMs.toFixed(3) : "0";
  publishCommandCounts();
  pumpUi();
  if (hud && gate.live) hud.update();
}

function createBarCanvas() {
  const bars = document.createElement("canvas");
  bars.style.cssText = "position:fixed;left:0;top:0;pointer-events:none";
  document.body.appendChild(bars);
  return bars;
}

function terrainInTiles(terrain) {
  const s = 1 / terrain.cellMilli;
  return { ...terrain, positions: terrain.positions.map((v) => v * s) };
}

function groundY(terrain, x, z) {
  const cols = terrain.width + 1;
  const cx = Math.min(terrain.width, Math.max(0, Math.round(x)));
  const cy = Math.min(terrain.height, Math.max(0, Math.round(z)));
  return terrain.positions[(cy * cols + cx) * 3 + 1];
}

const PROP_TINT = [0.55, 0.5, 0.42];

function unitHash(n) {
  let h = n | 0;
  h = Math.imul(h ^ (h >>> 16), 0x45d9f3b);
  h = Math.imul(h ^ (h >>> 16), 0x45d9f3b);
  return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
}
function placementHash(x, y, salt) {
  return unitHash(Math.imul(x | 0, 0x9e3779b1) ^ Math.imul(y | 0, 0x85ebca77) ^ Math.imul(salt, 0xc2b2ae3d));
}
const PROP_TINTS = [[1, 1, 1], [0.88, 0.92, 0.98], [1.08, 1.02, 0.9], [0.94, 1.05, 0.96]];
const propVariants = new Map();
function propVariant(base, kind, v) {
  if (v === 0) return base;
  const key = kind + ":" + v;
  if (!propVariants.has(key)) propVariants.set(key, { ...base, tint: PROP_TINTS[v] });
  return propVariants.get(key);
}

function addPropKind(code, base, lowBase, list) {
  if (!base || !list) return;
  const s = 1 / terrainTiles.cellMilli;
  for (const p of list) {
    const x = p.x * s;
    const z = p.y * s;
    const angle = (p.rotation / 1000) * 2 * Math.PI + placementHash(p.x, p.y, 1) * 2 * Math.PI;
    const k = (p.scale > 0 ? p.scale : 1000) / 1000;
    const variant = Math.floor(placementHash(p.x, p.y, 3) * PROP_TINTS.length);
    placedProps.push({
      id: p.id,
      kind: p.kind,
      variant,
      mesh: propVariant(base, p.kind, variant),
      lowMesh: lowBase ? propVariant(lowBase, `${p.kind}-low`, variant) : null,
      transform: multiply(multiply(translation(x, groundY(terrainTiles, x, z), z), rotationY(angle)), scaling(k, k, k)),
    });
  }
  propBatches = batchByMesh(placedProps);
  deadPropKey = null;
}

function groupByKind(placements) {
  const byKind = new Map();
  for (const p of placements) {
    const list = byKind.get(p.kind);
    if (list) list.push(p);
    else byKind.set(p.kind, [p]);
  }
  return byKind;
}

function placePropKinds(gl, usedKinds, placementsByKind, kinds) {
  const placing = [];
  for (let code = 0; code < kinds.length; code += 1) {
    if (!usedKinds.has(code)) continue;
    placing.push(kinds[code]
      .then(async ({ high, low }) => {
        const base = high ? await modelOrRecord(high.glbUrl, toMesh(gl, high, PROP_TINT)) : null;
        const lowBase = low ? await modelOrRecord(low.glbUrl, toMesh(gl, low, PROP_TINT)) : null;
        propByKindState[code] = { base, lowBase };
        addPropKind(code, base, lowBase, placementsByKind.get(code));
      })
      .catch((err) => recordAssetError(`prop ${code}`, err)));
  }
  return Promise.all(placing);
}

async function rebuildProps() {
  if (!renderGl || !localGame || !mapBytes) return;
  const placements = readProps(mapBytes);
  const usedKinds = new Set(placements.map((p) => p.kind));
  propVariants.clear();
  propModelNames.length = 0;
  propByKindState = new Array(PROP_KINDS.length).fill(null);
  placedProps = [];
  propBatches = [];
  const kinds = await loadPropModels(usedKinds);
  placePropKinds(renderGl, usedKinds, groupByKind(placements), kinds);
  console.log(`[hmr] props: ${usedKinds.size} kind(s) re-placed in place`);
}

async function build3d(gl, game) {
  renderGl = gl;
  const marks = (globalThis.__stageMarks ??= []);
  const mark = (name) => marks.push([name, performance.now()]);
  window.__stageMarks = marks;
  mark("build3d:begin");
  const terrain = terrainInTiles(buildTerrain(mapBytes));
  terrainTiles = terrain;
  terrainMesh = terrainMeshOf(terrain);
  mark("build3d:terrain");
  waterMesh = waterMeshOf(mapBytes);
  mark("build3d:water");
  const placements = readProps(mapBytes);
  mark("build3d:props");
  const usedKinds = new Set(placements.map((p) => p.kind));
  const placementsByKind = groupByKind(placements);
  const neededTypes = new Set(currentUnits().map((u) => u.type).filter((t) => t >= 0));
  bootHeroTypes = chooseBootHeroes(neededTypes);
  const splatReady = buildSplat(gl);
  splatReady.then(() => mark("build3d:splat"));
  // Unit meshes no longer sit on the first-frame critical path: the established
  // queue fits every type after the first frame at UNIT_MODEL_CONCURRENCY, and
  // collectUnitDraws queue-flags any type a draw passes through un-requested.
  splat = await splatReady;
  mark("build3d:after-splat");
  propByKindState = new Array(PROP_KINDS.length).fill(null);
  placedProps = [];
  propBatches = [];
  if (renderer3d.setShadows) renderer3d.setShadows({ enabled: SHADOW_ON, size: SHADOW_SIZE, center: [terrain.width / 2, 0, terrain.height / 2], halfExtent: Math.max(terrain.width, terrain.height) / 2 + 4 });
  mark("build3d:end");
  deferUntilDrawn(() => {
    const propKindsReady = loadPropModels(usedKinds);
    queueUnitModelBacklog();
    loadArrowMesh(gl).then(
      (mesh) => {
        arrowMesh = mesh;
      },
      (err) => recordAssetError("model arrow", err),
    );
    sceneryReady = propKindsReady.then((kinds) => placePropKinds(gl, usedKinds, placementsByKind, kinds), (err) => recordAssetError("prop kinds", err));
  });
}

async function buildSplat(gl) {
  const layers = await Promise.all(SPLAT_LAYERS.map(async (name) => {
    const [albedo, height] = await Promise.all(SPLAT_TEXTURES[name].map(async (key) => {
      const uri = assetUrl(key);
      const image = await tallyAsset(loadBaseColorImage({ uri }, location.href));
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

function terrainMeshOf(terrain) {
  return { positions: terrain.positions, normals: terrain.normals, uvs: planarGroundUvs(terrain.positions), indices: terrain.indices, color: MAP_COLOR };
}
function waterMeshOf(bytes, surface = waterSurface) {
  const water = surface(bytes);
  return water.indices.length ? { ...water, color: MAP_COLOR } : null;
}
function splatMaps(bytes, tiles = { controlMap, waterMap }) {
  const control = tiles.controlMap(bytes);
  return { control, variant: control.variant, width: control.width, height: control.height, waterMap: tiles.waterMap(bytes) };
}

window.addEventListener("oscraft:hmr", (e) => {
  const { url, kind, value } = e.detail || {};
  if (kind === "asset" || kind === "lua" || kind === "map" || kind === "model") {
    hmrInPlace(kind, url).catch((err) => console.error(`[hmr] ${kind} ${url}: rebuild failed: ${err.message}`));
    return;
  }
  if (kind === "wasm") {
    scheduleWasmReload(url);
    return;
  }
  if (kind && kind !== "module") {
    if (kind !== "shader") console.warn(`[hmr] ${url}: kind "${kind}" has no handler on this page; nothing applied`);
    return;
  }
  if (!value) return;
  const file = url.slice(url.lastIndexOf("/") + 1);
  if (hosting && localGame) {
    announceSwap(file, url, value);
    return;
  }
  hmrApply(file, url, value).catch((err) => console.error(`[hmr] ${file}: rebuild failed: ${err.message}`));
});

async function hmrInPlace(kind, url) {
  const file = decodeURIComponent(new URL(url, location.href).pathname.split("/").pop());
  if (kind === "asset") return hmrAsset(file);
  if (kind === "lua") return hmrLua(url);
  if (kind === "map") return hmrMap(file);
  return hmrModel(file);
}

function assetKeyOf(file) {
  return Object.keys(manifest.assets).find((key) => manifest.assets[key] === file) ?? null;
}

async function rereadAsset(key) {
  const rel = assetRel(key);
  const res = await fetch(packUrl(rel), { cache: "no-store" });
  if (!res.ok) throw new Error(`${rel} HTTP ${res.status}`);
  hotAssets.set(key, JSON.parse(await res.text()));
}

async function hmrAsset(file) {
  if (file === manifest.ui) return hmrUi();
  const key = assetKeyOf(file);
  if (key === "propSizes") {
    await rereadAsset(key);
    propSizesReady = null;
    await rebuildProps();
    await refitUnits();
    console.log(`[hmr] ${file}: footprints re-read; props and unit models refitted in place`);
    return;
  }
  if (key === "props") {
    await rereadAsset(key);
    await rebuildProps();
    console.log(`[hmr] ${file}: kind models re-read; props re-placed in place`);
    return;
  }
  if (key === "ktx2") {
    console.warn(`[hmr] ${file}: not applied in place. texture.js keeps its shipped KTX2 list (shippedKtx2) for the page's life; texture.js must drop that cache when ktx2.json changes.`);
    return;
  }
  if (key === "catalogue") {
    console.log(`[hmr] ${file}: nothing to rebuild, no page code reads the asset catalogue (props.json names every kind's model)`);
    return;
  }
  console.log(`[hmr] ${file}: not a pack asset this page reads; nothing to rebuild`);
}

async function hmrUi() {
  const before = { scale: UI.heroDrawScale, kinds: JSON.stringify(UI.propKinds), effects: JSON.stringify(UI.effects) };
  const res = await fetch(packUrl(manifest.ui), { cache: "no-store" });
  if (!res.ok) throw new Error(`${manifest.ui} HTTP ${res.status}`);
  const next = JSON.parse(await res.text());
  for (const key of Object.keys(UI)) delete UI[key];
  Object.assign(UI, next);
  HERO_DRAW_SCALE = UI.heroDrawScale ?? 1;
  PROP_KINDS = UI.propKinds ?? [];
  await rebuildHud();
  if (before.kinds !== JSON.stringify(UI.propKinds)) await rebuildProps();
  if (before.scale !== UI.heroDrawScale) await refitUnits();
  if (before.effects !== JSON.stringify(UI.effects)) console.warn(`[hmr] ${manifest.ui}: UI.effects changed, not applied in place; fx.js must re-read it (it reads UI.effects once at load)`);
  console.log(`[hmr] ${manifest.ui}: UI refilled, HUD rebuilt`);
}

async function rebuildHud() {
  const args = globalThis.__oscraftHot?.get("hud");
  if (!args) return;
  args.grid = mapGrid;
  const url = new URL("./hud.js", import.meta.url).href;
  const mod = await import(`${url}?v=${Date.now()}`);
  const value = mod.hmrReload(mod);
  if (value) await hmrApply("hud.js", url, value);
}

async function refitUnits() {
  if (!renderGl || !localGame) return;
  for (const name of new Set(localGame.unit_model_names())) if (name) await reloadUnitModel(name, false);
}

async function rebuildWorld(bytes) {
  mapBytes = bytes;
  setMapSize(readGrid(bytes));
  const terrain = terrainInTiles(buildTerrain(bytes));
  terrainTiles = terrain;
  terrainMesh = terrainMeshOf(terrain);
  waterMesh = waterMeshOf(bytes);
  if (splat) Object.assign(splat, splatMaps(bytes), { gpu: null });
  await rebuildProps();
  await rebuildHud();
  console.log("[swap] map image applied: terrain, water, splat and props rebuilt in place");
}

async function hmrMap(file) {
  if (file.endsWith(".osd")) console.warn(`[hmr] ${file} changed: moba.osdt must be regenerated from it by mapexport (scripts/dev/hot-sync.mjs); the page cannot build the image`);
  if (!hosting || !localGame) return;
  const bytes = await fetchMapImage();
  if (fnv1a32(bytes) === fnv1a32(mapBytes)) {
    console.log(`[hmr] ${manifest.image}: the served image is the one this match runs; no swap`);
    return;
  }
  if (pendingSwap || swapFetching) {
    console.warn(`[hmr] ${manifest.image}: a swap is already pending; the new image is announced on the next change`);
    return;
  }
  announcePackSwap("data", manifest.image, bytes);
}

async function hmrLua(url) {
  const rel = watchedPackRel(url);
  if (rel === null) {
    console.warn(`[hmr] ${url}: not a file of the pack's mods, data or UI; nothing swapped`);
    return;
  }
  if (!hosting || !localGame) {
    console.log(`[hmr] ${rel}: the host applies a changed mod at a shared tick`);
    return;
  }
  if (await publishIfChanged(rel)) console.log(`[hmr] ${rel}: sent to the match; the mod set reloads at the swap tick`);
}

async function publishIfChanged(rel) {
  const res = await fetch(packUrl(rel), { cache: "no-store" });
  if (!res.ok) throw new Error(`${rel} HTTP ${res.status}`);
  const bytes = new Uint8Array(await res.arrayBuffer());
  const hash = fnv1a32(bytes);
  packWatchSeen ??= new Map();
  const prev = packWatchSeen.get(rel);
  packWatchSeen.set(rel, hash);
  if (prev === hash) return false;
  publishPackChanges([{ path: rel, bytes, hash }]);
  return true;
}

function watchedPackRel(url) {
  const path = new URL(url, location.href).pathname;
  const rels = watchedPackPaths();
  return rels.find((rel) => new URL(packUrl(rel), location.href).pathname === path) ?? rels.find((rel) => path.endsWith(`/${rel}`)) ?? null;
}

async function hmrModel(file) {
  const name = file.replace(/\.glb$/i, "");
  if (!localGame || !renderGl) {
    console.log(`[hmr] ${file}: no game on this page yet; the model loads with it`);
    return;
  }
  const unit = localGame.unit_model_names().includes(name);
  const prop = propModelNames.some((n) => n && (n.high === name || n.low === name));
  if (!unit && !prop) {
    console.log(`[hmr] ${file}: not loaded on this page; nothing to replace`);
    return;
  }
  if (unit) await reloadUnitModel(name);
  if (prop) await reloadPropModel(name);
}

let wasmReloadTimer = null;
function scheduleWasmReload(url) {
  console.log(`[hmr] ${url}: wasm build changed; reloading the page to load it`);
  clearTimeout(wasmReloadTimer);
  wasmReloadTimer = setTimeout(() => location.reload(), 300);
}
let swapFetching = false;
let lastSwapResult = null;

async function fetchMapImage() {
  const res = await fetch(mapUrl, { cache: "no-store" });
  if (!res.ok) throw new Error(`${mapUrl} HTTP ${res.status}`);
  return new Uint8Array(await res.arrayBuffer());
}

function fnv1a32(bytes) {
  let h = 0x811c9dc5;
  for (let i = 0; i < bytes.length; i += 1) h = Math.imul(h ^ bytes[i], 0x01000193) >>> 0;
  return h >>> 0;
}

async function announceSwap(file, url, value) {
  if (pendingSwap || swapFetching) { refuseSwap(`swap already pending for ${pendingSwap ? pendingSwap.file : file} at tick ${pendingSwap ? pendingSwap.tick : "?"}`, null, file, "[hmr]"); return; }
  swapFetching = true;
  try {
    const bytes = await fetchMapImage();
    const hash = fnv1a32(bytes);
    const tick = localGame.ticks() + SWAP_LEAD_TICKS;
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
    if (fnv1a32(due.image) !== fnv1a32(mapBytes)) rebuildWorld(due.image).catch((err) => console.error(`[swap] ${due.file}: world rebuild failed: ${err.message}`));
  } catch (err) {
    fail(err && err.message ? err.message : String(err));
  }
}

const SWAP_FILE = /^[A-Za-z0-9_.-]+\.(m?js|lua|osd|osdt|json|tsv|txt)$/;

function swapRefusal(sender, file) {
  if (hosting) return "this page is the host";
  if (hostKey === null || sender !== hostKey) return `sender ${String(sender).slice(0, 8)} is not the host`;
  if (typeof file !== "string" || !SWAP_FILE.test(file) || file.includes("..")) return `file name ${JSON.stringify(file)} is not a plain module, Lua or data file name`;
  return null;
}

function refuseSwap(reason, sender, file, tag = "[swap]") {
  const entry = { reason, sender: sender === null ? null : String(sender).slice(0, 8), file: String(file).slice(0, 80), at: Date.now() };
  swapRejections.push(entry);
  if (swapRejections.length > 20) swapRejections.shift();
  console.warn(`${tag} swap refused: ${reason} (file=${JSON.stringify(entry.file)}${entry.sender ? ` sender=${entry.sender}` : ""})`);
}

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
    if (pendingSwap !== swap || hostKey === null) return;
    try { sendTo(hostKey, encodeFrame({ type: FRAME.SWAP_READY, tick: swap.tick, hash: swap.hash })); } catch (err) { console.warn(`[swap] ${swap.file}: ready report not sent: ${err.message}`); }
  }, (err) => { swap.error = err.message; });
}

function noteSwapReady(report, sender) {
  const p = pendingSwap;
  if (!p || !peers.has(sender) || report.tick !== p.tick || report.hash !== p.hash) return;
  p.readyPeers.add(sender);
}

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
  pendingSwap = null;
  session.broadcast(encodeFrame({ type: FRAME.SWAP_CANCEL, tick: p.tick, hash: p.hash }));
  lastSwapResult = { tick: localGame.ticks(), file: p.file, ok: false, carried: null, refused: null, error: `a peer was still not ready after ${p.deferrals} re-announces`, readiness: "failed", deferred: true, deferrals: p.deferrals };
  console.warn(`[swap] ${p.file}: cancelled at tick ${p.tick}, a peer was never ready (${p.deferrals} re-announces)`);
}

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

function swapClock() {
  if (netState) return netTick;
  return localGame ? localGame.ticks() : -1;
}

function applyDueSwap() {
  if (!pendingSwap || !localGame || swapClock() < pendingSwap.tick) return;
  const due = pendingSwap;
  pendingSwap = null;
  if (lastApplied && due.file !== lastApplied.file && !swapIsNew(lastApplied.tick, due.tick)) {
    console.warn(`[swap] ${due.file} at tick ${due.tick}: already applied at tick ${lastApplied.tick}, ignored`);
    return;
  }
  if (!due.image) {
    lastSwapResult = { tick: localGame.ticks(), file: due.file, ok: false, carried: null, refused: null, error: due.error || "image not ready by the swap tick", readiness: due.error ? "failed" : "in-flight", deferred: false, deferrals: due.deferrals };
    console.warn(`[swap] ${due.file} at tick ${due.tick}: image not ready, swap not applied here; reported to the host`);
    if (!hosting && hostKey !== null) {
      try { sendTo(hostKey, encodeFrame({ type: FRAME.SWAP_NOT_READY, tick: due.tick, hash: due.hash })); } catch (err) { console.warn(`[swap] not-ready report not sent: ${err.message}`); }
    }
    return;
  }
  lastApplied = { tick: due.tick, hash: due.hash, file: due.file };
  if (due.kind === "lua") return applyLuaSwap(due);
  if (due.kind === "rules") return applyRulesSwap(due);
  applyDataSwap(due);
  if (due.kind === "data") return;
  if (!due.value) {
    peerReloadModule(due.file, due.url).catch((err) => console.error(`[hmr] ${due.file}: peer reload failed: ${err.message}`));
    return;
  }
  hmrApply(due.file, due.url, due.value).catch((err) => console.error(`[hmr] ${due.file}: swap failed: ${err.message}`));
}

function applyLuaSwap(due) {
  const tick = localGame.ticks();
  const error = localGame.load_mods(new TextDecoder().decode(due.image)) || "";
  logModError(error);
  lastSwapResult = { tick, file: due.file, ok: !error, carried: null, refused: null, error: error || null, readiness: error ? "failed" : "ready", deferred: false, deferrals: due.deferrals };
  console.log(`[swap] ${due.file} at tick ${tick}: ${error ? `mods kept, reload failed: ${error}` : "mods reloaded"}`);
}

function applyRulesSwap(due) {
  const tick = localGame.ticks();
  const error = localGame.set_attr_rules(new TextDecoder().decode(due.image)) || "";
  logModError(error);
  lastSwapResult = { tick, file: due.file, ok: !error, carried: null, refused: null, error: error || null, readiness: error ? "failed" : "ready", deferred: false, deferrals: due.deferrals };
  console.log(`[swap] ${due.file} at tick ${tick}: ${error ? `rules kept, reload failed: ${error}` : "rules reloaded"}`);
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

function hmrTerrain(raw) {
  const terrain = terrainInTiles(raw);
  terrainTiles = terrain;
  terrainMesh = terrainMeshOf(terrain);
  if (splat) Object.assign(splat, splatMaps(mapBytes), { gpu: null });
}

async function hmrTiles(url) {
  const fresh = await import(`${url}?v=${Date.now()}`);
  waterMesh = waterMeshOf(mapBytes, fresh.waterSurface);
  if (splat) Object.assign(splat, splatMaps(mapBytes, fresh), { gpu: null });
}

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

const bootEl = document.getElementById("boot");
const worldSpin = document.getElementById("world-spin");
const bootT0 = performance.now();
const bootStages = ["modules", "game", "world", "ready"];
const bootStageText = { modules: "loading modules", game: "starting the game code and map", world: "loading world assets", ready: "ready" };
let bootStage = "modules";
let bootStall = false;
let bootHeroTypes = [];
const bootHeroEls = [];
const bootTally = { total: 0, done: 0, failed: 0 };
function tallyAsset(promise) {
  bootTally.total += 1;
  const settle = () => { bootTally.done += 1; };
  return promise.then((value) => { settle(); return value; }, (err) => { bootTally.failed += 1; settle(); throw err; });
}
const bootParts = {
  elapsed: document.getElementById("boot-elapsed"),
  title: document.getElementById("boot-title"),
  bar: document.getElementById("boot-progress"),
  count: document.getElementById("boot-count"),
  stages: document.getElementById("boot-stages"),
  heroes: document.getElementById("boot-heroes"),
};
const paintBoot = () => {
  if (!bootEl || bootEl.dataset.failed) return;
  bootParts.elapsed.textContent = `${((performance.now() - bootT0) / 1000).toFixed(1)} s`;
  const { total, done, failed } = bootTally;
  if (total > 0) { bootParts.bar.max = total; bootParts.bar.value = done; } else bootParts.bar.removeAttribute("value");
  const at = bootStages.indexOf(bootStage);
  let text = bootStageText[bootStage];
  if (bootStage === "world" && total > 0) text = `${text}: ${done} of ${total} done${failed ? `, ${failed} failed` : ""}`;
  if (bootStall && bootStage !== "ready") text += " (still loading after 15 s; check the network)";
  bootParts.count.textContent = text;
  [...bootParts.stages.children].forEach((li, i) => { li.dataset.state = i < at ? "done" : i === at ? "current" : "pending"; });
  for (const h of bootHeroEls) if (unitMeshes?.[h.type]) h.el.dataset.ready = "1";
};
const bootTimer = setInterval(paintBoot, 250);
function bootAt(stage) {
  bootStage = stage;
  console.log(`[boot] ${stage}`);
  paintBoot();
}
function bootDone(failure) {
  if (failure && bootEl) {
    clearInterval(bootTimer);
    bootEl.hidden = false;
    bootEl.dataset.failed = "1";
    bootParts.title.textContent = "Start failed";
    bootParts.count.textContent = `during ${bootStageText[bootStage]}: ${failure}`;
    bootParts.bar.removeAttribute("value");
    return;
  }
  clearInterval(bootTimer);
  if (bootEl) bootEl.hidden = true;
}
function chooseBootHeroes(neededTypes) {
  const flags = typeof localGame.unit_flags === "function" ? localGame.unit_flags() : [];
  const names = localGame.unit_model_names();
  const isHero = (t) => flags[t] === 1 && Boolean(names[t]);
  const needed = [...neededTypes].filter(isHero);
  const heroes = needed.length ? needed : names.map((_, t) => t).filter(isHero);
  return heroes.slice(0, 8);
}
function renderBootHeroes(iconKey, packImage) {
  const names = localGame.unit_model_names();
  for (const type of bootHeroTypes) {
    const name = names[type];
    const el = document.createElement("img");
    el.alt = name;
    el.width = 40;
    el.height = 40;
    el.addEventListener("error", () => { el.hidden = true; });
    const icon = `heroes/${name}.png`;
    packImage(`${iconKey}/${icon}`).then((url) => { el.src = url ?? assetUrl(iconKey, icon); });
    bootParts.heroes.append(el);
    bootHeroEls.push({ type, el });
  }
}

async function main() {
  resize();
  bindInput();
  const gl = canvas.getContext("webgl2", { antialias: true });
  if (!gl) throw new Error("WebGL2 is required: this browser gave no WebGL2 context for the game canvas");
  bootAt("game");
  localGame = await loadSim();
  bootAt("world");
  if (worldSpin) worldSpin.hidden = false;
  renderer3d = createRenderer3d(gl);
  const packCam = packCameraRules(UI);
  zoomKeys = createZoomKeys({ renderer: () => renderer3d, rules: packCam });
  renderer3d.setCamera({ target: orbitOrigin, distance: cameraDistance, maxDistance: cameraMaxDistance, far: cameraMaxDistance * CAMERA_FAR_PER_MAX, bounds: cameraBounds, pitch: 0.9, yawLocked: cameraYawLocked, ...(packCam ? { minDistance: packCam.min, fovY: packCam.fovY, pitch: packCam.pitch, far: packCam.far } : {}) });
  setStatus("loading world assets");
  const stalled = setTimeout(() => { bootStall = true; setStatus("still loading world assets; check the network"); }, WORLD_STALL_MS);
  fogview = createFogView({ game: liveGame, localPlayer: localPlayerId, toClip: worldClip, groundAt });
  window.__fogView = fogview;
  const worldReady = build3d(gl, liveGame).finally(() => clearTimeout(stalled));
  worldBuiltPromise = worldReady;
  hud = createHud({
    game: liveGame,
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
    playerColor,
  });
  installBuildingMinimap();
  if (renderer3d) ambientFx = createAmbientFx({ renderer: renderer3d, groundAt, enabled: !ambientOff });
  document.body.dataset.ambient = ambientOff ? "off" : "on";
  if (renderer3d && manifest.assets?.voices) voices = createVoices({ url: (rel) => assetUrl("voices", rel) });
  if (renderer3d) fx = createFx({ renderer: renderer3d, game: () => localGame, toScreen: worldToScreen, groundAt, playerColor, selected: () => selectedId, arrow: () => arrowMesh, localPlayer: localPlayerId });
  widgets = createWidgets({ send: (call, args) => sendInput("ui", { call, args }) });
  draftClock = createDraftClock({ send: (ticks) => sendInput("ui", { call: "draft_clock", args: { ticks } }), allowed: () => !netState || hosting });
  const packsDir = `${manifest.assets?.root ?? "."}/packs`;
  const packIndex = (() => {
    let index = null;
    return () => (index ??= packBytes(`${packsDir}/index.json`).then((bytes) => JSON.parse(new TextDecoder().decode(bytes))).catch(() => null));
  })();
  const packFiles = new Map();
  const packImages = new Map();
  const packFile = (file) => {
    if (!packFiles.has(file)) packFiles.set(file, packBytes(`${packsDir}/${file}`));
    return packFiles.get(file);
  };
  const atlasImagesByCategory = new Map();
  const atlasImageOnce = (category, atlas) => {
    if (!atlasImagesByCategory.has(category)) {
      atlasImagesByCategory.set(category, packFile(atlas.webp).then((data) => createImageBitmap(new Blob([data], { type: "image/webp" }))));
    }
    return atlasImagesByCategory.get(category);
  };
  const atlasOf = (index, rel) => Object.entries(index?.atlases ?? {}).find(([, atlas]) => atlas.icons[rel]);
  packIndex()
    .then((index) => (index?.atlases?.heroes ? atlasImageOnce("heroes", index.atlases.heroes) : null))
    .catch(() => null)
    .then(releaseWidgets);
  const cellUrl = (bitmap, [x, y, size]) => {
    const canvas = document.createElement("canvas");
    canvas.width = size;
    canvas.height = size;
    canvas.getContext("2d").drawImage(bitmap, x, y, size, size, 0, 0, size, size);
    return new Promise((resolve) => canvas.toBlob((blob) => resolve(URL.createObjectURL(blob)), "image/webp"));
  };
  const loadPackImage = async (rel) => {
    const index = await packIndex();
    const found = atlasOf(index, rel);
    if (found) {
      const [category, atlas] = found;
      return cellUrl(await atlasImageOnce(category, atlas), atlas.icons[rel]);
    }
    const entry = index?.entries?.[rel];
    if (!entry) return null;
    const [file, offset, length] = entry;
    const data = await packFile(file);
    const bytes = data instanceof ArrayBuffer ? new Uint8Array(data) : data;
    return URL.createObjectURL(new Blob([bytes.subarray(offset, offset + length)], { type: "image/png" }));
  };
  const packImageUrl = (rel) => {
    if (!packImages.has(rel)) packImages.set(rel, loadPackImage(rel).catch(() => null));
    return packImages.get(rel);
  };
  setPreviewRig(previewRig);
  setImageSrcResolver((src) => {
    const rel = src.replace(/^\.?\//, "");
    const key = manifest.assets?.icons;
    if (!key || !rel.startsWith(`${key}/`)) return packUrl(rel);
    const icon = rel.slice(key.length + 1);
    return packImageUrl(rel).then((url) => url ?? assetUrl(key, icon));
  });
  renderBootHeroes(manifest.assets?.icons ?? "icons", packImageUrl);
  const sfx = manifest.assets?.sounds ? createSfx({ url: (rel) => assetUrl("sounds", rel) }) : null;
  const eventsBase = params.get("events");
  const modUi = createModUi({
    root: document.body,
    renderer: () => renderer3d,
    localPlayer: uiPlayerId,
    defaultDistance: () => cameraDistance,
    baseUrl: eventsBase === null ? undefined : eventsBase.replace(/\/$/, ""),
    onBind: (bind) => sendInput("bind", bind),
    onSound: manifest.assets?.sounds ? ((name) => sfx.play(name)) : undefined,
  });
  window.__modUi = modUi;
  createChat({
    send: sendInput,
    notify: (text) => modUi.push({ kind: "message", text, seconds: 10 }),
    clearLog: () => document.querySelector(".modui-log")?.replaceChildren(),
    camera: {
      get: () => renderer3d?.camera.distance ?? cameraDistance,
      set: (distance) => renderer3d?.setCamera({ distance: Math.min(renderer3d.camera.maxDistance, Math.max(renderer3d.camera.minDistance, distance)) }),
      defaultDistance: () => cameraDistance,
    },
  });
  const packButton = document.createElement("button");
  packButton.type = "button";
  packButton.id = "pack-open";
  packButton.textContent = "pack";
  packButton.addEventListener("click", openPackEditor);
  document.getElementById("ui-left").append(packButton);
  if (params.get("host") === "1") startHost();
  joinRoom();

  let worldBuilt = false;
  function frame(now) {
    const dtS = lastFrameMs ? Math.min(0.1, (now - lastFrameMs) / 1000) : 0;
    lastFrameMs = now;
    panByKeys(dtS);
    modUi.step(dtS);
    if (pendingStartCentre && renderer3d) {
      const own = simUnits().find((u) => u.player === localPlayerId() && isHeroUnit(u.id));
      if (own) {
        renderer3d.setCamera({ target: [own.x, groundAt(own.x, own.z), own.z], distance: Math.min(packCameraRules(UI)?.distance ?? CAMERA_START_DISTANCE, cameraMaxDistance) });
        if (hud) hud.input.setSelection([own.id]); else selectedId = own.id;
        pendingStartCentre = false;
      }
    }
    if (!netState && localGame && !hosting && gate.live && !decided()) {
      localTickAcc += dtS;
      while (localTickAcc >= SIM_TICK_S && localGame && !decided()) {
        if (stepSim("tick")) {
          applyDueSwap();
          advanceTickClock(simUnits());
        }
        localTickAcc -= SIM_TICK_S;
      }
    }
    if (localGame && worldBuilt) {
      if (gate.live) {
        drawWorld3d();
        fogview.draw();
        plainCanvas = false;
      } else if (!plainCanvas && !gate.loading) {
        renderer3d.beginFrame(BACKGROUND);
        renderer3d.endFrame();
        plainCanvas = true;
      }
      runDeferredUntilDrawn();
    }
    if (ambientFx && worldBuilt && gate.live) {
      if (!ambientSeeded && placedProps.length) {
        for (const d of placedProps) if (AMBIENT_KINDS[d.kind]) ambientFx.addEmitter(d.transform[12], d.transform[14], AMBIENT_KINDS[d.kind]);
        ambientSeeded = true;
      }
      ambientFx.update(dtS);
      ambientFx.draw();
    }
    publishWitness();
    requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);
  worldReady.then(() => {
    worldBuilt = true;
    bootDone();
    if (worldSpin) worldSpin.hidden = true;
    document.body.dataset.worldBuilt = "1";
    worldBuiltForLobby = true;
    announceWorldReady();
    setStatus("local preview");
    document.body.dataset.ready = "1";
    if (!UI.text?.lobby && !gate.live && !isPeerPage()) beginLoadGate([]);
  }).catch((err) => {
    const message = err && err.message ? err.message : String(err);
    document.body.dataset.error = message;
    if (worldSpin) worldSpin.hidden = true;
    bootDone(message);
    failStart(err);
  });
}

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
  fxCounts: () => (fx ? fx.counts() : null),
  voices: () => (voices ? voices.heroes() : null),
  playVoice: (hero, event) => (voices ? voices.play(hero, event) : Promise.resolve(false)),
  modErrors: () => modErrorLog.map((e) => ({ ...e })),
  statusLog: () => statusLog.map((e) => ({ ...e })),
  swap: () => ({
    applied: lastSwapResult ? { ...lastSwapResult } : null,
    pending: pendingSwap ? { tick: pendingSwap.tick, file: pendingSwap.file, hash: pendingSwap.hash, deferrals: pendingSwap.deferrals, ready: Boolean(pendingSwap.image) } : null,
    rejections: swapRejections.slice(-10),
  }),
  run: (name, args = null) => {
    if ((!localGame && !netState) || !gate.live) return false;
    sendInput(name, args);
    return true;
  },
  lobby: (call, args) => {
    if (!localGame && !netState) return false;
    sendInput("ui", { call, args });
    return true;
  },
  isHero: (id) => localGame !== null && isHeroUnit(id),
  selected: () => (hud ? hud.input.selection() : selectedId === -1 ? [] : [selectedId]),
  select: (ids) => { if (hud) hud.input.setSelection(ids); else selectedId = ids[0] ?? -1; },
  pick: (world) => pick(world),
  order: (id, x, z) => gate.live && commands.order(id, x, z),
  attack: (id, targetId) => gate.live && commands.attack(id, targetId),
  overrides: (id) => (localGame ? { classOverride: localGame.class_override(id), holdOverride: localGame.hold_override(id) } : null),
  setClassOverride: (id, cls) => gate.live && commands.classOverride(id, cls),
  setAttackHold: (id, hold) => gate.live && commands.holdOverride(id, hold),
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
  window.__ciRoster = () => simUnits().map((u) => ({ id: u.id, x: u.x, z: u.z, player: u.player, type: u.type }));
  window.__ciOrder = (unitId, x, z) => session.broadcast(sendOrder(unitId, x, z));
  window.__ciMove = (unitId, x, z) => commands.order(unitId, x, z);
  window.__ciAttack = (unitId, targetId) => commands.attack(unitId, targetId);
  window.__ciCast = (unitId, slot, target, x, z) => session.broadcast(sendCast(unitId, slot, target, x, z));
  window.__ciLearn = (unitId, slot) => (localGame ? localGame.skill_up(unitId, slot) : false);
  window.__ciItemIndex = (name) => JSON.parse(localGame.item_catalog_json()).findIndex((e) => e.name === name);
  window.__ciGiveItem = (unitId, index, count) => localGame.give_item(unitId, index, count);
  window.__ciInventory = (unitId) => JSON.parse(localGame.inventory_json(unitId)).slots;
  window.__ciTargetedSlot =(unitId) => { const list = JSON.parse(localGame.abilities_json(unitId)); const t = list.find((a) => (a.target === "unit" || a.target === "point") && a.slot < 4); return t ? t.slot : -1; };
  window.__ciHostile = (a, b) => localGame.hostile(a, b);
  window.__ciIsHero = (unitId) => isHeroUnit(unitId);
  window.__ciAttack = (unitId, targetId) => session.broadcast(sendAttack(unitId, targetId));
  window.__ciLocalCast = (unitId, slot, target, x, z) => localGame.cast(unitId, slot, target, x, z);
  window.__ciLocalAttack = (unitId, targetId) => localGame.attack(unitId, targetId);
  window.__ciFxCounts = () => (fx ? fx.counts() : null);
  window.__ciSent = new Map();
  window.__ciRecv = new Map();
  window.__ciSentTicks = () => [...window.__ciSent];
  window.__ciRecvTicks = () => [...window.__ciRecv];
  window.__ciSelected = () => selectedId;
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
  window.__ciSelect = (unitId) => { if (hud) hud.input.setSelection([unitId]); else selectedId = unitId; };
  window.__ciInputSelection = () => (hud ? hud.input.selection() : []);
  window.__ciLocalPlayer = () => myPlayer;
  window.__ciFocus = (x, z) => { if (renderer3d) renderer3d.setCamera({ target: [x, groundAt(x, z), z] }); };
  window.__ciToScreen = (x, z) => worldToScreen(x, z, groundAt(x, z));
  window.__ciKillSide = (player) => {
    for (const u of simUnits()) if (u.player === player) localGame.hurt(u.id, 10000);
  };
  window.__ciHeroNames = () => {
    const names = typeof localGame.unit_type_names === "function" ? localGame.unit_type_names() : [];
    return spawnedSeats.map(({ slot, pick }) => {
      const kind = names.indexOf(pick);
      const u = simUnits().find((v) => v.player === slot && isHeroUnit(v.id) && v.type === kind) || null;
      return { slot, picked: pick, spawned: u ? names[u.type] ?? null : null };
    });
  };
  window.__ciRampCell = () => {
    const grid = readGrid(mapBytes);
    const [cell] = rampLowCells(grid);
    return cell === undefined ? null : { x: (cell % grid.width) + 0.5, z: Math.floor(cell / grid.width) + 0.5 };
  };
  window.__ciFx = () => (fx ? fx.counts() : null);
  window.oscraft = Object.assign(window.oscraft || {}, {
    setCamera: (opts = {}) => (renderer3d ? renderer3d.setCamera(opts) : null),
    centreOn: (x, z) => (renderer3d ? renderer3d.setCamera({ target: [x, groundAt(x, z), z] }) : null),
    fx: () => (fx ? fx.counts() : null),
    swapRejections: () => swapRejections.slice(),
    lastSwap: () => (lastSwapResult ? { ...lastSwapResult } : null),
  });
  window.__ciBarProbe = () => ({ tick: tickStampMs, shown: shownUnits().map((u) => [u.id, u.x, u.z]), raw: currentUnits().map((u) => [u.id, u.x, u.z]) });
  window.__ciDebug = () => [...session.peers.values()].map((p) => ({ ice: p.pc?.iceConnectionState, conn: p.pc?.connectionState, remoteMdns: (p.pc?.remoteDescription?.sdp || '').split('.local').length - 1 }));
}

const MODEL_WATCH_MS = 2000;
let renderGl = null;
let modelWatchSig = new Map();
let modelWatchBusy = false;

async function checkModelWatch() {
  if (modelWatchBusy || !localGame || !renderGl) return;
  modelWatchBusy = true;
  try {
    const names = [...new Set([...localGame.unit_model_names(), ...propModelNames.flatMap((n) => (n ? [n.high, n.low] : []))].filter((n) => n))];
    for (const name of names) {
      const url = new URL(assetUrl("models", `${name}.glb`), location.href).href;
      let sig;
      try {
        const head = await fetch(url, { method: "HEAD", cache: "no-store" });
        if (!head.ok) continue;
        sig = [head.headers.get("etag"), head.headers.get("last-modified"), head.headers.get("content-length")].join("|");
      } catch {
        continue;
      }
      const prev = modelWatchSig.get(url);
      modelWatchSig.set(url, sig);
      if (prev !== undefined && prev !== sig) {
        await reloadUnitModel(name);
        await reloadPropModel(name);
      }
    }
  } catch (err) {
    console.error(`[model] watch failed: ${err.message}`);
  } finally {
    modelWatchBusy = false;
  }
}

async function reloadPropModel(name) {
  const sizes = await assetNow("propSizes");
  for (let code = 0; code < propModelNames.length; code++) {
    const names = propModelNames[code];
    if (!names) continue;
    const high = names.high === name;
    const low = names.low === name;
    if (!high && !low) continue;
    parsedModels.delete(name);
    let mesh;
    try {
      mesh = await toMesh(renderGl, await fetchModel(name, sizes[name]), PROP_TINT);
    } catch (err) {
      console.error(`[model] prop ${name} kept the old model: ${err.message}`);
      continue;
    }
    const slot = propByKindState[code];
    if (!slot || !slot.base) continue;
    if (high) {
      slot.base = mesh;
      for (const key of [...propVariants.keys()]) if (key.startsWith(`${code}:`)) propVariants.delete(key);
    } else {
      slot.lowBase = mesh;
      for (const key of [...propVariants.keys()]) if (key.startsWith(`${code}-low:`)) propVariants.delete(key);
    }
    for (const e of placedProps) {
      if (e.kind !== code) continue;
      if (high) e.mesh = propVariant(slot.base, code, e.variant);
      else e.lowMesh = slot.lowBase ? propVariant(slot.lowBase, `${code}-low`, e.variant) : null;
    }
    propBatches = batchByMesh(placedProps);
    console.log(`[model] prop ${name} reloaded in place for kind ${code}`);
  }
}

async function reloadUnitModel(name, fresh = true) {
  if (fresh) parsedModels.delete(name);
  const names = localGame.unit_model_names();
  const heroFlags = typeof localGame.unit_flags === "function" ? localGame.unit_flags() : [];
  const sizes = await assetNow("propSizes");
  for (let type = 0; type < names.length; type++) {
    if (names[type] !== name) continue;
    const hero = heroFlags[type] === 1;
    const size = (sizes[name] ?? 2 * UNIT_HALF) * (hero ? HERO_DRAW_SCALE : 1);
    try {
      const fitted = await fetchModel(name, size);
      unitMeshes[type] = await toMesh(renderGl, fitted, [1, 1, 1], { animated: true });
    } catch (err) {
      console.error(`[model] ${name} for unit type ${type} kept the old model: ${err.message}`);
      continue;
    }
    for (const key of [...unitMeshCache.keys()]) if (key.startsWith(`${type}:`)) unitMeshCache.delete(key);
    for (const key of [...animators.keys()]) if (key.endsWith(`:${type}`)) animators.delete(key);
    console.log(`[model] ${name} reloaded in place for unit type ${type}`);
  }
}

if (["localhost", "127.0.0.1"].includes(location.hostname)) setInterval(checkModelWatch, MODEL_WATCH_MS);

const WASM_WATCH_MS = 2000;
let wasmWatchSig = null;
let wasmWatchBusy = false;

async function checkWasmWatch() {
  if (wasmWatchBusy || !hosting || !localGame) return;
  const wasmUrl = new URL("web_bg" + ".wasm", pkgUrl).href;
  let sig;
  try {
    const head = await fetch(wasmUrl, { method: "HEAD", cache: "no-store" });
    if (!head.ok) return;
    sig = [head.headers.get("etag"), head.headers.get("last-modified"), head.headers.get("content-length")].join("|");
  } catch {
    return;
  }
  if (wasmWatchSig === null) { wasmWatchSig = sig; return; }
  if (sig === wasmWatchSig) return;
  if (peers.size > 0) {
    console.warn("[wasm] a new build is on disk; peers are connected, so it is not swapped in");
    return;
  }
  wasmWatchBusy = true;
  try {
    await swapWasm(wasmUrl);
  } catch (err) {
    console.error(`[wasm] swap failed, the running module stays: ${err.message}`);
  } finally {
    wasmWatchSig = sig;
    wasmWatchBusy = false;
  }
}

async function swapWasm(wasmUrl) {
  const saved = localGame.save_state();
  const fresh = await import(`${pkgUrl}?v=${Date.now()}`);
  await fresh.default({ module_or_path: fetch(`${wasmUrl}?v=${Date.now()}`, { cache: "no-store" }) });
  const game = new fresh.Game(mapBytes);
  logModError(game.set_attr_rules(rulesText));
  game.load_state(saved);
  if (!sameBytes(game.save_state(), saved)) throw new Error("the new module does not save back the same state");
  finishGame(game, JSON.stringify(noMods ? [] : await moduleClosure(manifest, packText)));
  wasmModule = fresh;
  localGame = game;
  lastGoodSnapshot = captureSnapshot(game);
  telemetry(`wasm: the build changed on disk; swapped in at tick ${game.ticks()}`);
  console.log(`[wasm] swapped the running module in at tick ${game.ticks()}`);
}

if (["localhost", "127.0.0.1"].includes(location.hostname)) setInterval(checkWasmWatch, WASM_WATCH_MS);

const PACK_WATCH_MS = 1000;
const PACK_WATCH_TEXT = /\.(lua|osd|json|tsv|txt)$/i;
let packWatchSeen = null;
let packWatchBusy = false;

function watchedPackPaths() {
  const m = manifest;
  const list = [...modulePaths(m), ...(m.data ?? []), ...(m.tables ?? []), m.rules, m.ui, "manifest.json"];
  const text = list.filter((p) => typeof p === "string" && PACK_WATCH_TEXT.test(p));
  return [...new Set([...text, m.image].filter((p) => typeof p === "string"))];
}

async function checkPackWatch() {
  if (packWatchBusy || !hosting || !localGame || !manifest) return;
  packWatchBusy = true;
  try {
    packWatchSeen ??= new Map();
    const changes = [];
    for (const rel of watchedPackPaths()) {
      const res = await fetch(packUrl(rel), { cache: "no-store" });
      if (!res.ok) continue;
      const bytes = new Uint8Array(await res.arrayBuffer());
      const hash = fnv1a32(bytes);
      const prev = packWatchSeen.get(rel);
      packWatchSeen.set(rel, hash);
      if (prev !== undefined && prev !== hash) changes.push({ path: rel, bytes, hash });
    }
    if (changes.length > 0) {
      console.log(`[pack] ${changes.map((c) => c.path).join(", ")}: changed on disk, sent to the match`);
      publishPackChanges(changes);
    }
  } catch (err) {
    console.error(`[pack] watch failed: ${err.message}`);
  } finally {
    packWatchBusy = false;
  }
}

if (["localhost", "127.0.0.1"].includes(location.hostname)) setInterval(checkPackWatch, PACK_WATCH_MS);

main().catch((err) => {
  const message = err && err.message ? err.message : String(err);
  document.body.dataset.error = message;
  failStart(err);
});
// Net client: WebGL2 map and unit view. Map and units come from the same assets as the browser
// page (map.bin and the wasm sim pkg). Joins a wireweave DataSession room named by the URL hash.
import { createDataSession, createFSM, NostrAuth, RelayPool } from "wireweave";
import * as NostrTools from "nostr-tools";
import * as xstate from "xstate";
import { encodeFrame, decodeFrame, FRAME } from "./wire/frame.js";
import { iceServersFrom } from "./ice.js";
import { applyFrame, createStateEncoder } from "./delta.js";

// The map grid from data/map.osd: grid(32, 24), one tile per cell.
const W = 32;
const H = 24;
const BACKGROUND = [0.114, 0.145, 0.173];
const MAP_COLOR = [0.2, 0.28, 0.22];
const GRID_COLOR = [0.3, 0.38, 0.3];
const PLAYER_COLORS = [[0.227, 0.482, 0.835], [0.835, 0.329, 0.227]];
const SELECT_COLOR = [0.95, 0.85, 0.3];
const UNIT_HALF = 0.4;
const PICK_RADIUS = 1.0;
const BASE_PX_PER_UNIT = 20;
const MIN_ZOOM = 0.5;
const MAX_ZOOM = 4;
const DRAG_SLOP_PX = 4;
const RIGHT_BUTTON = 2;
const PAN_TILES_PER_S = 12;
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

const camera = { cx: W / 2, cz: H / 2, zoom: 1 };
const peers = new Set();
let netState = null;
let casts = 0;
let attacks = 0;
let attackMode = false;
let localGame = null;
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
  const game = new mod.Game(image);
  // The moba preview runs without fog so every enemy is a valid target; other pages keep the sim's fog.
  game.set_fog(location.hash !== "#moba");
  return game;
}

function compile(gl, type, src) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, src);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(shader));
  return shader;
}

function createRenderer() {
  const gl = canvas.getContext("webgl2", { antialias: true });
  if (!gl) throw new Error("WebGL2 unavailable");
  const program = gl.createProgram();
  gl.attachShader(program, compile(gl, gl.VERTEX_SHADER, `#version 300 es
    in vec2 aPos; in vec3 aColor; uniform vec4 uView; out vec3 vColor;
    void main() {
      vec2 p = (aPos - uView.xy) * uView.zw;
      gl_Position = vec4(p, 0.0, 1.0); vColor = aColor;
    }`));
  gl.attachShader(program, compile(gl, gl.FRAGMENT_SHADER, `#version 300 es
    precision mediump float; in vec3 vColor; out vec4 outColor;
    void main() { outColor = vec4(vColor, 1.0); }`));
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(program));
  gl.useProgram(program);
  const buffer = gl.createBuffer();
  const loc = { pos: gl.getAttribLocation(program, "aPos"), color: gl.getAttribLocation(program, "aColor"), view: gl.getUniformLocation(program, "uView") };
  gl.enableVertexAttribArray(loc.pos);
  gl.enableVertexAttribArray(loc.color);
  return { gl, buffer, loc };
}

// Interleaved vertices: x, z, r, g, b. World x runs east, world z runs south (screen down).
function pushQuad(out, x0, z0, x1, z1, color) {
  const corners = [[x0, z0], [x1, z0], [x1, z1], [x0, z0], [x1, z1], [x0, z1]];
  for (const [x, z] of corners) out.push(x, z, color[0], color[1], color[2]);
}

function pushLine(out, x0, z0, x1, z1, color) {
  out.push(x0, z0, color[0], color[1], color[2], x1, z1, color[0], color[1], color[2]);
}

function buildMapMeshes() {
  const tris = [];
  pushQuad(tris, 0, 0, W, H, MAP_COLOR);
  const lines = [];
  for (let x = 0; x <= W; x++) pushLine(lines, x, 0, x, H, GRID_COLOR);
  for (let z = 0; z <= H; z++) pushLine(lines, 0, z, W, z, GRID_COLOR);
  return { tris: new Float32Array(tris), lines: new Float32Array(lines) };
}

function currentUnits() {
  if (netState) return netState.units;
  if (!localGame) return [];
  const flat = localGame.units();
  const ids = localGame.unit_ids();
  const out = [];
  for (let i = 0; i < ids.length; i++) {
    out.push({ id: ids[i], x: flat[3 * i], z: flat[3 * i + 1], player: flat[3 * i + 2] });
  }
  return out;
}

function unitVerts(units) {
  const out = [];
  for (const u of units) {
    const color = u.id === selectedId ? SELECT_COLOR : PLAYER_COLORS[u.player] || PLAYER_COLORS[0];
    pushQuad(out, u.x - UNIT_HALF, u.z - UNIT_HALF, u.x + UNIT_HALF, u.z + UNIT_HALF, color);
  }
  return new Float32Array(out);
}

function draw(r, mode, verts) {
  const { gl, buffer, loc } = r;
  gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
  gl.bufferData(gl.ARRAY_BUFFER, verts, gl.DYNAMIC_DRAW);
  gl.vertexAttribPointer(loc.pos, 2, gl.FLOAT, false, 20, 0);
  gl.vertexAttribPointer(loc.color, 3, gl.FLOAT, false, 20, 8);
  gl.drawArrays(mode, 0, verts.length / 5);
}

function pxPerUnit() {
  return BASE_PX_PER_UNIT * camera.zoom;
}

// Arrow keys and the pointer at the window edge pan the camera at a constant speed in tiles per second.
// A drag pans by itself, so edge scroll waits while one is in progress.
function panCamera(dtS) {
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
  const step = PAN_TILES_PER_S * dtS;
  camera.cx = Math.min(W, Math.max(0, camera.cx + dx * step));
  camera.cz = Math.min(H, Math.max(0, camera.cz + dz * step));
}

function resize() {
  const dpr = window.devicePixelRatio || 1;
  canvas.width = Math.floor(window.innerWidth * dpr);
  canvas.height = Math.floor(window.innerHeight * dpr);
}

// Maps a client-pixel point to the world (x, z) under the camera; screen up is map north (-z).
function screenToWorld(clientX, clientY) {
  const dx = clientX - window.innerWidth / 2;
  const dy = clientY - window.innerHeight / 2;
  return { x: camera.cx + dx / pxPerUnit(), z: camera.cz + dy / pxPerUnit() };
}

function pick(world) {
  let best = null;
  let bestD = PICK_RADIUS * PICK_RADIUS;
  for (const u of currentUnits()) {
    const d = (u.x - world.x) ** 2 + (u.z - world.z) ** 2;
    if (d <= bestD) { bestD = d; best = u; }
  }
  return best;
}

// Left button selects the unit under the pointer. Right button orders the selected unit to walk to the
// clicked point: a guest sends the order to the host; the host and the local preview apply it to the sim.
function onClick(e) {
  const world = screenToWorld(e.clientX, e.clientY);
  if (e.button === RIGHT_BUTTON) {
    issueMove(world);
    return;
  }
  if (attackMode) {
    attackMode = false;
    const target = pick(world);
    if (target && localGame && localGame.attack(selectedId, target.id)) attacks += 1;
    return;
  }
  const hit = pick(world);
  selectedId = hit ? hit.id : -1;
}

function issueMove(world) {
  const selected = currentUnits().find((u) => u.id === selectedId);
  if (!selected) return;
  if (netState) {
    session.broadcast(sendOrder(selected.id, world.x, world.z));
    orders += 1;
  } else if (localGame.order_move(selected.id, world.x, world.z)) {
    orders += 1;
  }
  setStatus(`order move unit ${selected.id} to ${world.x.toFixed(2)},${world.z.toFixed(2)}${netState ? "" : " (local)"}`);
}

// Q, W and E cast the selected unit's ability in slot 0, 1 and 2 at the pointer, aimed at the unit under it.
// A arms an attack: the next left click on an enemy unit orders the selected unit to attack it.
const ABILITY_SLOTS = { q: 0, w: 1, e: 2 };
function handleAbilityKey(key) {
  if (!localGame || selectedId < 0) return;
  if (key === "a") {
    attackMode = true;
    return;
  }
  if (!(key in ABILITY_SLOTS)) return;
  const world = pointer ? screenToWorld(pointer.x, pointer.y) : { x: 0, z: 0 };
  const target = pick(world);
  if (localGame.cast(selectedId, ABILITY_SLOTS[key], target ? target.id : -1, world.x, world.z)) casts += 1;
}

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
    camera.cx -= dx / pxPerUnit();
    camera.cz -= dy / pxPerUnit();
    drag.x = e.clientX;
    drag.y = e.clientY;
  });
  canvas.addEventListener("pointerup", (e) => {
    const wasDrag = drag && drag.moved;
    drag = null;
    if (!wasDrag) onClick(e);
  });
  canvas.addEventListener("contextmenu", (e) => e.preventDefault());
  window.addEventListener("keydown", (e) => {
    if (e.target instanceof HTMLInputElement) return;
    if (e.key.startsWith("Arrow")) {
      keysDown.add(e.key);
      e.preventDefault();
      return;
    }
    handleAbilityKey(e.key.toLowerCase());
  });
  window.addEventListener("keyup", (e) => keysDown.delete(e.key));
  window.addEventListener("blur", () => keysDown.clear());
  window.addEventListener("pointermove", (e) => { pointer = { x: e.clientX, y: e.clientY }; });
  document.documentElement.addEventListener("pointerleave", () => { pointer = null; });
  canvas.addEventListener("wheel", (e) => {
    e.preventDefault();
    camera.zoom = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, camera.zoom * Math.exp(-e.deltaY * 0.001)));
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

function applyPeerOrder(peerPubkey, order) {
  const player = hostPlayers.get(peerPubkey);
  const unit = currentUnits().find((u) => u.id === order.unitId);
  if (player === undefined || !unit || unit.player !== player) {
    setStatus(`order refused not_owner peer=${String(peerPubkey).slice(0, 8)} unit=${order.unitId}`);
    document.body.dataset.lastRefusal = `not_owner unit=${order.unitId}`;
    return false;
  }
  localGame.order_move(order.unitId, order.x, order.y);
  return true;
}

function startHost() {
  hosting = true;
  const encoder = createStateEncoder();
  let hostTick = 0;
  setInterval(() => {
    localGame.tick();
    hostTick += 1;
    const hash = Number(localGame.state_hash() & 0xffffffffn) >>> 0;
    const units = currentUnits().map((u) => ({ id: u.id, kind: 0, player: u.player, x: Math.round(u.x * WIRE_SCALE), y: Math.round(u.z * WIRE_SCALE), hp: 0 }));
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
// ?ice=<JSON list> replaces the default STUN list. Relay test mode (?relay=) keeps no ICE servers unless ?ice is given.
const iceConfig = (() => {
  try {
    return iceServersFrom(params.get("ice"));
  } catch (err) {
    setStatus(`ice config refused: ${err.message}`);
    return iceServersFrom(null);
  }
})();
const session = createDataSession({ fsm, xstate, relayPool, auth, namespace, iceServers: relayUrls && !params.has("ice") ? [] : iceConfig.servers });

session.addEventListener("peer-open", (e) => {
  peers.add(e.detail.peerPubkey);
  if (hosting) assignPlayer(hostPlayers, e.detail.peerPubkey);
  reconnectAttempt = 0;
  setStatus(`peer-open ${String(e.detail.peerPubkey).slice(0, 8)}`);
});
session.addEventListener("peer-close", (e) => {
  peers.delete(e.detail.peerPubkey);
  setStatus(`peer-close ${String(e.detail.peerPubkey).slice(0, 8)}`);
  if (peers.size === 0) scheduleReconnect();
});
const netUnits = new Map();
let netBaseline = false;
session.addEventListener("data", (e) => {
  let frame = null;
  try { frame = decodeFrame(new Uint8Array(e.detail.data)); } catch { return; }
  if (frame.type === FRAME.ORDER) {
    if (hosting) applyPeerOrder(e.detail.peerPubkey, frame);
    return;
  }
  if (hosting) return;
  if (frame.type === FRAME.STATE) netBaseline = true;
  else if (frame.type !== FRAME.DELTA || !netBaseline) return;
  applyFrame(netUnits, frame);
  netState = { tick: frame.tick, units: [...netUnits.values()].map((u) => ({ id: u.id, x: u.x / WIRE_SCALE, z: u.y / WIRE_SCALE, player: u.player, hp: u.hp })) };
});

// Page state for headless checks: the committed map is drawn by the same Game the sim ticks.
// The minimap draws every unit as a dot at its tile position: the selected unit in white, others in their player colour.
const minimap = document.getElementById("minimap");
function drawMinimap() {
  const ctx = minimap.getContext("2d");
  ctx.fillStyle = "#2a343c";
  ctx.fillRect(0, 0, minimap.width, minimap.height);
  let dots = 0;
  for (const u of currentUnits()) {
    const c = PLAYER_COLORS[u.player] || PLAYER_COLORS[0];
    ctx.fillStyle = u.id === selectedId ? "#ffffff" : `rgb(${c.map((v) => Math.round(v * 255)).join(",")})`;
    ctx.fillRect((u.x / W) * minimap.width - 2, (u.z / H) * minimap.height - 2, 4, 4);
    dots += 1;
  }
  document.body.dataset.minimap = String(dots);
}

function publishWitness() {
  document.body.dataset.units = String(currentUnits().length);
  document.body.dataset.ticks = String(localGame ? localGame.ticks() : netState ? netState.tick : 0);
  document.body.dataset.center = `${camera.cx},${camera.cz}`;
  document.body.dataset.zoom = camera.zoom.toFixed(3);
  document.body.dataset.orders = String(orders);
  document.body.dataset.casts = String(casts);
  document.body.dataset.attacks = String(attacks);
  if (localGame) {
    const hp = selectedId >= 0 ? localGame.unit_hp(selectedId) : -1;
    const gold = localGame.gold(0);
    const winner = localGame.winner();
    document.body.dataset.hp = String(hp);
    document.body.dataset.gold = String(gold);
    document.body.dataset.winner = String(winner);
    document.getElementById("hud").textContent = `hp ${hp >= 0 ? hp : "-"}   gold ${gold}`;
    const win = document.getElementById("win");
    win.hidden = winner < 0;
    win.textContent = `side ${winner} wins`;
  }
  drawMinimap();
}

async function main() {
  resize();
  bindInput();
  const renderer = createRenderer();
  const mapMeshes = buildMapMeshes();
  localGame = await loadSim();
  if (params.get("host") === "1") startHost();
  joinRoom();

  const { gl } = renderer;
  gl.clearColor(BACKGROUND[0], BACKGROUND[1], BACKGROUND[2], 1);
  function frame(now) {
    const dtS = lastFrameMs ? Math.min(0.1, (now - lastFrameMs) / 1000) : 0;
    lastFrameMs = now;
    panCamera(dtS);
    if (!netState && localGame && !hosting) localGame.tick();
    gl.viewport(0, 0, canvas.width, canvas.height);
    gl.clear(gl.COLOR_BUFFER_BIT);
    const ppu = pxPerUnit();
    gl.uniform4f(renderer.loc.view, camera.cx, camera.cz, (2 * ppu) / window.innerWidth, (-2 * ppu) / window.innerHeight);
    draw(renderer, gl.TRIANGLES, mapMeshes.tris);
    draw(renderer, gl.LINES, mapMeshes.lines);
    draw(renderer, gl.TRIANGLES, unitVerts(currentUnits()));
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
  window.__ciUnits = () => currentUnits().map((u) => ({ id: u.id, x: u.x, z: u.z, player: u.player }));
  window.__ciOrder = (unitId, x, z) => session.broadcast(sendOrder(unitId, x, z));
  window.__ciSelected = () => selectedId;
  window.__ciSelect = (unitId) => { selectedId = unitId; };
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
import init, { Game } from "./pkg/web.js";

const TICKS_PER_SECOND = 16;
const MAX_TICKS_PER_FRAME = 256;
const MARGIN_PX = 24;
const WHEEL_ZOOM = 1.1;
const FOLLOW_RADIUS_PX = 18;
const PLAYER_COLORS = ["#3a7bd5", "#d5543a", "#3ab26b", "#b23ab2", "#d5b33a", "#3ab2b2", "#d58a3a", "#7a3ad5", "#a5a5a5", "#d53a7a"];
const NEUTRAL_PLAYER = 255;
const NEUTRAL_COLOR = "#8a8f94";
const TEXT_ENTRY = "input, select, textarea, [contenteditable]";
const KEY_HINT = "space play/pause   k pause   p play   , . step   f fit";

const params = new URLSearchParams(location.search);
const $ = (id) => document.getElementById(id);
const canvas = $("view");
const ctx = canvas.getContext("2d");
const statusEl = $("status");
const seekEl = $("seek");
const speedEl = $("speed");
const tickEl = $("tick");

const state = { game: null, end: 0, loaded: false, playing: false, seekTo: null, carry: 0, last: 0, follow: null, camera: { cx: 0, cy: 0, scale: 10 }, map: [32, 24], error: "", shownKey: "" };

const ready = () => state.loaded && !state.error;

function say(text) {
  statusEl.textContent = text;
}

function syncControls() {
  const on = ready();
  $("play").disabled = !on || state.playing;
  $("pause").disabled = !on || !state.playing;
  seekEl.disabled = !on;
}

function setPlaying(on) {
  state.playing = on && ready();
  state.carry = 0;
  state.shownKey = "";
  syncControls();
}

function fail(e) {
  state.error = String(e && e.message ? e.message : e);
  setPlaying(false);
  say("error: " + state.error);
}

function play() {
  if (!ready()) return;
  if (state.game.ticks() >= state.end) state.seekTo = 0;
  setPlaying(true);
}

function pause() {
  setPlaying(false);
}

function step(delta) {
  if (!ready()) return;
  setPlaying(false);
  const g = state.game;
  const target = Math.max(0, Math.min(state.end, g.ticks() + delta));
  if (target === g.ticks()) return;
  if (delta === 1) {
    try {
      g.replay_step(1);
    } catch (e) {
      fail(e);
    }
  } else {
    state.seekTo = target;
  }
}

function fitCamera() {
  const [w, h] = state.map;
  const room = { w: canvas.clientWidth - 2 * MARGIN_PX, h: canvas.clientHeight - 2 * MARGIN_PX };
  state.camera = { cx: w / 2, cy: h / 2, scale: Math.max(1, Math.min(room.w / w, room.h / h)) };
  state.follow = null;
}

function resize() {
  canvas.width = canvas.clientWidth * devicePixelRatio;
  canvas.height = canvas.clientHeight * devicePixelRatio;
}

function toScreen(x, y) {
  const c = state.camera;
  return [(x - c.cx) * c.scale + canvas.clientWidth / 2, (y - c.cy) * c.scale + canvas.clientHeight / 2];
}

function fromScreen(px, py) {
  const c = state.camera;
  return [(px - canvas.clientWidth / 2) / c.scale + c.cx, (py - canvas.clientHeight / 2) / c.scale + c.cy];
}

function showStatus(g, unitCount) {
  if (!state.loaded) return;
  const ticks = g.ticks();
  const winner = g.winner();
  const incomplete = g.replay_incomplete();
  const speed = speedEl.value;
  const key = [ticks, winner, incomplete, speed, state.playing, state.error].join("|");
  if (key === state.shownKey) return;
  state.shownKey = key;
  const mode = state.playing ? "playing" : ticks >= state.end ? "ended" : "paused";
  const lines = [
    `${mode}, speed ${speed}x`,
    `tick ${ticks} / ${state.end}  hash ${g.state_hash().toString(16).padStart(16, "0")}`,
    `observer: ${unitCount} units, no fog`,
  ];
  if (winner >= 0) lines.push(`winner: side ${winner} won the match`);
  if (incomplete >= 0) lines.push(`incomplete from tick ${incomplete}`);
  if (state.error) lines.push(`error: ${state.error}`);
  lines.push(KEY_HINT);
  say(lines.join("\n"));
  seekEl.value = ticks;
  tickEl.textContent = `${ticks} / ${state.end}`;
}

function draw() {
  ctx.setTransform(devicePixelRatio, 0, 0, devicePixelRatio, 0, 0);
  ctx.fillStyle = "#1d252c";
  ctx.fillRect(0, 0, canvas.clientWidth, canvas.clientHeight);
  const g = state.game;
  if (!g) return;
  const [w, h] = state.map;
  const [x0, y0] = toScreen(0, 0);
  const [x1, y1] = toScreen(w, h);
  ctx.fillStyle = "#33473a";
  ctx.fillRect(x0, y0, x1 - x0, y1 - y0);
  const flat = g.units();
  const ids = g.unit_ids();
  const hps = g.unit_hps();
  if (state.follow !== null) {
    const i = ids.indexOf(state.follow);
    if (i >= 0) {
      state.camera.cx = flat[i * 3];
      state.camera.cy = flat[i * 3 + 1];
    }
  }
  const radius = Math.max(2, state.camera.scale * 0.6);
  for (let i = 0; i < ids.length; i++) {
    const [sx, sy] = toScreen(flat[i * 3], flat[i * 3 + 1]);
    if (sx < -radius || sy < -radius || sx > canvas.clientWidth + radius || sy > canvas.clientHeight + radius) continue;
    const p = flat[i * 3 + 2];
    ctx.fillStyle = p === NEUTRAL_PLAYER ? NEUTRAL_COLOR : PLAYER_COLORS[p % PLAYER_COLORS.length];
    ctx.beginPath();
    ctx.arc(sx, sy, radius, 0, 2 * Math.PI);
    ctx.fill();
    if (ids[i] === state.follow) {
      ctx.strokeStyle = "#fff";
      ctx.stroke();
    }
    if (state.camera.scale >= 8 && hps[i] > 0) {
      ctx.fillStyle = "#fff";
      ctx.font = "10px system-ui";
      ctx.fillText(String(hps[i]), sx + radius + 2, sy + 3);
    }
  }
  showStatus(g, ids.length);
}

function frame(now) {
  const dt = Math.min(0.25, (now - state.last) / 1000);
  state.last = now;
  const g = state.game;
  if (g && ready()) {
    try {
      if (state.seekTo !== null) {
        g.replay_seek(state.seekTo);
        state.seekTo = null;
      } else if (state.playing) {
        const wonBefore = g.winner() >= 0;
        state.carry += dt * TICKS_PER_SECOND * Number(speedEl.value);
        const due = Math.min(MAX_TICKS_PER_FRAME, Math.floor(state.carry));
        state.carry -= due;
        if (due > 0) g.replay_step(due);
        // Playback stops at the first victory and at the end of the recording. A replay already past
        // its victory (reached by a seek) plays on when play is pressed, so the controls keep working.
        if (g.ticks() >= state.end || (!wonBefore && g.winner() >= 0)) setPlaying(false);
      }
    } catch (e) {
      fail(e);
    }
  }
  draw();
  requestAnimationFrame(frame);
}

async function bytesOf(url) {
  const r = await fetch(url);
  if (!r.ok) throw new Error(`${url}: ${r.status}`);
  return new Uint8Array(await r.arrayBuffer());
}

async function open(text) {
  state.error = "";
  const g = state.game;
  state.end = g.replay_load(text);
  state.map = Array.from(g.map_size());
  seekEl.max = state.end;
  state.loaded = true;
  state.seekTo = null;
  setPlaying(false);
  fitCamera();
}

async function start() {
  for (const o of speedEl.options) o.textContent = `${o.value}x`;
  syncControls();
  resize();
  const pkg = new URL(document.body.dataset.pkg, location.href).href;
  await init(pkg.replace(/web\.js$/, "web_bg.wasm"));
  const imageUrl = new URL(params.get("image") || document.body.dataset.map, location.href).href;
  state.game = new Game(await bytesOf(imageUrl));
  state.map = Array.from(state.game.map_size());
  fitCamera();
  const source = params.get("replay");
  if (source) {
    const r = await fetch(source);
    if (!r.ok) throw new Error(`${source}: ${r.status}`);
    await open(await r.text());
    setPlaying(params.get("autoplay") !== "0");
  } else {
    say("choose a .replay file (or ?replay=URL to any static .replay file)");
  }
}

$("file").addEventListener("change", async (e) => {
  const file = e.target.files[0];
  if (!file) return;
  try {
    await open(await file.text());
  } catch (err) {
    fail(err);
  }
});
$("play").addEventListener("click", play);
$("pause").addEventListener("click", pause);
seekEl.addEventListener("input", () => {
  state.seekTo = Number(seekEl.value);
});
$("fit").addEventListener("click", fitCamera);
$("free").addEventListener("click", () => {
  state.follow = null;
});

let drag = null;
canvas.addEventListener("pointerdown", (e) => {
  drag = { x: e.clientX, y: e.clientY, moved: false };
  canvas.setPointerCapture(e.pointerId);
});
canvas.addEventListener("pointermove", (e) => {
  if (!drag) return;
  const dx = e.clientX - drag.x;
  const dy = e.clientY - drag.y;
  if (Math.abs(dx) + Math.abs(dy) > 3) drag.moved = true;
  state.camera.cx -= dx / state.camera.scale;
  state.camera.cy -= dy / state.camera.scale;
  if (drag.moved) state.follow = null;
  drag.x = e.clientX;
  drag.y = e.clientY;
});
canvas.addEventListener("pointerup", (e) => {
  const click = drag && !drag.moved;
  drag = null;
  if (!click || !state.game) return;
  const flat = state.game.units();
  const ids = state.game.unit_ids();
  let best = null;
  let bestD = FOLLOW_RADIUS_PX * FOLLOW_RADIUS_PX;
  for (let i = 0; i < ids.length; i++) {
    const [sx, sy] = toScreen(flat[i * 3], flat[i * 3 + 1]);
    const d = (sx - e.offsetX) ** 2 + (sy - e.offsetY) ** 2;
    if (d < bestD) {
      bestD = d;
      best = ids[i];
    }
  }
  state.follow = best;
});
canvas.addEventListener(
  "wheel",
  (e) => {
    e.preventDefault();
    const before = fromScreen(e.offsetX, e.offsetY);
    state.camera.scale = Math.max(1, Math.min(200, state.camera.scale * (e.deltaY < 0 ? WHEEL_ZOOM : 1 / WHEEL_ZOOM)));
    if (state.follow === null) {
      const after = fromScreen(e.offsetX, e.offsetY);
      state.camera.cx += before[0] - after[0];
      state.camera.cy += before[1] - after[1];
    }
  },
  { passive: false }
);
addEventListener("resize", resize);
addEventListener("keydown", (e) => {
  if (e.ctrlKey || e.metaKey || e.altKey) return;
  if (e.target instanceof Element && e.target.closest(TEXT_ENTRY)) return;
  if (e.key === " ") {
    e.preventDefault();
    if (e.repeat) return;
    if (e.target instanceof HTMLButtonElement) e.target.blur();
    if (state.playing) pause();
    else play();
  } else if (e.key === "p") play();
  else if (e.key === "k") pause();
  else if (e.key === "ArrowRight" || e.key === ".") step(1);
  else if (e.key === "ArrowLeft" || e.key === ",") step(-1);
  else if (e.key === "f") fitCamera();
  else return;
  e.preventDefault();
});

start().then(() => requestAnimationFrame(frame)).catch(fail);

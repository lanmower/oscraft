import { buildMatrix } from "./matrix_panel.js";
const POLL_MS = 500;
const POLL_TIMEOUT_MS = 4000;
const LOG_LINES = 6;

const STYLE = `
.modui { position: fixed; z-index: 2; inset: 0; pointer-events: none; font: 14px system-ui, sans-serif; color: #fff; }
.modui-log { position: absolute; left: 8px; bottom: 180px; max-width: 40vw; }
.modui-line { margin: 2px 0; padding: 3px 8px; background: rgba(0,0,0,.6); border-radius: 4px; transition: opacity .4s; }
.modui-banner { position: absolute; left: 50%; top: 18%; transform: translateX(-50%); padding: 10px 28px; background: rgba(0,0,0,.75); border-radius: 6px; font-size: 30px; transition: opacity .4s; }
.modui-panels { position: absolute; left: 8px; top: 60px; display: flex; flex-direction: column; gap: 6px; max-width: 30vw; }
.modui-panel { padding: 6px 10px; background: rgba(0,0,0,.6); border: 1px solid rgba(255,255,255,.3); border-radius: 4px; white-space: pre-wrap; }
.modui-panel b { display: block; margin-bottom: 2px; }
@media (max-width: 600px) {
.modui-log { left: auto; right: 8px; bottom: 128px; max-width: 46vw; }
.modui-panels { top: auto; bottom: 128px; max-width: 46vw; max-height: 30vh; overflow: hidden; }
}
`;

const COLOUR_CODE = /\|c[0-9a-fA-F]{2}([0-9a-fA-F]{6})|\|r/g;

export function paintColoured(node, text) {
  node.replaceChildren();
  let colour = null;
  let from = 0;
  const put = (end) => {
    if (end <= from) return;
    const piece = text.slice(from, end);
    if (colour === null) {
      node.appendChild(node.ownerDocument.createTextNode(piece));
    } else {
      const span = node.ownerDocument.createElement("span");
      span.style.color = `#${colour}`;
      span.textContent = piece;
      node.appendChild(span);
    }
  };
  for (const m of text.matchAll(COLOUR_CODE)) {
    put(m.index);
    colour = m[1] === undefined ? null : m[1];
    from = m.index + m[0].length;
  }
  put(text.length);
}

function easeInOut(t) { return t < 0.5 ? 2 * t * t : 1 - 2 * (1 - t) ** 2; }

const CAMERA_FOLLOW_MS = 100;

const MATCH_KEY = "match";
const matchListeners = new Set();
let lobbySeen = false;
let matchObserver = null;

export function matchUnderWay() {
  const body = typeof document === "undefined" ? null : document.body;
  if (!body) return false;
  const lobby = document.getElementById("ui-modal");
  if (lobby) lobbySeen = true;
  if (body.dataset[MATCH_KEY] === "1" || body.dataset.seated !== undefined) return true;
  return lobbySeen && !lobby;
}

export function setMatchUnderWay(on) {
  const body = typeof document === "undefined" ? null : document.body;
  if (body) body.dataset[MATCH_KEY] = on ? "1" : "0";
}

export function watchMatchStart(onChange) {
  if (typeof onChange === "function") matchListeners.add(onChange);
  if (!matchObserver && typeof document !== "undefined" && document.body) {
    let last = matchUnderWay();
    matchObserver = new MutationObserver(() => {
      const now = matchUnderWay();
      if (now === last) return;
      last = now;
      for (const fn of matchListeners) fn(now);
    });
    matchObserver.observe(document.body, { childList: true, attributes: true, attributeFilter: ["data-match", "data-seated"] });
  }
  return () => { if (typeof onChange === "function") matchListeners.delete(onChange); };
}

export function createModUi(opts) {
  const { root, renderer, localPlayer, defaultDistance, baseUrl, onSound, onBind } = opts;
  const doc = root.ownerDocument;
  const style = doc.createElement("style");
  style.textContent = STYLE;
  const el = doc.createElement("div");
  el.className = "modui";
  el.innerHTML = '<div class="modui-panels" aria-live="polite"></div><div class="modui-log" role="log"></div>';
  el.prepend(style);
  root.appendChild(el);
  const logEl = el.querySelector(".modui-log");
  const panelsEl = el.querySelector(".modui-panels");
  const panels = new Map();
  const timers = new Set();
  let bannerEl = null;
  let move = null;
  let lastCameraAt = -Infinity;
  let since = 0;
  let poller = null;

  const mine = (ev) => ev.player === undefined || ev.player < 0 || ev.player === localPlayer();

  const applyMatchGate = (underWay) => {
    const button = doc.getElementById("pack-open");
    if (button) button.hidden = underWay;
  };
  const stopMatchWatch = watchMatchStart(applyMatchGate);
  applyMatchGate(matchUnderWay());

  function later(fn, seconds) {
    const h = setTimeout(() => { timers.delete(h); fn(); }, seconds * 1000);
    timers.add(h);
  }

  function fading(node, seconds) {
    later(() => { node.style.opacity = "0"; later(() => node.remove(), 0.4); }, seconds);
  }

  const handlers = {
    message(ev) {
      const line = doc.createElement("div");
      line.className = "modui-line";
      paintColoured(line, String(ev.text ?? ""));
      logEl.appendChild(line);
      while (logEl.childNodes.length > LOG_LINES) logEl.firstChild.remove();
      if (ev.seconds > 0) fading(line, ev.seconds);
    },
    banner(ev) {
      if (bannerEl) bannerEl.remove();
      bannerEl = doc.createElement("div");
      bannerEl.className = "modui-banner";
      bannerEl.setAttribute("role", "status");
      bannerEl.textContent = ev.text;
      el.appendChild(bannerEl);
      fading(bannerEl, ev.seconds);
    },
    panel(ev) {
      const old = panels.get(ev.id);
      if (ev.remove) {
        if (old) old.remove();
        panels.delete(ev.id);
        return;
      }
      const node = old || doc.createElement("div");
      node.className = "modui-panel";
      node.replaceChildren();
      if (ev.title) {
        const t = doc.createElement("b");
        paintColoured(t, ev.title);
        node.appendChild(t);
      }
      node.appendChild(doc.createTextNode(ev.text || ""));
      if (Array.isArray(ev.rows)) node.appendChild(buildMatrix(doc, ev, paintColoured));
      if (!old) { panelsEl.appendChild(node); panels.set(ev.id, node); }
    },
    camera(ev) {
      const r = renderer();
      if (!r) return;
      const cam = r.camera;
      const to = { target: [ev.x, cam.target[1], ev.y], distance: Math.min(cam.maxDistance, Math.max(cam.minDistance, ev.zoom === 1 ? cam.distance : defaultDistance() / ev.zoom)) };
      const now = performance.now();
      const dragging = now - lastCameraAt < CAMERA_FOLLOW_MS;
      lastCameraAt = now;
      if (dragging) { move = null; r.setCamera(to); return; }
      move = { from: { target: [...cam.target], distance: cam.distance }, to, elapsed: 0, seconds: ev.seconds };
      if (!(ev.seconds > 0)) step(0);
    },
    camera_distance(ev) {
      const r = renderer();
      if (!r) return;
      const cam = r.camera;
      const to = { target: [...cam.target], distance: Math.min(cam.maxDistance, Math.max(cam.minDistance, defaultDistance() / ev.zoom)) };
      move = { from: { target: [...cam.target], distance: cam.distance }, to, elapsed: 0, seconds: ev.seconds };
      if (!(ev.seconds > 0)) step(0);
    },
    camera_lock(ev) {
      const r = renderer();
      if (r) r.setCamera({ yawLocked: ev.locked === true });
    },
    sound(ev) { if (onSound) onSound(ev.name); },
    ping(ev) {
      const d = ev.data || {};
      if (d.player !== undefined && d.player !== localPlayer()) return;
      doc.defaultView.dispatchEvent(new CustomEvent("moba:ping", { detail: d }));
    },
    bind(ev) { if (onBind) onBind({ key: ev.key, action: ev.action }); },
    unbind(ev) { if (onBind) onBind({ key: ev.key, remove: true }); },
  };

  function step(dtS) {
    if (!move) return;
    const r = renderer();
    if (!r) { move = null; return; }
    move.elapsed += dtS;
    const t = move.seconds > 0 ? Math.min(1, move.elapsed / move.seconds) : 1;
    const k = easeInOut(t);
    const { from, to } = move;
    r.setCamera({
      target: from.target.map((v, i) => v + (to.target[i] - v) * k),
      distance: from.distance + (to.distance - from.distance) * k,
    });
    if (t >= 1) move = null;
  }

  function push(ev) {
    if (!ev || !mine(ev)) return;
    const handler = handlers[ev.kind];
    if (handler) handler(ev);
  }

  let inFlight = null;
  function poll() {
    if (inFlight) return inFlight;
    inFlight = fetchEvents().finally(() => { inFlight = null; });
    return inFlight;
  }
  async function fetchEvents() {
    try {
      const res = await fetch(`${baseUrl}/events?since=${since}`, { signal: AbortSignal.timeout(POLL_TIMEOUT_MS) });
      if (!res.ok) return;
      const body = await res.json();
      if (body.seq < since) since = 0;
      for (const ev of body.events) {
        since = Math.max(since, ev.id);
        push(ev);
      }
    } catch {  }
  }

  async function pollLoop() {
    await poll();
    if (poller !== null) poller = setTimeout(pollLoop, POLL_MS);
  }
  if (baseUrl !== undefined) { poller = setTimeout(pollLoop, POLL_MS); }

  function dispose() {
    stopMatchWatch();
    if (poller) clearTimeout(poller);
    poller = null;
    for (const h of timers) clearTimeout(h);
    el.remove();
  }

  return { push, step, poll, dispose };
}

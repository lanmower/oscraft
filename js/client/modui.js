// Mod UI overlay: message log, banners, text panels, camera moves and sound events from the mod host.
// Events arrive by polling the server's GET /events?since=<id> (or push() from an in-browser host) and
// carry {id, tick, kind, ...}. A player of -1 addresses every client; otherwise only that player's.
const POLL_MS = 500;
const LOG_LINES = 6;

const STYLE = `
.modui { position: fixed; z-index: 2; inset: 0; pointer-events: none; font: 14px system-ui, sans-serif; color: #fff; }
.modui-log { position: absolute; left: 8px; bottom: 60px; max-width: 40vw; }
.modui-line { margin: 2px 0; padding: 3px 8px; background: rgba(0,0,0,.6); border-radius: 4px; transition: opacity .4s; }
.modui-banner { position: absolute; left: 50%; top: 18%; transform: translateX(-50%); padding: 10px 28px; background: rgba(0,0,0,.75); border-radius: 6px; font-size: 30px; transition: opacity .4s; }
.modui-panels { position: absolute; left: 8px; top: 60px; display: flex; flex-direction: column; gap: 6px; max-width: 30vw; }
.modui-panel { padding: 6px 10px; background: rgba(0,0,0,.6); border: 1px solid rgba(255,255,255,.3); border-radius: 4px; white-space: pre-wrap; }
.modui-panel b { display: block; margin-bottom: 2px; }
`;

function easeInOut(t) { return t < 0.5 ? 2 * t * t : 1 - 2 * (1 - t) ** 2; }

// opts: root (parent element), renderer() (the render3d renderer or null), localPlayer() (number),
// defaultDistance() (camera distance at zoom 1), baseUrl (server origin, optional), onSound(name) (optional).
export function createModUi(opts) {
  const { root, renderer, localPlayer, defaultDistance, baseUrl, onSound } = opts;
  const doc = root.ownerDocument;
  const style = doc.createElement("style");
  style.textContent = STYLE;
  const el = doc.createElement("div");
  el.className = "modui";
  el.innerHTML = '<div class="modui-panels"></div><div class="modui-log"></div>';
  el.prepend(style);
  root.appendChild(el);
  const logEl = el.querySelector(".modui-log");
  const panelsEl = el.querySelector(".modui-panels");
  const panels = new Map();
  const timers = new Set();
  let bannerEl = null;
  let move = null;
  let since = 0;
  let poller = null;

  const mine = (ev) => ev.player === undefined || ev.player < 0 || ev.player === localPlayer();

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
      line.textContent = ev.text;
      logEl.appendChild(line);
      while (logEl.childNodes.length > LOG_LINES) logEl.firstChild.remove();
      if (ev.seconds > 0) fading(line, ev.seconds);
    },
    banner(ev) {
      if (bannerEl) bannerEl.remove();
      bannerEl = doc.createElement("div");
      bannerEl.className = "modui-banner";
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
        t.textContent = ev.title;
        node.appendChild(t);
      }
      node.appendChild(doc.createTextNode(ev.text || ""));
      if (!old) { panelsEl.appendChild(node); panels.set(ev.id, node); }
    },
    camera(ev) {
      const r = renderer();
      if (!r) return;
      const cam = r.camera;
      const to = { target: [ev.x, cam.target[1], ev.y], distance: Math.min(cam.maxDistance, Math.max(cam.minDistance, defaultDistance() / ev.zoom)) };
      move = { from: { target: [...cam.target], distance: cam.distance }, to, elapsed: 0, seconds: ev.seconds };
      if (!(ev.seconds > 0)) step(0);
    },
    sound(ev) { if (onSound) onSound(ev.name); },
    // Key bindings a mod registers at runtime: the input layer (moba_input.js) listens for these on window.
    bind(ev) { doc.defaultView.dispatchEvent(new doc.defaultView.CustomEvent("moba:bind", { detail: { key: ev.key, action: ev.action } })); },
    unbind(ev) { doc.defaultView.dispatchEvent(new doc.defaultView.CustomEvent("moba:bind", { detail: { key: ev.key, remove: true } })); },
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

  async function poll() {
    try {
      const res = await fetch(`${baseUrl}/events?since=${since}`);
      if (!res.ok) return;
      const body = await res.json();
      if (body.seq < since) since = 0;
      for (const ev of body.events) {
        since = Math.max(since, ev.id);
        push(ev);
      }
    } catch { /* the server is away; the next poll retries */ }
  }

  if (baseUrl !== undefined) poller = setInterval(poll, POLL_MS);

  function dispose() {
    if (poller) clearInterval(poller);
    for (const h of timers) clearTimeout(h);
    el.remove();
  }

  return { push, step, poll, dispose };
}

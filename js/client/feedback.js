// Visual feedback for MOBA orders: click markers (move green, attack-move red), attack rings on a target, the targeted-cast
// reticle with its area ring, and a refusal line. Events arrive on window as "moba:feedback" CustomEvents. Every element is
// created once, pooled under document.body, and never takes pointer input. draw() reads no DOM and writes a style property
// only when its value changed. The layer is on unless window.__mobaFeedback.enabled is false or window.__fxOff is true.

const MARKER_POOL = 12;
const RING_POOL = 4;
const MARKER_MS = 500;
const RING_MS = 2500;
const REFUSAL_MS = 1200;
const MARKER_PX = 28;
const RING_PX = 40;
const COLOUR = { move: "#4caf50", attackMove: "#e53935", attack: "#e53935", reticle: "#ffeb3b" };

const OVERLAY_CSS = "position:fixed;left:0;top:0;display:none;pointer-events:none;box-sizing:border-box;transform:translate(-50%,-50%);";

function overlay(ui, css) {
  const node = document.createElement("div");
  node.dataset.ui = ui;
  node.style.cssText = OVERLAY_CSS + css;
  document.body.appendChild(node);
  return { node, cache: {}, t0: -1, wx: 0, wz: 0, targetId: -1 };
}

// Writes one style property only when its value differs from the last write.
function put(item, prop, value) {
  if (item.cache[prop] === value) return;
  item.cache[prop] = value;
  item.node.style[prop] = value;
}

function place(item, screen) {
  put(item, "transform", `translate(${Number(screen.x).toFixed(2)}px,${Number(screen.y).toFixed(2)}px) translate(-50%,-50%)`);
}

function ringCss(px, colour) {
  return `width:${px}px;height:${px}px;border:2px solid ${colour};border-radius:50%;`;
}

export function createFeedback({ toScreen, toWorld, units, heroOf }) {
  const now = () => performance.now();
  const markers = Array.from({ length: MARKER_POOL }, () => overlay("fx", ringCss(MARKER_PX, COLOUR.move) + "background:transparent;"));
  const rings = Array.from({ length: RING_POOL }, () => overlay("fx", ringCss(RING_PX, COLOUR.attack)));
  const reticle = overlay("reticle", `width:24px;height:24px;border:2px dashed ${COLOUR.reticle};border-radius:4px;color:${COLOUR.reticle};font:700 12px sans-serif;`);
  const area = overlay("area-ring", `border:2px solid ${COLOUR.reticle};border-radius:50%;background:rgba(255,235,59,0.08);`);
  const refusal = overlay("refusal", "transform:translate(-50%,0);background:rgba(0,0,0,0.7);color:#fff;font:600 14px sans-serif;padding:6px 12px;border-radius:4px;white-space:nowrap;");
  refusal.node.style.bottom = "12%";
  refusal.node.style.top = "";
  refusal.node.style.left = "50%";

  let nextMarker = 0;
  let nextRing = 0;
  let wasOn = false;
  const armed = { on: false, slot: -1, radius: 0, targetKind: "", client: null, world: null };
  let refusalT0 = -1;

  let frameUnits = null;

  function unitAt(id) {
    if (heroOf) {
      const h = heroOf(id);
      if (h) return h;
    }
    if (!frameUnits) frameUnits = units();
    return frameUnits.find((u) => u.id === id);
  }

  function onEvent(d) {
    if (!d || typeof d.kind !== "string") return;
    const t = now();
    if (d.kind === "move" || d.kind === "attackMove") {
      for (const o of markers) o.pinned = false;
      const m = markers[nextMarker];
      nextMarker = (nextMarker + 1) % MARKER_POOL;
      m.t0 = t;
      m.wx = d.x;
      m.wz = d.z;
      m.pinned = d.kind === "attackMove" && Array.isArray(d.ids) && d.ids.length > 0;
      m.ids = m.pinned ? d.ids : null;
      m.node.style.borderColor = d.kind === "move" ? COLOUR.move : COLOUR.attackMove;
      m.node.style.borderRadius = d.kind === "move" ? "50%" : "0";
    } else if (d.kind === "attack") {
      for (const o of markers) o.pinned = false;
      const r = rings[nextRing];
      nextRing = (nextRing + 1) % RING_POOL;
      r.t0 = t;
      r.targetId = d.targetId;
      r.wx = d.x;
      r.wz = d.z;
    } else if (d.kind === "targetArm") {
      armed.on = true;
      armed.slot = d.slot;
      armed.radius = d.radius > 0 ? d.radius : 0;
      armed.targetKind = d.targetKind || "";
      if (!armed.client) armed.client = { x: window.innerWidth / 2, y: window.innerHeight / 2 };
      reticle.node.textContent = String(d.slot);
    } else if (d.kind === "targetEnd") {
      armed.on = false;
      armed.world = null;
    } else if (d.kind === "refusal") {
      if (!enabled()) return;
      refusal.node.textContent = String(d.text);
      refusalT0 = t;
      // Shown now, not on the next frame: a slow frame loop can otherwise miss the whole 1.2 s window.
      put(refusal, "display", "block");
    }
  }

  window.addEventListener("moba:feedback", (e) => onEvent(e.detail));
  window.addEventListener("pointermove", (e) => {
    armed.client = { x: e.clientX, y: e.clientY };
  });

  if (!window.__mobaFeedback) window.__mobaFeedback = { enabled: true };

  function enabled() {
    return window.__mobaFeedback.enabled !== false && window.__fxOff !== true;
  }

  function hideAll() {
    for (const item of [...markers, ...rings, reticle, area, refusal]) put(item, "display", "none");
  }

  // An attack-move marker stays up while any ordered unit is still short of its point, and is replaced by the next order.
  function pinDone(m) {
    return m.ids.every((id) => {
      const u = unitAt(id);
      return !u || Math.hypot(u.x - m.wx, u.z - m.wz) <= 1;
    });
  }

  function drawMarkers(t) {
    for (const m of markers) {
      const age = t - m.t0;
      const pinned = m.pinned && !pinDone(m);
      if (m.t0 < 0 || (!pinned && age >= MARKER_MS)) {
        put(m, "display", "none");
        m.pinned = false;
        continue;
      }
      const p = toScreen(m.wx, m.wz, 0);
      put(m, "display", "block");
      put(m, "opacity", pinned ? "1" : String(1 - age / MARKER_MS));
      place(m, p);
    }
  }

  function drawRings(t) {
    for (const r of rings) {
      const age = t - r.t0;
      if (r.t0 < 0 || age >= RING_MS) {
        put(r, "display", "none");
        continue;
      }
      const u = unitAt(r.targetId);
      const p = u ? toScreen(u.x, u.z, u.top || 0) : toScreen(r.wx, r.wz, 0);
      put(r, "display", "block");
      put(r, "opacity", String(1 - age / RING_MS));
      place(r, p);
    }
  }

  function drawTargeting() {
    if (!armed.on || !armed.client) {
      put(reticle, "display", "none");
      put(area, "display", "none");
      return;
    }
    put(reticle, "display", "block");
    place(reticle, { x: armed.client.x + 16, y: armed.client.y + 16 });
    if (armed.radius <= 0) {
      put(area, "display", "none");
      return;
    }
    const w = toWorld(armed.client.x, armed.client.y);
    const c = toScreen(w.x, w.z, 0);
    const e = toScreen(w.x + armed.radius, w.z, 0);
    const px = Math.hypot(e.x - c.x, e.y - c.y);
    if (!Number.isFinite(px)) {
      put(area, "display", "none");
      return;
    }
    put(area, "display", "block");
    put(area, "width", `${(2 * px).toFixed(2)}px`);
    put(area, "height", `${(2 * px).toFixed(2)}px`);
    place(area, c);
  }

  function drawRefusal(t) {
    const live = refusalT0 >= 0 && t - refusalT0 < REFUSAL_MS;
    put(refusal, "display", live ? "block" : "none");
  }

  function draw() {
    frameUnits = null;
    const on = enabled();
    if (!on) {
      if (wasOn) hideAll();
      wasOn = false;
      return;
    }
    wasOn = true;
    const t = now();
    drawMarkers(t);
    drawRings(t);
    drawTargeting();
    drawRefusal(t);
  }

  function destroy() {
    for (const item of [...markers, ...rings, reticle, area, refusal]) item.node.remove();
  }

  function setEnabled(value) {
    window.__mobaFeedback.enabled = value === true;
  }

  return { draw, destroy, setEnabled };
}

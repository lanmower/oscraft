import { PING_GLYPHS, WHEEL_KINDS, pingGlyphSvg } from "./ping_glyphs.js";

export const WHEEL_KEYS = Object.freeze(["g", "v"]);

const RING_PX = 64;
const DEAD_PX = 14;
const ICON_PX = 28;
const SEGMENT = (2 * Math.PI) / WHEEL_KINDS.length;
const DIM_OPACITY = "0.55";

export function wheelKindAt(dx, dy) {
  if (Math.hypot(dx, dy) < DEAD_PX) return null;
  const angle = (Math.atan2(dx, -dy) + 2 * Math.PI) % (2 * Math.PI);
  return WHEEL_KINDS[Math.round(angle / SEGMENT) % WHEEL_KINDS.length];
}

export function createPingWheel() {
  const root = document.createElement("div");
  root.dataset.ui = "ping-wheel";
  root.style.cssText = "position:fixed;left:0;top:0;width:0;height:0;display:none;pointer-events:none;z-index:40;color:#fff;";

  const icons = WHEEL_KINDS.map((kind, i) => {
    const angle = i * SEGMENT;
    const node = document.createElement("div");
    const x = Math.sin(angle) * RING_PX;
    const y = -Math.cos(angle) * RING_PX;
    node.style.cssText = `position:absolute;width:${ICON_PX}px;height:${ICON_PX}px;left:${x - ICON_PX / 2}px;top:${y - ICON_PX / 2}px;display:flex;align-items:center;justify-content:center;`;
    node.innerHTML = pingGlyphSvg(kind);
    node.setAttribute("role", "img");
    node.setAttribute("aria-label", PING_GLYPHS[kind].name);
    node.title = PING_GLYPHS[kind].name;
    root.appendChild(node);
    return node;
  });
  document.body.appendChild(root);

  let shown = false;
  let centre = { x: 0, y: 0 };

  function paint(kind) {
    icons.forEach((node, i) => {
      node.style.opacity = WHEEL_KINDS[i] === kind ? "1" : DIM_OPACITY;
    });
  }

  function onMove(e) {
    if (shown) paint(wheelKindAt(e.clientX - centre.x, e.clientY - centre.y));
  }
  window.addEventListener("pointermove", onMove);

  function open(x, y) {
    centre = { x, y };
    shown = true;
    root.style.left = `${x}px`;
    root.style.top = `${y}px`;
    root.style.display = "block";
    paint(null);
  }

  function close() {
    shown = false;
    root.style.display = "none";
  }

  function pickAt(x, y) {
    return shown ? wheelKindAt(x - centre.x, y - centre.y) : null;
  }

  function destroy() {
    window.removeEventListener("pointermove", onMove);
    root.remove();
  }

  return { open, close, pickAt, isOpen: () => shown, destroy };
}

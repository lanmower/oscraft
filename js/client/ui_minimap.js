// Minimap, the right block of the bottom bar: the map's walkable layer (lanes light, rock and water dark), units, buildings
// and the camera's ground rectangle. Own units are white and the selected unit is yellow, as in the world view; allies are their player colour with a ring; enemies
// their player colour; unowned (neutral) units a fixed grey. Buildings are plain squares, smaller than a hero dot. The panel repaints at most
// every REDRAW_MS, and only when a unit, the camera, the selection or the canvas size changed. A press or drag on the
// minimap moves the camera to that map point.
import { teamOf } from "./moba_select.js";

const SIZE = { hero: 8, own: 5, creep: 3, building: 7 };
const NEUTRAL_PLAYER = 255;
const NEUTRAL_FILL = "#c2c8ce";
const OWN_FILL = "#ffffff";
const SELECTED_FILL = "#f2d84c";
const ALLY_RING = "#7cc7ff";
const HERO_OUTLINE = "#0b0f13";
const CAMERA_UNDER = "rgba(0,0,0,.7)";
const CAMERA_FILL = "#ffd54f";
const FRAME = "#3b4752";
const BACKGROUND = "#1a2129";
const REDRAW_MS = 1000 / 30;
const HERO_BIT = 1;
const BUILDING_BIT = 2;

function sizeFor(own, hero) {
  if (hero) return SIZE.hero;
  return own ? SIZE.own : SIZE.creep;
}

export function dotSize({ own, hero }) {
  return sizeFor(own, hero);
}

function clamp01(f) {
  return f < 0 ? 0 : f > 1 ? 1 : f;
}

function mix(h, v) {
  return Math.imul(h ^ (v | 0), 16777619);
}

export function createMinimap({ canvas, mapSize, lanes, units, selected, localPlayer, playerColor, teams, viewRect, centre, isHero = () => false, isBuilding = () => false }) {
  hot.set("ui-minimap", { canvas, mapSize, lanes, units, selected, localPlayer, playerColor, teams, viewRect, centre, isHero, isBuilding });
  canvas.dataset.ui = "minimap";
  LIVE_CANVAS_DISPOSERS.get(canvas)?.();
  let fittedWidth = -1;
  function fit() {
    if (canvas.width === fittedWidth) return;
    fittedWidth = canvas.width;
    canvas.height = Math.max(1, Math.round((canvas.width * mapSize.h) / mapSize.w));
    canvas.style.height = `${canvas.height}px`;
  }
  fit();
  const ctx = canvas.getContext("2d");
  const fillByPlayer = new Map();
  const kindByType = new Map();
  let lastSig = NaN;
  let lastPaintAt = -Infinity;
  let lastCount = 0;

  function playerFill(player) {
    let fill = fillByPlayer.get(player);
    if (fill === undefined) {
      const [r, g, b] = playerColor(player);
      fill = `rgb(${(r * 255) | 0},${(g * 255) | 0},${(b * 255) | 0})`;
      fillByPlayer.set(player, fill);
    }
    return fill;
  }

  // Hero-ness and building-ness are properties of a unit type, so they are read once per type. Units without a type are read per id.
  function kindOf(u) {
    const known = u.type === undefined ? undefined : kindByType.get(u.type);
    if (known !== undefined) return known;
    const flags = (isHero(u.id) ? HERO_BIT : 0) | (isBuilding(u.id) ? BUILDING_BIT : 0);
    if (u.type !== undefined) kindByType.set(u.type, flags);
    return flags;
  }

  function fillFor(u, me, active) {
    if (u.id === active) return SELECTED_FILL;
    if (u.player === NEUTRAL_PLAYER) return NEUTRAL_FILL;
    if (u.player === me) return OWN_FILL;
    return playerFill(u.player);
  }

  function signature(list, view, me, active) {
    let h = list.length;
    h = mix(h, me);
    h = mix(h, active);
    h = mix(h, canvas.width);
    h = mix(h, canvas.height);
    if (view) {
      h = mix(h, view.x0 * 8);
      h = mix(h, view.z0 * 8);
      h = mix(h, view.x1 * 8);
      h = mix(h, view.z1 * 8);
    }
    for (let i = 0; i < list.length; i++) {
      const u = list[i];
      h = mix(h, u.id);
      h = mix(h, u.x * 8);
      h = mix(h, u.z * 8);
      h = mix(h, u.player);
    }
    return h;
  }

  function paint(list, view, me, active) {
    ctx.fillStyle = BACKGROUND;
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    if (lanes) ctx.drawImage(lanes, 0, 0, canvas.width, canvas.height);
    const sx = canvas.width / mapSize.w;
    const sy = canvas.height / mapSize.h;
    const myTeam = teamOf(teams, me);
    const s = SIZE.building;
    for (let i = 0; i < list.length; i++) {
      const u = list[i];
      if (!(kindOf(u) & BUILDING_BIT)) continue;
      ctx.fillStyle = fillFor(u, me, active);
      ctx.fillRect(u.x * sx - s / 2, u.z * sy - s / 2, s, s);
    }
    for (let i = 0; i < list.length; i++) {
      const u = list[i];
      const flags = kindOf(u);
      if (flags & BUILDING_BIT) continue;
      const neutral = u.player === NEUTRAL_PLAYER;
      const own = !neutral && u.player === me;
      const ally = !neutral && !own && myTeam >= 0 && teamOf(teams, u.player) === myTeam;
      const size = sizeFor(own, (flags & HERO_BIT) !== 0);
      const x = u.x * sx;
      const y = u.z * sy;
      ctx.fillStyle = fillFor(u, me, active);
      ctx.fillRect(x - size / 2, y - size / 2, size, size);
      if (flags & HERO_BIT) {
        ctx.strokeStyle = HERO_OUTLINE;
        ctx.lineWidth = 1;
        ctx.strokeRect(x - size / 2 - 0.5, y - size / 2 - 0.5, size + 1, size + 1);
      }
      if (ally) {
        ctx.strokeStyle = ALLY_RING;
        ctx.lineWidth = 1;
        ctx.strokeRect(x - 4, y - 4, 8, 8);
      }
    }
    if (view) {
      const x = view.x0 * sx;
      const y = view.z0 * sy;
      const w = (view.x1 - view.x0) * sx;
      const h = (view.z1 - view.z0) * sy;
      ctx.lineWidth = 3;
      ctx.strokeStyle = CAMERA_UNDER;
      ctx.strokeRect(x, y, w, h);
      ctx.lineWidth = 1;
      ctx.strokeStyle = CAMERA_FILL;
      ctx.strokeRect(x, y, w, h);
    }
    ctx.strokeStyle = FRAME;
    ctx.lineWidth = 1;
    ctx.strokeRect(0.5, 0.5, canvas.width - 1, canvas.height - 1);
  }

  function draw(list) {
    const now = performance.now();
    if (now - lastPaintAt < REDRAW_MS) return lastCount;
    lastPaintAt = now;
    fit();
    const me = localPlayer();
    const active = selected();
    const view = viewRect();
    const sig = signature(list, view, me, active);
    if (sig === lastSig) return lastCount;
    lastSig = sig;
    paint(list, view, me, active);
    lastCount = list.length;
    return lastCount;
  }

  // The canvas's content box, so the mapping holds at any CSS size and with a border; the result is clamped to the map.
  function centreAt(e) {
    const r = canvas.getBoundingClientRect();
    const w = r.width - 2 * canvas.clientLeft;
    const h = r.height - 2 * canvas.clientTop;
    if (w <= 0 || h <= 0) return;
    const fx = clamp01((e.clientX - r.left - canvas.clientLeft) / w);
    const fz = clamp01((e.clientY - r.top - canvas.clientTop) / h);
    centre(fx * mapSize.w, fz * mapSize.h);
  }

  let pressed = false;
  const onPointerDown = (e) => {
    pressed = true;
    e.stopPropagation();
    centreAt(e);
  };
  const onPointerMove = (e) => {
    if (pressed) centreAt(e);
  };
  const onPointerEnd = () => { pressed = false; };
  canvas.addEventListener("pointerdown", onPointerDown);
  canvas.addEventListener("pointermove", onPointerMove);
  window.addEventListener("pointerup", onPointerEnd);
  window.addEventListener("pointercancel", onPointerEnd);
  LIVE_CANVAS_DISPOSERS.set(canvas, () => {
    canvas.removeEventListener("pointerdown", onPointerDown);
    canvas.removeEventListener("pointermove", onPointerMove);
    window.removeEventListener("pointerup", onPointerEnd);
    window.removeEventListener("pointercancel", onPointerEnd);
  });

  return { draw };
}

// Hot-update hook (see hmr.js): re-runs createMinimap on the inputs of its last call. The canvas is kept; the returned
// panel's draw() replaces the one hud.js holds.
const hot = (globalThis.__oscraftHot ??= new Map());
// Keyed by canvas and kept on globalThis, so a hot-reloaded module instance removes the listeners the previous one added.
const LIVE_CANVAS_DISPOSERS = (globalThis.__oscraftMinimapDisposers ??= new WeakMap());
export function hmrReload() {
  const args = hot.get("ui-minimap");
  return args ? createMinimap(args) : null;
}

import { teamOf } from "./moba_select.js";
import { pingKindOf } from "./moba_input.js";
import { pingGlyph } from "./ping_glyphs.js";

const SIZE = { hero: 8, own: 5, creep: 3, building: 7 };
const NEUTRAL_PLAYER = 255;
const NEUTRAL_FILL = "#c2c8ce";
const OWN_FILL = "#ffffff";
const SELECTED_FILL = "#f2d84c";
const ALLY_RING = "#7cc7ff";
const HERO_OUTLINE = "#0b0f13";
const ENEMY_CROSS = "#0b0f13";
const CAMERA_UNDER = "rgba(0,0,0,.7)";
const CAMERA_FILL = "#ffd54f";
const FRAME = "#3b4752";
const BACKGROUND = "#1a2129";
const REDRAW_MS = 1000 / 30;
const DRAG_SLOP_PX = 4;
const PING_RING_PX = 16;
const PING_RING_MS = 700;
const ROUTE_DASH = [3, 3];
const SOLID = [];
const HERO_BIT = 1;
const BUILDING_BIT = 2;

function sizeFor(own, hero) {
  if (hero) return SIZE.hero;
  return own ? SIZE.own : SIZE.creep;
}

function clamp01(f) {
  return f < 0 ? 0 : f > 1 ? 1 : f;
}

function mix(h, v) {
  return Math.imul(h ^ (v | 0), 16777619);
}

function createHitLayer(win) {
  const regions = new WeakMap();
  let active = null;
  win.addEventListener("pointerdown", (e) => {
    const region = regions.get(e.target);
    if (region === undefined) return;
    e.stopPropagation();
    active = region;
    region.down(e);
  }, true);
  win.addEventListener("pointermove", (e) => {
    if (active !== null) active.move(e);
  }, true);
  win.addEventListener("pointerup", (e) => {
    const region = active;
    active = null;
    if (region !== null) region.up(e);
  }, true);
  win.addEventListener("pointercancel", () => {
    const region = active;
    active = null;
    if (region !== null) region.cancel();
  }, true);
  win.addEventListener("contextmenu", (e) => {
    if (regions.has(e.target)) e.preventDefault();
  }, true);
  return {
    add(canvas, region) {
      regions.set(canvas, region);
    },
    remove(canvas, region) {
      if (regions.get(canvas) === region) regions.delete(canvas);
    },
  };
}

function createMinimapImpl({ canvas, mapSize, lanes, selected, localPlayer, playerColor, teams, viewRect, centre, drag = () => {}, order = () => {}, isHero = () => false, isBuilding = () => false, palette = {} }) {
  hot.set("ui-minimap", { canvas, mapSize, lanes, selected, localPlayer, playerColor, teams, viewRect, centre, drag, order, isHero, isBuilding, palette });
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
  const pings = [];
  const onPing = (e) => {
    const d = e.detail || {};
    if (!Number.isFinite(d.x) || !Number.isFinite(d.z)) return;
    const from = d.from && Number.isFinite(d.from.x) && Number.isFinite(d.from.z) ? d.from : null;
    pings.push({
      x: d.x,
      z: d.z,
      from,
      kind: typeof d.kind === "string" ? d.kind : "ping",
      until: performance.now() + (d.seconds > 0 ? d.seconds : 3) * 1000,
      colour: typeof d.colour === "string" ? d.colour : "#ffffff",
    });
  };
  window.addEventListener("moba:ping", onPing);

  function paintPingGlyph(glyph, x, y) {
    ctx.beginPath();
    if (glyph.points) {
      glyph.points.forEach(([gx, gy], i) => {
        if (i === 0) ctx.moveTo(x + gx - 8, y + gy - 8);
        else ctx.lineTo(x + gx - 8, y + gy - 8);
      });
      ctx.closePath();
    } else {
      ctx.arc(x, y, glyph.circle, 0, Math.PI * 2);
    }
    ctx.fill();
  }

  function paintPings(now, sx, sy) {
    for (let i = pings.length - 1; i >= 0; i--) {
      if (pings[i].until <= now) pings.splice(i, 1);
    }
    for (const p of pings) {
      const phase = (now % PING_RING_MS) / PING_RING_MS;
      const x = p.x * sx;
      const y = p.z * sy;
      ctx.strokeStyle = p.colour;
      ctx.fillStyle = p.colour;
      ctx.lineWidth = 2;
      if (p.from) {
        ctx.setLineDash(ROUTE_DASH);
        ctx.beginPath();
        ctx.moveTo(p.from.x * sx, p.from.z * sy);
        ctx.lineTo(x, y);
        ctx.stroke();
        ctx.setLineDash(SOLID);
      }
      ctx.beginPath();
      ctx.arc(x, y, 2 + phase * PING_RING_PX, 0, Math.PI * 2);
      ctx.stroke();
      paintPingGlyph(pingGlyph(p.kind), x, y);
    }
  }

  function playerFill(player) {
    let fill = fillByPlayer.get(player);
    if (fill === undefined) {
      const [r, g, b] = playerColor(player);
      fill = `rgb(${(r * 255) | 0},${(g * 255) | 0},${(b * 255) | 0})`;
      fillByPlayer.set(player, fill);
    }
    return fill;
  }

  function kindOf(u) {
    const known = u.type === undefined ? undefined : kindByType.get(u.type);
    if (known !== undefined) return known;
    const flags = (isHero(u) ? HERO_BIT : 0) | (isBuilding(u) ? BUILDING_BIT : 0);
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
      const x = u.x * sx;
      const y = u.z * sy;
      ctx.fillStyle = fillFor(u, me, active);
      ctx.fillRect(x - s / 2, y - s / 2, s, s);
      const neutral = u.player === NEUTRAL_PLAYER;
      const own = !neutral && u.player === me;
      if (!neutral && !own && myTeam >= 0 && teamOf(teams, u.player) === myTeam) {
        ctx.strokeStyle = palette.ally ?? ALLY_RING;
        ctx.lineWidth = 1;
        ctx.strokeRect(x - s / 2 - 1.5, y - s / 2 - 1.5, s + 3, s + 3);
      } else if (!neutral && !own) {
        ctx.strokeStyle = palette.enemy ?? ENEMY_CROSS;
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(x - 2, y - 2);
        ctx.lineTo(x + 2, y + 2);
        ctx.moveTo(x + 2, y - 2);
        ctx.lineTo(x - 2, y + 2);
        ctx.stroke();
      }
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
        ctx.strokeStyle = palette.heroOutline ?? HERO_OUTLINE;
        ctx.lineWidth = 1;
        ctx.strokeRect(x - size / 2 - 0.5, y - size / 2 - 0.5, size + 1, size + 1);
      }
      if (ally) {
        ctx.strokeStyle = palette.ally ?? ALLY_RING;
        ctx.lineWidth = 1;
        ctx.strokeRect(x - 4, y - 4, 8, 8);
      } else if (!neutral && !own) {
        ctx.strokeStyle = palette.enemy ?? ENEMY_CROSS;
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(x - 2, y - 2);
        ctx.lineTo(x + 2, y + 2);
        ctx.moveTo(x + 2, y - 2);
        ctx.lineTo(x - 2, y + 2);
        ctx.stroke();
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
    paintPings(performance.now(), sx, sy);
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
    const sig = pings.length > 0 ? mix(signature(list, view, me, active), Math.floor(now / 50)) : signature(list, view, me, active);
    if (sig === lastSig) return lastCount;
    lastSig = sig;
    paint(list, view, me, active);
    lastCount = list.length;
    return lastCount;
  }

  function mapPointAt(e) {
    const r = canvas.getBoundingClientRect();
    const w = r.width - 2 * canvas.clientLeft;
    const h = r.height - 2 * canvas.clientTop;
    if (w <= 0 || h <= 0) return null;
    const fx = clamp01((e.clientX - r.left - canvas.clientLeft) / w);
    const fz = clamp01((e.clientY - r.top - canvas.clientTop) / h);
    return { x: fx * mapSize.w, z: fz * mapSize.h };
  }

  function orderAt(e) {
    const p = mapPointAt(e);
    if (!p) return;
    order("down", p.x, p.z);
    order("up", p.x, p.z);
  }

  function pingAt(e) {
    const p = mapPointAt(e);
    if (p) order(pingKindOf(e), p.x, p.z);
  }

  let press = null;
  const hits = (globalThis.__oscraftHitLayer ??= createHitLayer(window));
  const region = {
    down(e) {
      if (e.button === 2) {
        orderAt(e);
        return;
      }
      if (e.button !== 0) return;
      if (e.altKey) {
        pingAt(e);
        return;
      }
      press = { x: e.clientX, y: e.clientY, moved: false };
      canvas.setPointerCapture(e.pointerId);
    },
    move(e) {
      if (!press) return;
      if (!press.moved && Math.hypot(e.clientX - press.x, e.clientY - press.y) <= DRAG_SLOP_PX) return;
      press.moved = true;
      const p = mapPointAt(e);
      if (p) drag(p.x, p.z);
    },
    up(e) {
      if (!press) return;
      const click = !press.moved;
      press = null;
      const p = click ? mapPointAt(e) : null;
      if (canvas.hasPointerCapture(e.pointerId)) canvas.releasePointerCapture(e.pointerId);
      if (p) centre(p.x, p.z);
    },
    cancel() {
      press = null;
    },
  };
  canvas.style.touchAction = "none";
  hits.add(canvas, region);
  LIVE_CANVAS_DISPOSERS.set(canvas, () => {
    hits.remove(canvas, region);
    window.removeEventListener("moba:ping", onPing);
  });

  return { draw };
}

const hot = (globalThis.__oscraftHot ??= new Map());
const LIVE_CANVAS_DISPOSERS = (globalThis.__oscraftMinimapDisposers ??= new WeakMap());
hot.set("ui-minimap-impl", createMinimapImpl);
export function createMinimap(options) {
  return hot.get("ui-minimap-impl")(options);
}
export function hmrReload() {
  const args = hot.get("ui-minimap");
  return args ? createMinimap(args) : null;
}

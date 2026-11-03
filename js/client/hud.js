
import { cellKind, voidMask } from "./tiles.js";
import { createMobaInput } from "./moba_input.js";
import { createFeedback } from "./feedback.js";
import { createMinimap } from "./ui_minimap.js";
import { UI } from "./pack_load.js";
import { teamOf } from "./moba_select.js";

const BAR_WIDTH_PX = 20;
const BAR_HEIGHT_PX = 3;
const BAR_LIFT_PX = 5;
const SELECTED_BAR_WIDTH_PX = 28;
const SELECTED_BAR_HEIGHT_PX = 5;
const OWN_BAR_RGB = "#4caf50";
const ALLY_BAR_RGB = "#3f8fd2";
const ENEMY_BAR_RGB = "#e53935";
const MESSAGE_LIMIT = 4;
const MESSAGE_HOLD_MS = 3000;
const MESSAGE_FADE_MS = 700;
const HERO_FLAG = 1;
const BUILDING_FILTER_BIT = 2048;
const XP_STRIP_PX = 4;
const XP_RGB = "#8e6cff";
const SLOW_REFRESH_MS = 100;

export function barColour(player, me, myTeam) {
  if (player === me) return OWN_BAR_RGB;
  return myTeam >= 0 && teamOf(UI.teams, player) === myTeam ? ALLY_BAR_RGB : ENEMY_BAR_RGB;
}

export function xpProgress(xp, curve) {
  const reached = curve.filter((threshold) => threshold <= xp).length;
  const level = reached + 1;
  if (reached >= curve.length) return { level, fraction: 1 };
  const floor = reached > 0 ? curve[reached - 1] : 0;
  const ceiling = curve[reached];
  return { level, fraction: ceiling > floor ? (xp - floor) / (ceiling - floor) : 1 };
}

export function courierLine(state) {
  if (!state) return "";
  if (state.alive > 0) return "Courier " + state.alive + "/" + state.limit;
  if (state.revive > 0) return "Courier revive in " + state.revive + "s";
  return "No courier";
}

function createMessageLine() {
  const line = document.createElement("div");
  line.dataset.ui = "messages";
  line.setAttribute("role", "status");
  line.setAttribute("aria-live", "polite");
  line.style.cssText = "position:fixed;left:50%;bottom:calc(var(--bar-h) + 12px);transform:translateX(-50%);z-index:5;display:flex;flex-direction:column;align-items:center;gap:4px;max-width:92vw;pointer-events:none;";
  document.body.appendChild(line);
  function add(text, kind) {
    const item = document.createElement("div");
    item.textContent = text;
    item.style.cssText = `background:rgba(0,0,0,0.78);color:${kind === "refusal" ? "#ffb4a8" : "#e6ecf1"};font:600 var(--t-body, 13px) var(--font, sans-serif);padding:4px 10px;border-radius:4px;white-space:nowrap;transition:opacity ${MESSAGE_FADE_MS}ms ease;`;
    line.appendChild(item);
    while (line.childElementCount > MESSAGE_LIMIT) line.firstElementChild.remove();
    setTimeout(() => { item.style.opacity = "0"; }, MESSAGE_HOLD_MS);
    setTimeout(() => item.remove(), MESSAGE_HOLD_MS + MESSAGE_FADE_MS);
  }
  const onFeedback = (e) => {
    if (e.detail && e.detail.kind === "refusal") add(String(e.detail.text), "refusal");
  };
  window.addEventListener("moba:feedback", onFeedback);
  return {
    dispose() {
      window.removeEventListener("moba:feedback", onFeedback);
      line.remove();
    },
  };
}

function createStatusPanel() {
  const strip = document.createElement("div");
  strip.dataset.ui = "xp";
  strip.style.cssText = `position:fixed;left:0;right:0;bottom:var(--bar-h, 160px);height:${XP_STRIP_PX}px;background:rgba(0,0,0,0.6);pointer-events:none;z-index:2;display:none;`;
  const fill = document.createElement("div");
  fill.style.cssText = `height:100%;width:0;background:${XP_RGB};`;
  strip.append(fill);
  const line = document.createElement("div");
  line.dataset.ui = "status";
  line.style.cssText = "position:fixed;right:8px;bottom:calc(var(--bar-h, 160px) + 10px);z-index:5;pointer-events:none;color:#e6ecf1;font:600 var(--t-body, 13px) var(--font, sans-serif);text-shadow:0 1px 2px #000;";
  document.body.append(strip, line);
  let shownText = null;
  let shownWidth = null;
  return {
    show(text, fraction) {
      if (text !== shownText) {
        shownText = text;
        line.textContent = text;
      }
      if (fraction === null) {
        strip.style.display = "none";
        return;
      }
      strip.style.display = "block";
      const width = (Math.max(0, Math.min(1, fraction)) * 100).toFixed(2) + "%";
      if (width !== shownWidth) {
        shownWidth = width;
        fill.style.width = width;
      }
    },
    dispose() {
      strip.remove();
      line.remove();
    },
  };
}

const VOID_RGB = [20, 24, 28];
const OPEN_RGB = [104, 124, 94];
const ROCK_RGB = [58, 64, 70];
const WATER_RGB = [48, 96, 150];

function walkableLayer(grid) {
  if (!grid) return null;
  const layer = document.createElement("canvas");
  layer.width = grid.width;
  layer.height = grid.height;
  const ctx = layer.getContext("2d");
  const pixels = ctx.createImageData(grid.width, grid.height);
  const empty = voidMask(grid);
  for (let i = 0; i < grid.width * grid.height; i++) {
    const kind = cellKind(grid.flags[i], grid.tiles[i]);
    const rgb = empty[i] ? VOID_RGB : kind === "water" ? WATER_RGB : kind === "cliff" ? ROCK_RGB : OPEN_RGB;
    pixels.data.set([rgb[0], rgb[1], rgb[2], 255], i * 4);
  }
  ctx.putImageData(pixels, 0, 0);
  return layer;
}

export function createHud({ game, minimapEl, barCanvas, mapSize, grid, units, selected, localPlayer, send, toWorld, toScreen, playerColor, heroViewLock }) {
  hot.get("hud-teardown")?.();
  hot.set("hud", { game, minimapEl, barCanvas, mapSize, grid, units, selected, localPlayer, send, toWorld, toScreen, playerColor, heroViewLock });
  const undo = [];
  const listen = (type, fn) => {
    window.addEventListener(type, fn);
    undo.push(() => window.removeEventListener(type, fn));
  };
  const barCtx = barCanvas.getContext("2d");
  const walkable = walkableLayer(grid);
  if (!minimapEl.title) minimapEl.title = "Minimap: click to move the camera, drag to pan";
  minimapEl.setAttribute("aria-label", minimapEl.title);

  const input = createMobaInput({ game, toWorld, toScreen, send, dragSlopPx: UI.dragSlopPx });
  undo.push(input.destroy);
  const feedback = createFeedback({ toScreen, toWorld, units });
  const messages = createMessageLine();
  undo.push(messages.dispose);
  const status = createStatusPanel();
  undo.push(status.dispose);

  let courierState = null;
  listen("moba:courier", (e) => {
    courierState = e.detail ?? null;
  });

  const sendCameraTo = (x, z) => {
    heroViewLock.release();
    send("camera_to", { x, z });
  };

  const centreButton = document.getElementById("centre-hero");
  if (centreButton) {
    centreButton.onclick = () => {
      const flags = typeof game.unit_flags === "function" ? game.unit_flags() : null;
      const hero = flags && units().find((u) => u.player === localPlayer() && flags[u.type] === 1);
      if (hero) sendCameraTo(hero.x, hero.z);
    };
  }

  const heroLockButton = document.getElementById("hero-lock");
  if (heroLockButton) {
    const showLockState = () => heroLockButton.setAttribute("aria-pressed", String(heroViewLock.isLocked()));
    heroLockButton.onclick = () => {
      heroViewLock.toggle();
      showLockState();
    };
    showLockState();
  }

  const mapToggle = document.getElementById("map-toggle");
  if (mapToggle) {
    const onMapToggle = () => {
      const on = document.body.dataset.map !== "on";
      document.body.dataset.map = on ? "on" : "off";
      mapToggle.setAttribute("aria-expanded", String(on));
    };
    mapToggle.addEventListener("click", onMapToggle);
    undo.push(() => mapToggle.removeEventListener("click", onMapToggle));
  }

  function viewRect() {
    const corners = [[0, 0], [window.innerWidth, 0], [0, window.innerHeight], [window.innerWidth, window.innerHeight]].map(([x, y]) => toWorld(x, y));
    return {
      x0: Math.min(...corners.map((c) => c.x)),
      z0: Math.min(...corners.map((c) => c.z)),
      x1: Math.max(...corners.map((c) => c.x)),
      z1: Math.max(...corners.map((c) => c.z)),
    };
  }

  const liveGame = () => hot.get("hud")?.game ?? null;
  const unitFlagsOf = () => {
    const g = liveGame();
    return g && typeof g.unit_flags === "function" ? g.unit_flags() : [];
  };
  const isHeroRecord = (u) => unitFlagsOf()[u.type] === HERO_FLAG;
  const isBuildingRecord = (u) => {
    const g = liveGame();
    if (!g || typeof g.unit_filter_bits !== "function" || typeof g.unit_ids !== "function") return false;
    const at = g.unit_ids().indexOf(u.id);
    return at >= 0 && (g.unit_filter_bits()[at] & BUILDING_FILTER_BIT) !== 0;
  };

  let minimap = createMinimap({
    canvas: minimapEl,
    mapSize,
    lanes: walkable,
    selected,
    localPlayer,
    playerColor,
    teams: UI.teams,
    viewRect,
    centre: sendCameraTo,
    drag: (x, z) => {
      heroViewLock.release();
      window.dispatchEvent(new CustomEvent("moba:minimap-drag", { detail: { x, z } }));
    },
    order: (kind, x, z) => send("pointer", { kind, x, y: z, button: 2 }),
    isHero: isHeroRecord,
    isBuilding: isBuildingRecord,
  });
  listen("oscraft:hmr", (e) => {
    const { url, value } = e.detail;
    if (value && url.endsWith("/ui_minimap.js")) minimap = value;
  });

  function gameMaxHps() {
    if (!game) return null;
    let ids = null;
    let maxes = null;
    if (typeof game.unit_columns === "function") {
      const packed = game.unit_columns();
      if (packed && packed.length % 7 === 0) {
        const live = packed.length / 7;
        ids = packed.subarray(0, live);
        maxes = packed.subarray(5 * live, 6 * live);
      }
    }
    if (!ids || !maxes) {
      if (typeof game.unit_ids !== "function" || typeof game.unit_max_hps !== "function") return null;
      ids = game.unit_ids();
      maxes = game.unit_max_hps();
    }
    const byId = new Map();
    for (let i = 0; i < ids.length; i++) byId.set(ids[i], maxes[i]);
    return byId;
  }

  function drawTeamCue(kind, x, y, width, height) {
    barCtx.strokeStyle = "rgba(255,255,255,0.9)";
    barCtx.lineWidth = 1;
    if (kind === "own") {
      barCtx.setLineDash([]);
      barCtx.strokeRect(x - 0.5, y - 0.5, width + 1, height + 1);
    } else if (kind === "ally") {
      barCtx.setLineDash([2, 2]);
      barCtx.strokeRect(x - 0.5, y - 0.5, width + 1, height + 1);
      barCtx.setLineDash([]);
    } else {
      barCtx.fillStyle = "#ffffff";
      barCtx.fillRect(x - 2, y - 2, 2, height + 4);
    }
  }

  function drawBars(list) {
    const w = window.innerWidth;
    const h = window.innerHeight;
    const dpr = window.devicePixelRatio || 1;
    const pw = Math.round(w * dpr);
    const ph = Math.round(h * dpr);
    if (barCanvas.width !== pw || barCanvas.height !== ph) {
      barCanvas.width = pw;
      barCanvas.height = ph;
      barCanvas.style.width = `${w}px`;
      barCanvas.style.height = `${h}px`;
    }
    barCtx.setTransform(dpr, 0, 0, dpr, 0, 0);
    barCtx.clearRect(0, 0, w, h);
    const maxes = gameMaxHps();
    if (!maxes) return 0;
    let drawn = 0;
    const me = localPlayer();
    const myTeam = teamOf(UI.teams, me);
    const selectedIds = new Set(input.selection());
    for (const u of list) {
      if (typeof u.hp !== "number" || u.hp < 0) continue;
      const max = maxes.get(u.id) ?? 0;
      if (!(max > 0)) continue;
      const p = toScreen(u.x, u.z, u.top || 0);
      const picked = selectedIds.has(u.id);
      const width = picked ? SELECTED_BAR_WIDTH_PX : BAR_WIDTH_PX;
      const height = picked ? SELECTED_BAR_HEIGHT_PX : BAR_HEIGHT_PX;
      const x = p.x - width / 2;
      const y = p.y - BAR_LIFT_PX;
      if (x + width < 0 || x > w || y + height < 0 || y > h) continue;
      barCtx.fillStyle = "rgba(0,0,0,0.6)";
      barCtx.fillRect(x, y, width, height);
      barCtx.strokeStyle = "rgba(0,0,0,0.85)";
      barCtx.lineWidth = 1;
      barCtx.strokeRect(x - 0.5, y - 0.5, width + 1, height + 1);
      barCtx.fillStyle = barColour(u.player, me, myTeam);
      barCtx.fillRect(x, y, width * Math.min(1, u.hp / max), height);
      const team = u.player === 255 ? null : u.player === me ? "own" : myTeam >= 0 && teamOf(UI.teams, u.player) === myTeam ? "ally" : "enemy";
      if (picked) {
        barCtx.strokeStyle = "#ffffff";
        barCtx.setLineDash(team === "ally" ? [2, 2] : []);
        barCtx.strokeRect(x - 0.5, y - 0.5, width + 1, height + 1);
        barCtx.setLineDash([]);
      }
      if (team) drawTeamCue(team, x, y, width, height);
      drawn += 1;
    }
    return drawn;
  }

  function refreshCooldowns() {
    for (const cell of document.querySelectorAll("[data-cd-unit][data-cd-slot]")) {
      const unit = Number(cell.dataset.cdUnit);
      const slot = Number(cell.dataset.cdSlot);
      const total = input.cooldownTotal(unit, slot);
      const fraction = total > 0 ? Math.min(1, Math.max(0, input.cooldownLeft(unit, slot) / total)) : 0;
      cell.style.setProperty("--cd", String(fraction));
    }
  }

  let slowAt = -Infinity;
  function refreshSlow(list, me, gold) {
    const now = performance.now();
    if (now - slowAt < SLOW_REFRESH_MS) return;
    slowAt = now;
    refreshCooldowns();
    const flags = unitFlagsOf();
    const hero = list.find((u) => u.player === me && flags[u.type] === HERO_FLAG);
    const curve = typeof game.xp_curve === "function" ? Array.from(game.xp_curve()) : [];
    const progress = hero && curve.length > 0 ? xpProgress(game.xp(hero.id), curve) : null;
    const parts = ["Gold " + gold];
    if (progress) parts.push("Level " + progress.level);
    const courier = courierLine(courierState);
    if (courier) parts.push(courier);
    status.show(parts.join("   "), progress ? progress.fraction : null);
  }

  const shown = {};
  function publish(key, value) {
    const text = String(value);
    if (shown[key] === text) return;
    shown[key] = text;
    document.body.dataset[key] = text;
  }

  function update() {
    const list = units();
    const hasGame = game && typeof game.gold === "function";
    const me = localPlayer();
    const gold = hasGame ? game.gold(me) : 0;
    const winner = hasGame ? game.winner() : -1;
    const primary = input.selection()[0];
    const hp = primary !== undefined && hasGame ? game.unit_hp(primary) : -1;
    const bars = drawBars(list);
    const dots = minimap.draw(list);
    input.draw();
    feedback.draw();
    if (hasGame) refreshSlow(list, me, gold);
    publish("gold", gold);
    publish("winner", winner);
    publish("hp", hp);
    publish("bars", bars);
    publish("minimap", dots);
    publish("selected", input.selection().length);
    return { gold, winner, hp };
  }

  hot.set("hud-teardown", () => {
    for (const fn of undo.splice(0)) fn();
  });
  return { update, drawBars, input };
}

const hot = (globalThis.__oscraftHot ??= new Map());
export function hmrReload() {
  const args = hot.get("hud");
  return args ? createHud(args) : null;
}

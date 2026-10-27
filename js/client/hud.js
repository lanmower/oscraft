
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

function barColour(player, me, myTeam) {
  if (player === me) return OWN_BAR_RGB;
  return myTeam >= 0 && teamOf(UI.teams, player) === myTeam ? ALLY_BAR_RGB : ENEMY_BAR_RGB;
}

function createMessageLine() {
  hot.get("hud-messages")?.dispose();
  const line = document.createElement("div");
  line.dataset.ui = "messages";
  line.setAttribute("role", "status");
  line.setAttribute("aria-live", "polite");
  line.style.cssText = "position:fixed;left:50%;bottom:calc(var(--bar-h, 160px) + 12px);transform:translateX(-50%);z-index:5;display:flex;flex-direction:column;align-items:center;gap:4px;max-width:92vw;pointer-events:none;";
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
  const onMessage = (e) => {
    if (e.detail && typeof e.detail.text === "string") add(e.detail.text, e.detail.kind === "chat" ? "chat" : "info");
  };
  window.addEventListener("moba:feedback", onFeedback);
  window.addEventListener("moba:message", onMessage);
  const panel = {
    add,
    dispose() {
      window.removeEventListener("moba:feedback", onFeedback);
      window.removeEventListener("moba:message", onMessage);
      line.remove();
    },
  };
  hot.set("hud-messages", panel);
  return panel;
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

export function createHud({ game, minimapEl, barCanvas, mapSize, grid, units, selected, localPlayer, send, toWorld, toScreen, playerColor }) {
  hot.set("hud", { game, minimapEl, barCanvas, mapSize, grid, units, selected, localPlayer, send, toWorld, toScreen, playerColor });
  const maxHp = hot.get("hud-maxhp") ?? new Map();
  hot.set("hud-maxhp", maxHp);
  const barCtx = barCanvas.getContext("2d");
  const walkable = walkableLayer(grid);
  if (!minimapEl.title) minimapEl.title = "Minimap: click to move the camera";
  minimapEl.setAttribute("aria-label", minimapEl.title);

  const input = createMobaInput({ game, toWorld, toScreen, send, dragSlopPx: UI.dragSlopPx });
  const feedback = createFeedback({ toScreen, toWorld, units });
  createMessageLine();

  const centreButton = document.getElementById("centre-hero");
  if (centreButton) {
    centreButton.onclick = () => {
      const flags = typeof game.unit_flags === "function" ? game.unit_flags() : null;
      const hero = flags && units().find((u) => u.player === localPlayer() && flags[u.type] === 1);
      if (hero) send("camera_to", { x: hero.x, z: hero.z });
    };
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

  let minimap = hot.get("hud-minimap") ?? createMinimap({
    canvas: minimapEl,
    mapSize,
    lanes: walkable,
    selected,
    localPlayer,
    playerColor,
    teams: UI.teams,
    viewRect,
    centre: (x, z) => send("camera_to", { x, z }),
    order: (kind, x, z) => send("pointer", { kind, x, y: z, button: 2 }),
  });
  hot.set("hud-minimap", minimap);
  window.addEventListener("oscraft:hmr", (e) => {
    const { url, value } = e.detail;
    if (value && url.endsWith("/ui_minimap.js")) {
      minimap = value;
      hot.set("hud-minimap", value);
    }
  });

  function gameMaxHps() {
    if (!game || typeof game.unit_max_hps !== "function" || typeof game.unit_ids !== "function") return null;
    const ids = game.unit_ids();
    const maxes = game.unit_max_hps();
    const byId = new Map();
    for (let i = 0; i < ids.length; i++) byId.set(ids[i], maxes[i]);
    return byId;
  }

  function trackedMaxHp(id, hp, authoritative) {
    if (authoritative > 0) return authoritative;
    const best = Math.max(maxHp.get(id) || 0, hp);
    maxHp.set(id, best);
    return best;
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
    }
    barCtx.setTransform(dpr, 0, 0, dpr, 0, 0);
    barCtx.clearRect(0, 0, w, h);
    let drawn = 0;
    const me = localPlayer();
    const myTeam = teamOf(UI.teams, me);
    const selectedIds = new Set(input.selection());
    const maxes = gameMaxHps();
    const margin = SELECTED_BAR_WIDTH_PX;
    for (const u of list) {
      if (typeof u.hp !== "number" || u.hp < 0) continue;
      const max = trackedMaxHp(u.id, u.hp, maxes ? maxes.get(u.id) : 0);
      if (max <= 0) continue;
      const p = toScreen(u.x, u.z, u.top || 0);
      if (p.x < -margin || p.x > w + margin || p.y < -margin || p.y > h + margin) continue;
      const picked = selectedIds.has(u.id);
      const width = picked ? SELECTED_BAR_WIDTH_PX : BAR_WIDTH_PX;
      const height = picked ? SELECTED_BAR_HEIGHT_PX : BAR_HEIGHT_PX;
      const x = p.x - width / 2;
      const y = p.y - BAR_LIFT_PX;
      barCtx.fillStyle = "rgba(0,0,0,0.6)";
      barCtx.fillRect(x, y, width, height);
      barCtx.strokeStyle = "rgba(0,0,0,0.85)";
      barCtx.lineWidth = 1;
      barCtx.strokeRect(x - 0.5, y - 0.5, width + 1, height + 1);
      barCtx.fillStyle = barColour(u.player, me, myTeam);
      barCtx.fillRect(x, y, width * Math.min(1, u.hp / max), height);
      if (picked) {
        barCtx.strokeStyle = "#ffffff";
        barCtx.setLineDash([]);
        barCtx.strokeRect(x - 0.5, y - 0.5, width + 1, height + 1);
      } else if (u.player !== 255) {
        drawTeamCue(u.player === me ? "own" : myTeam >= 0 && teamOf(UI.teams, u.player) === myTeam ? "ally" : "enemy", x, y, width, height);
      }
      drawn += 1;
    }
    return drawn;
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
    const gold = hasGame ? game.gold(localPlayer()) : 0;
    const winner = hasGame ? game.winner() : -1;
    const primary = input.selection()[0];
    const hp = primary !== undefined && hasGame ? game.unit_hp(primary) : -1;
    const bars = drawBars(list);
    const dots = minimap.draw(list);
    input.draw();
    feedback.draw();
    publish("gold", gold);
    publish("winner", winner);
    publish("hp", hp);
    publish("bars", bars);
    publish("minimap", dots);
    publish("selected", input.selection().length);
    return { gold, winner, hp };
  }

  return { update, drawBars, input };
}

const hot = (globalThis.__oscraftHot ??= new Map());
export function hmrReload() {
  const args = hot.get("hud");
  return args ? createHud(args) : null;
}

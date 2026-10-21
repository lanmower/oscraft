// MOBA HUD and controls. createHud builds the input layer (moba_input.js) and the panels: selection, minimap and
// scoreboard. Text and key bindings come from moba_ui.json. update() refreshes them from the wasm Game each frame,
// along with the winner banner and unit health bars. Left pointer and key input is handled by the input layer, so
// onKey and the left-click part of onPointerUp are no-ops kept for play.js.

import { cellKind, voidMask } from "./tiles.js";
import { UI } from "./pack_load.js";
import { fill } from "./moba_select.js";
import { createMobaInput, centreCamera } from "./moba_input.js";
import { createFeedback } from "./feedback.js";
import { createMinimap } from "./ui_minimap.js";
import { createSelectionPanel } from "./ui_selection.js";
import { createScoreboard } from "./ui_scoreboard.js";

const TICK_S = 1 / 16;
const RIGHT_BUTTON = 2;
const BAR_WIDTH_PX = 20;
const BAR_HEIGHT_PX = 3;
const BAR_LIFT_PX = 5;

export function formatClock(ticks) {
  const total = Math.floor(ticks * TICK_S);
  const minutes = Math.floor(total / 60);
  const seconds = String(total % 60).padStart(2, "0");
  return `${minutes}:${seconds}`;
}

const VOID_RGB = [20, 24, 28];
const OPEN_RGB = [104, 124, 94];
const ROCK_RGB = [58, 64, 70];
const WATER_RGB = [48, 96, 150];

// The map grid as a small canvas: void dark, walkable cells light, rock and cliff edges between, water where tiles say so.
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

export function createHud({ game, hudEl, winEl, minimapEl, barCanvas, mapSize, grid, units, selected, select, localPlayer, commands, pick, toWorld, toScreen, playerColor }) {
  hot.set("hud", { game, hudEl, winEl, minimapEl, barCanvas, mapSize, grid, units, selected, select, localPlayer, commands, pick, toWorld, toScreen, playerColor });
  const maxHp = new Map();
  const barCtx = barCanvas.getContext("2d");
  const lanes = walkableLayer(grid);
  // The clock reads 0:00 at the horn (the lobby's "oscraft:horn" event). Before the horn the match is in pre-match.
  let hornTicks = null;
  window.addEventListener("oscraft:horn", () => {
    if (game && typeof game.ticks === "function") hornTicks = game.ticks();
  });

  const hasHeroes = typeof game.attributes === "function";
  const isHero = (id) => hasHeroes && game.attributes(id).some((v) => v !== 0);
  const typeNames = typeof game.unit_model_names === "function" ? game.unit_model_names() : [];
  const unitName = (type) => typeNames[type] || UI.text.unitUnknown;

  const input = createMobaInput({ game, units, localPlayer, pick, toWorld, toScreen, commands, isHero, select, config: UI });
  const feedback = createFeedback({ toScreen, toWorld, units });

  function viewRect() {
    const corners = [[0, 0], [window.innerWidth, 0], [0, window.innerHeight], [window.innerWidth, window.innerHeight]].map(([x, y]) => toWorld(x, y));
    return {
      x0: Math.min(...corners.map((c) => c.x)),
      z0: Math.min(...corners.map((c) => c.z)),
      x1: Math.max(...corners.map((c) => c.x)),
      z1: Math.max(...corners.map((c) => c.z)),
    };
  }

  // let, not const: a hot update of a panel module hands its new panel to this HUD (see the oscraft:hmr listener below).
  let minimap = createMinimap({
    canvas: minimapEl,
    mapSize,
    lanes,
    units,
    selected,
    localPlayer,
    playerColor,
    isHero,
    teams: UI.teams,
    viewRect,
    centre: centreCamera,
  });
  let selectionPanel = createSelectionPanel({ game, input, units, localPlayer, unitName, isHero });
  let scoreboard = createScoreboard({ game, units, localPlayer, unitName, isHero, teams: UI.teams });
  // A hot update of ui_minimap / ui_selection / ui_scoreboard re-creates that panel and dispatches it (see hmr.js).
  window.addEventListener("oscraft:hmr", (e) => {
    const { url, value } = e.detail;
    if (!value) return;
    if (url.endsWith("/ui_minimap.js")) minimap = value;
    else if (url.endsWith("/ui_selection.js")) selectionPanel = value;
    else if (url.endsWith("/ui_scoreboard.js")) scoreboard = value;
  });

  function clockText(hasGame) {
    if (!hasGame || hornTicks === null) return UI.text.preMatch;
    return formatClock(game.ticks() - hornTicks);
  }

  function postGameText(winner) {
    const banner = fill(UI.text.winnerBanner, { side: winner });
    if (!game || typeof game.post_game_json !== "function") return banner;
    const report = JSON.parse(game.post_game_json());
    const rows = report.players.map((p) => fill(UI.text.postGameRow, { player: p.player, kills: p.kills, deaths: p.deaths, assists: p.assists, gold: p.gold, xp: p.xp, items: p.items.length }));
    return [banner, ...rows].join("\n");
  }

  function trackedMaxHp(id, hp) {
    const best = Math.max(maxHp.get(id) || 0, hp);
    maxHp.set(id, best);
    return best;
  }

  function drawBars(list) {
    barCanvas.width = window.innerWidth;
    barCanvas.height = window.innerHeight;
    barCtx.clearRect(0, 0, barCanvas.width, barCanvas.height);
    let drawn = 0;
    for (const u of list) {
      if (typeof u.hp !== "number" || u.hp < 0) continue;
      const max = trackedMaxHp(u.id, u.hp);
      if (max <= 0) continue;
      const p = toScreen(u.x, u.z, u.top || 0);
      const x = p.x - BAR_WIDTH_PX / 2;
      const y = p.y - BAR_LIFT_PX;
      barCtx.fillStyle = "rgba(0,0,0,0.6)";
      barCtx.fillRect(x, y, BAR_WIDTH_PX, BAR_HEIGHT_PX);
      barCtx.fillStyle = u.player === localPlayer() ? "#4caf50" : "#e53935";
      barCtx.fillRect(x, y, BAR_WIDTH_PX * Math.min(1, u.hp / max), BAR_HEIGHT_PX);
      drawn += 1;
    }
    return drawn;
  }

  function update() {
    const list = units();
    const hasGame = game && typeof game.gold === "function";
    const gold = hasGame ? game.gold(localPlayer()) : 0;
    const winner = hasGame ? game.winner() : -1;
    const primary = input.selection()[0];
    const hp = primary !== undefined && hasGame ? game.unit_hp(primary) : -1;
    hudEl.textContent = fill(UI.text.hud, { hp: hp >= 0 ? hp : UI.text.noValue, gold, time: clockText(hasGame) });
    winEl.hidden = winner < 0;
    if (winner >= 0) winEl.textContent = postGameText(winner);
    const bars = drawBars(list);
    const dots = minimap.draw(list);
    selectionPanel.update();
    scoreboard.update();
    input.draw();
    feedback.draw();
    const body = document.body.dataset;
    body.gold = String(gold);
    body.winner = String(winner);
    body.hp = String(hp);
    body.bars = String(bars);
    body.minimap = String(dots);
    body.selected = String(input.selection().length);
    return { gold, winner, hp };
  }

  // Right-click is routed here by play.js (its pointer-up on the canvas); left clicks and drags never reach it.
  function onPointerUp(e) {
    if (e.button !== RIGHT_BUTTON) return false;
    return input.commandAt(e);
  }

  // Keys are read by the input layer on window, so the page's key hook has nothing left to do.
  function onKey() {
    return false;
  }

  return { update, onPointerUp, onKey, drawBars, input };
}

// Hot-update hook (see hmr.js): re-runs createHud on the inputs of its last call. HUD and canvases are the same DOM
// elements, so the returned HUD takes over for play.js; the old instance keeps its window listeners (dev only).
const hot = (globalThis.__oscraftHot ??= new Map());
export function hmrReload() {
  const args = hot.get("hud");
  return args ? createHud(args) : null;
}

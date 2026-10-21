// Page HUD: the input capture (input.js), the feedback layer, the minimap and the unit health bars. Every panel's
// content and every button is the pack's (panels.lua, drawn by widgets.js); keys and ground clicks are the
// pack's (controls.lua). update() refreshes the page's own layers from the wasm Game each frame.

import { cellKind, voidMask } from "./tiles.js";
import { createInput } from "./input.js";
import { createFeedback } from "./feedback.js";
import { createMinimap } from "./ui_minimap.js";
import { UI } from "./pack_load.js";

const BAR_WIDTH_PX = 20;
const BAR_HEIGHT_PX = 3;
const BAR_LIFT_PX = 5;

// The map grid as a small canvas: void dark, walkable cells light, rock and cliff edges between, water where tiles say so.
function walkableLayer(grid) {
  if (!grid) return null;
  const layer = document.createElement("canvas");
  layer.width = grid.width;
  layer.height = grid.height;
  const ctx = layer.getContext("2d");
  const pixels = ctx.createImageData(grid.width, grid.height);
  const empty = voidMask(grid);
  const terrain = UI.colours.minimap.terrain;
  for (let i = 0; i < grid.width * grid.height; i++) {
    const kind = cellKind(grid.flags[i], grid.tiles[i]);
    const rgb = empty[i] ? terrain.void : kind === "water" ? terrain.water : kind === "cliff" ? terrain.rock : terrain.open;
    pixels.data.set([rgb[0], rgb[1], rgb[2], 255], i * 4);
  }
  ctx.putImageData(pixels, 0, 0);
  return layer;
}

export function createHud({ game, minimapEl, barCanvas, mapSize, grid, units, selected, localPlayer, send, toWorld, toScreen, playerColor, isControlled }) {
  hot.set("hud", { game, minimapEl, barCanvas, mapSize, grid, units, selected, localPlayer, send, toWorld, toScreen, playerColor, isControlled });
  const maxHp = new Map();
  const barCtx = barCanvas.getContext("2d");
  const walkable = walkableLayer(grid);

  const input = createInput({ send, dragSlopPx: UI.dragSlopPx });
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

  // let, not const: a hot update of ui_minimap hands its new minimap to this HUD (see the oscraft:hmr listener below).
  let minimap = createMinimap({
    canvas: minimapEl,
    mapSize,
    walkable,
    units,
    selected,
    localPlayer,
    playerColor,
    teams: UI.teams,
    viewRect,
    centre: (x, z) => send("camera_to", { x, z }),
    isControlled,
  });
  // A hot update of ui_minimap re-creates the minimap and dispatches it (see hmr.js).
  window.addEventListener("oscraft:hmr", (e) => {
    const { url, value } = e.detail;
    if (value && url.endsWith("/ui_minimap.js")) minimap = value;
  });

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
      barCtx.fillStyle = UI.colours.bars.back;
      barCtx.fillRect(x, y, BAR_WIDTH_PX, BAR_HEIGHT_PX);
      barCtx.fillStyle = u.player === localPlayer() ? UI.colours.bars.own : UI.colours.bars.other;
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
    const bars = drawBars(list);
    const dots = minimap.draw(list);
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

  return { update, drawBars, input };
}

// Hot-update hook (see hmr.js): re-runs createHud on the inputs of its last call. HUD and canvases are the same DOM
// elements, so the returned HUD takes over for play.js; the old instance keeps its window listeners (dev only).
const hot = (globalThis.__oscraftHot ??= new Map());
export function hmrReload() {
  const args = hot.get("hud");
  return args ? createHud(args) : null;
}

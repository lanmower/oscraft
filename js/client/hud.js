// Page HUD: the input capture (moba_input.js), the feedback layer, the minimap and the unit health bars. Every panel's
// content and every button is the pack's (scenarios/moba/panels.lua, drawn by widgets.js); keys and ground clicks are the
// pack's (controls.lua). update() refreshes the page's own layers from the wasm Game each frame.

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

// Own units are green, a teammate's blue, and everything else red. Neutral units have no team (teamOf gives -1).
function barColour(player, me, myTeam) {
  if (player === me) return OWN_BAR_RGB;
  return myTeam >= 0 && teamOf(UI.teams, player) === myTeam ? ALLY_BAR_RGB : ENEMY_BAR_RGB;
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

export function createHud({ game, minimapEl, barCanvas, mapSize, grid, units, selected, localPlayer, send, toWorld, toScreen, playerColor }) {
  hot.set("hud", { game, minimapEl, barCanvas, mapSize, grid, units, selected, localPlayer, send, toWorld, toScreen, playerColor });
  // The largest hp seen per unit survives a HUD rebuild, so a hot reload does not redraw damaged units at full bars.
  const maxHp = hot.get("hud-maxhp") ?? new Map();
  hot.set("hud-maxhp", maxHp);
  const barCtx = barCanvas.getContext("2d");
  const walkable = walkableLayer(grid);
  // The minimap is a pointer control (a click moves the camera), so it names itself for tooltips and assistive tech.
  if (!minimapEl.title) minimapEl.title = "Minimap: click to move the camera";
  minimapEl.setAttribute("aria-label", minimapEl.title);

  const input = createMobaInput({ game, toWorld, toScreen, send, dragSlopPx: UI.dragSlopPx });
  const feedback = createFeedback({ toScreen, toWorld, units });

  // Phones hide the minimap and have no right-drag or Space, so this visible button glides the camera to the local hero.
  // onclick (not addEventListener) replaces the handler on each hot rebuild instead of stacking one per rebuild.
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

  // let, not const: a hot update of ui_minimap hands its new minimap to this HUD (see the oscraft:hmr listener below).
  // A HUD rebuild keeps the latest minimap: the base module's createMinimap would bring back the pre-edit code.
  let minimap = hot.get("hud-minimap") ?? createMinimap({
    canvas: minimapEl,
    mapSize,
    lanes: walkable,
    units,
    selected,
    localPlayer,
    playerColor,
    teams: UI.teams,
    viewRect,
    centre: (x, z) => send("camera_to", { x, z }),
  });
  hot.set("hud-minimap", minimap);
  // A hot update of ui_minimap re-creates the minimap and dispatches it (see hmr.js).
  window.addEventListener("oscraft:hmr", (e) => {
    const { url, value } = e.detail;
    if (value && url.endsWith("/ui_minimap.js")) {
      minimap = value;
      hot.set("hud-minimap", value);
    }
  });

  function trackedMaxHp(id, hp) {
    const best = Math.max(maxHp.get(id) || 0, hp);
    maxHp.set(id, best);
    return best;
  }

  function drawBars(list) {
    const w = window.innerWidth;
    const h = window.innerHeight;
    // Assigning a canvas size clears and reallocates it, so the size is set only when the window changed.
    if (barCanvas.width !== w || barCanvas.height !== h) {
      barCanvas.width = w;
      barCanvas.height = h;
    }
    barCtx.clearRect(0, 0, w, h);
    let drawn = 0;
    const me = localPlayer();
    const myTeam = teamOf(UI.teams, me);
    const selectedIds = new Set(input.selection());
    for (const u of list) {
      if (typeof u.hp !== "number" || u.hp < 0) continue;
      const max = trackedMaxHp(u.id, u.hp);
      if (max <= 0) continue;
      const p = toScreen(u.x, u.z, u.top || 0);
      const picked = selectedIds.has(u.id);
      const width = picked ? SELECTED_BAR_WIDTH_PX : BAR_WIDTH_PX;
      const height = picked ? SELECTED_BAR_HEIGHT_PX : BAR_HEIGHT_PX;
      const x = p.x - width / 2;
      const y = p.y - BAR_LIFT_PX;
      barCtx.fillStyle = "rgba(0,0,0,0.6)";
      barCtx.fillRect(x, y, width, height);
      barCtx.fillStyle = barColour(u.player, me, myTeam);
      barCtx.fillRect(x, y, width * Math.min(1, u.hp / max), height);
      if (picked) {
        barCtx.strokeStyle = "#ffffff";
        barCtx.strokeRect(x - 0.5, y - 0.5, width + 1, height + 1);
      }
      drawn += 1;
    }
    return drawn;
  }

  // The HUD runs every frame, so a body data value is written only when it changed.
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

// Hot-update hook (see hmr.js): re-runs createHud on the inputs of its last call. HUD and canvases are the same DOM
// elements, so the returned HUD takes over for play.js; the old instance keeps its window listeners (dev only).
const hot = (globalThis.__oscraftHot ??= new Map());
export function hmrReload() {
  const args = hot.get("hud");
  return args ? createHud(args) : null;
}

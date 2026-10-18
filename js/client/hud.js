// MOBA HUD and controls. createHud returns one object that play.js wires: pointer and key handlers route
// through injected commands (local sim or wireweave host), update() refreshes the HUD text, winner banner,
// unit health bars and minimap from the wasm Game each frame.

import { cellKind, voidMask } from "./tiles.js";

const TICK_S = 0.1;
const ABILITY_SLOTS = { q: 0, w: 1, e: 2 };
const ATTACK_KEY = "a";
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
  const maxHp = new Map();
  const barCtx = barCanvas.getContext("2d");
  minimapEl.height = Math.max(1, Math.round((minimapEl.width * mapSize.h) / mapSize.w));
  minimapEl.style.height = `${minimapEl.height}px`;
  const minimapCtx = minimapEl.getContext("2d");
  const lanes = walkableLayer(grid);
  let attackArmed = false;
  // The clock reads 0:00 at the horn (the lobby's "oscraft:horn" event). Before the horn the match is in pre-match.
  let hornTicks = null;
  window.addEventListener("oscraft:horn", () => {
    if (game && typeof game.ticks === "function") hornTicks = game.ticks();
  });

  function clockText(hasGame) {
    if (!hasGame) return "pre-match";
    if (hornTicks === null) return "pre-match";
    return formatClock(game.ticks() - hornTicks);
  }

  function postGameText(winner) {
    if (!game || typeof game.post_game_json !== "function") return `side ${winner} wins`;
    const report = JSON.parse(game.post_game_json());
    const rows = report.players.map((p) => `P${p.player}  K ${p.kills}  D ${p.deaths}  A ${p.assists}  G ${p.gold}  XP ${p.xp}  items ${p.items.length}`);
    return [`side ${winner} wins`, ...rows].join("\n");
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

  function drawMinimap(list) {
    minimapCtx.fillStyle = "#2a343c";
    minimapCtx.fillRect(0, 0, minimapEl.width, minimapEl.height);
    if (lanes) minimapCtx.drawImage(lanes, 0, 0, minimapEl.width, minimapEl.height);
    for (const u of list) {
      const [r, g, b] = playerColor(u.player);
      minimapCtx.fillStyle = u.id === selected() ? "#ffffff" : `rgb(${r * 255 | 0},${g * 255 | 0},${b * 255 | 0})`;
      minimapCtx.fillRect((u.x / mapSize.w) * minimapEl.width - 2, (u.z / mapSize.h) * minimapEl.height - 2, 4, 4);
    }
    return list.length;
  }

  function update() {
    const list = units();
    const hasGame = game && typeof game.gold === "function";
    const gold = hasGame ? game.gold(localPlayer()) : 0;
    const winner = hasGame ? game.winner() : -1;
    const hp = selected() >= 0 && hasGame ? game.unit_hp(selected()) : -1;
    hudEl.textContent = `hp ${hp >= 0 ? hp : "-"}   gold ${gold}   time ${clockText(hasGame)}`;
    winEl.hidden = winner < 0;
    if (winner >= 0) winEl.textContent = postGameText(winner);
    const bars = drawBars(list);
    const dots = drawMinimap(list);
    const body = document.body.dataset;
    body.gold = String(gold);
    body.winner = String(winner);
    body.hp = String(hp);
    body.bars = String(bars);
    body.minimap = String(dots);
    return { gold, winner, hp };
  }

  function issueMove(e) {
    const world = toWorld(e.clientX, e.clientY);
    const unit = units().find((u) => u.id === selected());
    if (!unit) return false;
    return commands.order(unit.id, world.x, world.z);
  }

  function onPointerUp(e) {
    const world = toWorld(e.clientX, e.clientY);
    if (e.button === RIGHT_BUTTON) {
      const target = pick(world);
      if (target && target.player !== localPlayer() && selected() >= 0) return commands.attack(selected(), target.id);
      return issueMove(e);
    }
    if (attackArmed) {
      attackArmed = false;
      const target = pick(world);
      return target && selected() >= 0 ? commands.attack(selected(), target.id) : false;
    }
    const hit = pick(world);
    select(hit ? hit.id : -1);
    return true;
  }

  function onKey(key, pointer) {
    if (selected() < 0) return false;
    const k = key.toLowerCase();
    if (k === ATTACK_KEY) {
      attackArmed = true;
      return true;
    }
    if (!(k in ABILITY_SLOTS)) return false;
    const world = pointer ? toWorld(pointer.x, pointer.y) : { x: 0, z: 0 };
    const target = pick(world);
    return commands.cast(selected(), ABILITY_SLOTS[k], target ? target.id : -1, world.x, world.z);
  }

  return { update, onPointerUp, onKey, drawBars, drawMinimap };
}

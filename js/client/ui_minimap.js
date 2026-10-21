// Minimap, the right block of the bottom bar: the map's walkable layer (open ground light, rock and water dark), every
// visible unit and the camera's ground rectangle. Controlled units are larger than the rest. The own unit and the selection
// are white, allies get a ring, and the rest show their player colour. The camera rectangle is drawn with a dark under-stroke
// so it reads on both light and dark ground. A press or drag on the minimap moves the camera to that map point. Colours and
// sizes are the pack's (ui.json colours.minimap).
import { UI } from "./pack_load.js";

const C = UI.colours.minimap;

// The team of a player slot, from the teams list; a slot past the list belongs to no team.
function teamOf(teams, player) {
  return player >= 0 && player < teams.length ? teams[player] : -1;
}

export function dotSize({ own, controlled }) {
  return own || controlled ? C.sizes.controlled : C.sizes.other;
}

export function createMinimap({ canvas, mapSize, walkable, units, selected, localPlayer, playerColor, teams, viewRect, centre, isControlled = () => false }) {
  hot.set("ui-minimap", { canvas, mapSize, walkable, units, selected, localPlayer, playerColor, teams, viewRect, centre, isControlled });
  canvas.dataset.ui = "minimap";
  canvas.height = Math.max(1, Math.round((canvas.width * mapSize.h) / mapSize.w));
  canvas.style.height = `${canvas.height}px`;
  const ctx = canvas.getContext("2d");

  function draw(list) {
    ctx.fillStyle = C.background;
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    if (walkable) ctx.drawImage(walkable, 0, 0, canvas.width, canvas.height);
    const sx = canvas.width / mapSize.w;
    const sy = canvas.height / mapSize.h;
    const me = localPlayer();
    const myTeam = teamOf(teams, me);
    const active = selected();
    for (const u of list) {
      const [r, g, b] = playerColor(u.player);
      const own = u.player === me;
      const ally = !own && teamOf(teams, u.player) === myTeam;
      const controlled = isControlled(u.id);
      const size = dotSize({ own, controlled });
      const x = u.x * sx;
      const y = u.z * sy;
      ctx.fillStyle = u.id === active || own ? C.own : `rgb(${(r * 255) | 0},${(g * 255) | 0},${(b * 255) | 0})`;
      ctx.fillRect(x - size / 2, y - size / 2, size, size);
      if (controlled) {
        ctx.strokeStyle = C.outline;
        ctx.lineWidth = 1;
        ctx.strokeRect(x - size / 2 - 0.5, y - size / 2 - 0.5, size + 1, size + 1);
      }
      if (ally) {
        ctx.strokeStyle = C.ally;
        ctx.lineWidth = 1;
        ctx.strokeRect(x - 4, y - 4, 8, 8);
      }
    }
    const view = viewRect();
    if (view) {
      const x = view.x0 * sx;
      const y = view.z0 * sy;
      const w = (view.x1 - view.x0) * sx;
      const h = (view.z1 - view.z0) * sy;
      ctx.lineWidth = 3;
      ctx.strokeStyle = C.viewShadow;
      ctx.strokeRect(x, y, w, h);
      ctx.lineWidth = 1;
      ctx.strokeStyle = C.view;
      ctx.strokeRect(x, y, w, h);
    }
    ctx.strokeStyle = C.frame;
    ctx.lineWidth = 1;
    ctx.strokeRect(0.5, 0.5, canvas.width - 1, canvas.height - 1);
    return list.length;
  }

  function toMap(e) {
    const r = canvas.getBoundingClientRect();
    return { x: ((e.clientX - r.left) / r.width) * mapSize.w, z: ((e.clientY - r.top) / r.height) * mapSize.h };
  }

  let pressed = false;
  canvas.addEventListener("pointerdown", (e) => {
    pressed = true;
    e.stopPropagation();
    const p = toMap(e);
    centre(p.x, p.z);
  });
  canvas.addEventListener("pointermove", (e) => {
    if (!pressed) return;
    const p = toMap(e);
    centre(p.x, p.z);
  });
  window.addEventListener("pointerup", () => { pressed = false; });

  return { draw };
}

// Hot-update hook (see hmr.js): re-runs createMinimap on the inputs of its last call. The canvas is kept; the returned
// panel's draw() replaces the one hud.js holds.
const hot = (globalThis.__oscraftHot ??= new Map());
export function hmrReload() {
  const args = hot.get("ui-minimap");
  return args ? createMinimap(args) : null;
}

// Minimap: the map's walkable layer, every visible unit (own in white, allies and enemies in their player colour), the
// selection in white, and the camera's ground rectangle. A press or drag on it moves the camera to that map point.
import { teamOf } from "./moba_select.js";

export function createMinimap({ canvas, mapSize, lanes, units, selected, localPlayer, playerColor, teams, viewRect, centre }) {
  canvas.dataset.ui = "minimap";
  canvas.height = Math.max(1, Math.round((canvas.width * mapSize.h) / mapSize.w));
  canvas.style.height = `${canvas.height}px`;
  const ctx = canvas.getContext("2d");

  function draw(list) {
    ctx.fillStyle = "#2a343c";
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    if (lanes) ctx.drawImage(lanes, 0, 0, canvas.width, canvas.height);
    const sx = canvas.width / mapSize.w;
    const sy = canvas.height / mapSize.h;
    const me = localPlayer();
    const myTeam = teamOf(teams, me);
    const active = selected();
    for (const u of list) {
      const [r, g, b] = playerColor(u.player);
      const own = u.player === me;
      const ally = !own && teamOf(teams, u.player) === myTeam;
      ctx.fillStyle = u.id === active || own ? "#ffffff" : `rgb(${(r * 255) | 0},${(g * 255) | 0},${(b * 255) | 0})`;
      const size = own || ally ? 4 : 3;
      ctx.fillRect(u.x * sx - size / 2, u.z * sy - size / 2, size, size);
      if (ally) {
        ctx.strokeStyle = "#7cc7ff";
        ctx.strokeRect(u.x * sx - 3, u.z * sy - 3, 6, 6);
      }
    }
    const view = viewRect();
    if (view) {
      ctx.strokeStyle = "#ffd54f";
      ctx.lineWidth = 1;
      ctx.strokeRect(view.x0 * sx, view.z0 * sy, (view.x1 - view.x0) * sx, (view.z1 - view.z0) * sy);
    }
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

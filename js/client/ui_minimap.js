// Minimap, the right block of the bottom bar: the map's walkable layer (lanes light, rock and water dark), every visible
// unit and the camera's ground rectangle. Heroes are larger than creeps. Own hero and selection are white, allies are
// their team colour with a ring, enemies their team colour. The camera rectangle is drawn with a dark under-stroke so it
// reads on both light lanes and dark rock. A press or drag on the minimap moves the camera to that map point.
import { teamOf } from "./moba_select.js";

const SIZE = { hero: 5, creep: 3 };

export function dotSize({ own, hero }) {
  return own || hero ? SIZE.hero : SIZE.creep;
}

export function createMinimap({ canvas, mapSize, lanes, units, selected, localPlayer, playerColor, teams, viewRect, centre, isHero = () => false }) {
  canvas.dataset.ui = "minimap";
  canvas.height = Math.max(1, Math.round((canvas.width * mapSize.h) / mapSize.w));
  canvas.style.height = `${canvas.height}px`;
  const ctx = canvas.getContext("2d");

  function draw(list) {
    ctx.fillStyle = "#1a2129";
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
      const hero = isHero(u.id);
      const size = dotSize({ own, hero });
      const x = u.x * sx;
      const y = u.z * sy;
      ctx.fillStyle = u.id === active || own ? "#ffffff" : `rgb(${(r * 255) | 0},${(g * 255) | 0},${(b * 255) | 0})`;
      ctx.fillRect(x - size / 2, y - size / 2, size, size);
      if (hero) {
        ctx.strokeStyle = "#0b0f13";
        ctx.lineWidth = 1;
        ctx.strokeRect(x - size / 2 - 0.5, y - size / 2 - 0.5, size + 1, size + 1);
      }
      if (ally) {
        ctx.strokeStyle = "#7cc7ff";
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
      ctx.strokeStyle = "rgba(0,0,0,.7)";
      ctx.strokeRect(x, y, w, h);
      ctx.lineWidth = 1;
      ctx.strokeStyle = "#ffd54f";
      ctx.strokeRect(x, y, w, h);
    }
    ctx.strokeStyle = "#3b4752";
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

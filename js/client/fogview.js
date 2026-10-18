// Client fog and day-night view. A 2D canvas laid over the WebGL view darkens every vision cell the local
// player cannot see and tints the whole scene at night; enemy units outside vision are filtered out of the
// unit list. The visible set comes from the wasm Game (vision_dims, vision, day_cycle).

const FOG_ALPHA = 0.55;
const NIGHT_RGB = "16,24,72";
const NIGHT_ALPHA = 0.32;
const DUSK_TICKS = 160;

export function createFogView({ game, localPlayer, toScreen, groundAt }) {
  const supported = typeof game.vision === "function" && typeof game.vision_dims === "function";
  const canvas = document.createElement("canvas");
  canvas.style.cssText = "position:fixed;left:0;top:0;width:100vw;height:100vh;pointer-events:none;z-index:1";
  document.getElementById("view").after(canvas);
  const ctx = canvas.getContext("2d");
  let plane = null;
  let dims = [0, 0, 1];

  function refresh() {
    if (!supported) return;
    dims = game.vision_dims();
    plane = game.vision(localPlayer());
  }

  function seen(x, z) {
    if (!plane) return true;
    const cx = Math.floor(x / dims[2]);
    const cz = Math.floor(z / dims[2]);
    if (cx < 0 || cz < 0 || cx >= dims[0] || cz >= dims[1]) return false;
    return plane[cz * dims[0] + cx] !== 0;
  }

  // Own and allied-visible units stay; an enemy on a cell outside the local team's vision is hidden.
  function filter(units) {
    refresh();
    const me = localPlayer();
    return units.filter((u) => u.player === me || seen(u.x, u.z));
  }

  // 0 at midday, 1 at deep night, easing across DUSK_TICKS at each boundary.
  function nightness() {
    if (typeof game.day_cycle !== "function") return 0;
    const [clock, day, night] = game.day_cycle();
    const intoNight = clock - day;
    if (intoNight >= 0) return Math.min(1, intoNight / DUSK_TICKS, (night - intoNight) / DUSK_TICKS);
    return 0;
  }

  function draw() {
    const dpr = window.devicePixelRatio || 1;
    const w = Math.floor(window.innerWidth * dpr);
    const h = Math.floor(window.innerHeight * dpr);
    if (canvas.width !== w || canvas.height !== h) {
      canvas.width = w;
      canvas.height = h;
    }
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, window.innerWidth, window.innerHeight);
    const night = nightness();
    if (night > 0) {
      ctx.fillStyle = `rgba(${NIGHT_RGB},${NIGHT_ALPHA * night})`;
      ctx.fillRect(0, 0, window.innerWidth, window.innerHeight);
    }
    if (!plane) return;
    const [vw, vh, s] = dims;
    let anyHidden = false;
    for (let i = 0; i < plane.length; i++) if (plane[i] === 0) { anyHidden = true; break; }
    if (!anyHidden) return;
    const stride = vw + 1;
    const corners = new Array(stride * (vh + 1));
    for (let j = 0; j <= vh; j++) {
      for (let i = 0; i <= vw; i++) corners[j * stride + i] = toScreen(i * s, j * s, groundAt(i * s, j * s));
    }
    ctx.fillStyle = `rgba(0,0,0,${FOG_ALPHA})`;
    ctx.beginPath();
    for (let j = 0; j < vh; j++) {
      for (let i = 0; i < vw; i++) {
        if (plane[j * vw + i] !== 0) continue;
        const a = corners[j * stride + i];
        const b = corners[j * stride + i + 1];
        const c = corners[(j + 1) * stride + i + 1];
        const d = corners[(j + 1) * stride + i];
        ctx.moveTo(a.x, a.y);
        ctx.lineTo(b.x, b.y);
        ctx.lineTo(c.x, c.y);
        ctx.lineTo(d.x, d.y);
        ctx.closePath();
      }
    }
    ctx.fill();
  }

  return { filter, draw };
}

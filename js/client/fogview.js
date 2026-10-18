// Client fog and day-night view. A 2D canvas laid over the WebGL view darkens every vision cell the local
// player cannot see and tints the whole scene at night; enemy units outside vision are filtered out of the
// unit list. The visible set comes from the wasm Game (vision_dims, vision, day_cycle).

const FOG_ALPHA = 0.55;
const NIGHT_RGB = "16,24,72";
const NIGHT_ALPHA = 0.32;
const DUSK_TICKS = 160;
// Clip-space w below this is behind (or at) the camera plane; cell outlines are cut there before projection.
const NEAR_W = 1e-3;

// toClip(x, z, y) returns homogeneous clip coordinates [X, Y, Z, W] of a world point.
export function createFogView({ game, localPlayer, toClip, groundAt }) {
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

  // Sutherland-Hodgman cut of an n-vertex clip-space polygon (4 floats per vertex) at w = NEAR_W.
  // Writes the kept vertices into out and returns their count. A vertex behind the plane is dropped, so a
  // cell that crosses the camera keeps exactly its in-front part instead of projecting to a far-off point.
  function clipToNear(src, n, out) {
    let m = 0;
    for (let k = 0; k < n; k++) {
      const a = k * 4;
      const b = ((k + 1) % n) * 4;
      const aIn = src[a + 3] >= NEAR_W;
      const bIn = src[b + 3] >= NEAR_W;
      if (aIn) {
        for (let c = 0; c < 4; c++) out[m * 4 + c] = src[a + c];
        m++;
      }
      if (aIn !== bIn) {
        const t = (NEAR_W - src[a + 3]) / (src[b + 3] - src[a + 3]);
        for (let c = 0; c < 4; c++) out[m * 4 + c] = src[a + c] + (src[b + c] - src[a + c]) * t;
        m++;
      }
    }
    return m;
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
    const clip = new Float64Array(stride * (vh + 1) * 4);
    const known = new Uint8Array(stride * (vh + 1));
    const quad = new Float64Array(16);
    const cut = new Float64Array(32);
    const px = new Float64Array(8);
    const py = new Float64Array(8);
    const corner = (i, j) => {
      const k = j * stride + i;
      if (!known[k]) {
        const c = toClip(i * s, j * s, groundAt(i * s, j * s));
        clip[k * 4] = c[0];
        clip[k * 4 + 1] = c[1];
        clip[k * 4 + 2] = c[2];
        clip[k * 4 + 3] = c[3];
        known[k] = 1;
      }
      return k;
    };
    const vwPx = window.innerWidth;
    const vhPx = window.innerHeight;
    ctx.fillStyle = `rgba(0,0,0,${FOG_ALPHA})`;
    ctx.beginPath();
    for (let j = 0; j < vh; j++) {
      for (let i = 0; i < vw; i++) {
        if (plane[j * vw + i] !== 0) continue;
        const ks = [corner(i, j), corner(i + 1, j), corner(i + 1, j + 1), corner(i, j + 1)];
        for (let n = 0; n < 4; n++) for (let c = 0; c < 4; c++) quad[n * 4 + c] = clip[ks[n] * 4 + c];
        const m = clipToNear(quad, 4, cut);
        if (m < 3) continue;
        let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
        for (let n = 0; n < m; n++) {
          const W = cut[n * 4 + 3];
          px[n] = ((cut[n * 4] / W + 1) / 2) * vwPx;
          py[n] = ((1 - cut[n * 4 + 1] / W) / 2) * vhPx;
          minX = Math.min(minX, px[n]); maxX = Math.max(maxX, px[n]);
          minY = Math.min(minY, py[n]); maxY = Math.max(maxY, py[n]);
        }
        if (maxX < 0 || minX > vwPx || maxY < 0 || minY > vhPx) continue;
        ctx.moveTo(px[0], py[0]);
        for (let n = 1; n < m; n++) ctx.lineTo(px[n], py[n]);
        ctx.closePath();
      }
    }
    ctx.fill();
  }

  return { filter, draw };
}

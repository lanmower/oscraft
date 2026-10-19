// Client fog and day-night view. A 2D canvas laid over the WebGL view darkens every vision cell the local
// player cannot see, dims cells it has explored but cannot see now, and tints the whole scene at night; enemy
// units outside vision are filtered out of the unit list. The visible set comes from the wasm Game
// (vision_dims, vision, vision_generation, fog_on, day_cycle).

const FOG_ALPHA = 0.55;
const EXPLORED_ALPHA = 0.3;
const NIGHT_RGB = "16,24,72";
const NIGHT_ALPHA = 0.32;
const DUSK_TICKS = 160;
// Clip-space w below this is behind (or at) the camera plane; cell outlines are cut there before projection.
const NEAR_W = 1e-3;
// The fog is composed at 1/LAYER_DIV of CSS pixels, then scaled up bilinearly: the layer is cheap to fill.
const LAYER_DIV = 4;
// Blur sigma as a fraction of one projected cell: the visible-to-unseen transition spans about half a cell.
const EDGE_SIGMA_CELLS = 0.25;
const FALLBACK_CELL_PX = 32;

// toClip(x, z, y) returns homogeneous clip coordinates [X, Y, Z, W] of a world point.
export function createFogView({ game, localPlayer, toClip, groundAt }) {
  const supported = typeof game.vision === "function" && typeof game.vision_dims === "function";
  const versioned = typeof game.vision_generation === "function";
  const canvas = document.createElement("canvas");
  canvas.style.cssText = "position:fixed;left:0;top:0;width:100vw;height:100vh;pointer-events:none;z-index:1";
  document.getElementById("view").after(canvas);
  const ctx = canvas.getContext("2d");
  const layer = document.createElement("canvas");
  const layerCtx = layer.getContext("2d");
  const softEdge = "filter" in layerCtx;

  const stats = { refreshes: 0, rebuilds: 0, draws: 0, refreshMs: 0, pathMs: 0, fillMs: 0, rebuildMs: 0, drawMs: 0 };
  let plane = null;
  let dims = [0, 0, 1];
  let hidden = 0;
  let explored = null;
  let planeGeneration = -1;
  let viewer = null;
  let planeVersion = 0;
  let buffers = null;
  const sample = new Float64Array(12);
  const builtSample = new Float64Array(12);
  let builtVersion = -1;
  let builtW = -1;
  let builtH = -1;

  function refresh() {
    if (!supported) return;
    const who = localPlayer();
    if (versioned) {
      const generation = game.vision_generation();
      if (generation === planeGeneration && who === viewer) return;
      planeGeneration = generation;
    }
    const t0 = performance.now();
    stats.refreshes++;
    viewer = who;
    const nextDims = game.vision_dims();
    const next = game.vision(who);
    const sameShape = plane !== null && plane.length === next.length
      && nextDims[0] === dims[0] && nextDims[1] === dims[1] && nextDims[2] === dims[2];
    let changed = !sameShape;
    if (sameShape) {
      for (let i = 0; i < next.length; i++) {
        if (next[i] !== plane[i]) { changed = true; break; }
      }
    }
    if (!sameShape) {
      dims = nextDims;
      explored = new Uint8Array(next.length);
      buffers = null;
    }
    if (changed) {
      plane = next;
      planeVersion++;
      hidden = 0;
      const fogOn = typeof game.fog_on === "function" ? game.fog_on() : true;
      for (let i = 0; i < next.length; i++) {
        if (next[i] === 0) hidden++;
        else if (fogOn) explored[i] = 1;
      }
    }
    stats.refreshMs += performance.now() - t0;
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

  function allocBuffers() {
    const [vw, vh] = dims;
    const stride = vw + 1;
    return {
      stride,
      clip: new Float64Array(stride * (vh + 1) * 4),
      known: new Uint8Array(stride * (vh + 1)),
      quad: new Float64Array(16),
      cut: new Float64Array(32),
      px: new Float64Array(8),
      py: new Float64Array(8),
    };
  }

  // Writes the camera-dependent part of the key: three world points projected to clip space.
  function sampleCamera() {
    const [vw, vh, s] = dims;
    const points = [[0, 0], [vw * s, vh * s], [(vw * s) / 2, (vh * s) / 2]];
    for (let n = 0; n < 3; n++) {
      const [x, z] = points[n];
      const c = toClip(x, z, groundAt(x, z));
      for (let k = 0; k < 4; k++) sample[n * 4 + k] = c[k];
    }
  }

  function rebuildLayer(cssW, cssH) {
    const [vw, vh, s] = dims;
    if (!buffers) buffers = allocBuffers();
    const { stride, clip, known, quad, cut, px, py } = buffers;
    known.fill(0);
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
    const screen = (k) => {
      const W = clip[k * 4 + 3];
      return W < NEAR_W ? null : [((clip[k * 4] / W + 1) / 2) * cssW, ((1 - clip[k * 4 + 1] / W) / 2) * cssH];
    };
    const ci = vw >> 1;
    const cj = vh >> 1;
    corner(ci, cj);
    corner(ci + 1, cj);
    const a = screen(cj * stride + ci);
    const b = screen(cj * stride + ci + 1);
    let cellPx = FALLBACK_CELL_PX;
    if (a && b) cellPx = Math.hypot(b[0] - a[0], b[1] - a[1]) || FALLBACK_CELL_PX;
    const sigma = Math.min(40, Math.max(0.5, EDGE_SIGMA_CELLS * cellPx)) / LAYER_DIV;

    const lw = Math.max(1, Math.ceil(cssW / LAYER_DIV));
    const lh = Math.max(1, Math.ceil(cssH / LAYER_DIV));
    if (layer.width !== lw || layer.height !== lh) {
      layer.width = lw;
      layer.height = lh;
    }
    layerCtx.setTransform(1 / LAYER_DIV, 0, 0, 1 / LAYER_DIV, 0, 0);
    layerCtx.clearRect(0, 0, cssW, cssH);
    const unseenPath = new Path2D();
    const exploredPath = new Path2D();
    const tPath = performance.now();
    for (let j = 0; j < vh; j++) {
      for (let i = 0; i < vw; i++) {
        const idx = j * vw + i;
        if (plane[idx] !== 0) continue;
        const target = explored[idx] ? exploredPath : unseenPath;
        const ks = [corner(i, j), corner(i + 1, j), corner(i + 1, j + 1), corner(i, j + 1)];
        for (let n = 0; n < 4; n++) for (let c = 0; c < 4; c++) quad[n * 4 + c] = clip[ks[n] * 4 + c];
        const m = clipToNear(quad, 4, cut);
        if (m < 3) continue;
        let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
        for (let n = 0; n < m; n++) {
          const W = cut[n * 4 + 3];
          px[n] = ((cut[n * 4] / W + 1) / 2) * cssW;
          py[n] = ((1 - cut[n * 4 + 1] / W) / 2) * cssH;
          minX = Math.min(minX, px[n]); maxX = Math.max(maxX, px[n]);
          minY = Math.min(minY, py[n]); maxY = Math.max(maxY, py[n]);
        }
        if (maxX < 0 || minX > cssW || maxY < 0 || minY > cssH) continue;
        target.moveTo(px[0], py[0]);
        for (let n = 1; n < m; n++) target.lineTo(px[n], py[n]);
        target.closePath();
      }
    }
    const tFill = performance.now();
    stats.pathMs += tFill - tPath;
    if (softEdge) layerCtx.filter = `blur(${sigma}px)`;
    layerCtx.fillStyle = `rgba(0,0,0,${EXPLORED_ALPHA})`;
    layerCtx.fill(exploredPath);
    layerCtx.fillStyle = `rgba(0,0,0,${FOG_ALPHA})`;
    layerCtx.fill(unseenPath);
    if (softEdge) layerCtx.filter = "none";
    stats.fillMs += performance.now() - tFill;
    stats.rebuilds++;
  }

  // Rebuilds the fog layer only when the visible plane, the viewport or the camera sample changed.
  function ensureLayer(cssW, cssH) {
    sampleCamera();
    let same = builtVersion === planeVersion && builtW === cssW && builtH === cssH;
    for (let k = 0; same && k < 12; k++) if (sample[k] !== builtSample[k]) same = false;
    if (same) return;
    const t0 = performance.now();
    rebuildLayer(cssW, cssH);
    builtSample.set(sample);
    builtVersion = planeVersion;
    builtW = cssW;
    builtH = cssH;
    stats.rebuildMs += performance.now() - t0;
  }

  // Night tint first, then the cached fog layer as one scaled blit: no per-cell work runs on a frame.
  function draw() {
    const t0 = performance.now();
    stats.draws++;
    refresh();
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
    if (plane && hidden > 0) {
      ensureLayer(window.innerWidth, window.innerHeight);
      ctx.drawImage(layer, 0, 0, window.innerWidth, window.innerHeight);
    }
    stats.drawMs += performance.now() - t0;
  }

  return { filter, draw, stats };
}

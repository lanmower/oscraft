// Client visual effects. The sim queues display events (game.effect, rust/crates/sim/src/fx.rs) and the page drains them
// once per frame. Each event is drawn by its kind's renderer (ring, burst, float, projectile, flash or flag), with the
// geometry and colours the pack gives that kind (UI.effects in the scenario's moba_ui.json). Nothing here decides which
// effect shows: that is a mod rule. Team and selection rings are drawn for every unit, since they show state, not events.
// Every effect lives in a fixed-capacity pool of preallocated slots: a spawn past the cap is dropped, an expired slot is
// swap-removed, and nothing allocates per frame except the overlay's text draw. Glow meshes are drawn through the
// renderer's drawMesh with the additive blend mode (mesh.blend = "add", mesh.alpha); numbers are drawn on a 2D overlay.
import { translation } from "./render3d.js";
import { UI } from "./pack_load.js";

const FIXED_ONE = 65536;
const TEAM_RING_RADIUS = 0.8;
const SELECT_RING_RADIUS = 1.1;
const RING_LIFT = 0.12;
const DEFAULT_BURST_S = 0.6;
const DEFAULT_LIFT = 0.6;
const DEFAULT_COUNT = 14;
const DEFAULT_SPEED = 2.5;
const DEFAULT_PROJECTILE_SPEED = 14;
const DEFAULT_FLOAT_S = 1.1;
const DEFAULT_RISE_PX = 48;
const DEFAULT_TEXT_PX = 16;
const DEFAULT_FLASH_S = 0.18;
const COST_SMOOTHING = 0.05;
const SELECT_COLOR = [0.95, 0.85, 0.3];
const WHITE = [1, 1, 1];
const CAP_RINGS = 48;
const CAP_PARTICLES = 160;
const CAP_PROJECTILES = 48;
const CAP_FLOATS = 32;
const CAP_FLASHES = 32;
const NO_PARAMS = {};

// The pack's effect data: colours by name ([r, g, b] in 0..1, or a CSS colour) and the kinds the page knows how to draw.
const EFFECTS = UI.effects ?? { colours: {}, kinds: {} };

function colourOf(name) {
  const c = EFFECTS.colours[name];
  return c === undefined ? WHITE : c;
}

// An [r, g, b] list in 0..1 from a pack colour, for the glow meshes.
function rgbOf(c) {
  if (Array.isArray(c)) return c;
  const n = parseInt(c.slice(1), 16);
  return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255];
}

// A CSS colour from a pack colour, for the overlay text.
function cssOf(c) {
  if (!Array.isArray(c)) return c;
  return `rgb(${c.map((v) => Math.round(v * 255)).join(",")})`;
}

function glowMesh(positions, indices) {
  const normals = new Float32Array(positions.length);
  for (let i = 1; i < normals.length; i += 3) normals[i] = 1;
  return { positions: new Float32Array(positions), normals, indices: new Uint16Array(indices), blend: "add", alpha: 1, color: [1, 1, 1] };
}

// Flat annulus in the XZ plane with outer radius 1 and the given inner/outer ratio.
function ringMesh(inner, segments = 32) {
  const pos = [];
  const idx = [];
  for (let i = 0; i < segments; i++) {
    const a = (i / segments) * 2 * Math.PI;
    pos.push(Math.cos(a) * inner, 0, Math.sin(a) * inner, Math.cos(a), 0, Math.sin(a));
    const j = (i + 1) % segments;
    idx.push(2 * i, 2 * i + 1, 2 * j, 2 * i + 1, 2 * j + 1, 2 * j);
  }
  return glowMesh(pos, idx);
}

// Unit-size octahedron: particles, orb projectiles and flash body.
function octaMesh() {
  const pos = [1, 0, 0, -1, 0, 0, 0, 1, 0, 0, -1, 0, 0, 0, 1, 0, 0, -1];
  const idx = [0, 2, 4, 2, 1, 4, 1, 3, 4, 3, 0, 4, 2, 0, 5, 1, 2, 5, 3, 1, 5, 0, 3, 5];
  return glowMesh(pos, idx);
}

// Fixed pool: `slots` are allocated once; `count` live entries sit at the front. Spawn returns null when full.
function makePool(cap, make) {
  const slots = new Array(cap);
  for (let i = 0; i < cap; i++) slots[i] = make();
  return { slots, count: 0 };
}

function spawn(pool) {
  return pool.count < pool.slots.length ? pool.slots[pool.count++] : null;
}

// Removes entries whose `t` reached their lifetime, swapping the last live slot into each gap.
function sweep(pool, lifeOf) {
  for (let i = pool.count - 1; i >= 0; i--) {
    const e = pool.slots[i];
    if (e.t >= lifeOf(e)) {
      pool.count -= 1;
      pool.slots[i] = pool.slots[pool.count];
      pool.slots[pool.count] = e;
    }
  }
}

// The arrow model's tip turn about y, added to its heading. Zero when the model's tip points along its shaft's +x or +z.
const ARROW_TIP_TURN = 0;

// True when a model's footprint runs farther along x than along z.
function alongX(positions) {
  let x0 = Infinity, x1 = -Infinity, z0 = Infinity, z1 = -Infinity;
  for (let v = 0; v < positions.length; v += 3) {
    x0 = Math.min(x0, positions[v]);
    x1 = Math.max(x1, positions[v]);
    z0 = Math.min(z0, positions[v + 2]);
    z1 = Math.max(z1, positions[v + 2]);
  }
  return x1 - x0 >= z1 - z0;
}

// Unit-scale transform turned by `yaw` about y (the same column-major layout placed writes), at a point.
function oriented(next, x, y, z, yaw) {
  const m = placed(next, x, y, z, 1);
  const c = Math.cos(yaw);
  const s = Math.sin(yaw);
  m[0] = c;
  m[2] = -s;
  m[8] = s;
  m[10] = c;
  return m;
}

// Scratch matrices for one frame: a draw reads its transform synchronously, so each draw takes the next slot.
// The list grows to the largest frame seen (one team ring per unit) and is then reused without allocating.
const matrixScratch = [];

// Column-major uniform-scale then translate matrix written into the next scratch slot.
function placed(next, x, y, z, sx, sy = sx, sz = sx) {
  if (next.n === matrixScratch.length) matrixScratch.push(new Float32Array(16));
  const m = matrixScratch[next.n++];
  translation(x, y, z, m);
  m[0] = sx;
  m[5] = sy;
  m[10] = sz;
  return m;
}

// Writes the flag a flag effect names, and tells the page controls that read it.
function setFlag(name, value) {
  if (typeof name !== "string" || name === "") return;
  const on = value !== 0;
  window.__mobaFlags = { ...(window.__mobaFlags ?? {}), [name]: on };
  window.dispatchEvent(new CustomEvent("moba:flag", { detail: { name, on } }));
}

export function createFx({ renderer, game, toScreen, groundAt, playerColor, selected, arrow = () => null }) {
  const thin = ringMesh(0.88);
  const thick = ringMesh(0.6);
  const octa = octaMesh();
  let arrowModel = null;
  let arrowAlongX = true;
  const rings = makePool(CAP_RINGS, () => ({ x: 0, z: 0, r0: 0, r1: 0, t: 0, dur: 0, color: null, mesh: thick }));
  const particles = makePool(CAP_PARTICLES, () => ({ x: 0, y: 0, z: 0, vx: 0, vy: 0, vz: 0, t: 0, dur: 0, color: null, size: 0 }));
  const projectiles = makePool(CAP_PROJECTILES, () => ({
    from: { x: 0, y: 0, z: 0 }, pos: { x: 0, y: 0, z: 0 }, to: { x: 0, y: 0, z: 0 }, targetId: -1,
    color: null, colourName: "", arrow: false, speed: DEFAULT_PROJECTILE_SPEED, arrive: [],
  }));
  const flashes = makePool(CAP_FLASHES, () => ({ x: 0, y: 0, z: 0, t: 0, dur: DEFAULT_FLASH_S, color: null }));
  const floats = makePool(CAP_FLOATS, () => ({ x: 0, z: 0, y: 0, text: "", color: "", t: 0, dur: DEFAULT_FLOAT_S, rise: DEFAULT_RISE_PX, px: DEFAULT_TEXT_PX }));
  // Each unit's position at its last frame, kept after it dies so an effect on a dead unit still lands where it fell.
  const lastSeen = new Map();
  const draw = { n: 0 };
  let unitsById = new Map();
  // Smoothed CPU milliseconds the effect pass takes per frame (not the GPU cost of its draws).
  let costMs = 0;

  const overlay = document.createElement("canvas");
  overlay.style.cssText = "position:fixed;left:0;top:0;pointer-events:none";
  document.body.appendChild(overlay);
  const ctx = overlay.getContext("2d");

  function glow(mesh, color, alpha, transform) {
    // An effect wholly outside the visible ground box is not drawn: its translation is the fourth column and the largest
    // entry of its 3x3 part bounds the mesh reach (oriented arrows included), plus a margin. Without a view box
    // (horizon on screen) every effect draws.
    let scale = 0;
    for (let i = 0; i < 12; i++) if ((i & 3) !== 3) scale = Math.max(scale, Math.abs(transform[i]));
    if (renderer.isVisible && !renderer.isVisible(transform[12], transform[14], 2 * scale + 1)) return;
    mesh.color = color;
    mesh.alpha = alpha;
    renderer.drawMesh(mesh, transform);
  }

  function ground(x, z) { return groundAt(x, z); }

  function addRing(x, z, r0, r1, dur, color, mesh) {
    const r = spawn(rings);
    if (!r) return;
    r.x = x; r.z = z; r.r0 = r0; r.r1 = r1; r.t = 0; r.dur = dur; r.color = color; r.mesh = mesh;
  }

  function burst(x, y, z, color, count, speed, seconds) {
    for (let i = 0; i < count; i++) {
      const p = spawn(particles);
      if (!p) return;
      const a = Math.random() * 2 * Math.PI;
      const up = 0.4 + Math.random();
      const s = speed * (0.4 + Math.random() * 0.6);
      p.x = x; p.y = y; p.z = z;
      p.vx = Math.cos(a) * s; p.vy = up * speed; p.vz = Math.sin(a) * s;
      p.t = 0; p.dur = seconds * (0.6 + Math.random() * 0.6); p.color = color; p.size = 0.09 + Math.random() * 0.08;
    }
  }

  function addFloat(at, text, color, kind) {
    const f = spawn(floats);
    if (!f) return;
    f.x = at.x; f.z = at.z; f.y = at.top; f.text = text; f.color = color; f.t = 0;
    f.dur = kind.seconds ?? DEFAULT_FLOAT_S;
    f.rise = kind.rise ?? DEFAULT_RISE_PX;
    f.px = kind.size ?? DEFAULT_TEXT_PX;
  }

  function addFlash(at, color, kind) {
    const f = spawn(flashes);
    if (!f) return;
    f.x = at.x; f.y = at.top * 0.5; f.z = at.z; f.t = 0; f.dur = kind.seconds ?? DEFAULT_FLASH_S; f.color = color;
  }

  // A projectile from `from` to `to` (followed while it lives, when it names a unit). An arrow kind draws the arrow
  // model; other kinds keep the octahedron. Its arrival draws the kinds it names.
  function launch(from, to, targetId, colourName, kind) {
    const p = spawn(projectiles);
    if (!p) return;
    p.from.x = from.x; p.from.y = from.top * 0.6; p.from.z = from.z;
    p.pos.x = from.x; p.pos.y = from.top * 0.6; p.pos.z = from.z;
    p.to.x = to.x; p.to.y = to.top * 0.6; p.to.z = to.z;
    p.targetId = targetId;
    p.colourName = colourName;
    p.color = rgbOf(colourOf(colourName));
    p.arrow = kind.model === "arrow";
    p.speed = kind.speed ?? DEFAULT_PROJECTILE_SPEED;
    p.arrive = kind.arrive ?? [];
  }

  // Draws one kind at a point. `e` is the event (its text and flag fields); a colour the event leaves out is the kind's own.
  function show(kind, colourName, at, e) {
    const c = colourOf(colourName);
    switch (kind.renderer) {
      case "ring":
        addRing(at.x, at.z, kind.r0, kind.r1, kind.seconds, rgbOf(c), kind.mesh === "thick" ? thick : thin);
        break;
      case "burst":
        burst(at.x, kind.lift ?? DEFAULT_LIFT, at.z, rgbOf(c), kind.count ?? DEFAULT_COUNT, kind.speed ?? DEFAULT_SPEED, kind.seconds ?? DEFAULT_BURST_S);
        break;
      case "float":
        addFloat(at, e.text || kind.text || "", cssOf(c), kind);
        break;
      case "flash":
        addFlash(at, rgbOf(c), kind);
        if (kind.count) burst(at.x, kind.lift ?? DEFAULT_LIFT, at.z, rgbOf(c), kind.count, kind.speed ?? DEFAULT_SPEED, DEFAULT_BURST_S);
        break;
      case "flag":
        setFlag(e.name, e.value);
        break;
      default:
        break;
    }
  }

  // Where an event sits: the position of unit `id` (its last one once it has died), else the fixed-point point x, y.
  function anchorOf(id, x, y) {
    if (id >= 0) {
      const u = unitsById.get(id);
      if (u) return { x: u.x, z: u.z, top: u.top ?? 1 };
      const seen = lastSeen.get(id);
      if (seen) return seen;
    }
    return { x: x / FIXED_ONE, z: y / FIXED_ONE, top: 0 };
  }

  function spawnEvent(e) {
    const kind = EFFECTS.kinds[e.kind];
    if (!kind) return;
    const colourName = e.colour || kind.colour || "";
    if (kind.renderer === "projectile") {
      launch(anchorOf(e.source, e.x, e.y), anchorOf(e.target, e.x, e.y), e.target, colourName, kind);
      return;
    }
    show(kind, colourName, anchorOf(e.target >= 0 ? e.target : e.source, e.x, e.y), e);
  }

  // Draws the kinds a projectile's arrival names. Arrivals are drawn only: a projectile is not spawned again on arrival.
  function arrive(p) {
    for (const name of p.arrive) {
      const kind = EFFECTS.kinds[name];
      if (!kind || kind.renderer === "projectile" || kind.renderer === "flag") continue;
      show(kind, p.colourName, { x: p.to.x, z: p.to.z, top: p.to.y / 0.6 }, NO_PARAMS);
    }
  }

  // The last drained events, kept for the debug API (oscraft.debug.effects()); the queue itself is drained once.
  const RECENT_EFFECTS = 64;
  const recent = [];
  function drainEvents() {
    const g = game();
    if (typeof g.drain_effects !== "function") return;
    for (const e of JSON.parse(g.drain_effects())) {
      spawnEvent(e);
      recent.push(e);
      if (recent.length > RECENT_EFFECTS) recent.shift();
    }
  }

  function remember(u) {
    let rec = lastSeen.get(u.id);
    if (!rec) {
      rec = { x: 0, z: 0, top: 0 };
      lastSeen.set(u.id, rec);
    }
    rec.x = u.x;
    rec.z = u.z;
    rec.top = u.top ?? 1;
  }

  function advance(dt) {
    sweep(rings, (e) => e.dur);
    sweep(flashes, (e) => e.dur);
    sweep(floats, (e) => e.dur);
    for (let i = 0; i < rings.count; i++) rings.slots[i].t += dt;
    for (let i = 0; i < flashes.count; i++) flashes.slots[i].t += dt;
    for (let i = 0; i < floats.count; i++) floats.slots[i].t += dt;
    sweep(particles, (e) => e.dur);
    for (let i = 0; i < particles.count; i++) {
      const p = particles.slots[i];
      p.t += dt;
      p.x += p.vx * dt;
      p.y += p.vy * dt;
      p.z += p.vz * dt;
      p.vy -= 6 * dt;
    }
    for (let i = projectiles.count - 1; i >= 0; i--) {
      const p = projectiles.slots[i];
      const live = p.targetId >= 0 ? (unitsById.get(p.targetId) ?? lastSeen.get(p.targetId)) : null;
      if (live) { p.to.x = live.x; p.to.y = (live.top ?? 1) * 0.6; p.to.z = live.z; }
      const dx = p.to.x - p.pos.x;
      const dy = p.to.y - p.pos.y;
      const dz = p.to.z - p.pos.z;
      const d = Math.hypot(dx, dy, dz);
      const step = p.speed * dt;
      if (d <= step) {
        arrive(p);
        projectiles.count -= 1;
        projectiles.slots[i] = projectiles.slots[projectiles.count];
        projectiles.slots[projectiles.count] = p;
      } else {
        p.pos.x += (dx / d) * step;
        p.pos.y += (dy / d) * step;
        p.pos.z += (dz / d) * step;
      }
    }
  }

  function drawGlow(units) {
    const sel = selected();
    draw.n = 0;
    for (const u of units) {
      const y = ground(u.x, u.z) + RING_LIFT;
      const sz = u.id === sel ? SELECT_RING_RADIUS : TEAM_RING_RADIUS;
      glow(thin, u.id === sel ? SELECT_COLOR : playerColor(u.player), u.id === sel ? 0.95 : 0.55, placed(draw, u.x, y, u.z, sz, 1, sz));
    }
    for (let i = 0; i < flashes.count; i++) {
      const f = flashes.slots[i];
      const k = 1 - f.t / f.dur;
      glow(octa, f.color, 0.6 * k, placed(draw, f.x, f.y, f.z, 0.7, 0.9, 0.7));
    }
    for (let i = 0; i < rings.count; i++) {
      const r = rings.slots[i];
      const k = r.t / r.dur;
      const rad = r.r0 + (r.r1 - r.r0) * k;
      glow(r.mesh, r.color, 0.9 * (1 - k), placed(draw, r.x, ground(r.x, r.z) + RING_LIFT, r.z, rad, 1, rad));
    }
    for (let i = 0; i < particles.count; i++) {
      const p = particles.slots[i];
      glow(octa, p.color, 1 - p.t / p.dur, placed(draw, p.x, p.y, p.z, p.size));
    }
    const model = arrow();
    for (let i = 0; i < projectiles.count; i++) {
      const p = projectiles.slots[i];
      if (p.arrow && model) glow(model, p.color, 1, arrowAt(model, p));
      else glow(octa, p.color, 1, placed(draw, p.pos.x, p.pos.y, p.pos.z, 0.25));
    }
  }

  // The arrow's transform: its shaft lies along the flight heading. The fitted model runs along x or z (its longer
  // footprint side), so the yaw is taken on that axis.
  function arrowAt(model, p) {
    if (model !== arrowModel) {
      arrowModel = model;
      arrowAlongX = alongX(model.positions);
    }
    const hx = p.to.x - p.pos.x;
    const hz = p.to.z - p.pos.z;
    const yaw = arrowAlongX ? Math.atan2(-hz, hx) : Math.atan2(hx, hz);
    return oriented(draw, p.pos.x, p.pos.y, p.pos.z, yaw + ARROW_TIP_TURN);
  }

  function drawNumbers() {
    const dpr = window.devicePixelRatio || 1;
    const w = Math.round(window.innerWidth * dpr);
    const h = Math.round(window.innerHeight * dpr);
    if (overlay.width !== w || overlay.height !== h) {
      overlay.width = w;
      overlay.height = h;
      overlay.style.width = `${window.innerWidth}px`;
      overlay.style.height = `${window.innerHeight}px`;
    }
    ctx.clearRect(0, 0, w, h);
    if (floats.count === 0) return;
    ctx.textAlign = "center";
    for (let i = 0; i < floats.count; i++) {
      const f = floats.slots[i];
      const k = f.t / f.dur;
      const p = toScreen(f.x, f.z, f.y);
      ctx.font = `bold ${Math.round(f.px * dpr)}px sans-serif`;
      ctx.globalAlpha = 1 - k * k;
      ctx.lineWidth = 3 * dpr;
      ctx.strokeStyle = "rgba(0,0,0,0.8)";
      ctx.fillStyle = f.color;
      const px = p.x * dpr;
      const py = (p.y - f.rise * k) * dpr;
      ctx.strokeText(f.text, px, py);
      ctx.fillText(f.text, px, py);
    }
    ctx.globalAlpha = 1;
  }

  // One frame: drain the sim's display events, advance the effects by dt seconds, draw them. Call after the units
  // are drawn (glow meshes blend over the scene).
  function frame(units, dt) {
    const t0 = performance.now();
    unitsById = new Map();
    for (const u of units) {
      unitsById.set(u.id, u);
      remember(u);
    }
    drainEvents();
    advance(dt);
    drawGlow(units);
    drawNumbers();
    costMs += ((performance.now() - t0) - costMs) * COST_SMOOTHING;
  }

  function rangedInFlight() {
    let n = 0;
    for (let i = 0; i < projectiles.count; i++) if (projectiles.slots[i].arrow) n += 1;
    return n;
  }

  return {
    frame,
    counts: () => ({ rings: rings.count, particles: particles.count, projectiles: projectiles.count, ranged: rangedInFlight(), floats: floats.count, flashes: flashes.count, costMs: Number(costMs.toFixed(3)) }),
    recentEffects: () => recent.slice(),
  };
}

import { translation } from "./render3d.js";
import { UI } from "./pack_load.js";
import { packVoices } from "./audio.js";
import { barGeometry, cylinderGeometry, coneGeometry, discGeometry } from "./fx_shapes.js";

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
const DEFAULT_RING_S = 0.6;
const COST_SMOOTHING = 0.05;
const LAST_SEEN_FRAMES = 600;
const PRUNE_EVERY = 120;
const durOf = (e) => e.dur;
const SELECT_COLOR = [0.95, 0.85, 0.3];
const WHITE = [1, 1, 1];
const VOICE_GAP_MS = 1500;
const VOICE_ATTACK_GAP_MS = 8000;
const VOICE_CUES = new Map([
  ["attack_arrow", { event: "attack", speaker: "source", gap: VOICE_ATTACK_GAP_MS }],
  ["cast_ring", { event: "ability", speaker: "source", gap: VOICE_GAP_MS }],
  ["cast_burst", { event: "ability", speaker: "source", gap: VOICE_GAP_MS }],
  ["spell_orb", { event: "ability", speaker: "source", gap: VOICE_GAP_MS }],
  ["kill_burst", { event: "death", speaker: "target", gap: VOICE_GAP_MS }],
  ["respawn_ring", { event: "spawn", speaker: "target", gap: VOICE_GAP_MS }],
]);
const VOICE_LINES_MAX = 3;
const CAP_RINGS = 48;
const CAP_PARTICLES = 160;
const CAP_PROJECTILES = 48;
const CAP_FLOATS = 32;
const CAP_FLASHES = 32;
const CAP_BEAMS = 24;
const CAP_COLUMNS = 24;
const CAP_NOVAS = 24;
const CAP_AURAS = 32;
const CAP_ZONES = 24;
const CAP_ORBITS = 24;
const CAP_SPIRALS = 24;
const CAP_CONES = 24;
const NO_PARAMS = {};
const WARMED_KINDS = ["ring", "burst", "projectile", "beam", "column", "nova", "aura", "zone", "orbit", "spiral", "cone"];
const TAU = 2 * Math.PI;
const MAX_TRAIL = 12;
const MAX_MOTES = 48;
const MAX_SPOKES = 32;
const MAX_SEGMENTS = 24;
const DEFAULT_BEAM_S = 0.35;
const DEFAULT_COLUMN_S = 0.8;
const DEFAULT_NOVA_S = 0.6;
const DEFAULT_AURA_S = 3;
const DEFAULT_ZONE_S = 4;
const DEFAULT_ORBIT_S = 3;
const DEFAULT_SPIRAL_S = 1.2;
const DEFAULT_CONE_S = 0.5;
const DEFAULT_PROJECTILE_SIZE = 0.25;
const BEAM_LIFT = 0.4;
const NO_TRAIL = 1e9;
const MOTE_SIZE = 0.1;

function hash(x) {
  const s = Math.sin(x * 12.9898) * 43758.5453;
  return (s - Math.floor(s)) * 2 - 1;
}

function envelope(k) {
  return Math.min(1, k * 6, (1 - k) * 4);
}

function beamLift(top) {
  return top > 0 ? top * 0.6 : BEAM_LIFT;
}

function cap(n, max) {
  return Math.max(1, Math.min(max, Math.round(n)));
}

const EFFECTS = UI.effects ?? { colours: {}, kinds: {} };

function colourOf(name) {
  const c = EFFECTS.colours[name];
  return c === undefined ? WHITE : c;
}

function rgbOf(c) {
  if (Array.isArray(c)) return c;
  const n = parseInt(c.slice(1), 16);
  return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255];
}

function cssOf(c) {
  if (!Array.isArray(c)) return c;
  return `rgb(${c.map((v) => Math.round(v * 255)).join(",")})`;
}

function transformReach(transform) {
  let reach = 0;
  for (let i = 0; i < 12; i++) if ((i & 3) !== 3) reach = Math.max(reach, Math.abs(transform[i]));
  return reach;
}

function glowMesh(positions, indices) {
  const normals = new Float32Array(positions.length);
  for (let i = 1; i < normals.length; i += 3) normals[i] = 1;
  return { positions: new Float32Array(positions), normals, indices: new Uint16Array(indices), blend: "add", alpha: 1, color: [1, 1, 1] };
}

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

function octaMesh() {
  const pos = [1, 0, 0, -1, 0, 0, 0, 1, 0, 0, -1, 0, 0, 0, 1, 0, 0, -1];
  const idx = [0, 2, 4, 2, 1, 4, 1, 3, 4, 3, 0, 4, 2, 0, 5, 1, 2, 5, 3, 1, 5, 0, 3, 5];
  return glowMesh(pos, idx);
}

function makePool(cap, make) {
  const slots = new Array(cap);
  for (let i = 0; i < cap; i++) slots[i] = make();
  return { slots, count: 0 };
}

function spawn(pool) {
  return pool.count < pool.slots.length ? pool.slots[pool.count++] : null;
}

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

const ARROW_TIP_TURN = 0;

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

const matrixScratch = [];

function placed(next, x, y, z, sx, sy = sx, sz = sx) {
  if (next.n === matrixScratch.length) matrixScratch.push(new Float32Array(16));
  const m = matrixScratch[next.n++];
  translation(x, y, z, m);
  m[0] = sx;
  m[5] = sy;
  m[10] = sz;
  return m;
}

function barBetween(next, ax, ay, az, bx, by, bz, w) {
  const dx = bx - ax;
  const dy = by - ay;
  const dz = bz - az;
  const len = Math.hypot(dx, dy, dz);
  if (len < 1e-4) return null;
  let sx = dz;
  let sz = -dx;
  let sl = Math.hypot(sx, sz);
  if (sl < 1e-6) { sx = 0; sz = 1; sl = 1; }
  sx /= sl;
  sz /= sl;
  const ux = (dy * sz) / len;
  const uy = (dz * sx - dx * sz) / len;
  const uz = (-dy * sx) / len;
  const m = placed(next, ax, ay, az, 1);
  m[0] = dx; m[1] = dy; m[2] = dz;
  m[4] = ux * w; m[5] = uy * w; m[6] = uz * w;
  m[8] = sx * w; m[9] = 0; m[10] = sz * w;
  return m;
}

function coneAt(next, x, y, z, yaw, length, radius) {
  const c = Math.cos(yaw);
  const s = Math.sin(yaw);
  const m = placed(next, x, y, z, 1);
  m[0] = c * length; m[1] = 0; m[2] = s * length;
  m[4] = 0; m[5] = radius; m[6] = 0;
  m[8] = -s * radius; m[9] = 0; m[10] = c * radius;
  return m;
}

function setFlag(name, value) {
  if (typeof name !== "string" || name === "") return;
  const on = value !== 0;
  window.__mobaFlags = { ...(window.__mobaFlags ?? {}), [name]: on };
  window.dispatchEvent(new CustomEvent("moba:flag", { detail: { name, on } }));
}

export function createFx({ renderer, game, toScreen, groundAt, playerColor, selected, arrow = () => null, localPlayer = () => null }) {
  const thin = ringMesh(0.88);
  const thick = ringMesh(0.6);
  const octa = octaMesh();
  const barShape = barGeometry();
  const cylinderShape = cylinderGeometry();
  const coneShape = coneGeometry();
  const discShape = discGeometry();
  const bar = glowMesh(barShape.positions, barShape.indices);
  const cylinder = glowMesh(cylinderShape.positions, cylinderShape.indices);
  const cone = glowMesh(coneShape.positions, coneShape.indices);
  const disc = glowMesh(discShape.positions, discShape.indices);
  let arrowModel = null;
  let arrowAlongX = true;
  const rings = makePool(CAP_RINGS, () => ({ x: 0, z: 0, r0: 0, r1: 0, t: 0, dur: 0, color: null, mesh: thick }));
  const particles = makePool(CAP_PARTICLES, () => ({ x: 0, y: 0, z: 0, vx: 0, vy: 0, vz: 0, t: 0, dur: 0, color: null, size: 0 }));
  const projectiles = makePool(CAP_PROJECTILES, () => ({
    from: { x: 0, y: 0, z: 0 }, pos: { x: 0, y: 0, z: 0 }, to: { x: 0, y: 0, z: 0 }, targetId: -1,
    color: null, colourName: "", arrow: false, speed: DEFAULT_PROJECTILE_SPEED, arrive: [],
    size: DEFAULT_PROJECTILE_SIZE, trailCount: 0, trailSeconds: 0, trailAcc: 0, trailHead: 0,
    trailX: new Float32Array(MAX_TRAIL), trailY: new Float32Array(MAX_TRAIL), trailZ: new Float32Array(MAX_TRAIL),
    trailAge: new Float32Array(MAX_TRAIL).fill(NO_TRAIL),
  }));
  const makeShape = () => ({
    t: 0, dur: 1, color: null, id: -1, id2: -1, x: 0, z: 0, top: 0, x2: 0, z2: 0, top2: 0,
    r0: 0, r1: 0, h: 0, w: 0, n: 0, m: 0, spin: 0, aux: 0, yaw: 0, seed: 0,
  });
  const beams = makePool(CAP_BEAMS, makeShape);
  const columns = makePool(CAP_COLUMNS, makeShape);
  const novas = makePool(CAP_NOVAS, makeShape);
  const auras = makePool(CAP_AURAS, makeShape);
  const zones = makePool(CAP_ZONES, makeShape);
  const orbits = makePool(CAP_ORBITS, makeShape);
  const spirals = makePool(CAP_SPIRALS, makeShape);
  const cones = makePool(CAP_CONES, makeShape);
  const shapePools = [beams, columns, novas, auras, zones, orbits, spirals, cones];
  let shapeSeed = 0;
  const flashes = makePool(CAP_FLASHES, () => ({ x: 0, y: 0, z: 0, t: 0, dur: DEFAULT_FLASH_S, color: null }));
  const floats = makePool(CAP_FLOATS, () => ({ x: 0, z: 0, y: 0, text: "", color: "", t: 0, dur: DEFAULT_FLOAT_S, rise: DEFAULT_RISE_PX, px: DEFAULT_TEXT_PX }));
  const lastSeen = new Map();
  const draw = { n: 0 };
  const unitsById = new Map();
  let frameNo = 0;
  let fontPx = 0;
  let overlayInked = false;
  let costMs = 0;

  const overlay = document.createElement("canvas");
  overlay.style.cssText = "position:fixed;left:0;top:0;pointer-events:none";
  document.body.appendChild(overlay);
  const ctx = overlay.getContext("2d");

  function glow(mesh, color, alpha, transform) {
    if (renderer.isVisible && !renderer.isVisible(transform[12], transform[14], 2 * transformReach(transform) + 1)) return;
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
    p.speed = kind.speed > 0 ? kind.speed : DEFAULT_PROJECTILE_SPEED;
    p.arrive = kind.arrive ?? [];
    p.size = kind.size > 0 ? kind.size : DEFAULT_PROJECTILE_SIZE;
    const trail = kind.trail;
    p.trailCount = trail && trail.count > 0 && trail.seconds > 0 ? Math.min(MAX_TRAIL, trail.count | 0) : 0;
    p.trailSeconds = p.trailCount > 0 ? trail.seconds : 0;
    p.trailAcc = 0;
    p.trailHead = 0;
    p.trailAge.fill(NO_TRAIL);
  }

  function follow(s) {
    if (s.id >= 0) {
      const u = unitsById.get(s.id);
      if (u) { s.x = u.x; s.z = u.z; s.top = u.top ?? 1; } else {
        const seen = lastSeen.get(s.id);
        if (seen) { s.x = seen.x; s.z = seen.z; s.top = seen.top; }
      }
    }
    if (s.id2 >= 0) {
      const u = unitsById.get(s.id2);
      if (u) { s.x2 = u.x; s.z2 = u.z; s.top2 = u.top ?? 1; } else {
        const seen = lastSeen.get(s.id2);
        if (seen) { s.x2 = seen.x; s.z2 = seen.z; s.top2 = seen.top; }
      }
    }
  }

  function addShape(pool, at, id, seconds, color) {
    const s = spawn(pool);
    if (!s) return null;
    s.t = 0; s.dur = seconds; s.color = color; s.id = id; s.id2 = -1;
    s.x = at.x; s.z = at.z; s.top = at.top;
    s.x2 = at.x; s.z2 = at.z; s.top2 = at.top;
    s.seed = (shapeSeed += 1) * 7.31;
    return s;
  }

  const num = (v, fallback, sc = 1) => (v ?? fallback) * sc;

  function show(kind, colourName, at, e) {
    const c = colourOf(colourName);
    switch (kind.renderer) {
      case "ring":
        addRing(at.x, at.z, kind.r0, kind.r1, kind.seconds ?? DEFAULT_RING_S, rgbOf(c), kind.mesh === "thick" ? thick : thin);
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
        showShape(kind, c, at, e);
        break;
    }
  }

  function groundSpot(at, e) {
    if (e.target >= 0 || e.x === undefined || (e.x === 0 && e.y === 0 && e.source >= 0)) return at;
    return { x: e.x / FIXED_ONE, z: e.y / FIXED_ONE, top: 0 };
  }

  function showShape(kind, c, at, e) {
    const sc = e.scale > 0 ? e.scale / 1000 : 1;
    const dur = e.duration > 0 ? e.duration / 1000 : kind.seconds;
    const color = rgbOf(c);
    const anchor = e.target >= 0 ? e.target : (e.source ?? -1);
    let s = null;
    switch (kind.renderer) {
      case "beam": {
        const ex = e.x ?? at.x * FIXED_ONE;
        const ey = e.y ?? at.z * FIXED_ONE;
        const from = anchorOf(e.source ?? -1, ex, ey);
        const to = anchorOf(e.target ?? -1, ex, ey);
        s = addShape(beams, from, e.source ?? -1, dur ?? DEFAULT_BEAM_S, color);
        if (!s) return;
        s.id2 = e.target ?? -1; s.x2 = to.x; s.z2 = to.z; s.top2 = to.top;
        s.w = num(kind.width, 0.12, sc);
        s.n = cap(kind.segments ?? 8, MAX_SEGMENTS);
        s.r0 = num(kind.jitter, 0.15, sc);
        s.aux = kind.flicker ?? 0.3;
        break;
      }
      case "column":
        s = addShape(columns, groundSpot(at, e), -1, dur ?? DEFAULT_COLUMN_S, color);
        if (!s) return;
        s.r0 = num(kind.r, 1, sc);
        s.h = num(kind.height, 6, sc);
        s.aux = kind.grow ?? 0.2;
        break;
      case "nova":
        s = addShape(novas, groundSpot(at, e), -1, dur ?? DEFAULT_NOVA_S, color);
        if (!s) return;
        s.r0 = num(kind.r0, 0.5, sc);
        s.r1 = num(kind.r1, 5, sc);
        s.n = cap(num(kind.spokes, 8, sc), MAX_SPOKES);
        s.spin = kind.spin ?? 0;
        break;
      case "aura":
        s = addShape(auras, at, anchor, dur ?? DEFAULT_AURA_S, color);
        if (!s) return;
        s.r0 = num(kind.r, 1.2, sc);
        s.h = num(kind.rise, 1.6, sc);
        s.n = cap(num(kind.motes, 8, sc), MAX_MOTES);
        s.aux = kind.pulse ?? 1;
        break;
      case "zone":
        s = addShape(zones, groundSpot(at, e), -1, dur ?? DEFAULT_ZONE_S, color);
        if (!s) return;
        s.r0 = num(kind.r, 3, sc);
        s.n = cap(num(kind.spokes, 6, sc), MAX_SPOKES);
        s.spin = kind.spin ?? 0.5;
        s.m = cap(num(kind.motes, 10, sc), MAX_MOTES);
        break;
      case "orbit":
        s = addShape(orbits, at, anchor, dur ?? DEFAULT_ORBIT_S, color);
        if (!s) return;
        s.n = cap(num(kind.count, 3, sc), MAX_MOTES);
        s.r0 = num(kind.r, 1, sc);
        s.h = num(kind.height, 1, sc);
        s.spin = kind.speed ?? 2;
        break;
      case "spiral":
        s = addShape(spirals, at, anchor, dur ?? DEFAULT_SPIRAL_S, color);
        if (!s) return;
        s.aux = kind.turns ?? 2;
        s.h = num(kind.height, 2.5, sc);
        s.n = cap(num(kind.motes, 16, sc), MAX_MOTES);
        break;
      case "cone": {
        const ex = e.x ?? at.x * FIXED_ONE;
        const ey = e.y ?? at.z * FIXED_ONE;
        const from = anchorOf(e.source ?? -1, ex, ey);
        const to = anchorOf(e.target ?? -1, ex, ey);
        s = addShape(cones, from, -1, dur ?? DEFAULT_CONE_S, color);
        if (!s) return;
        s.r0 = num(kind.length, 4, sc);
        s.aux = ((kind.angle ?? 60) * Math.PI) / 180;
        s.n = cap(num(kind.count, 10, sc), MAX_MOTES);
        s.yaw = Math.atan2(to.z - from.z, to.x - from.x);
        break;
      }
      default:
        break;
    }
  }

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

  function arrive(p) {
    for (const name of p.arrive) {
      const kind = EFFECTS.kinds[name];
      if (!kind || kind.renderer === "projectile" || kind.renderer === "flag") continue;
      show(kind, p.colourName, { x: p.to.x, z: p.to.z, top: p.to.y / 0.6 }, NO_PARAMS);
    }
  }

  const RECENT_EFFECTS = 64;
  const recent = [];

  let namesGame = null;
  let typeNames = [];
  function typeNameOf(type) {
    const g = game();
    if (g !== namesGame) {
      namesGame = g;
      typeNames = typeof g.unit_type_names === "function" ? g.unit_type_names() : [];
    }
    return typeNames[type] ?? null;
  }

  let voicedHeroes = null;
  let voicedLoading = false;
  function heroesWithVoice() {
    const voices = packVoices();
    if (voicedHeroes === null && voices && !voicedLoading) {
      voicedLoading = true;
      voices.heroes().then((names) => { voicedHeroes = new Set(names); });
    }
    return voicedHeroes;
  }

  const cuePending = new Set();
  const cueStarted = new Map();
  function cueVoice(e) {
    const cue = VOICE_CUES.get(e.kind);
    if (!cue) return;
    const speaker = e[cue.speaker];
    if (!(speaker >= 0)) return;
    const voices = packVoices();
    if (!voices || voices.isMuted()) return;
    const seen = unitsById.get(speaker) ?? lastSeen.get(speaker);
    if (!seen || seen.player !== localPlayer()) return;
    const hero = typeNameOf(seen.type);
    const heroes = heroesWithVoice();
    if (!hero || !heroes || !heroes.has(hero)) return;
    if (cuePending.has(hero) || cuePending.size + voices.playing() >= VOICE_LINES_MAX) return;
    const last = cueStarted.get(hero);
    if (last !== undefined && performance.now() - last < cue.gap) return;
    cuePending.add(hero);
    voices.play(hero, cue.event).then((started) => {
      cuePending.delete(hero);
      if (started) cueStarted.set(hero, performance.now());
    });
  }

  function drainEvents() {
    const g = game();
    if (typeof g.drain_effects !== "function") return;
    for (const e of JSON.parse(g.drain_effects())) {
      spawnEvent(e);
      cueVoice(e);
      recent.push(e);
      if (recent.length > RECENT_EFFECTS) recent.shift();
    }
  }

  function remember(u) {
    let rec = lastSeen.get(u.id);
    if (!rec) {
      rec = { x: 0, z: 0, top: 0, frame: 0 };
      lastSeen.set(u.id, rec);
    }
    rec.x = u.x;
    rec.z = u.z;
    rec.top = u.top ?? 1;
    rec.type = u.type;
    rec.player = u.player;
    rec.frame = frameNo;
  }

  function forgetStale() {
    for (const [id, rec] of lastSeen) {
      if (frameNo - rec.frame > LAST_SEEN_FRAMES) lastSeen.delete(id);
    }
  }

  function advance(dt) {
    for (let i = 0; i < rings.count; i++) rings.slots[i].t += dt;
    for (let i = 0; i < flashes.count; i++) flashes.slots[i].t += dt;
    for (let i = 0; i < floats.count; i++) floats.slots[i].t += dt;
    for (let i = 0; i < particles.count; i++) {
      const p = particles.slots[i];
      p.t += dt;
      p.x += p.vx * dt;
      p.y += p.vy * dt;
      p.z += p.vz * dt;
      p.vy -= 6 * dt;
    }
    sweep(rings, durOf);
    sweep(flashes, durOf);
    sweep(floats, durOf);
    sweep(particles, durOf);
    for (const pool of shapePools) {
      for (let i = 0; i < pool.count; i++) {
        const s = pool.slots[i];
        s.t += dt;
        follow(s);
      }
      sweep(pool, durOf);
    }
    for (let i = projectiles.count - 1; i >= 0; i--) {
      const p = projectiles.slots[i];
      if (p.trailCount > 0) advanceTrail(p, dt);
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

  function advanceTrail(p, dt) {
    for (let i = 0; i < MAX_TRAIL; i++) p.trailAge[i] += dt;
    p.trailAcc += dt;
    if (p.trailAcc < p.trailSeconds / p.trailCount) return;
    p.trailAcc = 0;
    const h = p.trailHead;
    p.trailX[h] = p.pos.x; p.trailY[h] = p.pos.y; p.trailZ[h] = p.pos.z; p.trailAge[h] = 0;
    p.trailHead = (h + 1) % p.trailCount;
  }

  function drawTrail(p) {
    for (let i = 0; i < p.trailCount; i++) {
      const k = p.trailAge[i] / p.trailSeconds;
      if (k >= 1) continue;
      glow(octa, p.color, 0.6 * (1 - k), placed(draw, p.trailX[i], p.trailY[i], p.trailZ[i], p.size * (1 - k * 0.7)));
    }
  }

  function drawBeams() {
    for (let i = 0; i < beams.count; i++) {
      const b = beams.slots[i];
      const k = b.t / b.dur;
      const step = Math.floor(b.t * 30);
      const ay = ground(b.x, b.z) + beamLift(b.top);
      const by = ground(b.x2, b.z2) + beamLift(b.top2);
      const alpha = 0.9 * (1 - k * k) * (1 - b.aux * (hash(b.seed + step) * 0.5 + 0.5));
      let px = b.x;
      let py = ay;
      let pz = b.z;
      for (let j = 1; j <= b.n; j++) {
        const f = j / b.n;
        let qx = b.x + (b.x2 - b.x) * f;
        let qy = ay + (by - ay) * f;
        let qz = b.z + (b.z2 - b.z) * f;
        if (j < b.n) {
          const q = b.seed + step * 3.7 + j * 1.9;
          qx += hash(q) * b.r0;
          qy += hash(q + 0.5) * b.r0;
          qz += hash(q + 1.1) * b.r0;
        }
        const m = barBetween(draw, px, py, pz, qx, qy, qz, b.w);
        if (m) glow(bar, b.color, alpha, m);
        px = qx; py = qy; pz = qz;
      }
    }
  }

  function drawColumns() {
    for (let i = 0; i < columns.count; i++) {
      const c = columns.slots[i];
      const k = c.t / c.dur;
      const rise = c.aux > 0 ? Math.min(1, k / c.aux) : 1;
      const fade = k < 0.5 ? 1 : 2 * (1 - k);
      glow(cylinder, c.color, 0.7 * fade, placed(draw, c.x, ground(c.x, c.z), c.z, c.r0, c.h * rise, c.r0));
    }
  }

  function drawNovas() {
    for (let i = 0; i < novas.count; i++) {
      const n = novas.slots[i];
      const k = n.t / n.dur;
      const rad = n.r0 + (n.r1 - n.r0) * (1 - (1 - k) * (1 - k));
      const y = ground(n.x, n.z) + RING_LIFT;
      const alpha = 0.9 * (1 - k);
      glow(thick, n.color, alpha, placed(draw, n.x, y, n.z, rad, 1, rad));
      for (let j = 0; j < n.n; j++) {
        const a = n.spin * n.t + (j / n.n) * TAU;
        const ca = Math.cos(a);
        const sa = Math.sin(a);
        const m = barBetween(draw, n.x + ca * rad * 0.55, y, n.z + sa * rad * 0.55, n.x + ca * rad, y, n.z + sa * rad, 0.07);
        if (m) glow(bar, n.color, alpha, m);
      }
    }
  }

  function drawAuras() {
    for (let i = 0; i < auras.count; i++) {
      const a = auras.slots[i];
      const k = a.t / a.dur;
      const env = envelope(k);
      const beat = 0.5 + 0.5 * Math.sin(a.t * a.aux * TAU);
      const y = ground(a.x, a.z);
      const rad = a.r0 * (0.92 + 0.08 * beat);
      glow(thin, a.color, env * (0.4 + 0.3 * beat), placed(draw, a.x, y + RING_LIFT, a.z, rad, 1, rad));
      for (let j = 0; j < a.n; j++) {
        const phase = (a.t * 0.7 + j / a.n) % 1;
        const ang = j * 2.399963 + a.t * 0.8;
        glow(octa, a.color, env * (1 - phase) * 0.9, placed(draw, a.x + Math.cos(ang) * rad * 0.85, y + phase * a.h, a.z + Math.sin(ang) * rad * 0.85, MOTE_SIZE));
      }
    }
  }

  function drawZones() {
    for (let i = 0; i < zones.count; i++) {
      const z = zones.slots[i];
      const k = z.t / z.dur;
      const env = envelope(k);
      const y = ground(z.x, z.z) + RING_LIFT * 0.5;
      glow(disc, z.color, env * 0.22, placed(draw, z.x, y, z.z, z.r0, 1, z.r0));
      glow(thin, z.color, env * 0.8, placed(draw, z.x, y + 0.02, z.z, z.r0, 1, z.r0));
      for (let j = 0; j < z.n; j++) {
        const a = z.spin * z.t + (j / z.n) * TAU;
        const ca = Math.cos(a);
        const sa = Math.sin(a);
        const m = barBetween(draw, z.x + ca * z.r0 * 0.15, y + 0.01, z.z + sa * z.r0 * 0.15, z.x + ca * z.r0 * 0.92, y + 0.01, z.z + sa * z.r0 * 0.92, 0.05);
        if (m) glow(bar, z.color, env * 0.5, m);
      }
      for (let j = 0; j < z.m; j++) {
        const phase = (z.t * 0.6 + hash(z.seed + j) * 0.5 + 0.5) % 1;
        const ang = hash(z.seed + j * 2.3) * Math.PI + z.spin * z.t * 0.5;
        const rr = z.r0 * (0.2 + 0.75 * (hash(z.seed + j * 5.1) * 0.5 + 0.5));
        glow(octa, z.color, env * (1 - phase) * 0.9, placed(draw, z.x + Math.cos(ang) * rr, y + phase * 1.2, z.z + Math.sin(ang) * rr, MOTE_SIZE));
      }
    }
  }

  function drawOrbits() {
    for (let i = 0; i < orbits.count; i++) {
      const o = orbits.slots[i];
      const env = envelope(o.t / o.dur);
      const y = ground(o.x, o.z) + o.h;
      for (let j = 0; j < o.n; j++) {
        const a = o.spin * o.t + (j / o.n) * TAU;
        const bob = Math.sin(o.t * 3 + j) * 0.1;
        glow(octa, o.color, env * 0.95, placed(draw, o.x + Math.cos(a) * o.r0, y + bob, o.z + Math.sin(a) * o.r0, MOTE_SIZE * 1.4));
      }
    }
  }

  function drawSpirals() {
    for (let i = 0; i < spirals.count; i++) {
      const s = spirals.slots[i];
      const k = s.t / s.dur;
      const env = envelope(k);
      const y = ground(s.x, s.z);
      for (let j = 0; j < s.n; j++) {
        const f = s.n > 1 ? j / (s.n - 1) : 0;
        const a = TAU * s.aux * f + s.t * 4;
        const rad = 0.7 * (1 - 0.3 * f);
        glow(octa, s.color, env * (0.4 + 0.6 * f), placed(draw, s.x + Math.cos(a) * rad, y + f * s.h, s.z + Math.sin(a) * rad, MOTE_SIZE));
      }
    }
  }

  function drawCones() {
    for (let i = 0; i < cones.count; i++) {
      const c = cones.slots[i];
      const k = c.t / c.dur;
      const env = envelope(k);
      const base = c.r0 * Math.tan(c.aux / 2);
      const y = ground(c.x, c.z) + BEAM_LIFT;
      glow(cone, c.color, env * 0.35, coneAt(draw, c.x, y, c.z, c.yaw, c.r0, base));
      const cy = Math.cos(c.yaw);
      const sy = Math.sin(c.yaw);
      for (let j = 0; j < c.n; j++) {
        const f = (c.t * 1.5 + j / c.n) % 1;
        const side = hash(c.seed + j * 3.3) * f * base;
        const up = hash(c.seed + j * 6.7) * f * base * 0.5;
        glow(octa, c.color, env * (1 - f) * 0.9, placed(draw, c.x + cy * c.r0 * f - sy * side, y + up, c.z + sy * c.r0 * f + cy * side, MOTE_SIZE));
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
      else glow(octa, p.color, 1, placed(draw, p.pos.x, p.pos.y, p.pos.z, p.size));
      if (p.trailCount > 0) drawTrail(p);
    }
    drawBeams();
    drawColumns();
    drawNovas();
    drawAuras();
    drawZones();
    drawOrbits();
    drawSpirals();
    drawCones();
  }

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
      fontPx = 0;
    }
    if (floats.count === 0) {
      if (overlayInked) ctx.clearRect(0, 0, w, h);
      overlayInked = false;
      return;
    }
    ctx.clearRect(0, 0, w, h);
    overlayInked = true;
    ctx.textAlign = "center";
    for (let i = 0; i < floats.count; i++) {
      const f = floats.slots[i];
      const k = f.t / f.dur;
      const p = toScreen(f.x, f.z, f.y);
      const size = Math.round(f.px * dpr);
      if (size !== fontPx) {
        fontPx = size;
        ctx.font = `bold ${size}px sans-serif`;
      }
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

  const GROUND_COLOR = [1, 0.82, 0.3];
  let groundClock = 0;
  function drawGroundItems(dt) {
    const g = game();
    if (!g || typeof g.ground_items !== "function") return;
    const flat = g.ground_items();
    groundClock += dt;
    for (let i = 0; i + 4 < flat.length; i += 5) {
      const x = flat[i + 3];
      const z = flat[i + 4];
      const y = ground(x, z);
      const bob = 0.05 * Math.sin(groundClock * 3 + flat[i]);
      glow(octa, GROUND_COLOR, 0.9, placed(draw, x, y + 0.45 + bob, z, 0.28, 0.4, 0.28));
      glow(thin, GROUND_COLOR, 0.6, placed(draw, x, y + RING_LIFT, z, 0.45, 1, 0.45));
    }
  }

  function frame(units, dt) {
    const t0 = performance.now();
    unitsById.clear();
    for (const u of units) {
      unitsById.set(u.id, u);
      remember(u);
    }
    drainEvents();
    advance(dt);
    drawGlow(units);
    drawGroundItems(dt);
    drawNumbers();
    frameNo += 1;
    if (frameNo % PRUNE_EVERY === 0) forgetStale();
    costMs += ((performance.now() - t0) - costMs) * COST_SMOOTHING;
  }

  function warm(at) {
    for (const mesh of [thin, thick, octa, bar, cylinder, cone, disc]) renderer.drawMesh(mesh, at);
    const model = arrow();
    if (model) renderer.drawMesh(model, at);
    return WARMED_KINDS;
  }

  function rangedInFlight() {
    let n = 0;
    for (let i = 0; i < projectiles.count; i++) if (projectiles.slots[i].arrow) n += 1;
    return n;
  }

  return {
    frame,
    warm,
    counts: () => ({ rings: rings.count, particles: particles.count, projectiles: projectiles.count, ranged: rangedInFlight(), floats: floats.count, flashes: flashes.count, beams: beams.count, columns: columns.count, novas: novas.count, auras: auras.count, zones: zones.count, orbits: orbits.count, spirals: spirals.count, cones: cones.count, costMs: Number(costMs.toFixed(3)) }),
    recentEffects: () => recent.slice(),
  };
}

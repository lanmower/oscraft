// Ambient scenery effects: the Dota 6.88g doodads that are not objects but behaviour (bats, birds, flies, bubbles,
// smoke smudge, blighted mist). A generated mesh cannot express any of them, so each is drawn as particles: additive
// octahedra through renderer.drawMesh, which fx.js already uses, so this module needs no renderer changes.
//
// Placement is 1:1 with the reference map: each doodad record keeps its position and becomes an emitter. Every kind has
// its own motion and its own body shape, so the six read as six different things:
//   bats    erratic flutter over a roost: a flattened, wing-beat body that darts, wheels and drops
//   birds   a slow glide on a wide loop at height, with a long lazy undulation
//   flies   a tight buzzing knot close to the ground, pulsing in and out of sight
//   bubbles small pale spheres that rise from the water, swell, and pop near the top
//   smoke   soft grey puffs that climb, widen and drift on the wind, then thin out
//   mist    low, wide, flattened banks that roll slowly and never climb far
//
// Hook (three lines in play.js, next to the fx update):
//   import { createAmbientFx, AMBIENT_KINDS } from "./ambient_fx.js";
//   const ambient = createAmbientFx({ renderer, groundAt });
//   for (const d of doodads) if (AMBIENT_KINDS[d.kind]) ambient.addEmitter(d.x, d.z, AMBIENT_KINDS[d.kind]);
// then once per frame: ambient.update(dtSeconds); ambient.draw();
// ambient.setEnabled(false) switches the whole layer off: update and draw then return at once and no particle moves.
import { translation } from "./render3d.js";

// Doodad type -> emitter kind. The types are the four-letter codes from doodad-types-688g.tsv; the names come from
// Doodads.slk resolved through WorldEditStrings.txt.
export const AMBIENT_KINDS = {
  NObt: "bats",
  AObd: "birds",
  LOfl: "flies",
  ZWbg: "bubbles",
  LOsm: "smoke",
  CObl: "mist",
  LOth: "flame",
  LOtz: "flame",
  NOfp: "flame",
  NOfg: "embers",
};

// Per kind: particles per emitter, the radius they roam, their height above the ground, their body size, the colour
// and opacity of the body, and `max`, the most particles of that kind alive at once, so one kind cannot take the
// whole pool. `life` is the mean lifetime in seconds; `count` keeps that many aloft at once per emitter.
const KINDS = {
  bats: { count: 4, max: 96, spread: 1.1, height: 1.6, size: 0.085, life: 3.4, color: [0.07, 0.06, 0.09], alpha: 0.85 },
  birds: { count: 3, max: 48, spread: 2.2, height: 3.0, size: 0.075, life: 5.5, color: [0.34, 0.28, 0.22], alpha: 0.70 },
  flies: { count: 7, max: 96, spread: 0.30, height: 0.45, size: 0.030, life: 2.2, color: [0.78, 0.74, 0.30], alpha: 0.80 },
  bubbles: { count: 5, max: 32, spread: 0.28, height: 0.4, size: 0.055, life: 2.8, color: [0.66, 0.88, 0.94], alpha: 0.55 },
  smoke: { count: 4, max: 64, spread: 0.45, height: 0.6, size: 0.42, life: 5.5, color: [0.27, 0.26, 0.25], alpha: 0.40 },
  mist: { count: 4, max: 384, spread: 1.6, height: 0.25, size: 0.55, life: 7.0, color: [0.36, 0.44, 0.38], alpha: 0.22 },
  flame: { count: 5, max: 320, spread: 0.10, height: 0.55, size: 0.075, life: 0.9, color: [1.0, 0.62, 0.22], alpha: 0.95 },
  embers: { count: 6, max: 32, spread: 0.22, height: 1.1, size: 0.030, life: 2.4, color: [1.0, 0.45, 0.12], alpha: 0.85 },
};

// The reference map's doodads (scenarios/moba/doodads-688g.tsv) place 864 particles in all: 566 for the six scenery
// kinds and 298 for flame and embers. The per-kind `max` values leave headroom above that. CAP is the pool all kinds share.
const CAP = 1024;
const TAU = Math.PI * 2;
// Extra reach, in world units, so an emitter whose swarm drifts just past the view edge is not culled early.
const EMITTER_MARGIN = 2;

function glowMesh(positions, indices) {
  const normals = new Float32Array(positions.length);
  for (let i = 1; i < normals.length; i += 3) normals[i] = 1;
  return { positions: new Float32Array(positions), normals, indices: new Uint16Array(indices), blend: "add", alpha: 1, color: [1, 1, 1] };
}

// Unit-size octahedron, the same particle body fx.js uses. Scaled per axis to make the flat, round and wing shapes.
function octaMesh() {
  const pos = [1, 0, 0, -1, 0, 0, 0, 1, 0, 0, -1, 0, 0, 0, 1, 0, 0, -1];
  const idx = [0, 2, 4, 2, 1, 4, 1, 3, 4, 3, 0, 4, 2, 0, 5, 1, 2, 5, 3, 1, 5, 0, 3, 5];
  return glowMesh(pos, idx);
}

function rand(a, b) {
  return a + Math.random() * (b - a);
}

// Each kind's motion. `shape` writes the particle's world position (x, y, z) and its body scale (sx, sy, sz) from its
// own clock `p.t` and its per-particle seeds; `u` is the life fraction, 0 at birth, 1 at death.
const BEHAVIOUR = {
  bats(p, k, u, dt, ground) {
    // Roost wheel with erratic darting: the heading wanders, so the path breaks and reforms rather than circling cleanly.
    p.heading += (Math.random() - 0.5) * dt * 9;
    p.phase += dt * p.speed * (1 + 0.6 * Math.sin(p.t * 1.7));
    const r = k.spread * (0.45 + 0.55 * p.radial) * (0.8 + 0.2 * Math.sin(p.t * 0.9));
    p.x = p.ex + Math.cos(p.phase) * r + Math.cos(p.heading) * 0.15;
    p.z = p.ez + Math.sin(p.phase) * r * 0.7 + Math.sin(p.heading) * 0.15;
    p.y = ground(p.x, p.z) + k.height * (0.7 + 0.3 * Math.sin(p.t * 1.3 + p.seed)) + Math.sin(p.phase * 2.2) * 0.12;
    // Wing beat: the body flaps wide and narrow fast, flat like a wing when seen edge-on.
    const flap = 0.5 + 0.5 * Math.sin(p.t * 22 + p.seed);
    p.sx = k.size * (0.9 + 1.4 * flap);
    p.sy = k.size * 0.28;
    p.sz = k.size * 0.7;
  },
  birds(p, k, u, dt, ground) {
    // A long glide on a wide loop at height, rising and sinking on a slow undulation.
    p.phase += dt * p.speed * 0.35;
    const r = k.spread * (0.7 + 0.3 * p.radial);
    p.x = p.ex + Math.cos(p.phase) * r;
    p.z = p.ez + Math.sin(p.phase) * r;
    p.y = ground(p.x, p.z) + k.height + 0.35 * Math.sin(p.t * 0.9 + p.seed) + 0.5 * (u - 0.5);
    const flap = 0.5 + 0.5 * Math.sin(p.t * 4.2 + p.seed);
    p.sx = k.size * (1.1 + 0.5 * flap);
    p.sy = k.size * 0.22;
    p.sz = k.size * 0.6;
  },
  flies(p, k, u, dt, ground) {
    // A tight knot of buzzing: a figure-of-eight at a high frequency, close to the ground.
    p.phase += dt * p.speed;
    const r = k.spread * (0.3 + 0.7 * p.radial);
    p.x = p.ex + Math.cos(p.phase) * r + Math.cos(p.phase * 2) * r * 0.25;
    p.z = p.ez + Math.sin(p.phase * 2) * r * 0.5;
    p.y = ground(p.x, p.z) + k.height * (0.6 + 0.4 * Math.sin(p.phase * 3 + p.seed)) + 0.02;
    p.sx = p.sy = p.sz = k.size * (0.8 + 0.4 * Math.sin(p.t * 40 + p.seed));
  },
  bubbles(p, k, u, dt, ground) {
    // Rise from the water, sway a little, swell, then pop as they reach the top.
    p.phase += dt * 0.9;
    p.x = p.ex + Math.sin(p.phase + p.seed) * k.spread * 0.4;
    p.z = p.ez + Math.cos(p.phase * 0.8 + p.seed) * k.spread * 0.4;
    p.y = ground(p.x, p.z) + k.height * u;
    const swell = Math.min(1, u * 3);
    const pop = u > 0.85 ? Math.max(0, 1 - (u - 0.85) / 0.15) : 1;
    p.sx = p.sy = p.sz = k.size * (0.4 + 0.6 * swell) * pop;
  },
  smoke(p, k, u, dt, ground) {
    // A puff that climbs, widens and drifts on the wind; it thins as it goes.
    p.phase += dt * 0.25;
    p.x = p.ex + Math.cos(p.seed) * k.spread * 0.3 + p.windX * p.t;
    p.z = p.ez + Math.sin(p.seed) * k.spread * 0.3 + p.windZ * p.t;
    p.y = ground(p.x, p.z) + k.height * u * 1.6 + 0.1;
    const grow = 0.4 + 1.6 * u;
    p.sx = p.sz = k.size * grow;
    p.sy = k.size * grow * 0.7;
  },
  mist(p, k, u, dt, ground) {
    // Low banks that roll slowly over a wide patch, flattened so they lie along the ground.
    p.phase += dt * 0.12;
    const r = k.spread * (0.5 + 0.5 * p.radial);
    p.x = p.ex + Math.cos(p.phase) * r;
    p.z = p.ez + Math.sin(p.phase * 0.8) * r;
    p.y = ground(p.x, p.z) + k.height * (0.8 + 0.2 * Math.sin(p.t * 0.5 + p.seed));
    const roll = 1 + 0.15 * Math.sin(p.t * 0.6 + p.seed);
    p.sx = p.sz = k.size * 2.2 * roll;
    p.sy = k.size * 0.25;
  },
  flame(p, k, u, dt, ground) {
    // A tongue that licks upward, tapering as it climbs and flickering on two beats so the rhythm is irregular.
    p.phase += dt * 3.0;
    const flick = 0.75 + 0.25 * Math.sin(p.t * 11 + p.seed) * Math.sin(p.t * 4.3 + p.seed * 1.7);
    p.x = p.ex + Math.cos(p.seed) * k.spread + p.windX * p.t * 0.5;
    p.z = p.ez + Math.sin(p.seed) * k.spread + p.windZ * p.t * 0.5;
    p.y = ground(p.x, p.z) + k.height * u * flick;
    const taper = 1 - 0.65 * u;
    p.sx = p.sz = k.size * taper * (0.8 + 0.3 * flick);
    p.sy = k.size * (1.4 - 0.5 * u) * flick;
  },
  embers(p, k, u, dt, ground) {
    // Sparks that leave the fire, spread as they rise, slow, then gutter out in the last part of their life.
    p.phase += dt * 1.4;
    p.x = p.ex + Math.cos(p.seed) * k.spread * (0.3 + u) + p.windX * p.t;
    p.z = p.ez + Math.sin(p.seed) * k.spread * (0.3 + u) + p.windZ * p.t;
    p.y = ground(p.x, p.z) + k.height * u * (1.4 - 0.4 * u);
    const gutter = u > 0.6 ? Math.max(0, 1 - (u - 0.6) / 0.4) : 1;
    p.sx = p.sy = p.sz = k.size * gutter * (0.7 + 0.5 * Math.sin(p.t * 17 + p.seed));
  },
};

// Fade in and out so particles do not pop: 0 at birth and death, 1 across the middle.
function fadeOf(u) {
  return Math.min(1, u * 6) * Math.min(1, (1 - u) * 6);
}

export function createAmbientFx({ renderer, groundAt, enabled = true }) {
  const mesh = octaMesh();
  const drawMatrix = new Float32Array(16);
  const emitters = [];
  const pool = new Array(CAP);
  for (let i = 0; i < CAP; i++) pool[i] = { kind: null, k: null, behave: null, em: null, ex: 0, ez: 0, x: 0, y: 0, z: 0, sx: 0, sy: 0, sz: 0, t: 0, dur: 1, phase: 0, speed: 1, radial: 0, seed: 0, heading: 0, windX: 0, windZ: 0 };
  const ground = (x, z) => (groundAt ? groundAt(x, z) : 0);
  // Particles are never freed, so the first `used` pool slots are the live ones; `seeded` counts them per kind.
  const seeded = {};
  let used = 0;
  let on = enabled;

  const take = () => (used < CAP ? pool[used++] : null);

  // Gives a particle its own life and seeds. Smoke and mist are born at random ages so an emitter is never in lockstep.
  function spawn(p, e, k, staggered) {
    p.kind = e.kind;
    p.k = k;
    p.behave = BEHAVIOUR[e.kind];
    p.em = e;
    p.ex = e.x;
    p.ez = e.z;
    p.dur = k.life * rand(0.7, 1.3);
    p.t = staggered ? Math.random() * p.dur : 0;
    p.phase = rand(0, TAU);
    p.speed = rand(0.7, 1.4) * (e.kind === "flies" ? 6 : e.kind === "bats" ? 3.2 : 1);
    p.radial = Math.random();
    p.seed = rand(0, TAU);
    p.heading = rand(0, TAU);
    p.windX = rand(0.02, 0.07);
    p.windZ = rand(-0.03, 0.03);
    BEHAVIOUR[e.kind](p, k, p.t / p.dur, 0, ground);
  }

  function seed(e) {
    const k = KINDS[e.kind];
    for (let i = 0; i < k.count; i++) {
      if ((seeded[e.kind] ?? 0) >= k.max) return;
      const p = take();
      if (!p) return;
      seeded[e.kind] = (seeded[e.kind] ?? 0) + 1;
      spawn(p, e, k, true);
    }
  }

  function addEmitter(x, z, kind) {
    if (!KINDS[kind]) return;
    const e = { x, z, kind, visible: true };
    emitters.push(e);
    seed(e);
  }

  // Advances every live particle. A particle past its life is reborn at its emitter, so each swarm persists.
  // Culls emitters the camera cannot see. Their particles keep their clocks, so an emitter resumes where it left off.
  function cullEmitters() {
    for (const e of emitters) {
      const k = KINDS[e.kind];
      e.visible = !renderer.isVisible || renderer.isVisible(e.x, e.z, k.spread + k.size * 3 + EMITTER_MARGIN);
    }
  }

  function update(dt) {
    if (!on) return;
    // A long frame (a backgrounded tab, a stall) must not teleport a swarm, so the step is clamped.
    const step = Math.min(Math.max(dt, 0), 0.05);
    cullEmitters();
    for (let i = 0; i < used; i++) {
      const p = pool[i];
      if (!p.em.visible) continue;
      p.t += step;
      if (p.t >= p.dur) {
        p.t = 0;
        p.dur = p.k.life * rand(0.7, 1.3);
        p.radial = Math.random();
        p.seed = rand(0, TAU);
      }
      p.behave(p, p.k, p.t / p.dur, step, ground);
    }
  }

  function draw() {
    if (!on) return;
    for (let i = 0; i < used; i++) {
      const p = pool[i];
      if (!p.em.visible) continue;
      const fade = fadeOf(p.t / p.dur);
      if (fade <= 0 || p.sx <= 0) continue;
      if (renderer.isVisible && !renderer.isVisible(p.x, p.z, 2 * Math.max(p.sx, p.sz) + 1)) continue;
      translation(p.x, p.y, p.z, drawMatrix);
      drawMatrix[0] = p.sx;
      drawMatrix[5] = p.sy;
      drawMatrix[10] = p.sz;
      mesh.color = p.k.color;
      mesh.alpha = p.k.alpha * fade;
      renderer.drawMesh(mesh, drawMatrix);
    }
  }

  function setEnabled(value) {
    on = Boolean(value);
  }

  return { addEmitter, update, draw, emitters, setEnabled, isEnabled: () => on };
}

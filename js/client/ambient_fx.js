// Ambient scenery effects: the Dota 6.88g doodads that are not objects but behaviour (bats, birds, flies, bubbles,
// smoke smudge, blighted mist). A generated mesh cannot express any of them, so they are drawn as particles instead.
//
// Placement is 1:1 with the reference map: each of these doodad records keeps its original position and becomes an
// emitter. The module owns no state beyond its pools, and draws through the same additive-mesh path fx.js uses, so it
// needs no renderer changes.
//
// Hook (three lines in play.js, next to the fx update):
//   import { createAmbientFx, AMBIENT_KINDS } from "./ambient_fx.js";
//   const ambient = createAmbientFx({ renderer, groundAt });
//   for (const d of doodads) if (AMBIENT_KINDS[d.kind]) ambient.addEmitter(d.x, d.z, AMBIENT_KINDS[d.kind]);
// then once per frame: ambient.update(dtSeconds); ambient.draw();
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
};

// Per kind: how many particles an emitter keeps alive, how far they wander, their size and rise, and their colour.
const KINDS = {
  bats: { count: 3, spread: 0.9, height: 1.4, size: 0.10, rise: 0.35, life: 3.2, color: [0.12, 0.10, 0.14], wobble: 2.6 },
  birds: { count: 3, spread: 1.6, height: 2.6, size: 0.09, rise: 0.25, life: 4.0, color: [0.30, 0.26, 0.22], wobble: 1.4 },
  flies: { count: 6, spread: 0.35, height: 0.5, size: 0.045, rise: 0.15, life: 1.6, color: [0.20, 0.22, 0.12], wobble: 7.0 },
  bubbles: { count: 4, spread: 0.25, height: 1.2, size: 0.07, rise: 0.55, life: 2.4, color: [0.55, 0.75, 0.85], wobble: 1.0 },
  smoke: { count: 4, spread: 0.5, height: 1.8, size: 0.30, rise: 0.30, life: 4.5, color: [0.20, 0.19, 0.18], wobble: 0.8 },
  mist: { count: 5, spread: 1.3, height: 0.35, size: 0.38, rise: 0.06, life: 5.0, color: [0.16, 0.20, 0.16], wobble: 0.5 },
};

const CAP = 512;

function glowMesh(positions, indices) {
  const normals = new Float32Array(positions.length);
  for (let i = 1; i < normals.length; i += 3) normals[i] = 1;
  return { positions: new Float32Array(positions), normals, indices: new Uint16Array(indices), blend: "add", alpha: 1, color: [1, 1, 1] };
}

// Unit-size octahedron, the same particle body fx.js uses.
function octaMesh() {
  const pos = [1, 0, 0, -1, 0, 0, 0, 1, 0, 0, -1, 0, 0, 0, 1, 0, 0, -1];
  const idx = [0, 2, 4, 2, 1, 4, 1, 3, 4, 3, 0, 4, 2, 0, 5, 1, 2, 5, 3, 1, 5, 0, 3, 5];
  return glowMesh(pos, idx);
}

export function createAmbientFx({ renderer, groundAt }) {
  const mesh = octaMesh();
  const emitters = [];
  const pool = new Array(CAP);
  for (let i = 0; i < CAP; i++) pool[i] = { ex: 0, ey: 0, ez: 0, x: 0, y: 0, z: 0, size: 0, t: 0, dur: 1, phase: 0, kind: null, live: false };
  let live = 0;

  const take = () => {
    for (let i = 0; i < CAP; i++) if (!pool[i].live) return pool[i];
    return null;
  };

  // One emitter keeps `count` particles aloft; a particle that expires is replaced, so the swarm persists.
  function seed(e) {
    const k = KINDS[e.kind];
    for (let i = 0; i < k.count; i++) {
      const p = take();
      if (!p) return;
      p.ex = e.x; p.ez = e.z; p.kind = e.kind; p.live = true;
      p.phase = Math.random() * Math.PI * 2;
      p.t = Math.random() * k.life;
      p.dur = k.life * (0.7 + Math.random() * 0.6);
      p.size = k.size * (0.7 + Math.random() * 0.6);
      p.x = e.x; p.y = 0; p.z = e.z;
      live++;
    }
  }

  function addEmitter(x, z, kind) {
    if (!KINDS[kind]) return;
    const e = { x, z, kind, base: groundAt ? groundAt(x, z) : 0 };
    emitters.push(e);
    seed(e);
  }

  function update(dt) {
    for (let i = 0; i < CAP; i++) {
      const p = pool[i];
      if (!p.live) continue;
      const k = KINDS[p.kind];
      p.t += dt;
      if (p.t >= p.dur) { p.t = 0; p.phase = Math.random() * Math.PI * 2; }
      const u = p.t / p.dur;
      p.phase += dt * k.wobble;
      p.x = p.ex + Math.cos(p.phase) * k.spread;
      p.z = p.ez + Math.sin(p.phase * 1.3) * k.spread;
      p.y = (groundAt ? groundAt(p.x, p.z) : 0) + k.rise * p.t + Math.sin(p.phase * 2.1) * 0.05 + u * k.height * 0.35;
    }
  }

  function draw() {
    for (let i = 0; i < CAP; i++) {
      const p = pool[i];
      if (!p.live) continue;
      const k = KINDS[p.kind];
      // Fade in and out so particles do not pop, and thin them as they climb.
      const u = p.t / p.dur;
      const fade = Math.min(1, u * 6) * Math.min(1, (1 - u) * 6);
      const s = p.size * (0.6 + u * 0.7);
      const m = translation(p.x, p.y, p.z);
      for (let c = 0; c < 3; c++) m[c * 4 + c] = s;
      if (renderer.isVisible && !renderer.isVisible(p.x, p.z, 2 * s + 1)) continue;
      mesh.color = k.color;
      mesh.alpha = 0.55 * fade;
      renderer.drawMesh(mesh, m);
    }
  }

  return { addEmitter, update, draw, emitters };
}

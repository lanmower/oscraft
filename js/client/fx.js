// Client visual effects: selection and team rings, floating damage and gold numbers, spell bursts,
// attack projectiles and melee impacts, area rings, hit flashes, level-up and respawn effects. Every effect
// lives in a fixed-capacity pool of preallocated slots: a spawn past the cap is dropped, an expired slot is
// swap-removed, and nothing allocates per frame except the overlay's text draw. Glow meshes are drawn through
// the renderer's drawMesh with the additive blend mode (mesh.blend = "add", mesh.alpha); numbers are drawn on a
// 2D overlay canvas. Gameplay state is only read, never changed: attack targets are not in the sim, so an attack
// is attributed on the client to the nearest enemy that started a strike within ATTACK_WINDOW_S of the hit.
import { translation } from "./render3d.js";
import UI from "./moba_ui.json" with { type: "json" };

const TEAM_RING_RADIUS = 0.8;
const SELECT_RING_RADIUS = 1.1;
const RING_LIFT = 0.12;
const HIT_FLASH_S = 0.18;
const FLOAT_S = 1.1;
const FLOAT_RISE_PX = 48;
const BURST_S = 0.6;
const BURST_COUNT = 14;
const PROJECTILE_SPEED = 14;
const MELEE_RANGE = 2.6;
const ATTACK_WINDOW_S = 0.4;
const ATTACK_MAX_RANGE = 14;
const AOE_RING_S = 0.55;
const AOE_RADIUS = 3;
const LEVEL_S = 1.0;
const RESPAWN_S = 0.9;
const GOLD_KILL_WINDOW_S = 1.0;
const MAX_PLAYERS = 4;
const CAP_RINGS = 48;
const CAP_PARTICLES = 160;
const CAP_PROJECTILES = 48;
const CAP_FLOATS = 32;
const CAP_FLASHES = 32;
const CAP_STRIKES = 64;
const DAMAGE_COLOR = "#ff5a4a";
const GOLD_COLOR = UI.fx.goldColor;
const SELECT_COLOR = [0.95, 0.85, 0.3];
const SPELL_COLORS = [[0.55, 0.4, 1], [0.3, 0.8, 1], [1, 0.55, 0.2], [0.4, 1, 0.5]];
const LEVEL_COLOR = UI.fx.levelColor;
const RESPAWN_COLOR = [0.6, 1, 0.8];
const FLASH_COLOR = [1, 0.85, 0.8];
const COST_SMOOTHING = 0.05;
const MISSILE_COLOR = [1, 0.95, 0.7];

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

// Unit-size octahedron: particles, spell projectiles and flash body.
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

export function createFx({ renderer, game, toScreen, groundAt, playerColor, selected, arrow = () => null }) {
  const thin = ringMesh(0.88);
  const thick = ringMesh(0.6);
  const octa = octaMesh();
  let arrowModel = null;
  let arrowAlongX = true;
  const rings = makePool(CAP_RINGS, () => ({ x: 0, z: 0, r0: 0, r1: 0, t: 0, dur: 0, color: null, mesh: thick }));
  const particles = makePool(CAP_PARTICLES, () => ({ x: 0, y: 0, z: 0, vx: 0, vy: 0, vz: 0, t: 0, dur: 0, color: null, size: 0 }));
  const projectiles = makePool(CAP_PROJECTILES, () => ({ from: { x: 0, y: 0, z: 0 }, pos: { x: 0, y: 0, z: 0 }, to: { x: 0, y: 0, z: 0 }, targetId: -1, t: 0, dur: 0, color: null, melee: false, ranged: false }));
  const flashes = makePool(CAP_FLASHES, () => ({ id: -1, t: 0 }));
  const floats = makePool(CAP_FLOATS, () => ({ x: 0, z: 0, y: 0, text: "", color: "", t: 0 }));
  const strikes = makePool(CAP_STRIKES, () => ({ id: -1, player: -1, x: 0, y: 0, z: 0, t: 0 }));
  const lastHp = new Map();
  const lastPos = new Map();
  const lastLevel = new Map();
  const lastAttacking = new Map();
  const missing = new Set();
  const killedBy = [];
  const hits = [];
  const goldPrev = new Array(MAX_PLAYERS).fill(null);
  const draw = { n: 0 };
  let unitsById = new Map();
  let clockS = 0;
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

  function addRing(x, z, r0, r1, dur, color, mesh = thick) {
    const r = spawn(rings);
    if (!r) return;
    r.x = x; r.z = z; r.r0 = r0; r.r1 = r1; r.t = 0; r.dur = dur; r.color = color; r.mesh = mesh;
  }

  function burst(x, y, z, color, count = BURST_COUNT, speed = 3) {
    for (let i = 0; i < count; i++) {
      const p = spawn(particles);
      if (!p) return;
      const a = Math.random() * 2 * Math.PI;
      const up = 0.4 + Math.random();
      const s = speed * (0.4 + Math.random() * 0.6);
      p.x = x; p.y = y; p.z = z;
      p.vx = Math.cos(a) * s; p.vy = up * speed; p.vz = Math.sin(a) * s;
      p.t = 0; p.dur = BURST_S * (0.6 + Math.random() * 0.6); p.color = color; p.size = 0.09 + Math.random() * 0.08;
    }
  }

  function addFloat(x, z, y, text, color) {
    const f = spawn(floats);
    if (!f) return;
    f.x = x; f.z = z; f.y = y; f.text = text; f.color = color; f.t = 0;
  }

  function flash(u) {
    const f = spawn(flashes);
    if (f) { f.id = u.id; f.t = 0; }
    burst(u.x, (u.top ?? 1) * 0.6, u.z, FLASH_COLOR, 5, 1.8);
  }

  // A projectile from `from` to unit `targetId` (followed while it lives) or, for melee, an instant impact. A ranged
  // attack's projectile is drawn as the arrow model; spells keep the octahedron.
  function launch(fromX, fromY, fromZ, targetId, color, melee, ranged = false) {
    const p = spawn(projectiles);
    if (!p) return;
    p.from.x = fromX; p.from.y = fromY; p.from.z = fromZ;
    p.pos.x = fromX; p.pos.y = fromY; p.pos.z = fromZ;
    p.to.x = fromX; p.to.y = fromY; p.to.z = fromZ;
    p.targetId = targetId; p.t = 0; p.dur = 0; p.color = color; p.melee = melee; p.ranged = ranged;
  }

  // Attack attribution: a unit whose weapon starts a strike is a candidate for hits landing within the window.
  function noteStrike(u) {
    const s = spawn(strikes);
    if (!s) return;
    s.id = u.id; s.player = u.player; s.x = u.x; s.z = u.z; s.y = (u.top ?? 1) * 0.6; s.t = 0;
  }

  function attackerOf(victim) {
    let best = null;
    let bestD = ATTACK_MAX_RANGE;
    for (let i = 0; i < strikes.count; i++) {
      const s = strikes.slots[i];
      if (s.t > ATTACK_WINDOW_S || s.player === victim.player || s.id === victim.id) continue;
      const d = Math.hypot(s.x - victim.x, s.z - victim.z);
      if (d < bestD) { bestD = d; best = s; }
    }
    return best;
  }

  function readEvents() {
    if (typeof game().drain_kill_events !== "function") return;
    const ev = game().drain_kill_events();
    for (let i = 0; i + 4 < ev.length; i += 5) {
      const victim = ev[i + 3];
      const pos = lastPos.get(victim);
      if (pos) {
        burst(pos.x, 0.8, pos.z, [1, 0.45, 0.35], 18, 3.5);
        killedBy.push({ player: ev[i + 2], x: pos.x, z: pos.z, y: pos.y, at: clockS });
      }
    }
  }

  function readGold() {
    const g = game();
    if (typeof g.gold !== "function") return;
    for (let p = 0; p < MAX_PLAYERS; p++) {
      let now;
      try { now = g.gold(p); } catch { now = null; }
      if (typeof now !== "number") continue;
      const before = goldPrev[p];
      goldPrev[p] = now;
      if (before === null || now <= before) continue;
      const k = killedBy.find((e) => e.player === p && clockS - e.at <= GOLD_KILL_WINDOW_S);
      if (k) addFloat(k.x, k.z, k.y, `+${now - before}`, GOLD_COLOR);
    }
    while (killedBy.length && clockS - killedBy[0].at > GOLD_KILL_WINDOW_S) killedBy.shift();
  }

  function readUnits(units) {
    const g = game();
    const seen = new Set();
    unitsById = new Map();
    hits.length = 0;
    for (const u of units) {
      unitsById.set(u.id, u);
      seen.add(u.id);
      const top = u.top ?? 1;
      lastPos.set(u.id, { x: u.x, z: u.z, y: top, player: u.player });
      if (missing.delete(u.id)) {
        addRing(u.x, u.z, 0.3, 2.2, RESPAWN_S, RESPAWN_COLOR, thick);
        burst(u.x, 0.2, u.z, RESPAWN_COLOR, 16, 2.5);
      }
      const attacking = Boolean(u.attacking);
      if (attacking && !lastAttacking.get(u.id)) noteStrike(u);
      lastAttacking.set(u.id, attacking);
      if (u.hp >= 0) {
        const before = lastHp.get(u.id);
        if (before !== undefined && u.hp < before) {
          addFloat(u.x, u.z, top, `-${before - u.hp}`, DAMAGE_COLOR);
          flash(u);
          hits.push(u);
        }
        lastHp.set(u.id, u.hp);
      }
      if (typeof g.level === "function" && u.type >= 0) {
        let level = 0;
        try { level = g.level(u.id); } catch { level = 0; }
        const before = lastLevel.get(u.id);
        if (before !== undefined && level > before) {
          addRing(u.x, u.z, 0.4, 2.4, LEVEL_S, LEVEL_COLOR, thin);
          burst(u.x, 0.2, u.z, LEVEL_COLOR, 20, 3);
          addFloat(u.x, u.z, top, UI.fx.levelUp, GOLD_COLOR);
        }
        lastLevel.set(u.id, level);
      }
    }
    // Attribution runs after every strike of this frame is noted, so a hit never misses its attacker by list order.
    for (const victim of hits) {
      const from = attackerOf(victim);
      if (!from) continue;
      if (Math.hypot(from.x - victim.x, from.z - victim.z) <= MELEE_RANGE) burst(victim.x, 0.3, victim.z, MISSILE_COLOR, 6, 1.5);
      else launch(from.x, from.y, from.z, victim.id, MISSILE_COLOR, false, true);
    }
    for (const id of lastPos.keys()) if (!seen.has(id)) { missing.add(id); lastPos.delete(id); lastHp.delete(id); lastAttacking.delete(id); }
  }

  // A cast by `unitId` on slot `slot`: a burst and ring at the caster, then a projectile to a unit target, or an
  // area ring at the ground point (negative target, finite x, z).
  function cast(unitId, slot, target, x, z) {
    const caster = unitsById.get(unitId) || lastPos.get(unitId);
    if (!caster) return;
    const color = SPELL_COLORS[Math.abs(slot) % SPELL_COLORS.length];
    const top = (caster.top ?? caster.y ?? 1) * 0.6;
    burst(caster.x, top, caster.z, color, BURST_COUNT, 2.5);
    addRing(caster.x, caster.z, 0.3, 1.4, AOE_RING_S, color, thin);
    const dest = target >= 0 ? unitsById.get(target) : null;
    if (dest) {
      launch(caster.x, top, caster.z, target, color, false);
    } else if (Number.isFinite(x) && Number.isFinite(z)) {
      addRing(x, z, 0.3, AOE_RADIUS, AOE_RING_S, color, thick);
      burst(x, 0.3, z, color, BURST_COUNT, 2);
    }
  }

  function advance(dt) {
    sweep(rings, (e) => e.dur);
    sweep(flashes, () => HIT_FLASH_S);
    sweep(floats, () => FLOAT_S);
    for (let i = 0; i < rings.count; i++) rings.slots[i].t += dt;
    for (let i = 0; i < flashes.count; i++) flashes.slots[i].t += dt;
    for (let i = 0; i < floats.count; i++) floats.slots[i].t += dt;
    for (let i = 0; i < strikes.count; i++) strikes.slots[i].t += dt;
    sweep(strikes, () => ATTACK_WINDOW_S);
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
      const live = p.targetId >= 0 ? unitsById.get(p.targetId) : null;
      if (live) { p.to.x = live.x; p.to.y = (live.top ?? 1) * 0.6; p.to.z = live.z; }
      const dx = p.to.x - p.pos.x;
      const dy = p.to.y - p.pos.y;
      const dz = p.to.z - p.pos.z;
      const d = Math.hypot(dx, dy, dz);
      const step = PROJECTILE_SPEED * dt;
      if (d <= step) {
        burst(p.to.x, p.to.y, p.to.z, p.color, 12, 2.5);
        addRing(p.to.x, p.to.z, 0.2, 1.3, AOE_RING_S, p.color, thin);
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
      const u = unitsById.get(f.id);
      if (!u) continue;
      const k = 1 - f.t / HIT_FLASH_S;
      glow(octa, FLASH_COLOR, 0.6 * k, placed(draw, u.x, (u.top ?? 1) * 0.5, u.z, 0.7, 0.9, 0.7));
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
      if (p.ranged) glow(model, p.color, 1, arrowAt(model, p));
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
    ctx.font = `bold ${Math.round(16 * dpr)}px sans-serif`;
    ctx.textAlign = "center";
    for (let i = 0; i < floats.count; i++) {
      const f = floats.slots[i];
      const k = f.t / FLOAT_S;
      const p = toScreen(f.x, f.z, f.y);
      ctx.globalAlpha = 1 - k * k;
      ctx.lineWidth = 3 * dpr;
      ctx.strokeStyle = "rgba(0,0,0,0.8)";
      ctx.fillStyle = f.color;
      const px = p.x * dpr;
      const py = (p.y - FLOAT_RISE_PX * k) * dpr;
      ctx.strokeText(f.text, px, py);
      ctx.fillText(f.text, px, py);
    }
    ctx.globalAlpha = 1;
  }

  // One frame: read the sim's events and unit changes, advance the effects by dt seconds, draw them.
  // Call after the units are drawn (glow meshes blend over the scene).
  function frame(units, dt) {
    const t0 = performance.now();
    clockS += dt;
    readUnits(units);
    readEvents();
    readGold();
    advance(dt);
    drawGlow(units);
    drawNumbers();
    costMs += ((performance.now() - t0) - costMs) * COST_SMOOTHING;
  }

  function rangedInFlight() {
    let n = 0;
    for (let i = 0; i < projectiles.count; i++) if (projectiles.slots[i].ranged) n += 1;
    return n;
  }

  return {
    frame,
    cast,
    counts: () => ({ rings: rings.count, particles: particles.count, projectiles: projectiles.count, ranged: rangedInFlight(), floats: floats.count, flashes: flashes.count, costMs: Number(costMs.toFixed(3)) }),
  };
}

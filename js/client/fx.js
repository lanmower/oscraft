// Client visual effects: selection and team rings, floating damage and gold numbers, spell bursts,
// projectiles, area rings, hit flashes, level-up and respawn effects. Glow meshes are drawn through the
// renderer's drawMesh with the optional additive blend mode (mesh.blend = "add", mesh.alpha); numbers are
// drawn on a 2D overlay canvas. Gameplay state is only read, never changed.
import { translation } from "./render3d.js";

const TEAM_RING_RADIUS = 0.8;
const SELECT_RING_RADIUS = 1.1;
const RING_LIFT = 0.12;
const HIT_FLASH_S = 0.18;
const FLOAT_S = 1.1;
const FLOAT_RISE_PX = 48;
const BURST_S = 0.6;
const BURST_COUNT = 14;
const PROJECTILE_SPEED = 14;
const AOE_RING_S = 0.55;
const AOE_RADIUS = 3;
const LEVEL_S = 1.0;
const RESPAWN_S = 0.9;
const GOLD_KILL_WINDOW_S = 1.0;
const MAX_PLAYERS = 4;
const DAMAGE_COLOR = "#ff5a4a";
const GOLD_COLOR = "#f2c53d";
const SELECT_COLOR = [0.95, 0.85, 0.3];
const SPELL_COLORS = [[0.55, 0.4, 1], [0.3, 0.8, 1], [1, 0.55, 0.2], [0.4, 1, 0.5]];
const LEVEL_COLOR = [1, 0.9, 0.4];
const RESPAWN_COLOR = [0.6, 1, 0.8];
const FLASH_COLOR = [1, 0.85, 0.8];

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

// Unit-size octahedron: particles, projectile and flash body.
function octaMesh() {
  const pos = [1, 0, 0, -1, 0, 0, 0, 1, 0, 0, -1, 0, 0, 0, 1, 0, 0, -1];
  const idx = [0, 2, 4, 2, 1, 4, 1, 3, 4, 3, 0, 4, 2, 0, 5, 1, 2, 5, 3, 1, 5, 0, 3, 5];
  return glowMesh(pos, idx);
}

// Column-major uniform-scale then translate matrix.
function placed(x, y, z, sx, sy = sx, sz = sx) {
  const m = translation(x, y, z);
  m[0] = sx;
  m[5] = sy;
  m[10] = sz;
  return m;
}

export function createFx({ renderer, game, toScreen, groundAt, playerColor, selected }) {
  const thin = ringMesh(0.88);
  const thick = ringMesh(0.6);
  const octa = octaMesh();
  const rings = [];
  const particles = [];
  const projectiles = [];
  const floats = [];
  const flashes = [];
  const lastHp = new Map();
  const lastPos = new Map();
  const lastLevel = new Map();
  const missing = new Set();
  const killedBy = [];
  const goldPrev = new Array(MAX_PLAYERS).fill(null);
  let unitsById = new Map();
  let clockS = 0;

  const overlay = document.createElement("canvas");
  overlay.style.cssText = "position:fixed;left:0;top:0;pointer-events:none";
  document.body.appendChild(overlay);
  const ctx = overlay.getContext("2d");

  function glow(mesh, color, alpha, transform) {
    mesh.color = color;
    mesh.alpha = alpha;
    renderer.drawMesh(mesh, transform);
  }

  function ground(x, z) { return groundAt(x, z); }

  function addRing(x, z, r0, r1, dur, color, mesh = thick) {
    rings.push({ x, z, r0, r1, t: 0, dur, color, mesh });
  }

  function burst(x, y, z, color, count = BURST_COUNT, speed = 3) {
    for (let i = 0; i < count; i++) {
      const a = Math.random() * 2 * Math.PI;
      const up = 0.4 + Math.random();
      const s = speed * (0.4 + Math.random() * 0.6);
      particles.push({ x, y, z, vx: Math.cos(a) * s, vy: up * speed, vz: Math.sin(a) * s, t: 0, dur: BURST_S * (0.6 + Math.random() * 0.6), color, size: 0.09 + Math.random() * 0.08 });
    }
  }

  function addFloat(x, z, y, text, color) {
    floats.push({ x, z, y, text, color, t: 0 });
  }

  function flash(u) {
    flashes.push({ id: u.id, t: 0 });
    burst(u.x, (u.top ?? 1) * 0.6, u.z, FLASH_COLOR, 5, 1.8);
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
    for (const u of units) {
      unitsById.set(u.id, u);
      seen.add(u.id);
      const top = u.top ?? 1;
      lastPos.set(u.id, { x: u.x, z: u.z, y: top });
      if (missing.delete(u.id)) {
        addRing(u.x, u.z, 0.3, 2.2, RESPAWN_S, RESPAWN_COLOR, thick);
        burst(u.x, 0.2, u.z, RESPAWN_COLOR, 16, 2.5);
      }
      if (u.hp >= 0) {
        const before = lastHp.get(u.id);
        if (before !== undefined && u.hp < before) {
          addFloat(u.x, u.z, top, `-${before - u.hp}`, DAMAGE_COLOR);
          flash(u);
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
          addFloat(u.x, u.z, top, "LEVEL UP", GOLD_COLOR);
        }
        lastLevel.set(u.id, level);
      }
    }
    for (const id of lastPos.keys()) if (!seen.has(id)) { missing.add(id); lastPos.delete(id); lastHp.delete(id); }
  }

  // A cast by `unitId` on slot `slot` at unit `target` (negative for the ground point x, z): a cast burst at
  // the caster, then a projectile to a unit target or an area ring at a ground point.
  function cast(unitId, slot, target, x, z) {
    const caster = unitsById.get(unitId) || lastPos.get(unitId);
    if (!caster) return;
    const color = SPELL_COLORS[Math.abs(slot) % SPELL_COLORS.length];
    const top = (caster.top ?? caster.y ?? 1) * 0.6;
    burst(caster.x, top, caster.z, color, BURST_COUNT, 2.5);
    addRing(caster.x, caster.z, 0.3, 1.4, AOE_RING_S, color, thin);
    const dest = target >= 0 ? unitsById.get(target) : null;
    if (dest) {
      projectiles.push({ from: { x: caster.x, y: top, z: caster.z }, targetId: target, to: { x: dest.x, y: (dest.top ?? 1) * 0.6, z: dest.z }, pos: { x: caster.x, y: top, z: caster.z }, color });
    } else if (Number.isFinite(x) && Number.isFinite(z)) {
      addRing(x, z, 0.3, AOE_RADIUS, AOE_RING_S, color, thick);
      burst(x, 0.3, z, color, BURST_COUNT, 2);
    }
  }

  function advance(dt) {
    for (const list of [rings, particles, floats, flashes]) {
      for (let i = list.length - 1; i >= 0; i--) {
        const e = list[i];
        e.t += dt;
        if (e.t >= (e.dur ?? (list === floats ? FLOAT_S : HIT_FLASH_S))) list.splice(i, 1);
      }
    }
    for (const p of particles) {
      p.x += p.vx * dt;
      p.y += p.vy * dt;
      p.z += p.vz * dt;
      p.vy -= 6 * dt;
    }
    for (let i = projectiles.length - 1; i >= 0; i--) {
      const p = projectiles[i];
      const live = p.targetId !== undefined ? unitsById.get(p.targetId) : null;
      if (live) p.to = { x: live.x, y: (live.top ?? 1) * 0.6, z: live.z };
      const dx = p.to.x - p.pos.x;
      const dy = p.to.y - p.pos.y;
      const dz = p.to.z - p.pos.z;
      const d = Math.hypot(dx, dy, dz);
      const step = PROJECTILE_SPEED * dt;
      if (d <= step) {
        burst(p.to.x, p.to.y, p.to.z, p.color, 12, 2.5);
        addRing(p.to.x, p.to.z, 0.2, 1.3, AOE_RING_S, p.color, thin);
        projectiles.splice(i, 1);
      } else {
        p.pos.x += (dx / d) * step;
        p.pos.y += (dy / d) * step;
        p.pos.z += (dz / d) * step;
      }
    }
  }

  function drawGlow(units) {
    const sel = selected();
    for (const u of units) {
      const y = ground(u.x, u.z) + RING_LIFT;
      glow(thin, u.id === sel ? SELECT_COLOR : playerColor(u.player), u.id === sel ? 0.95 : 0.55, placed(u.x, y, u.z, u.id === sel ? SELECT_RING_RADIUS : TEAM_RING_RADIUS, 1, u.id === sel ? SELECT_RING_RADIUS : TEAM_RING_RADIUS));
    }
    for (const f of flashes) {
      const u = unitsById.get(f.id);
      if (!u) continue;
      const k = 1 - f.t / HIT_FLASH_S;
      glow(octa, FLASH_COLOR, 0.6 * k, placed(u.x, (u.top ?? 1) * 0.5, u.z, 0.7, 0.9, 0.7));
    }
    for (const r of rings) {
      const k = r.t / r.dur;
      const rad = r.r0 + (r.r1 - r.r0) * k;
      glow(r.mesh, r.color, 0.9 * (1 - k), placed(r.x, ground(r.x, r.z) + RING_LIFT, r.z, rad, 1, rad));
    }
    for (const p of particles) glow(octa, p.color, 1 - p.t / p.dur, placed(p.x, p.y, p.z, p.size));
    for (const p of projectiles) glow(octa, p.color, 1, placed(p.pos.x, p.pos.y, p.pos.z, 0.25));
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
    ctx.font = `bold ${Math.round(16 * dpr)}px sans-serif`;
    ctx.textAlign = "center";
    for (const f of floats) {
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
    clockS += dt;
    readUnits(units);
    readEvents();
    readGold();
    advance(dt);
    drawGlow(units);
    drawNumbers();
  }

  return { frame, cast, counts: () => ({ rings: rings.length, particles: particles.length, projectiles: projectiles.length, floats: floats.length }) };
}

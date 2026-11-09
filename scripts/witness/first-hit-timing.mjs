export const LEGS = [
  { label: "first target, lock on", lock: true, judged: true },
  { label: "second target, lock on", lock: true, judged: true },
  { label: "lock off (control)", lock: false, judged: false },
];

export const CONFIG = {
  legMs: 900000,
  controlMs: 60000,
  tickPollMs: 2,
  orderEveryMs: 300,
  heroWaitMs: 60000,
  minStartHp: 200,
  abortHp: 100,
  targetOwner: 255,
  reachCells: 2,
  toleranceTicks: 1,
  isolationCells: 4,
  excluded: [],
  retreat: null,
  retreatMs: 240000,
};

export async function firstHitLeg(config) {
  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const me = window.__ciLocalPlayer();
  const roster = () => window.__ciRoster();
  const distance = (a, b) => Math.hypot(a.x - b.x, a.z - b.z);
  const alive = (u) => window.__ciHp(u.id) > 0;
  const hostileToMe = (u) => window.__ciHostile(me, u.player);
  const segmentDistance = (p, a, b) => {
    const dx = b.x - a.x;
    const dz = b.z - a.z;
    const lengthSq = dx * dx + dz * dz;
    const t = lengthSq === 0 ? 0 : Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.z - a.z) * dz) / lengthSq));
    return Math.hypot(p.x - (a.x + t * dx), p.z - (a.z + t * dz));
  };
  const nearestOf = (pool, from) => pool.reduce((best, u) => (best === null || distance(u, from) < distance(best, from) ? u : best), null);
  const stopPointOf = (from, c) => {
    const dx = from.x - c.x;
    const dz = from.z - c.z;
    const length = Math.hypot(dx, dz) || 1;
    return { x: c.x + (dx / length) * config.reachCells, z: c.z + (dz / length) * config.reachCells };
  };
  const interceptorsOf = (units, c, from, heroId) => {
    const stop = stopPointOf(from, c);
    return units.filter(
      (v) =>
        v.id !== heroId && v.id !== c.id && alive(v) && hostileToMe(v) &&
        (segmentDistance(v, from, c) <= config.isolationCells || distance(v, stop) < distance(c, stop)),
    );
  };
  const pickCreep = (units, from, heroId) => {
    const candidates = units.filter(
      (u) => u.id !== heroId && !config.excluded.includes(u.id) && u.player === config.targetOwner && alive(u) && hostileToMe(u),
    );
    if (candidates.length === 0) return { creep: null, aborted: "no hostile creep to strike" };
    const isolated = candidates.filter((c) => interceptorsOf(units, c, from, heroId).length === 0);
    if (isolated.length > 0) return { creep: nearestOf(isolated, from), aborted: null };
    const nearest = nearestOf(candidates, from);
    const stop = stopPointOf(from, nearest);
    const blockers = interceptorsOf(units, nearest, from, heroId).map((v) => {
      const cause = segmentDistance(v, from, nearest) <= config.isolationCells ? "on the approach" : "nearer the stop point";
      return `${v.id} (player ${v.player}, ${v.type}, ${cause}, ${distance(v, stop).toFixed(1)} cells from the stop point)`;
    });
    return {
      creep: null,
      aborted: `no clear creep: all ${candidates.length} candidates are blocked; nearest ${nearest.id} is blocked by ${blockers.join("; ")}`,
    };
  };
  const nextTick = async (after, until) => {
    while (window.__ciTicks() === after && Date.now() < until) await sleep(config.tickPollMs);
    return window.__ciTicks();
  };
  const empty = (aborted) => ({
    ready: false,
    startHp: null,
    targetId: null,
    struckOther: false,
    reachTick: null,
    launchTick: null,
    hitTick: null,
    attackPoint: 0,
    latencyTicks: null,
    samples: 0,
    aborted,
    trace: [],
  });

  const waitStart = Date.now();
  let units = roster();
  while (!units.find((u) => u.player === me && window.__ciIsHero(u.id)) && Date.now() - waitStart < config.heroWaitMs) {
    await sleep(500);
    units = roster();
  }
  const first = units.find((u) => u.player === me && window.__ciIsHero(u.id));
  if (!first) return empty("no hero to start the leg");
  const heroId = first.id;
  while (window.__ciHp(heroId) < config.minStartHp && Date.now() - waitStart < config.heroWaitMs) await sleep(1000);
  const startHp = window.__ciHp(heroId);
  if (startHp < config.minStartHp) return { ...empty("hero below start hp"), startHp };

  units = roster();
  const startAt = units.find((u) => u.id === heroId);
  const startPosition = { x: startAt.x, z: startAt.z };
  if (config.retreat !== null && units.some((v) => v.id !== heroId && alive(v) && hostileToMe(v) && distance(v, startAt) <= config.isolationCells)) {
    const retreatUntil = Date.now() + config.retreatMs;
    window.__ciMove(heroId, config.retreat.x, config.retreat.z);
    while (Date.now() < retreatUntil) {
      await sleep(300);
      const moved = roster().find((u) => u.id === heroId);
      if (!moved || distance(moved, config.retreat) <= 1) break;
    }
    units = roster();
  }
  const origin = units.find((u) => u.id === heroId);
  const start = pickCreep(units, origin, heroId);
  if (start.creep === null) return { ...empty(start.aborted), ready: true, startHp, origin: startPosition };
  let creepId = start.creep.id;

  let approaching = true;
  let lastOrderAt = 0;
  let stamp = null;
  let samples = 0;
  let aborted = null;
  let struckOther = false;
  const trace = [];

  const legStart = Date.now();
  let tick = window.__ciTicks();
  const startTick = tick;
  while (Date.now() - legStart < config.legMs) {
    units = roster();
    const h = units.find((u) => u.id === heroId);
    if (!h || !window.__ciIsHero(heroId)) {
      aborted = "hero dead";
      break;
    }
    if (window.__ciHp(heroId) < config.abortHp) {
      aborted = "hero below abort hp";
      break;
    }
    let creep = units.find((u) => u.id === creepId);
    if ((stamp === null || stamp[0] !== creepId || stamp[1] < 0) && (!creep || window.__ciHp(creepId) <= 0)) {
      const picked = pickCreep(units, h, heroId);
      if (picked.creep === null) {
        aborted = picked.aborted;
        break;
      }
      creep = picked.creep;
      creepId = creep.id;
    }
    if (creep === undefined || creep === null) {
      aborted = "target left the roster";
      break;
    }
    if (approaching && distance(h, creep) <= config.reachCells) {
      window.__ciMove(heroId, h.x, h.z);
      approaching = false;
    } else if (approaching && Date.now() - lastOrderAt > config.orderEveryMs) {
      window.__ciMove(heroId, creep.x, creep.z);
      lastOrderAt = Date.now();
    }

    tick = await nextTick(tick, legStart + config.legMs);
    if (Date.now() >= legStart + config.legMs) break;
    stamp = window.__ciStrikeStamp(heroId);
    samples += 1;
    if (stamp === null) {
      aborted = "no strike stamp hook";
      break;
    }
    const now = roster().find((u) => u.id === heroId);
    const target = roster().find((u) => u.id === creepId);
    const last = trace[trace.length - 1];
    if (last === undefined || last.stamp.join() !== stamp.join()) {
      trace.push({ tick, d: target && now ? +distance(now, target).toFixed(2) : null, heading: now ? now.heading : null, stamp });
    }
    if (stamp[0] >= 0 && stamp[0] !== creepId && stamp[1] >= startTick) struckOther = true;
    if (stamp[0] === creepId && stamp[4] >= 0) break;
  }

  const [, reach, launch, point, land] = stamp && stamp[0] === creepId ? stamp : [-1, -1, -1, 0, -1];
  const reachTick = reach >= 0 ? reach : null;
  const hitTick = land >= 0 ? land : null;
  return {
    ready: true,
    startHp,
    origin: startPosition,
    targetId: creepId,
    struckOther,
    reachTick,
    launchTick: launch >= 0 ? launch : null,
    hitTick,
    attackPoint: point,
    latencyTicks: reachTick !== null && hitTick !== null ? hitTick - reachTick : null,
    samples,
    aborted,
    trace,
  };
}

export function verdictFirstHit(legs) {
  const reasons = [];
  const judgedTargets = new Set();
  for (const leg of legs) {
    if (!leg.judged) continue;
    if (judgedTargets.has(leg.targetId)) reasons.push(`${leg.label}: target ${leg.targetId} was already judged in an earlier leg, so its strike stamp is not this leg's`);
    judgedTargets.add(leg.targetId);
    if (!leg.ready) reasons.push(`${leg.label}: the hero did not reach the start hp of ${CONFIG.minStartHp}`);
    if (leg.lock !== "true") reasons.push(`${leg.label}: the lock was not engaged`);
    if (leg.aborted) reasons.push(`${leg.label}: the leg ended early (${leg.aborted})`);
    if (leg.reachTick == null) reasons.push(`${leg.label}: the sim never put the hero in reach of its target`);
    if (leg.struckOther) reasons.push(`${leg.label}: the hero's first strike went to another unit`);
    if (leg.hitTick == null) {
      reasons.push(`${leg.label}: no first hit`);
      continue;
    }
    if (!(leg.attackPoint > 0)) reasons.push(`${leg.label}: no swing attack point was stamped at launch`);
    else if (leg.latencyTicks > leg.attackPoint + CONFIG.toleranceTicks) {
      reasons.push(`${leg.label}: first hit ${leg.latencyTicks} ticks after reach, later than the attack point ${leg.attackPoint} plus ${CONFIG.toleranceTicks} tick`);
    }
  }
  return { ok: reasons.length === 0, reasons };
}

export async function runFirstHitTiming(page, config = CONFIG) {
  const lockButton = page.locator("#hero-lock");
  await lockButton.waitFor({ state: "visible", timeout: 120000 });
  const legs = [];
  const struckTargets = [];
  let retreat = null;
  for (const leg of LEGS) {
    if ((await lockButton.getAttribute("aria-pressed")) !== String(leg.lock)) await lockButton.click();
    await page.waitForTimeout(600);
    const lock = await lockButton.getAttribute("aria-pressed");
    const legMs = leg.judged ? config.legMs : config.controlMs;
    const measured = await page.evaluate(firstHitLeg, { ...config, legMs, excluded: [...struckTargets], retreat });
    if (retreat === null && measured.origin) retreat = measured.origin;
    if (measured.targetId !== null) struckTargets.push(measured.targetId);
    legs.push({ label: leg.label, lock, judged: leg.judged, ...measured });
  }
  return { legs, verdict: verdictFirstHit(legs) };
}

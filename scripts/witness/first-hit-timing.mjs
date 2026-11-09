export const LEGS = [
  { label: "stopped, lock on", lock: true, walk: false, judged: true },
  { label: "walking, lock on", lock: true, walk: true, judged: true },
  { label: "walking, lock off (control)", lock: false, walk: true, judged: false },
];

export const CONFIG = {
  legMs: 480000,
  controlMs: 60000,
  sampleMs: 5,
  orderEveryMs: 300,
  heroWaitMs: 60000,
  minStartHp: 200,
  abortHp: 100,
  targetOwner: 255,
  reachCells: 2,
  oscillateCells: 1.6,
  arriveCells: 0.8,
  toleranceTicks: 1,
};

export async function firstHitLeg(config) {
  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const me = window.__ciLocalPlayer();
  const roster = () => window.__ciRoster();
  const heroUnit = () => roster().find((u) => u.player === me && window.__ciIsHero(u.id));
  const distance = (a, b) => Math.hypot(a.x - b.x, a.z - b.z);
  const pointToward = (target, from, cells) => {
    const dx = from.x - target.x;
    const dz = from.z - target.z;
    const length = Math.hypot(dx, dz) || 1;
    return { x: target.x + (dx / length) * cells, z: target.z + (dz / length) * cells };
  };
  const nearestCreep = (from, heroId) => {
    let best = null;
    let nearest = Infinity;
    for (const u of roster()) {
      if (u.id === heroId || u.player !== config.targetOwner || window.__ciHp(u.id) <= 0) continue;
      if (!window.__ciHostile(me, u.player)) continue;
      const d = distance(u, from);
      if (d < nearest) {
        nearest = d;
        best = u;
      }
    }
    return best;
  };
  const empty = (aborted) => ({
    ready: false,
    startHp: null,
    targetId: null,
    reachTick: null,
    launchTick: null,
    releaseTick: null,
    hitTick: null,
    attackPoint: 0,
    launchDelayTicks: null,
    latencyTicks: null,
    samples: 0,
    aborted,
  });

  const waitStart = Date.now();
  while (!heroUnit() && Date.now() - waitStart < config.heroWaitMs) await sleep(500);
  const first = heroUnit();
  if (!first) return empty("no hero to start the leg");
  const heroId = first.id;
  while (window.__ciHp(heroId) < config.minStartHp && Date.now() - waitStart < config.heroWaitMs) await sleep(1000);
  const startHp = window.__ciHp(heroId);
  if (startHp < config.minStartHp) return { ...empty("hero below start hp"), startHp };

  const origin = heroUnit();
  let creep = nearestCreep(origin, heroId);
  if (creep === null) return { ...empty("no hostile creep to strike"), startHp };
  let creepId = creep.id;

  const approachOnly = !config.walk;
  let approaching = approachOnly;
  let toward = null;
  let lastOrderAt = 0;
  let reachTick = null;
  let reachHp = null;
  let launchTick = null;
  let releaseTick = null;
  let hitTick = null;
  let attackPoint = 0;
  let lastPoint = 0;
  let samples = 0;
  let aborted = null;
  const trace = [];

  const legStart = Date.now();
  while (Date.now() - legStart < config.legMs) {
    const h = heroUnit();
    if (!h) {
      aborted = "hero dead";
      break;
    }
    if (window.__ciHp(heroId) < config.abortHp) {
      aborted = "hero below abort hp";
      break;
    }
    creep = roster().find((u) => u.id === creepId);
    if (reachTick === null && (!creep || window.__ciHp(creepId) <= 0)) {
      creep = nearestCreep(h, heroId);
      if (creep === null) {
        aborted = "no hostile creep left to strike";
        break;
      }
      creepId = creep.id;
    }
    if (creep === undefined || creep === null) {
      aborted = "target left the roster";
      break;
    }
    if (approaching && distance(h, creep) <= config.reachCells) {
      window.__ciMove(heroId, h.x, h.z);
      approaching = false;
    } else if (config.walk || approaching) {
      if (Date.now() - lastOrderAt > config.orderEveryMs) {
        const stations = { a: { x: creep.x, z: creep.z }, b: pointToward(creep, origin, config.oscillateCells) };
        if (toward === null || distance(h, stations[toward]) < config.arriveCells) toward = toward === "a" ? "b" : "a";
        window.__ciMove(heroId, stations[toward].x, stations[toward].z);
        lastOrderAt = Date.now();
      }
    }
    await sleep(config.sampleMs);
    const now = heroUnit();
    if (!now) {
      aborted = "hero dead";
      break;
    }
    samples += 1;
    const tick = window.__ciTicks();
    const point = window.__ciSwingPoint(heroId);
    const target = roster().find((u) => u.id === creepId);
    const hp = window.__ciHp(creepId);
    if (trace.length === 0 || trace[trace.length - 1].tick !== tick) {
      trace.push({ tick, d: target ? +distance(now, target).toFixed(2) : null, point, hp });
      if (trace.length > 60) trace.shift();
    }
    if (reachTick === null && target && hp > 0 && distance(now, target) <= config.reachCells) {
      reachTick = tick;
      reachHp = hp;
    }
    if (launchTick === null && point > 0) launchTick = tick;
    if (launchTick !== null && releaseTick === null) {
      if (point > attackPoint) attackPoint = point;
      if (point === 0 && lastPoint > 0) releaseTick = tick;
    }
    if (reachTick !== null && hitTick === null && hp < reachHp) hitTick = tick;
    if (reachTick !== null && hitTick !== null) break;
    lastPoint = point;
  }

  return {
    ready: true,
    startHp,
    targetId: creepId,
    reachTick,
    launchTick,
    releaseTick,
    hitTick,
    attackPoint,
    launchDelayTicks: reachTick !== null && launchTick !== null ? launchTick - reachTick : null,
    latencyTicks: reachTick !== null && hitTick !== null ? hitTick - reachTick : null,
    samples,
    aborted,
    trace,
  };
}

export function verdictFirstHit(legs) {
  const reasons = [];
  for (const leg of legs) {
    if (!leg.judged) continue;
    if (!leg.ready) reasons.push(`${leg.label}: the hero did not reach the start hp of ${CONFIG.minStartHp}`);
    if (leg.lock !== "true") reasons.push(`${leg.label}: the lock was not engaged`);
    if (leg.aborted) reasons.push(`${leg.label}: the leg ended early (${leg.aborted})`);
    if (leg.reachTick == null) reasons.push(`${leg.label}: the hero never came into reach`);
    if (leg.hitTick == null) {
      reasons.push(`${leg.label}: no first hit`);
      continue;
    }
    if (!(leg.attackPoint > 0)) reasons.push(`${leg.label}: no swing attack point was observed`);
    else if (leg.latencyTicks > leg.attackPoint + CONFIG.toleranceTicks) {
      reasons.push(`${leg.label}: first hit ${leg.latencyTicks} ticks after reach, later than the attack point ${leg.attackPoint} plus ${CONFIG.toleranceTicks} tick`);
    }
  }
  return { ok: reasons.length === 0, reasons };
}

export async function runFirstHitTiming(page, config = CONFIG) {
  const lockButton = page.locator("#hero-lock");
  const legs = [];
  for (const leg of LEGS) {
    if ((await lockButton.getAttribute("aria-pressed")) !== String(leg.lock)) await lockButton.click();
    await page.waitForTimeout(600);
    const lock = await lockButton.getAttribute("aria-pressed");
    const legMs = leg.judged ? config.legMs : config.controlMs;
    const measured = await page.evaluate(firstHitLeg, { ...config, walk: leg.walk, legMs });
    legs.push({ label: leg.label, lock, judged: leg.judged, ...measured });
  }
  return { legs, verdict: verdictFirstHit(legs) };
}

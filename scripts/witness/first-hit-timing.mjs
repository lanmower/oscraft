export const LEGS = [
  { label: "stopped, lock on", lock: true, walk: false, judged: true },
  { label: "walking, lock on", lock: true, walk: true, judged: true },
  { label: "walking, lock off (control)", lock: false, walk: true, judged: false },
];

export const CONFIG = {
  legMs: 25000,
  sampleMs: 5,
  orderEveryMs: 300,
  resetMs: 600000,
  stage: { x: 78.5, z: 82 },
  stageArriveCells: 3,
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

  const resetStart = Date.now();
  while (!heroUnit() && Date.now() - resetStart < config.resetMs) await sleep(500);
  const first = heroUnit();
  if (!first) return empty("no hero to start the leg");
  const heroId = first.id;

  let ready = false;
  while (Date.now() - resetStart < config.resetMs) {
    const h = heroUnit();
    if (h && distance(h, config.stage) > config.stageArriveCells) window.__ciMove(heroId, config.stage.x, config.stage.z);
    if (h && distance(h, config.stage) <= config.stageArriveCells && window.__ciHp(heroId) >= config.minStartHp) {
      ready = true;
      break;
    }
    await sleep(1000);
  }
  if (!ready) return { ...empty("hero did not reach the stage at start hp"), startHp: window.__ciHp(heroId) };

  const here = heroUnit();
  let target = null;
  let nearest = Infinity;
  for (const u of roster()) {
    if (u.id === heroId || u.player !== config.targetOwner || window.__ciHp(u.id) <= 0) continue;
    if (!window.__ciHostile(me, u.player)) continue;
    const d = distance(u, here);
    if (d < nearest) {
      nearest = d;
      target = u;
    }
  }
  if (!target) return { ...empty("no hostile creep to strike"), ready, startHp: window.__ciHp(heroId) };
  const targetId = target.id;

  if (!config.walk) window.__ciMove(heroId, here.x, here.z);
  const startHp = window.__ciHp(heroId);
  let reachTick = null;
  let reachHp = null;
  let launchTick = null;
  let releaseTick = null;
  let hitTick = null;
  let attackPoint = 0;
  let lastPoint = 0;
  let samples = 0;
  let toward = null;
  let lastOrderAt = 0;
  let aborted = null;

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
    if (config.walk && Date.now() - lastOrderAt > config.orderEveryMs) {
      const creep = roster().find((u) => u.id === targetId);
      if (creep) {
        const stations = { a: { x: creep.x, z: creep.z }, b: pointToward(creep, config.stage, config.oscillateCells) };
        if (toward === null || distance(h, stations[toward]) < config.arriveCells) toward = toward === "a" ? "b" : "a";
        window.__ciMove(heroId, stations[toward].x, stations[toward].z);
      }
      lastOrderAt = Date.now();
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
    const creep = roster().find((u) => u.id === targetId);
    const hp = window.__ciHp(targetId);
    if (reachTick === null) {
      if (creep && hp > 0 && distance(now, creep) <= config.reachCells) {
        reachTick = tick;
        reachHp = hp;
      }
    } else {
      if (launchTick === null && point > 0) launchTick = tick;
      if (launchTick !== null && releaseTick === null) {
        if (point > attackPoint) attackPoint = point;
        if (point === 0 && lastPoint > 0) releaseTick = tick;
      }
      if (hitTick === null && hp < reachHp) hitTick = tick;
      if (hitTick !== null) break;
    }
    lastPoint = point;
  }

  return {
    ready,
    startHp,
    targetId,
    reachTick,
    launchTick,
    releaseTick,
    hitTick,
    attackPoint,
    launchDelayTicks: reachTick !== null && launchTick !== null ? launchTick - reachTick : null,
    latencyTicks: reachTick !== null && hitTick !== null ? hitTick - reachTick : null,
    samples,
    aborted,
  };
}

export function verdictFirstHit(legs) {
  const reasons = [];
  for (const leg of legs) {
    if (!leg.judged) continue;
    if (!leg.ready) reasons.push(`${leg.label}: the hero did not reach the stage at ${CONFIG.minStartHp} hp`);
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
    const measured = await page.evaluate(firstHitLeg, { ...config, walk: leg.walk });
    legs.push({ label: leg.label, lock, judged: leg.judged, ...measured });
  }
  return { legs, verdict: verdictFirstHit(legs) };
}

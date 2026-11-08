export const LEGS = [
  { label: "lock on", lock: true },
  { label: "lock off", lock: false },
  { label: "lock on again", lock: true },
];

export const CONFIG = {
  legMs: 20000,
  sampleMs: 100,
  orderEveryMs: 300,
  resetMs: 120000,
  stage: { x: 78.5, z: 82 },
  stageArriveCells: 3,
  minStartHp: 200,
  abortHp: 100,
  targetOwner: 255,
  reachCells: 10,
};

export async function strikeWhileWalkingLeg(config) {
  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const me = window.__ciLocalPlayer();
  const roster = () => window.__ciRoster();
  const heroUnit = () => roster().find((u) => u.player === me && window.__ciIsHero(u.id));
  const unitById = (id) => roster().find((u) => u.id === id);
  const distance = (a, b) => Math.hypot(a.x - b.x, a.z - b.z);

  const emptyLeg = (aborted) => ({
    ready: false,
    startHp: null,
    samples: 0,
    movingSamples: 0,
    strikeSamplesWhileMoving: 0,
    strikeSamplesWhileStopped: 0,
    heroEffectsWhileMoving: 0,
    inReachMoving: 0,
    targetsSeen: [],
    kinds: {},
    aborted,
    heroHp: null,
  });

  const resetStart = Date.now();
  while (!heroUnit() && Date.now() - resetStart < config.resetMs) await sleep(500);
  const first = heroUnit();
  if (!first) return emptyLeg("no hero to start the leg");
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
  if (!ready) return { ...emptyLeg("hero did not reach the stage at start hp"), heroHp: window.__ciHp(heroId) };

  const startHp = window.__ciHp(heroId);
  const seenEffects = new Set(window.__ciRecentFx().map((effect) => JSON.stringify(effect)));
  const targetsSeen = new Set();
  const kinds = {};
  let samples = 0;
  let movingSamples = 0;
  let strikeSamplesWhileMoving = 0;
  let strikeSamplesWhileStopped = 0;
  let heroEffectsWhileMoving = 0;
  let inReachMoving = 0;
  let previous = null;
  let previousTarget = -1;
  let previousTargetHp = null;
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
    let nearest = null;
    let nearestDistance = Infinity;
    for (const u of roster()) {
      if (u.id === heroId || u.player === me || u.player !== config.targetOwner || window.__ciHp(u.id) <= 0) continue;
      if (!window.__ciHostile(me, u.player)) continue;
      const d = distance(u, h);
      if (d < nearestDistance) {
        nearestDistance = d;
        nearest = u;
      }
    }
    if (nearest && Date.now() - lastOrderAt > config.orderEveryMs) {
      window.__ciMove(heroId, nearest.x, nearest.z);
      lastOrderAt = Date.now();
    }
    await sleep(config.sampleMs);
    const now = heroUnit();
    if (!now) {
      aborted = "hero dead";
      break;
    }
    samples += 1;
    const moved = previous !== null && Math.hypot(now.x - previous.x, now.z - previous.z) > 0.05;
    if (moved) movingSamples += 1;

    const strikeTarget = window.__ciStrikeOf(heroId);
    if (strikeTarget >= 0) targetsSeen.add(strikeTarget);
    const targetUnit = strikeTarget >= 0 ? unitById(strikeTarget) : undefined;
    if (moved && targetUnit && distance(targetUnit, now) < config.reachCells) inReachMoving += 1;
    const targetHp = strikeTarget >= 0 ? window.__ciHp(strikeTarget) : null;
    const targetLostHp = strikeTarget >= 0 && strikeTarget === previousTarget && previousTargetHp !== null && targetHp < previousTargetHp;

    let heroEffects = 0;
    for (const effect of window.__ciRecentFx()) {
      const key = JSON.stringify(effect);
      if (seenEffects.has(key)) continue;
      seenEffects.add(key);
      if (effect.source === heroId) {
        heroEffects += 1;
        kinds[effect.kind] = (kinds[effect.kind] || 0) + 1;
      }
    }

    if (moved) heroEffectsWhileMoving += heroEffects;
    if (heroEffects > 0 || targetLostHp) {
      if (moved) strikeSamplesWhileMoving += 1;
      else strikeSamplesWhileStopped += 1;
    }
    previous = { x: now.x, z: now.z };
    previousTarget = strikeTarget;
    previousTargetHp = targetHp;
  }

  return {
    ready,
    startHp,
    samples,
    movingSamples,
    strikeSamplesWhileMoving,
    strikeSamplesWhileStopped,
    heroEffectsWhileMoving,
    inReachMoving,
    targetsSeen: [...targetsSeen],
    kinds,
    aborted,
    heroHp: heroUnit() ? window.__ciHp(heroId) : null,
  };
}

export function verdict(legs) {
  const reasons = [];
  for (const leg of legs) {
    if (!leg.ready) reasons.push(`${leg.label}: the hero did not reach the stage at ${CONFIG.minStartHp} hp`);
    if (leg.aborted) reasons.push(`${leg.label}: the leg ended early (${leg.aborted})`);
    if (leg.inReachMoving === 0) reasons.push(`${leg.label}: the hero was never in reach while moving`);
    if (leg.lock === "true" && leg.strikeSamplesWhileMoving === 0) reasons.push(`${leg.label}: no strike while moving with one-hand on`);
    if (leg.lock === "false" && leg.strikeSamplesWhileMoving !== 0) reasons.push(`${leg.label}: ${leg.strikeSamplesWhileMoving} strike samples while moving with one-hand off`);
  }
  return { ok: reasons.length === 0, reasons };
}

export async function runWitness(page, config = CONFIG) {
  const lockButton = page.locator("#hero-lock");
  const lockState = () => lockButton.getAttribute("aria-pressed");
  const legs = [];
  for (const leg of LEGS) {
    if ((await lockState()) !== String(leg.lock)) await lockButton.click();
    await page.waitForTimeout(600);
    const lock = await lockState();
    const measured = await page.evaluate(strikeWhileWalkingLeg, config);
    legs.push({ label: leg.label, lock, ...measured });
  }
  return { legs, verdict: verdict(legs) };
}

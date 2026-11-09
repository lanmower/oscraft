import { fileURLToPath } from "node:url";

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
  minStartHp: 200,
  abortHp: 100,
  targetOwner: 255,
  reachCells: 10,
  attackCells: 2.5,
  arriveCells: 0.8,
  oscillateCells: 1.6,
  stopCells: 2,
  isolationCells: 4,
  laneCells: 10,
  placeTarget: true,
  placeRadii: [8, 12, 16],
  placeStepDeg: 15,
  probeMs: 1500,
  probeCells: 1.5,
  probeLimit: 8,
};

export async function strikeWhileWalkingLeg(config) {
  const ATTACK_KINDS = new Set(["melee_hit", "attack_arrow"]);
  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const me = window.__ciLocalPlayer();
  const roster = () => window.__ciRoster();
  const heroUnit = () => roster().find((u) => u.player === me && window.__ciIsHero(u.id));
  const unitById = (id) => roster().find((u) => u.id === id);
  const rule = () => ({
    isolationCells: config.isolationCells,
    reachCells: config.stopCells,
    laneCells: config.laneCells,
    targetOwner: config.targetOwner,
    excluded: [],
    placeTarget: config.placeTarget,
    placeRadii: config.placeRadii,
    placeStepDeg: config.placeStepDeg,
    probeLimit: config.probeLimit,
    alive: (u) => window.__ciHp(u.id) > 0,
    hostile: (u) => window.__ciHostile(me, u.player),
  });
  const distance = (a, b) => Math.hypot(a.x - b.x, a.z - b.z);
  const probes = [];
  const reachable = async (point) => {
    const before = heroUnit();
    if (!before) return false;
    window.__ciMove(heroId, point.x, point.z);
    await sleep(config.probeMs);
    const after = heroUnit();
    const gained = after ? distance(before, point) - distance(after, point) : 0;
    probes.push({ to: [+point.x.toFixed(1), +point.z.toFixed(1)], gained: +gained.toFixed(2) });
    return gained >= config.probeCells;
  };
  const choosePicked = (h) =>
    window.clearTarget.choose(rule(), () => roster(), h, heroId, (kind, x, z) => window.__ciSpawnNeutral(kind, x, z), reachable);
  function overlapLength(previous, now) {
    for (let k = Math.min(previous.length, now.length); k > 0; k -= 1) {
      let same = true;
      for (let j = 0; j < k && same; j += 1) same = previous[previous.length - k + j] === now[j];
      if (same) return k;
    }
    return 0;
  }
  function pointToward(target, from, cells) {
    const dx = from.x - target.x;
    const dz = from.z - target.z;
    const length = Math.hypot(dx, dz) || 1;
    return { x: target.x + (dx / length) * cells, z: target.z + (dz / length) * cells };
  }

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
  const stage = { x: first.x, z: first.z };

  let ready = false;
  while (Date.now() - resetStart < config.resetMs) {
    if (heroUnit() && window.__ciHp(heroId) >= config.minStartHp) {
      ready = true;
      break;
    }
    await sleep(1000);
  }
  if (!ready) return { ...emptyLeg("hero below start hp"), heroHp: window.__ciHp(heroId) };

  const startHp = window.__ciHp(heroId);
  let previousFx = window.__ciRecentFx().map((effect) => JSON.stringify(effect));
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
  let creep = null;
  let toward = null;
  let aborted = null;

  let initialPick = null;
  const initial = await choosePicked(heroUnit() ?? first);
  if (initial.creep === null) aborted = initial.aborted;
  else {
    creep = initial.creep;
    window.__ciAttack(heroId, creep.id);
    await sleep(400);
    initialPick = {
      id: creep.id,
      type: creep.type,
      x: +creep.x.toFixed(1),
      z: +creep.z.toFixed(1),
      strikeAfterAttack: window.__ciStrikeOf(heroId),
      refusal: document.body.dataset.lastRefusal ?? null,
    };
  }
  previousFx = window.__ciRecentFx().map((effect) => JSON.stringify(effect));

  const legStart = Date.now();
  while (aborted === null && Date.now() - legStart < config.legMs) {
    const h = heroUnit();
    if (!h) {
      aborted = "hero dead";
      break;
    }
    if (window.__ciHp(heroId) < config.abortHp) {
      aborted = "hero below abort hp";
      break;
    }
    if (!creep || window.__ciHp(creep.id) <= 0) {
      const picked = await choosePicked(h);
      if (picked.creep === null) {
        aborted = picked.aborted;
        break;
      }
      creep = picked.creep;
      window.__ciAttack(heroId, creep.id);
    }
    if (creep && Date.now() - lastOrderAt > config.orderEveryMs) {
      if (window.__ciStrikeOf(heroId) !== creep.id) window.__ciAttack(heroId, creep.id);
      const stations = {
        a: pointToward(creep, stage, config.stopCells),
        b: pointToward(creep, stage, config.stopCells + config.oscillateCells),
      };
      if (toward === null || distance(h, stations[toward]) < config.arriveCells) toward = toward === "a" ? "b" : "a";
      window.__ciMove(heroId, stations[toward].x, stations[toward].z);
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
    let targetHits = 0;
    const nowFx = window.__ciRecentFx();
    const nowKeys = nowFx.map((effect) => JSON.stringify(effect));
    const fresh = nowFx.slice(overlapLength(previousFx, nowKeys));
    previousFx = nowKeys;
    for (const effect of fresh) {
      if (ATTACK_KINDS.has(effect.kind) && effect.target >= 0 && (effect.target === strikeTarget || effect.target === previousTarget)) {
        const struck = unitById(effect.target);
        if (struck && distance(struck, now) < config.attackCells) targetHits += 1;
      }
      if (effect.source === heroId) {
        heroEffects += 1;
        kinds[effect.kind] = (kinds[effect.kind] || 0) + 1;
      }
    }

    if (moved) heroEffectsWhileMoving += heroEffects;
    if (heroEffects > 0 || targetLostHp || targetHits > 0) {
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
    probes,
    initialPick,
  };
}

export function verdict(legs) {
  const reasons = [];
  for (const leg of legs) {
    if (!leg.ready) reasons.push(`${leg.label}: the hero did not reach the stage at ${CONFIG.minStartHp} hp`);
    if (leg.aborted) reasons.push(`${leg.label}: the leg ended early (${leg.aborted})`);
    if (leg.inReachMoving === 0) reasons.push(`${leg.label}: the hero was never in reach while moving`);
    if (leg.lock === "true" && leg.strikeSamplesWhileMoving === 0) reasons.push(`${leg.label}: no strike while moving with one-hand on`);
    if (leg.lock === "true" && Object.keys(leg.kinds ?? {}).length === 0) reasons.push(`${leg.label}: no hero-sourced effect kinds recorded`);
    if (leg.lock === "false" && leg.strikeSamplesWhileMoving !== 0) reasons.push(`${leg.label}: ${leg.strikeSamplesWhileMoving} strike samples while moving with one-hand off`);
  }
  return { ok: reasons.length === 0, reasons };
}

export async function runWitness(page, config = CONFIG) {
  const lockButton = page.locator("#hero-lock");
  await page.addScriptTag({ path: fileURLToPath(new URL("./clear-target.js", import.meta.url)) });
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

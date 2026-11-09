import { fileURLToPath } from "node:url";

const SIM_ORDER_MOVE = 0;

export const LEGS = [
  { label: "lock on", lock: true },
  { label: "lock off", lock: false },
  { label: "lock on again", lock: true },
];

export const CONFIG = {
  legMs: 25000,
  sampleMs: 100,
  stallRunSamples: 8,
  stallTraceCap: 400,
  stationTurns: 8,
  orderEveryMs: 300,
  resetMs: 120000,
  minStartHp: 200,
  abortHp: 100,
  targetOwner: 255,
  reachCells: 10,
  arriveCells: 0.8,
  oscillateCells: 1.6,
  stopCells: 2,
  isolationCells: 4,
  laneCells: 4,
  placeTarget: true,
  placeRadii: [8, 12, 16, 20, 24, 28, 32, 36, 40],
  placeRetries: 10,
  placeRetryMs: 400,
  placeStepDeg: 15,
  probeMs: 1500,
  probeCells: 1.5,
  probeLimit: 16,
  pickRangeCells: 100,
  attemptLimit: 40,
  walkingCells: 1.5,
  orderSettleMs: 250,
};

export async function strikeWhileWalkingLeg(config) {
  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const me = window.__ciLocalPlayer();
  const roster = () => window.__ciRoster();
  const heroUnit = () => roster().find((u) => u.player === me && window.__ciIsHero(u.id));
  const unitById = (id) => roster().find((u) => u.id === id);
  const rule = () => ({
    pickRangeCells: config.pickRangeCells,
    isolationCells: config.isolationCells,
    reachCells: config.stopCells,
    laneCells: config.laneCells,
    targetOwner: config.targetOwner,
    excluded: [],
    placeTarget: config.placeTarget,
    placeRadii: config.placeRadii,
    placeStepDeg: config.placeStepDeg,
    probeLimit: config.probeLimit,
    placeRetries: config.placeRetries,
    placeRetryMs: config.placeRetryMs,
    alive: (u) => window.__ciHp(u.id) > 0,
    hostile: (u) => window.__ciHostile(me, u.player),
  });
  const distance = (a, b) => Math.hypot(a.x - b.x, a.z - b.z);
  const probes = [];
  const attempts = [];
  let attemptsDropped = 0;
  const legT0 = Date.now();
  const reachable = async (point) => {
    const before = heroUnit();
    if (!before) return false;
    if (distance(before, point) <= config.probeCells) {
      probes.push({ to: [+point.x.toFixed(1), +point.z.toFixed(1)], gained: 0, alreadyThere: true });
      return true;
    }
    window.__ciMove(heroId, point.x, point.z);
    await sleep(config.probeMs);
    const after = heroUnit();
    const gained = after ? distance(before, point) - distance(after, point) : 0;
    probes.push({ to: [+point.x.toFixed(1), +point.z.toFixed(1)], gained: +gained.toFixed(2) });
    return gained >= config.probeCells;
  };
  const choosePicked = (h) =>
    window.clearTarget.choose(rule(), () => roster(), h, heroId, (kind, x, z) => window.__ciSpawnNeutral(kind, x, z), reachable);
  const commandAttack = (targetId, why) => {
    const terms = window.__ciAttackTerms(heroId, targetId);
    const accepted = window.__ciAttack(heroId, targetId);
    const h = heroUnit();
    const attempt = {
      atMs: Date.now() - legT0,
      why,
      targetId,
      accepted,
      visible: terms[3] === 1,
      inReach: terms[6] === 1,
      cooldownReady: terms[7] === 1,
      simRefusal: accepted ? null : window.__ciRefusal(),
      uiRefusal: accepted ? null : document.body.dataset.lastRefusal ?? null,
      hero: h ? [+h.x.toFixed(1), +h.z.toFixed(1)] : null,
    };
    if (attempts.length < config.attemptLimit) attempts.push(attempt);
    else attemptsDropped += 1;
    return accepted;
  };
  function overlapLength(previous, now) {
    for (let k = Math.min(previous.length, now.length); k > 0; k -= 1) {
      let same = true;
      for (let j = 0; j < k && same; j += 1) same = previous[previous.length - k + j] === now[j];
      if (same) return k;
    }
    return 0;
  }
  function pointToward(target, from, cells, turn) {
    const angle = (turn * Math.PI) / 4;
    const ax = from.x - target.x;
    const az = from.z - target.z;
    const dx = ax * Math.cos(angle) - az * Math.sin(angle);
    const dz = ax * Math.sin(angle) + az * Math.cos(angle);
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
  let stationTurn = 0;
  let stationRotations = 0;
  let refusedMoves = 0;
  let stillSamples = 0;
  let stillSince = 0;
  const stallRuns = [];
  const stallTrace = [];
  let lastOrderAt = 0;
  let creep = null;
  let toward = null;
  let aborted = null;

  let initialPick = null;
  const movingStrikes = [];
  const movingLaunches = [];
  let launchesWhileMoving = 0;
  let lastLaunch = window.__ciStrikeStamp(heroId)?.[2] ?? -1;
  let walkTo = null;
  const carried = window.__ciWitnessTarget ?? null;
  const carriedUnit = carried === null ? undefined : unitById(carried);
  const carriedInRange = carriedUnit !== undefined && window.__ciHp(carried) > 0 && distance(heroUnit() ?? first, carriedUnit) <= config.pickRangeCells;
  const carriedState = carried === null ? null : { id: carried, known: carriedUnit !== undefined, hp: window.__ciHp(carried), distance: carriedUnit ? +distance(heroUnit() ?? first, carriedUnit).toFixed(1) : null };
  const initial = carriedInRange ? { creep: carriedUnit, aborted: null } : await choosePicked(heroUnit() ?? first);
  if (initial.creep === null) aborted = initial.aborted;
  else {
    creep = initial.creep;
    window.__ciWitnessTarget = creep.id;
    commandAttack(creep.id, carriedInRange ? "carried target" : "initial pick");
    await sleep(400);
    initialPick = {
      id: creep.id,
      type: creep.type,
      x: +creep.x.toFixed(1),
      z: +creep.z.toFixed(1),
      strikeAfterAttack: window.__ciStrikeOf(heroId),
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
      commandAttack(creep.id, "re-pick");
    }
    if (creep && Date.now() - lastOrderAt > config.orderEveryMs) {
      if (window.__ciStrikeOf(heroId) !== creep.id) commandAttack(creep.id, "in-loop re-attack");
      const stations = {
        a: pointToward(creep, stage, config.stopCells, stationTurn),
        b: pointToward(creep, stage, config.stopCells + config.oscillateCells, stationTurn),
      };
      if (toward === null || distance(h, stations[toward]) < config.arriveCells) toward = toward === "a" ? "b" : "a";
      const accepted = window.__ciMove(heroId, stations[toward].x, stations[toward].z);
      if (!accepted) {
        stationTurn = (stationTurn + 1) % config.stationTurns;
        stationRotations += 1;
        refusedMoves += 1;
      }
      walkTo = stations[toward];
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
    const walkingOrder = (window.__ciOrderOf(heroId)?.[0] ?? -1) === SIM_ORDER_MOVE;
    const walking = moved && walkingOrder;
    if (moved) movingSamples += 1;
    if (moved) {
      if (stillSamples >= config.stallRunSamples) stallRuns.push({ startMs: stillSince, samples: stillSamples });
      stillSamples = 0;
    } else {
      if (stillSamples === 0) stillSince = Date.now() - legStart;
      stillSamples += 1;
      if (stillSamples >= config.stallRunSamples && stallTrace.length < config.stallTraceCap) {
        stallTrace.push({
          atMs: Date.now() - legStart,
          run: stillSamples,
          pos: [+now.x.toFixed(2), +now.z.toFixed(2)],
          walkTo: walkTo ? [+walkTo.x.toFixed(2), +walkTo.z.toFixed(2)] : null,
          order: window.__ciOrderOf(heroId),
          swingPoint: window.__ciSwingPoint(heroId),
          strikeTarget: window.__ciStrikeOf(heroId),
          tick: window.__ciTicks(),
          hp: window.__ciHp(heroId),
          refusal: window.__ciRefusal(),
          stationTurn,
        });
      }
    }
    const farFromStation = walkTo !== null && distance(now, walkTo) > config.walkingCells;
    const launch = window.__ciStrikeStamp(heroId)?.[2] ?? -1;
    if (launch !== lastLaunch) {
      const settled = Date.now() - lastOrderAt >= config.orderSettleMs;
      if (launch !== -1 && walking && farFromStation && settled) {
        launchesWhileMoving += 1;
        movingLaunches.push({
          atMs: Date.now() - legStart,
          launch,
          pos: [+now.x.toFixed(2), +now.z.toFixed(2)],
          walkTo: walkTo ? [+walkTo.x.toFixed(2), +walkTo.z.toFixed(2)] : null,
          order: window.__ciOrderOf(heroId),
          strikeTarget: window.__ciStrikeOf(heroId),
          tick: window.__ciTicks(),
        });
      }
      lastLaunch = launch;
    }

    const strikeTarget = window.__ciStrikeOf(heroId);
    if (strikeTarget >= 0) targetsSeen.add(strikeTarget);
    const targetUnit = strikeTarget >= 0 ? unitById(strikeTarget) : undefined;
    if (moved && targetUnit && distance(targetUnit, now) < config.reachCells) inReachMoving += 1;
    let heroEffects = 0;
    const nowFx = window.__ciRecentFx();
    const nowKeys = nowFx.map((effect) => JSON.stringify(effect));
    const fresh = nowFx.slice(overlapLength(previousFx, nowKeys));
    previousFx = nowKeys;
    for (const effect of fresh) {
      if (effect.source === heroId) {
        heroEffects += 1;
        kinds[effect.kind] = (kinds[effect.kind] || 0) + 1;
      }
    }

    if (walking) heroEffectsWhileMoving += heroEffects;
    if (heroEffects > 0) {
      if (walking) {
        strikeSamplesWhileMoving += 1;
        movingStrikes.push({ atMs: Date.now() - legStart, effects: heroEffects });
      } else strikeSamplesWhileStopped += 1;
    }
    previous = { x: now.x, z: now.z };
  }
  if (stillSamples >= config.stallRunSamples) stallRuns.push({ startMs: stillSince, samples: stillSamples });

  return {
    stallRuns,
    stallTrace,
    stationRotations,
    refusedMoves,
    ready,
    startHp,
    samples,
    movingSamples,
    strikeSamplesWhileMoving,
    strikeSamplesWhileStopped,
    heroEffectsWhileMoving,
    launchesWhileMoving,
    inReachMoving,
    targetsSeen: [...targetsSeen],
    kinds,
    aborted,
    heroHp: heroUnit() ? window.__ciHp(heroId) : null,
    probes,
    initialPick,
    carriedState,
    movingStrikes,
    movingLaunches,
    attempts,
    attemptsDropped,
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
    if (leg.lock === "false" && leg.launchesWhileMoving !== 0) reasons.push(`${leg.label}: ${leg.launchesWhileMoving} strikes launched while moving with one-hand off`);
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

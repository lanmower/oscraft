import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

export const LEGS = [
  { label: "lock on", lock: true },
  { label: "lock off", lock: false },
  { label: "lock on again", lock: true },
];

export const CONFIG = {
  legTicks: 640,
  sampleTicks: 2,
  sampleFloor: 288,
  legReruns: 2,
  pollMs: 25,
  stallPolls: 200,
  slowPollMs: 500,
  resetPolls: 240,
  stallRunSamples: 8,
  stallTraceCap: 400,
  strikeTraceCap: 300,
  stationTurns: 8,
  orderEveryTicks: 5,
  orderSettleTicks: 4,
  initialSettleTicks: 6,
  replaceFraction: 0.25,
  minStartHp: 200,
  abortHp: 100,
  targetOwner: 255,
  reachCells: 10,
  arriveCells: 0.8,
  stopCells: 1,
  isolationCells: 4,
  laneCells: 4,
  placeTarget: true,
  placeRadii: [8, 12, 16, 20, 24, 28, 32, 36, 40],
  placeRetries: 10,
  placeRetryMs: 400,
  placeStepDeg: 15,
  probeTicks: 24,
  probeCells: 1.5,
  probeLimit: 16,
  pickRangeCells: 100,
  attemptLimit: 40,
  walkingCells: 1.5,
  arrivedCells: 1,
  traceDir: fileURLToPath(new URL("../../.gm/witness-traces/", import.meta.url)),
};

export async function strikeWhileWalkingLeg(config) {
  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const tickNow = () => window.__ciTicks();
  const me = window.__ciLocalPlayer();
  const roster = () => window.__ciRoster();
  const heroUnit = () => roster().find((u) => u.player === me && window.__ciIsHero(u.id));
  const unitById = (id) => roster().find((u) => u.id === id);
  const waitTicks = async (ticks) => {
    const target = tickNow() + ticks;
    let last = tickNow();
    let idle = 0;
    while (tickNow() < target) {
      await sleep(config.pollMs);
      const now = tickNow();
      if (now === last) {
        idle += 1;
        if (idle >= config.stallPolls) return false;
      } else {
        idle = 0;
        last = now;
      }
    }
    return true;
  };
  const rule = (avoid = []) => ({
    pickRangeCells: config.pickRangeCells,
    isolationCells: config.isolationCells,
    reachCells: config.stopCells,
    laneCells: config.laneCells,
    targetOwner: config.targetOwner,
    excluded: [...stationBlocked, ...avoid],
    placeTarget: config.placeTarget,
    placeRadii: config.placeRadii,
    placeStepDeg: config.placeStepDeg,
    probeLimit: config.probeLimit,
    placeRetries: config.placeRetries,
    placeRetryMs: config.placeRetryMs,
    alive: (u) => window.__ciHp(u.id) > 0,
    hp: (u) => window.__ciHp(u.id),
    hostile: (u) => window.__ciHostile(me, u.player),
  });
  const distance = (a, b) => Math.hypot(a.x - b.x, a.z - b.z);
  const probes = [];
  const attempts = [];
  let attemptsDropped = 0;
  const startTick = tickNow();
  const used = (window.__ciWitnessUsed ??= []);
  const reachable = async (point) => {
    const before = heroUnit();
    if (!before) return false;
    if (distance(before, point) <= config.probeCells) {
      probes.push({ to: [+point.x.toFixed(1), +point.z.toFixed(1)], gained: 0, alreadyThere: true });
      return true;
    }
    window.__ciMove(heroId, point.x, point.z);
    await waitTicks(config.probeTicks);
    const after = heroUnit();
    const gained = after ? distance(before, point) - distance(after, point) : 0;
    probes.push({ to: [+point.x.toFixed(1), +point.z.toFixed(1)], gained: +gained.toFixed(2) });
    return gained >= config.probeCells;
  };
  const choosePicked = (h, avoid = []) =>
    window.clearTarget.choose(rule(avoid), () => roster(), h, heroId, (kind, x, z) => window.__ciSpawnNeutral(kind, x, z), reachable);
  const commandAttack = (targetId, why) => {
    const terms = window.__ciAttackTerms(heroId, targetId);
    const accepted = window.__ciAttack(heroId, targetId);
    const h = heroUnit();
    const attempt = {
      atTick: tickNow() - startTick,
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

  for (let polls = 0; !heroUnit() && polls < config.resetPolls; polls += 1) await sleep(config.slowPollMs);
  const first = heroUnit();
  if (!first) return emptyLeg("no hero to start the leg");
  const heroId = first.id;
  const stage = { x: first.x, z: first.z };

  let ready = false;
  for (let polls = 0; polls < config.resetPolls; polls += 1) {
    if (heroUnit() && window.__ciHp(heroId) >= config.minStartHp) {
      ready = true;
      break;
    }
    await sleep(config.slowPollMs);
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
  const moveRefusals = [];
  let moveRefusalsDropped = 0;
  let refusalRun = 0;
  const stationBlocked = [];
  let stillSamples = 0;
  let stillSince = 0;
  const stallRuns = [];
  const stallTrace = [];
  const strikeTrace = [];
  let lastOrderTick = -Infinity;
  let creep = null;
  let toward = null;
  let aborted = null;
  let pick = null;
  let pickEntry = null;
  let replaceHp = 0;
  const picks = [];

  let initialPick = null;
  const movingStrikes = [];
  const movingLaunches = [];
  let launchesWhileMoving = 0;
  let lastLaunch = window.__ciStrikeStamp(heroId)?.[2] ?? -1;
  let walkTo = null;
  const lastCreep = used.at(-1);
  let initial = await choosePicked(heroUnit() ?? first, lastCreep === undefined ? [] : [lastCreep]);
  if (initial.creep === null && lastCreep !== undefined) initial = await choosePicked(heroUnit() ?? first);
  if (initial.creep === null) aborted = initial.aborted;
  else {
    creep = initial.creep;
    replaceHp = Math.ceil(window.__ciHp(creep.id) * config.replaceFraction);
    used.push(creep.id);
    commandAttack(creep.id, "initial pick");
    await waitTicks(config.initialSettleTicks);
    initialPick = {
      id: creep.id,
      type: creep.type,
      x: +creep.x.toFixed(1),
      z: +creep.z.toFixed(1),
      hp: window.__ciHp(creep.id),
      replaceHp,
      strikeAfterAttack: window.__ciStrikeOf(heroId),
    };
  }
  previousFx = window.__ciRecentFx().map((effect) => JSON.stringify(effect));

  const legTick0 = tickNow();
  while (aborted === null && tickNow() - legTick0 < config.legTicks) {
    const h = heroUnit();
    if (!h) {
      aborted = "hero dead";
      break;
    }
    if (window.__ciHp(heroId) < config.abortHp) {
      aborted = "hero below abort hp";
      break;
    }
    const creepHp = creep ? window.__ciHp(creep.id) : 0;
    if (pick === null && (!creep || creepHp <= replaceHp)) {
      const started = { done: false, value: null };
      pick = started;
      pickEntry = {
        atTick: tickNow() - legTick0,
        reason: creep === null ? "no creep" : creepHp <= 0 ? "dead" : "below replace hp",
        from: creep ? creep.id : null,
        fromHp: creep ? creepHp : null,
        to: null,
        doneTick: null,
      };
      if (picks.length < 200) picks.push(pickEntry);
      const entry = pickEntry;
      choosePicked(h, creep ? [creep.id] : []).then(
        (value) => {
          started.value = value;
          started.done = true;
          entry.doneTick = tickNow() - legTick0;
        },
        (error) => {
          started.value = { creep: null, aborted: String(error) };
          started.done = true;
          entry.doneTick = tickNow() - legTick0;
        },
      );
    }
    if (pick !== null && pick.done) {
      const picked = pick.value;
      pick = null;
      if (picked.creep === null) {
        if (creep !== null && window.__ciHp(creep.id) > 0) replaceHp = 0;
        else {
          aborted = picked.aborted;
          break;
        }
      } else {
        pickEntry.to = picked.creep.id;
        creep = picked.creep;
        replaceHp = Math.ceil(window.__ciHp(creep.id) * config.replaceFraction);
        used.push(creep.id);
        commandAttack(creep.id, "re-pick");
      }
    }
    if (pick === null && creep && tickNow() - lastOrderTick >= config.orderEveryTicks) {
      if (window.__ciStrikeOf(heroId) !== creep.id) commandAttack(creep.id, "in-loop re-attack");
      const stations = {
        a: pointToward(creep, stage, config.stopCells, stationTurn),
        b: pointToward(creep, stage, config.stopCells, stationTurn + config.stationTurns / 2),
      };
      if (toward === null || distance(h, stations[toward]) < config.arriveCells) toward = toward === "a" ? "b" : "a";
      const accepted = window.__ciMove(heroId, stations[toward].x, stations[toward].z);
      if (!accepted) {
        if (moveRefusals.length < config.attemptLimit) {
          moveRefusals.push({
            atTick: tickNow() - startTick,
            turn: stationTurn,
            station: [+stations[toward].x.toFixed(2), +stations[toward].z.toFixed(2)],
            creep: [+creep.x.toFixed(2), +creep.z.toFixed(2)],
            hero: [+h.x.toFixed(2), +h.z.toFixed(2)],
            refusal: window.__ciRefusal(),
          });
        } else {
          moveRefusalsDropped += 1;
        }
        stationTurn = (stationTurn + 1) % config.stationTurns;
        stationRotations += 1;
        refusedMoves += 1;
        refusalRun += 1;
        if (refusalRun >= config.stationTurns) {
          stationBlocked.push(creep.id);
          creep = null;
          refusalRun = 0;
        }
      } else {
        refusalRun = 0;
      }
      walkTo = stations[toward];
      lastOrderTick = tickNow();
    }
    if (!(await waitTicks(config.sampleTicks))) {
      aborted = "sim stopped ticking";
      break;
    }
    const now = heroUnit();
    if (!now) {
      aborted = "hero dead";
      break;
    }
    samples += 1;
    const moved = previous !== null && Math.hypot(now.x - previous.x, now.z - previous.z) > 0.05;
    const order = window.__ciOrderOf(heroId);
    const walkingOrder = order?.[0] === 0 && distance({ x: order[1] / 65536, z: order[2] / 65536 }, now) > config.arrivedCells;
    const walking = moved && walkingOrder;
    if (moved) movingSamples += 1;
    if (moved) {
      if (stillSamples >= config.stallRunSamples) stallRuns.push({ startTick: stillSince, samples: stillSamples });
      stillSamples = 0;
    } else {
      if (stillSamples === 0) stillSince = tickNow() - legTick0;
      stillSamples += 1;
      if (stillSamples >= config.stallRunSamples && stallTrace.length < config.stallTraceCap) {
        stallTrace.push({
          atTick: tickNow() - legTick0,
          run: stillSamples,
          pos: [+now.x.toFixed(2), +now.z.toFixed(2)],
          walkTo: walkTo ? [+walkTo.x.toFixed(2), +walkTo.z.toFixed(2)] : null,
          order: window.__ciOrderOf(heroId),
          swingPoint: window.__ciSwingPoint(heroId),
          strikeTarget: window.__ciStrikeOf(heroId),
          tick: tickNow(),
          hp: window.__ciHp(heroId),
          refusal: window.__ciRefusal(),
          stationTurn,
        });
      }
    }
    const farFromStation = walkTo !== null && distance(now, walkTo) > config.walkingCells;
    const launch = window.__ciStrikeStamp(heroId)?.[2] ?? -1;
    if (launch !== lastLaunch) {
      const settled = tickNow() - lastOrderTick >= config.orderSettleTicks;
      if (launch !== -1 && walking && farFromStation && settled) {
        launchesWhileMoving += 1;
        movingLaunches.push({
          atTick: tickNow() - legTick0,
          launch,
          pos: [+now.x.toFixed(2), +now.z.toFixed(2)],
          walkTo: walkTo ? [+walkTo.x.toFixed(2), +walkTo.z.toFixed(2)] : null,
          order: window.__ciOrderOf(heroId),
          strikeTarget: window.__ciStrikeOf(heroId),
          tick: tickNow(),
        });
      }
      lastLaunch = launch;
    }

    const strikeTarget = window.__ciStrikeOf(heroId);
    if (strikeTarget >= 0) targetsSeen.add(strikeTarget);
    const targetUnit = strikeTarget >= 0 ? unitById(strikeTarget) : undefined;
    if (moved && targetUnit && distance(targetUnit, now) < config.reachCells) inReachMoving += 1;
    if (creep && strikeTrace.length < config.strikeTraceCap) {
      strikeTrace.push({
        atTick: tickNow() - legTick0,
        moved,
        pos: [+now.x.toFixed(2), +now.z.toFixed(2)],
        creep: [+creep.x.toFixed(2), +creep.z.toFixed(2)],
        dist: +distance(creep, now).toFixed(2),
        creepHp: window.__ciHp(creep.id),
        strikeTarget,
        order: window.__ciOrderOf(heroId),
        swing: window.__ciSwingPoint(heroId),
        launch: window.__ciStrikeStamp(heroId),
        walkTo: walkTo ? [+walkTo.x.toFixed(2), +walkTo.z.toFixed(2)] : null,
        terms: window.__ciAttackTerms(heroId, creep.id),
        swingLeft: window.__ciSwingPoint(heroId),
      });
    }
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
        movingStrikes.push({ atTick: tickNow() - legTick0, effects: heroEffects });
      } else strikeSamplesWhileStopped += 1;
    }
    previous = { x: now.x, z: now.z };
  }
  while (pick !== null && !pick.done) await sleep(config.pollMs);
  if (stillSamples >= config.stallRunSamples) stallRuns.push({ startTick: stillSince, samples: stillSamples });

  return {
    picks,
    stallRuns,
    stallTrace,
    strikeTrace,
    stationRotations,
    refusedMoves,
    moveRefusals,
    moveRefusalsDropped,
    ready,
    startHp,
    ticks: tickNow() - legTick0,
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
    movingStrikes,
    movingLaunches,
    attempts,
    attemptsDropped,
  };
}

export function verdict(legs) {
  const reasons = [];
  for (const leg of legs) {
    const attempts = (leg.reruns ?? []).length + 1;
    if (!leg.ready) reasons.push(`${leg.label}: the hero did not reach the stage at ${CONFIG.minStartHp} hp`);
    if (leg.aborted) reasons.push(`${leg.label}: the leg ended early (${leg.aborted})`);
    if (!(leg.samples >= CONFIG.sampleFloor)) reasons.push(`${leg.label}: ${leg.samples} samples, below the floor of ${CONFIG.sampleFloor} after ${attempts} attempts`);
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
  await page.evaluate(() => {
    window.__ciWitnessUsed = [];
  });
  const lockState = () => lockButton.getAttribute("aria-pressed");
  const legs = [];
  for (const leg of LEGS) {
    if ((await lockState()) !== String(leg.lock)) await lockButton.click();
    await page.waitForTimeout(600);
    const lock = await lockState();
    const reruns = [];
    let measured = await page.evaluate(strikeWhileWalkingLeg, config);
    while (measured.samples < config.sampleFloor && reruns.length < config.legReruns) {
      reruns.push({
        attempt: reruns.length + 1,
        samples: measured.samples,
        ticks: measured.ticks,
        aborted: measured.aborted,
        picks: measured.picks,
      });
      measured = await page.evaluate(strikeWhileWalkingLeg, config);
    }
    legs.push({ label: leg.label, lock, reruns, ...measured });
  }
  const result = { legs, verdict: verdict(legs) };
  result.traceFile = writeTrace(result, config);
  return result;
}

function writeTrace(result, config) {
  mkdirSync(config.traceDir, { recursive: true });
  const stamp = new Date().toISOString().replace(/[:.]/g, "-");
  const file = join(config.traceDir, "d1-" + stamp + "-" + (result.verdict.ok ? "ok" : "red") + ".json");
  writeFileSync(file, JSON.stringify({ config, verdict: result.verdict, legs: result.legs }, null, 1));
  return file;
}

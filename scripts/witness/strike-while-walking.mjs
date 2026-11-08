export const LEGS = [
  { label: "lock on", lock: true },
  { label: "lock off", lock: false },
  { label: "lock on again", lock: true },
];

export const CONFIG = {
  legMs: 20000,
  sampleMs: 100,
  orderEveryMs: 300,
  stage: { x: 78.5, z: 82 },
  minStartHp: 200,
  abortHp: 100,
  recoverMs: 120000,
  targetOwner: 255,
  reachCells: 10,
};

export async function strikeWhileWalkingLeg(config) {
  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const me = window.__ciLocalPlayer();
  const roster = () => window.__ciRoster();
  const hero = () => roster().find((u) => u.player === me && window.__ciIsHero(u.id));
  const unitById = (id) => roster().find((u) => u.id === id);
  const distance = (a, b) => Math.hypot(a.x - b.x, a.z - b.z);
  const heroId = hero().id;

  const foesOf = () => roster().filter((u) => u.id !== heroId
    && u.player !== me
    && window.__ciHostile(me, u.player)
    && (config.targetOwner === null || u.player === config.targetOwner)
    && window.__ciHp(u.id) > 0);

  const startHp = window.__ciHp(heroId);
  let ready = window.__ciHp(heroId) >= config.minStartHp;
  const recoverStart = Date.now();
  while (!ready && Date.now() - recoverStart < config.recoverMs) {
    const h = hero();
    if (h && distance(h, config.stage) > 3) window.__ciMove(heroId, config.stage.x, config.stage.z);
    await sleep(1000);
    ready = hero() !== undefined && window.__ciHp(heroId) >= config.minStartHp;
  }

  const seenFx = new Set();
  const kinds = {};
  const targetsSeen = new Set();
  let samples = 0;
  let movingSamples = 0;
  let strikesWhileMoving = 0;
  let strikesWhileStopped = 0;
  let inReachMoving = 0;
  let previous = null;
  let lastOrderAt = 0;
  let aborted = null;

  if (ready) {
    const legStart = Date.now();
    while (Date.now() - legStart < config.legMs) {
      const h = hero();
      if (!h) {
        aborted = "hero dead";
        break;
      }
      if (window.__ciHp(heroId) < config.abortHp) {
        aborted = "hero below abort hp";
        break;
      }
      const foes = foesOf();
      let nearest = null;
      let nearestDistance = Infinity;
      for (const foe of foes) {
        const d = distance(foe, h);
        if (d < nearestDistance) {
          nearestDistance = d;
          nearest = foe;
        }
      }
      if (nearest && Date.now() - lastOrderAt > config.orderEveryMs) {
        window.__ciMove(heroId, nearest.x, nearest.z);
        lastOrderAt = Date.now();
      }
      await sleep(config.sampleMs);
      const now = hero();
      if (!now) {
        aborted = "hero dead";
        break;
      }
      samples += 1;
      const moved = previous !== null && Math.hypot(now.x - previous.x, now.z - previous.z) > 0.05;
      if (moved) movingSamples += 1;
      const strikeTarget = window.__ciStrikeOf(heroId);
      if (strikeTarget >= 0) targetsSeen.add(strikeTarget);
      const struck = unitById(strikeTarget);
      if (moved && struck && distance(struck, now) < config.reachCells) inReachMoving += 1;
      let fresh = 0;
      for (const effect of window.__ciRecentFx()) {
        const key = JSON.stringify(effect);
        if (seenFx.has(key)) continue;
        seenFx.add(key);
        if (effect.source === heroId) {
          fresh += 1;
          kinds[effect.kind] = (kinds[effect.kind] || 0) + 1;
        }
      }
      if (moved) strikesWhileMoving += fresh;
      else strikesWhileStopped += fresh;
      previous = { x: now.x, z: now.z };
    }
  }

  return {
    ready,
    startHp,
    samples,
    movingSamples,
    strikesWhileMoving,
    strikesWhileStopped,
    inReachMoving,
    targetsSeen: [...targetsSeen],
    kinds,
    aborted,
    heroHp: hero() ? window.__ciHp(heroId) : null,
  };
}

export function verdict(legs) {
  const reasons = [];
  for (const leg of legs) {
    if (!leg.ready) reasons.push(`${leg.label}: the hero did not reach ${CONFIG.minStartHp} hp before the leg`);
    if (leg.aborted) reasons.push(`${leg.label}: the leg ended early (${leg.aborted})`);
    if (leg.lock === "true" && leg.strikesWhileMoving === 0) reasons.push(`${leg.label}: no strike while moving with one-hand on`);
    if (leg.lock === "false" && leg.inReachMoving === 0) reasons.push(`${leg.label}: the hero was never in reach while moving, so the red run proves nothing`);
    if (leg.lock === "false" && leg.strikesWhileMoving !== 0) reasons.push(`${leg.label}: ${leg.strikesWhileMoving} strikes while moving with one-hand off`);
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

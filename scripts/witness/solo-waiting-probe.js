(async () => {
  const OPTS = { ns: "lw-waiting", port: 8853, settleMs: 12000, interval: 500 };
  const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const built = () => document.body.dataset.worldBuilt === "1" && typeof window.oscraft?.lobby === "function";
  const read = () => {
    const fill = [...document.querySelectorAll("button")].find((b) => b.textContent.trim() === "Fill with bots");
    const waiting = [...document.querySelectorAll("body *")].some((e) => e.children.length === 0 && e.textContent.trim() === "waiting for a host to claim a seat");
    return { fillPresent: fill !== undefined, fillEnabled: fill !== undefined && !fill.disabled, waiting };
  };
  const page = await context.newPage();
  await page.goto(`http://127.0.0.1:${OPTS.port}/js/client/play.html?ci=1&ns=${OPTS.ns}`, { waitUntil: "domcontentloaded", timeout: 50000 });
  await page.waitForFunction(built, null, { timeout: 240000 });
  const claimed = await page.evaluate(() => window.oscraft.lobby("claim", { slot: 2 }));
  const samples = [];
  const t0 = Date.now();
  while (Date.now() - t0 < OPTS.settleMs) {
    samples.push({ t: Date.now() - t0, ...(await page.evaluate(read)) });
    await wait(OPTS.interval);
  }
  const afterSettle = samples.filter((s) => s.t >= 9000);
  state.soloWaiting = {
    claimed,
    samples: samples.length,
    enabledAnytime: samples.filter((s) => s.fillEnabled).length,
    afterSettleSamples: afterSettle.length,
    waitingAfterSettle: afterSettle.length > 0 && afterSettle.every((s) => s.waiting),
    final: samples[samples.length - 1] ?? null,
  };
  await page.close();
  return state.soloWaiting;
})();

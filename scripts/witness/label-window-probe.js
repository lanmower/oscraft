(async () => {
  const OPTS = { ns: "lw-final3", port: 8853, afterHostMs: 25000, interval: 200 };
  const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const url = `http://127.0.0.1:${OPTS.port}/js/client/play.html?ci=1&ns=${OPTS.ns}`;
  const built = () => document.body.dataset.worldBuilt === "1" && typeof window.oscraft?.lobby === "function";
  const read = () => {
    const fill = [...document.querySelectorAll("button")].find((b) => b.textContent.trim() === "Fill with bots");
    const seats = [...document.querySelectorAll("body *")]
      .filter((e) => e.children.length === 0 && /^Slot \d+: /.test(e.textContent.trim()))
      .map((e) => e.textContent.trim());
    return { fillPresent: fill !== undefined, fillEnabled: fill !== undefined && !fill.disabled, seats: seats.slice(0, 3) };
  };
  const joiner = await context.newPage();
  await joiner.goto(url, { waitUntil: "domcontentloaded", timeout: 50000 });
  await joiner.waitForFunction(built, null, { timeout: 240000 });
  const claimedSolo = await joiner.evaluate(() => window.oscraft.lobby("claim", { slot: 2 }));
  const solo = await joiner.evaluate(read);
  const host = await context.newPage();
  await host.goto(`${url}&host=1`, { waitUntil: "domcontentloaded", timeout: 50000 });
  await host.waitForFunction(built, null, { timeout: 240000 });
  const hostCalls = [];
  for (const [call, args] of [["mode", { code: "ap" }], ["claim", { slot: 0 }]]) {
    hostCalls.push({ call, ok: await host.evaluate(([c, a]) => window.oscraft.lobby(c, a), [call, args]) });
  }
  const samples = [];
  const t0 = Date.now();
  while (Date.now() - t0 < OPTS.afterHostMs) {
    samples.push({ t: Date.now() - t0, ...(await joiner.evaluate(read)) });
    await wait(OPTS.interval);
  }
  const firstOther = samples.findIndex((s) => s.seats.some((text) => /^Slot 1: (?!you)/.test(text)));
  const afterHost = firstOther < 0 ? [] : samples.slice(firstOther);
  const labelWindow = afterHost.filter((s) => s.fillEnabled);
  const result = {
    claimedSolo,
    solo,
    hostCalls,
    samples: samples.length,
    firstSeatOfHostAtMs: firstOther < 0 ? null : samples[firstOther].t,
    labelWindowSamples: labelWindow.length,
    labelWindowMs: labelWindow.length === 0 ? 0 : labelWindow[labelWindow.length - 1].t - labelWindow[0].t,
    reproduced: labelWindow.length > 0,
    finalJoiner: samples[samples.length - 1] ?? null,
  };
  state.labelProbe = result;
  await joiner.close();
  await host.close();
  return result;
})();

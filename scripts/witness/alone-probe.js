(async () => {
  const OPTS = { ns: "alone-probe", port: 8853, settleMs: 9000, stepMs: 1500 };
  const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const url = "http://127.0.0.1:" + OPTS.port + "/js/client/play.html?ci=1&ns=" + OPTS.ns;
  const built = () => document.body.dataset.worldBuilt === "1" && typeof window.oscraft?.lobby === "function";
  const read = () => {
    const button = (text) => [...document.querySelectorAll("button")].find((b) => b.textContent.trim() === text);
    const fill = button("Fill with bots");
    const alone = button("Play alone");
    const text = document.body.textContent;
    const seats = [...document.querySelectorAll("body *")]
      .filter((e) => e.children.length === 0 && /^Slot \d+: /.test(e.textContent.trim()))
      .map((e) => e.textContent.trim());
    return {
      fillPresent: fill !== undefined,
      fillEnabled: fill !== undefined && !fill.disabled,
      alonePresent: alone !== undefined,
      aloneEnabled: alone !== undefined && !alone.disabled,
      reasonShown: text.includes("Play alone starts a local match on this page."),
      waitingShown: text.includes("waiting for a host to claim a seat"),
      bots: seats.filter((s) => /bot/.test(s)).length,
      seats: seats.slice(0, 3),
    };
  };
  const joiner = await context.newPage();
  await joiner.goto(url, { waitUntil: "domcontentloaded", timeout: 50000 });
  await joiner.waitForFunction(built, null, { timeout: 240000 });
  const claimed = await joiner.evaluate(() => window.oscraft.lobby("claim", { slot: 2 }));
  await wait(OPTS.settleMs);
  const beforeAlone = await joiner.evaluate(read);
  await joiner.evaluate(() => [...document.querySelectorAll("button")].find((b) => b.textContent.trim() === "Play alone").click());
  await wait(OPTS.stepMs);
  const afterAlone = await joiner.evaluate(read);
  await joiner.evaluate(() => [...document.querySelectorAll("button")].find((b) => b.textContent.trim() === "Fill with bots").click());
  await wait(OPTS.stepMs);
  const afterFill = await joiner.evaluate(read);
  const result = { claimed, beforeAlone, afterAlone, afterFill };
  state.aloneProbe = result;
  await joiner.close();
  return result;
})();

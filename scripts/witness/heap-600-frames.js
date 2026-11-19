(async () => {
  const OPTS = { label: "after", beforeDir: "", ns: "hp-after", port: 8853, frames: 600, warm: 120 };
  const JS_SITES = ["syncDeadProps", "drawWorld3d", "viewRect", "gameMaxHps"];
  try {
    const page = await context.newPage();
    if (OPTS.beforeDir) {
      for (const name of ["play", "hud"]) {
        await page.route(new RegExp(`/js/client/${name}\\.js(\\?|$)`), (route) =>
          route.fulfill({ path: `${OPTS.beforeDir}/js/client/${name}.js`, contentType: "text/javascript" }));
      }
    }
    const built = () => document.body.dataset.worldBuilt === "1" && typeof window.oscraft?.lobby === "function";
    await page.goto(`http://127.0.0.1:${OPTS.port}/js/client/play.html?ci=1&ns=${OPTS.ns}&host=1`, { waitUntil: "domcontentloaded", timeout: 50000 });
    await page.waitForFunction(built, null, { timeout: 240000 });
    const lobby = (call, args) => page.evaluate(([c, a]) => window.oscraft.lobby(c, a), [call, args]);
    for (const [call, args] of [["mode", { code: "ap" }], ["claim", { slot: 0 }], ["pick", { slot: 0 }], ["fill", {}], ["ready", { slot: 0, ready: true }], ["start", {}]]) {
      await lobby(call, args);
    }
    await page.waitForFunction(() => document.body.dataset.phase === "live", null, { timeout: 420000 });
    const playHasJEdits = await page.evaluate(async () => (await (await fetch("/js/client/play.js", { cache: "no-store" })).text()).includes("savedDrawCounts"));
    await page.evaluate(() => {
      window.__heapFrames = 0;
      const tick = () => { window.__heapFrames += 1; requestAnimationFrame(tick); };
      requestAnimationFrame(tick);
    });
    await page.waitForFunction((n) => window.__heapFrames >= n, OPTS.warm, { timeout: 900000 });
    const cdp = await context.newCDPSession(page);
    await cdp.send("HeapProfiler.enable");
    await cdp.send("HeapProfiler.startSampling", { samplingInterval: 256, includeObjectsCollectedByMajorGC: true, includeObjectsCollectedByMinorGC: true });
    const startFrame = await page.evaluate(() => window.__heapFrames);
    const t0 = Date.now();
    await page.waitForFunction((n) => window.__heapFrames >= n, startFrame + OPTS.frames, { timeout: 1800000 });
    const endFrame = await page.evaluate(() => window.__heapFrames);
    const wallMs = Date.now() - t0;
    const { profile } = await cdp.send("HeapProfiler.stopSampling");
    await cdp.detach();
    const bySite = new Map();
    const walk = (node) => {
      const key = `${node.callFrame.functionName || "(anonymous)"} ${node.callFrame.url.split("/").pop()}`;
      bySite.set(key, (bySite.get(key) || 0) + node.selfSize);
      for (const child of node.children || []) walk(child);
    };
    walk(profile.head);
    const totalBytes = [...bySite.values()].reduce((sum, bytes) => sum + bytes, 0);
    const frames = endFrame - startFrame;
    const jSites = JS_SITES.map((name) => {
      const hits = [...bySite].filter(([key]) => key.startsWith(`${name} `));
      return { name, bytes: hits.reduce((sum, [, bytes]) => sum + bytes, 0) };
    });
    const top = [...bySite].sort((a, b) => b[1] - a[1]).slice(0, 25).map(([key, bytes]) => ({ key, bytes }));
    const result = { label: OPTS.label, ns: OPTS.ns, playHasJEdits, frames, wallMs, totalBytes, bytesPerFrame: Math.round(totalBytes / frames), jSites, top };
    state.heapResult = state.heapResult || {};
    state.heapResult[OPTS.ns] = result;
    await page.close();
    return result;
  } catch (error) {
    state.heapErrors = { ...(state.heapErrors || {}), [OPTS.ns]: String(error) };
    return { error: String(error) };
  }
})();

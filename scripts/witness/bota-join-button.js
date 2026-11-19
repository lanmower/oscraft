state.botaJoinButton = (() => {
  const base = "http://127.0.0.1:8853/js/client/play.html?ci=1&package=scenarios/bota&ns=bota-join-button";
  const pages = {};
  const built = (page) => page.evaluate(() => document.body.dataset.worldBuilt === "1" && typeof window.oscraft?.lobby === "function");
  const lobby = (page, call, args) => page.evaluate(([c, a]) => window.oscraft.lobby(c, a), [call, args]);
  const read = (page) => page.evaluate(() => {
    const joins = [...document.querySelectorAll("button.flow-action")]
      .filter((b) => b.textContent.trim().startsWith("Join"))
      .map((b) => ({ text: b.textContent.trim(), disabled: b.disabled }));
    const seats = [...document.querySelectorAll("body *")]
      .filter((e) => e.children.length === 0 && /^Slot \d+: /.test(e.textContent.trim()))
      .map((e) => e.textContent.trim());
    return { joins, enabledJoin: joins.some((j) => !j.disabled), botSeats: seats.filter((s) => /computer|bot/.test(s)).length, seats: seats.slice(0, 3) };
  });
  const steps = {
    openHost: async () => { pages.host = await context.newPage(); await pages.host.goto(base + "&host=1", { waitUntil: "domcontentloaded", timeout: 8000 }); return "opened host"; },
    openJoiner: async () => { pages.joiner = await context.newPage(); await pages.joiner.goto(base, { waitUntil: "domcontentloaded", timeout: 8000 }); return "opened joiner"; },
    built: async (role) => ({ built: await built(pages[role]) }),
    hostSetup: async () => ({ mode: await lobby(pages.host, "mode", { code: "ap" }), claim: await lobby(pages.host, "claim", { slot: 0 }), fill: await lobby(pages.host, "fill", {}) }),
    readJoiner: async () => ({ joiner: await read(pages.joiner) }),
    close: async () => { for (const role of Object.keys(pages)) await pages[role].close(); return "closed"; },
  };
  return { pages, steps };
})();
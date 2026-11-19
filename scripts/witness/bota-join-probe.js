state.botaJoin = (() => {
  const base = "http://127.0.0.1:8853/js/client/play.html?ci=1&package=scenarios/bota";
  const room = "";
  const pages = {};
  const read = (page) => page.evaluate(() => {
    const button = (text) => [...document.querySelectorAll("button")].find((b) => b.textContent.trim() === text);
    const fill = button("Fill with bots");
    const seats = [...document.querySelectorAll("body *")]
      .filter((e) => e.children.length === 0 && /^Slot \d+: /.test(e.textContent.trim()))
      .map((e) => e.textContent.trim());
    return {
      fillPresent: fill !== undefined,
      fillEnabled: fill !== undefined && !fill.disabled,
      alonePresent: button("Play alone") !== undefined,
      seats,
      bots: seats.filter((s) => /computer|bot/.test(s)).length,
      botaFetched: performance.getEntriesByType("resource").some((e) => e.name.includes("/scenarios/bota/")),
    };
  });
  const built = (page) => page.evaluate(() => document.body.dataset.worldBuilt === "1" && typeof window.oscraft?.lobby === "function");
  const lobby = (page, call, args) => page.evaluate(([c, a]) => window.oscraft.lobby(c, a), [call, args]);
  const hasHostSeat = (page) => page.evaluate(() => [...document.querySelectorAll("body *")].some((e) => e.children.length === 0 && /^Slot 1: (?!open)/.test(e.textContent.trim())));
  const steps = {
    open: async (role) => {
      pages[role] = await context.newPage();
      const host = role === "host" ? "&host=1" : "";
      await pages[role].goto(base + "&ns=bota-join" + host + (room ? "#" + room : ""), { waitUntil: "domcontentloaded", timeout: 8000 });
      return "opened " + role;
    },
    built: async (role) => ({ built: await built(pages[role]) }),
    hostSetup: async () => ({ mode: await lobby(pages.host, "mode", { code: "ap" }), claim: await lobby(pages.host, "claim", { slot: 0 }) }),
    hostSeen: async () => ({ hostSeen: await hasHostSeat(pages.joiner) }),
    joinerClaim: async () => ({ claim: await lobby(pages.joiner, "claim", { slot: 2 }) }),
    readBoth: async () => ({ host: await read(pages.host), joiner: await read(pages.joiner) }),
    seatedBoth: async () => {
      const host = await read(pages.host);
      const joiner = await read(pages.joiner);
      return { seated: host.seats.some((s) => /^Slot 3: player/.test(s)) && joiner.seats.some((s) => /^Slot 3: you/.test(s)) };
    },
    hostFill: async () => { await pages.host.evaluate(() => [...document.querySelectorAll("button")].find((b) => b.textContent.trim() === "Fill with bots").click()); return "clicked"; },
    close: async () => { for (const role of Object.keys(pages)) await pages[role].close(); return "closed"; },
  };
  return { room, pages, steps };
})();
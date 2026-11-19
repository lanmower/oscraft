(async () => {
  const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const deadline = performance.now() + 90000;
  while (document.body.dataset.worldBuilt !== "1" || typeof window.oscraft?.lobby !== "function") {
    if (performance.now() > deadline) return { ok: false, reason: "the bota world did not build within 90 s" };
    await wait(250);
  }
  const table = await (await fetch("scenarios/bota/hero_acronyms.lua", { cache: "no-store" })).text();
  const names = [...table.matchAll(/\["[^"]+"\] = "([^"]+)"/g)].map((match) => match[1]);
  const sent = [];
  for (const [call, args] of [["mode", { code: "ap" }], ["claim", { slot: 0 }], ["fill", {}]]) {
    sent.push({ call, ok: window.oscraft.lobby(call, args) });
  }
  let cards = [];
  while (performance.now() < deadline) {
    await wait(500);
    cards = [...document.querySelectorAll(".flow-hero")];
    if (cards.length > 0) break;
  }
  const labels = cards.map((card) => card.dataset.acro ?? "");
  const questionMarks = labels.filter((label) => label === "" || label === "?").length;
  const outside = labels.filter((label) => !names.includes(label));
  const unique = new Set(labels).size;
  return {
    ok: names.length === 24 && cards.length === names.length && questionMarks === 0 && outside.length === 0 && unique === cards.length,
    rosterNames: names.length,
    cards: cards.length,
    questionMarks,
    outside,
    unique,
    sent,
    labels,
  };
})();

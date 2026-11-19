(async () => {
  const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const deadline = performance.now() + 120000;
  while (document.body.dataset.worldBuilt !== "1" || typeof window.oscraft?.lobby !== "function") {
    if (performance.now() > deadline) return { ok: false, reason: "the bota world did not build within 120 s" };
    await wait(250);
  }
  const table = await (await fetch("/scenarios/bota/hero_acronyms.lua", { cache: "no-store" })).text();
  const roster = [...table.matchAll(/\["([^"]+)"\] = "/g)].map((match) => match[1]);
  const sent = [];
  for (const [call, args] of [["mode", { code: "ap" }], ["claim", { slot: 0 }], ["fill", {}]]) {
    sent.push({ call, ok: window.oscraft.lobby(call, args) });
  }
  const board = () => [...document.querySelectorAll(".flow-hero")].map((card) => card.dataset.acro ?? "");
  let labels = [];
  let stable = 0;
  while (performance.now() < deadline && stable < 3) {
    await wait(1000);
    const next = board();
    stable = next.length > 0 && next.length === labels.length ? stable + 1 : 0;
    labels = next;
  }
  const seats = [...document.querySelectorAll("body *")]
    .filter((element) => element.children.length === 0 && /^Slot \d+: /.test(element.textContent.trim()))
    .map((element) => element.textContent.trim());
  const computerSeats = seats.filter((text) => text.endsWith(": computer")).length;
  const questionMarks = labels.filter((label) => label === "" || label === "?").length;
  const outside = labels.filter((label) => !roster.includes(label));
  const absent = roster.filter((name) => !labels.includes(name));
  const unique = new Set(labels).size;
  return {
    ok: roster.length === 24 && labels.length > 0 && questionMarks === 0 && outside.length === 0 && unique === labels.length && labels.length + absent.length === roster.length && absent.length === computerSeats,
    roster: roster.length,
    cards: labels.length,
    questionMarks,
    outside,
    unique,
    computerSeats,
    absent,
    sent,
    labels,
    seats: seats.slice(0, 3),
  };
})();

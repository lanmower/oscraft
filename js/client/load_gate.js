
export const PHASE = { LOBBY: "lobby", LOADING: "loading", LIVE: "live" };
export const STALL_MS = 30000;

const STAGE_TEXT = {
  waiting: "waiting for its hero",
  loading: "loading hero models",
  warming: "warming up the GPU",
  ready: "ready",
  failed: "failed",
};

export function createLoadGate({ now = () => performance.now() } = {}) {
  let phase = PHASE.LOBBY;
  let seats = [];
  let local = { stage: "waiting", detail: "" };
  let remoteRows = null;
  let beganAt = 0;
  const peersReady = new Set();

  function seatRow(seat, ctx) {
    const mine = ctx.localOwner(seat.owner);
    let stage = "waiting";
    let detail = "";
    if (mine) {
      stage = local.stage;
      detail = local.detail;
    } else if (peersReady.has(seat.owner)) {
      stage = "ready";
    } else if (!ctx.connected(seat.owner)) {
      detail = "not connected";
    } else {
      stage = "loading";
    }
    const waited = now() - beganAt;
    const stalled = stage !== "ready" && waited >= STALL_MS ? ` (still ${stage === "failed" ? "failed" : "not ready"} after ${Math.round(waited / 1000)} s; the match waits for it)` : "";
    const who = seat.owner.startsWith("bot:") ? "bot" : mine ? "host" : seat.owner.slice(0, 8);
    return { slot: seat.slot, name: `seat ${seat.slot + 1}: ${seat.pick || "no hero"} (${who})`, stage, ready: stage === "ready", reason: `${detail || STAGE_TEXT[stage]}${stalled}` };
  }

  return {
    get phase() { return phase; },
    get live() { return phase === PHASE.LIVE; },
    get loading() { return phase === PHASE.LOADING; },
    get beganAt() { return beganAt; },
    get local() { return local; },
    seats: () => seats,
    begin(list) {
      if (phase !== PHASE.LOBBY) return false;
      seats = list.map((s) => ({ slot: s.slot, owner: String(s.owner || ""), pick: s.pick || "" }));
      local = { stage: "waiting", detail: "" };
      remoteRows = null;
      beganAt = now();
      phase = PHASE.LOADING;
      return true;
    },
    setLocal(stage, detail = "") { local = { stage, detail }; },
    notePeer(owner) { peersReady.add(String(owner)); },
    rows(ctx) {
      return remoteRows ?? seats.map((s) => seatRow(s, ctx));
    },
    canOpen(ctx) {
      return phase === PHASE.LOADING && local.stage === "ready" && seats.every((s) => seatRow(s, ctx).ready);
    },
    open() {
      if (phase === PHASE.LIVE) return false;
      phase = PHASE.LIVE;
      return true;
    },
    applyRemote(data) {
      remoteRows = Array.isArray(data.rows) ? data.rows : null;
    },
  };
}

export function createCover(root) {
  const el = document.createElement("div");
  el.id = "load-cover";
  el.hidden = true;
  el.setAttribute("role", "status");
  el.setAttribute("aria-live", "polite");
  el.style.cssText = "position:fixed;inset:0;z-index:100;background:#000;color:#e6ecf1;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:14px;font:14px system-ui,sans-serif";
  const title = document.createElement("div");
  title.textContent = "Loading match";
  const bar = document.createElement("progress");
  bar.style.width = "240px";
  const list = document.createElement("ul");
  list.style.cssText = "list-style:none;margin:0;padding:0;min-width:320px;font-size:12px;color:#9aa7b2";
  el.append(title, bar, list);
  root.append(el);
  let shown = "";
  return {
    show() { el.hidden = false; },
    hide() { el.hidden = true; },
    get visible() { return !el.hidden; },
    render(rows, headline) {
      const key = `${headline}|${rows.map((r) => `${r.name}:${r.reason}`).join("|")}`;
      if (key === shown) return;
      shown = key;
      title.textContent = headline;
      while (list.children.length > rows.length) list.lastChild.remove();
      while (list.children.length < rows.length) list.append(document.createElement("li"));
      rows.forEach((r, i) => {
        const li = list.children[i];
        li.textContent = `${r.name}: ${r.reason}`;
        li.style.color = r.ready ? "#4caf50" : r.stage === "failed" ? "#ff7a6b" : "#9aa7b2";
      });
    },
  };
}

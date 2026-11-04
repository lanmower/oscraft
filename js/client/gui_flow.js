import { sceneWidget } from "./widgets.js";
const TEAM_SIZE = 5;
const GROUPS = [
  { id: 1, name: "Strength" },
  { id: 2, name: "Agility" },
  { id: 3, name: "Intelligence" },
];

export function createGuiFlow({ root, iconUrl, lobby, me, act, onPodium, teamNames = ["The Sentinel", "The Scourge"] }) {
  const el = document.createElement("div");
  el.id = "gui-flow";
  el.dataset.stage = "";
  el.hidden = true;
  root.append(el);
  const style = document.createElement("style");
  style.textContent = `
#gui-flow { position: fixed; inset: 0; z-index: 40; display: flex; flex-direction: column;
  align-items: center; justify-content: center; gap: 3vh;
  background: radial-gradient(ellipse at 50% 30%, rgba(10,14,22,.86), rgba(6,8,12,.96)); color: #e8ecf4;
  font: 15px system-ui, sans-serif; }
#gui-flow[data-stage="podium"], #gui-flow[data-stage="wait"] { background: linear-gradient(to bottom, rgba(8,10,16,.92) 0, transparent 45%, rgba(8,10,16,.92) 100%); pointer-events: none; }
#gui-flow h1 { font-size: 34px; font-weight: 600; letter-spacing: .04em; margin: 0; text-align: center; }
#gui-flow[data-stage="podium"] h1, #gui-flow[data-stage="wait"] h1 { font-size: 22px; text-shadow: 0 2px 18px rgba(0,0,0,.7); }
#gui-flow[data-stage="podium"] .flow-hero-bar, #gui-flow[data-stage="podium"] .flow-board,
#gui-flow[data-stage="wait"] .flow-hero-bar, #gui-flow[data-stage="wait"] .flow-board { display: none; }
body[data-flow] #rooms, body[data-flow] #ui-modal, body[data-flow] #ui-left, body[data-flow] #ui-right,
body[data-flow] #ui-bar, body[data-flow] #centre-hero, body[data-flow] #hero-lock { display: none !important; }
.flow-modes-label { font-size: 12px; letter-spacing: .12em; text-transform: uppercase; color: #8fa4c4; }
.flow-modes { display: flex; flex-wrap: wrap; justify-content: center; gap: 8px; max-width: min(94vw, 1100px); }
.flow-mode { padding: 5px 12px; border: 1px solid #2c3648; border-radius: 999px; background: rgba(18,24,34,.8);
  color: #9fb0cc; font: inherit; font-size: 12px; }
button.flow-mode { cursor: pointer; }
button.flow-mode:hover, button.flow-mode:focus { border-color: #6f92d8; outline: none; }
.flow-mode[data-on="true"] { border-color: #6f92d8; color: #e8ecf4; background: rgba(30,44,74,.9); }
button.flow-mode:disabled { cursor: default; opacity: .6; }
.flow-hero-bar { display: flex; align-items: center; justify-content: center; gap: 18px; flex-wrap: wrap;
  width: min(94vw, 1240px); }
.flow-team-chip { padding: 5px 14px; border: 1px solid #2f5da8; border-radius: 999px; font-size: 13px;
  color: #cfe0ff; background: rgba(20,32,58,.8); }
.flow-team-chip[data-team="1"] { border-color: #a8422f; color: #ffd9cd; background: rgba(58,26,20,.8); }
.flow-board { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 14px;
  width: min(94vw, 1240px); height: 66vh; }
.flow-group { display: flex; flex-direction: column; min-height: 0; padding: 10px;
  border: 1px solid #2c3648; border-radius: 14px; background: rgba(13,17,26,.72); }
.flow-group-head { display: flex; align-items: baseline; justify-content: space-between; gap: 8px;
  margin-bottom: 8px; color: #8fa4c4; font-size: 12px; letter-spacing: .12em; text-transform: uppercase; }
.flow-group-head b { color: #e8ecf4; font-size: 15px; letter-spacing: .02em; text-transform: none; }
.flow-group .flow-heroes { --tile: 76px; display: grid; grid-auto-rows: var(--tile); gap: 6px; flex: 1 1 0;
  min-height: 0; overflow: hidden; align-content: start; justify-content: center; padding: 2px; }
.flow-dots { display: none; justify-content: center; gap: 8px; }
.flow-dot { width: 8px; height: 8px; border-radius: 50%; background: #8fa4c4; opacity: .35; }
.flow-dot[data-on="true"] { opacity: 1; background: #e8ecf4; }
.flow-empty { padding: 20px 6px; color: #7d8ba3; font-size: 13px; text-align: center; }
.flow-hero { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 2px; padding: 2px;
  width: var(--tile); height: var(--tile); box-sizing: border-box; overflow: hidden;
  border: 1px solid #2c3648; border-radius: 10px; background: rgba(18,24,34,.8); cursor: pointer; color: #e8ecf4; font: inherit; }
.flow-hero:hover, .flow-hero:focus { border-color: #6f92d8; outline: none; transform: translateY(-1px); }
.flow-hero img { width: calc(var(--tile) * .64); height: calc(var(--tile) * .64); object-fit: contain; }
.flow-acro { font-size: max(8px, calc(var(--tile) * .16)); letter-spacing: .08em; font-weight: 600; }
.flow-heads { display: flex; align-items: flex-start; justify-content: space-between; gap: 4vw; width: min(96vw, 1040px); }
.flow-head { display: flex; flex-direction: column; align-items: center; gap: 4px; }
.flow-head b { font-size: 16px; letter-spacing: .05em; }
.flow-head span { font-size: 11px; color: #8fa4c4; letter-spacing: .1em; text-transform: uppercase; }
.flow-head[data-team="0"] b { color: #8fb6ff; }
.flow-head[data-team="1"] b { color: #ffab8f; }
.flow-remove { margin-left: 4px; padding: 0 4px; border: 0; background: transparent; color: #9fb0cc; font: inherit; font-size: 13px; cursor: pointer; pointer-events: auto; }
.flow-remove:hover, .flow-remove:focus { color: #ffab8f; outline: none; }
.flow-bar { display: flex; align-items: center; justify-content: center; gap: 14px; flex-wrap: wrap; pointer-events: auto; }
.flow-action { padding: 10px 20px; border: 1px solid #3d5f96; border-radius: 10px; background: rgba(20,32,58,.9);
  color: #e8ecf4; font: inherit; cursor: pointer; }
.flow-action:disabled { opacity: .4; cursor: default; }
.flow-error { min-height: 1.2em; font-size: 13px; color: #ffab8f; text-align: center; }
.flow-wait { font-size: 14px; color: #93a3bd; }
@media (max-width: 600px) {
  .flow-board { display: flex; gap: 0; height: 64vh; overflow-x: auto; overflow-y: hidden;
    scroll-snap-type: x mandatory; touch-action: pan-x; }
  .flow-board > .flow-group { flex: 0 0 100%; box-sizing: border-box; scroll-snap-align: start; scroll-snap-stop: always; }
  .flow-dots { display: flex; }
}
`;
  el.append(style);

  let stage = null;
  let hidden = false;
  let podiumShown = false;
  const scene = sceneWidget();

  const node = (tag, className, text) => {
    const n = document.createElement(tag);
    if (className) n.className = className;
    if (text !== undefined) n.textContent = text;
    return n;
  };
  const seatsOf = (state) => Array.from({ length: TEAM_SIZE * 2 }, (_, i) => state.slots?.[i] ?? {});
  const teamOf = (index) => (index < TEAM_SIZE ? 0 : 1);
  const mySeat = (state) => seatsOf(state).findIndex((seat) => seat.owner === me());
  const heroesOf = (state) => new Map((state.heroes ?? []).map((hero) => [hero.unit, hero]));
  const groupOf = (hero) => (GROUPS.some((g) => g.id === hero.group) ? hero.group : 1);

  const stageOf = (state) => {
    if (hidden || state.draft === true || !state.slots?.length) return null;
    if (state.started === true) return "wait";
    const seat = mySeat(state);
    if (seat < 0) return "join";
    return seatsOf(state)[seat].hero ? "podium" : "hero";
  };

  const portrait = (model, alt) => {
    const img = node("img");
    img.dataset.model = model ?? "";
    img.alt = alt ?? "";
    img.width = 60;
    img.height = 60;
    img.hidden = true;
    img.addEventListener("error", () => { img.hidden = true; });
    return img;
  };

  function refreshIcons() {
    for (const img of el.querySelectorAll("img[data-model]")) {
      if (img.dataset.ready === "1" || !img.dataset.model) continue;
      const src = iconUrl?.(img.dataset.model) ?? null;
      if (typeof src !== "string" || !src) continue;
      img.src = src;
      img.hidden = false;
      img.dataset.ready = "1";
    }
  }

  const errorLine = (state) => node("div", "flow-error", state.error || "");

  const modeChips = (state, editable) => {
    const row = node("div", "flow-modes");
    const open = editable && (!state.host || state.host === me());
    for (const mode of state.modes ?? []) {
      if (!editable && !mode.on) continue;
      const chip = node(editable ? "button" : "span", "flow-mode", mode.name);
      chip.dataset.code = mode.code;
      chip.dataset.on = mode.on ? "true" : "false";
      if (editable) {
        chip.type = "button";
        chip.disabled = !open;
        if (open) chip.addEventListener("click", () => act("mode", { code: mode.code }));
      }
      row.append(chip);
    }
    return row;
  };

  const claimable = (seat) => !seat.owner || seat.owner.startsWith("bot:");
  const canJoin = (state) => !state.host || state.negotiated === true;
  const claimTeam = (state, team) => {
    const at = seatsOf(state).findIndex((seat, i) => teamOf(i) === team && claimable(seat));
    if (canJoin(state) && at >= 0) act("claim", { slot: at });
  };

  const joinButton = (state, team) => {
    const open = canJoin(state) && seatsOf(state).some((seat, i) => teamOf(i) === team && claimable(seat));
    const join = node("button", "flow-action", `Join ${teamNames[team] ?? `Team ${team + 1}`}`);
    join.type = "button";
    join.disabled = !open;
    join.addEventListener("click", () => claimTeam(state, team));
    return join;
  };

  const pickHero = (hero) => {
    const seat = mySeat(lobby() ?? {});
    if (seat >= 0) act("pick", { slot: seat, hero: hero.unit });
  };

  const heroTile = (hero) => {
    const card = node("button", "flow-hero");
    card.type = "button";
    card.dataset.acro = hero.acronym;
    card.append(portrait(hero.model, hero.acronym), node("span", "flow-acro", hero.acronym));
    card.addEventListener("click", () => pickHero(hero));
    return card;
  };

  const renderBoard = (board, state) => {
    const held = new Set(seatsOf(state).filter((seat) => seat.hero).map((seat) => seat.hero));
    const offered = (state.heroes ?? []).filter((hero) => hero.allowed && (state.duplicates === true || !held.has(hero.unit)));
    for (const group of GROUPS) {
      const members = offered.filter((hero) => groupOf(hero) === group.id);
      const col = node("div", "flow-group");
      const head = node("div", "flow-group-head");
      head.append(node("b", "", group.name), node("span", "", `${members.length}`));
      col.append(head);
      if (!members.length) {
        col.append(node("div", "flow-empty", "no heroes here"));
      } else {
        const grid = node("div", "flow-heroes");
        for (const hero of members) grid.append(heroTile(hero));
        col.append(grid);
      }
      board.append(col);
    }
  };

  const TILE_GAP = 6;
  const TILE_MIN = 28;
  const TILE_MAX = 96;
  const fitTiles = (grid) => {
    const count = grid.children.length;
    const w = grid.clientWidth - 4, h = grid.clientHeight - 4;
    if (!count || w <= 0 || h <= 0) return;
    let best = { size: 0, cols: 1 };
    for (let cols = 1; cols <= count; cols++) {
      const rows = Math.ceil(count / cols);
      const size = Math.min((w - TILE_GAP * (cols - 1)) / cols, (h - TILE_GAP * (rows - 1)) / rows);
      if (size > best.size) best = { size, cols };
    }
    const size = Math.max(TILE_MIN, Math.min(TILE_MAX, Math.floor(best.size)));
    grid.style.gridTemplateColumns = `repeat(${best.cols}, ${size}px)`;
    grid.style.setProperty("--tile", `${size}px`);
  };
  const tileFitter = typeof ResizeObserver === "function"
    ? new ResizeObserver((entries) => { for (const entry of entries) fitTiles(entry.target); })
    : null;
  let boardLeft = 0;

  const swipeDots = (board, count, remember = false) => {
    const dots = node("div", "flow-dots");
    const marks = Array.from({ length: count }, () => dots.appendChild(node("span", "flow-dot")));
    const show = () => {
      const at = Math.round(board.scrollLeft / Math.max(1, board.clientWidth));
      marks.forEach((mark, i) => { mark.dataset.on = String(i === at); });
    };
    board.addEventListener("scroll", () => {
      if (remember) boardLeft = board.scrollLeft;
      show();
    }, { passive: true });
    return { dots, show };
  };

  const heroStage = (state) => {
    const team = teamOf(mySeat(state));
    const chip = node("span", "flow-team-chip", `Locking in for ${teamNames[team] ?? ""}`);
    chip.dataset.team = String(team);
    const bar = node("div", "flow-hero-bar", "");
    bar.append(chip, modeChips(state, false));
    const board = node("div", "flow-board");
    renderBoard(board, state);
    const swipe = swipeDots(board, GROUPS.length, true);
    el.append(node("h1", "", "Choose your hero"), bar, board, swipe.dots, errorLine(state));
    board.scrollLeft = boardLeft;
    swipe.show();
    for (const grid of board.querySelectorAll(".flow-heroes")) {
      fitTiles(grid);
      tileFitter?.observe(grid);
    }
  };

  const seatPlate = (seat, mine, heroes, removeSlot = -1) => {
    if (!seat.owner) return { label: "open", data: { mine: "false", ready: "false" } };
    const hero = seat.hero ? heroes.get(seat.hero) : null;
    const extra = [];
    if (removeSlot >= 0 && !mine) {
      const remove = node("button", "flow-remove", "×");
      remove.type = "button";
      remove.title = "Remove";
      remove.setAttribute("aria-label", "Remove");
      remove.addEventListener("click", () => act("remove", { slot: removeSlot }));
      extra.push(remove);
    }
    return {
      model: hero?.model ?? seat.model ?? null,
      label: hero?.acronym || "picking",
      data: { mine: String(mine), ready: seat.ready ? "true" : "false" },
      extra,
    };
  };

  const actionBar = (state, seat, seats) => {
    const mine = seats[seat] ?? {};
    const bar = node("div", "flow-bar");
    const ready = node("button", "flow-action", mine.ready === true ? "Unready" : "Ready");
    ready.type = "button";
    ready.disabled = seat < 0;
    ready.addEventListener("click", () => act("ready", { slot: seat, ready: mine.ready !== true }));
    bar.append(ready);
    if (state.host === me()) {
      const fill = node("button", "flow-action", "Fill with bots");
      fill.type = "button";
      fill.disabled = seats.every((s) => s.owner);
      fill.addEventListener("click", () => act("fill", {}));
      const start = node("button", "flow-action", "Start match");
      start.type = "button";
      start.disabled = state.worldReady !== true || !seats.every((s) => s.owner && s.ready);
      start.addEventListener("click", () => act("start", {}));
      bar.append(fill, start);
    }
    return bar;
  };

  const lobbyBoard = (state, joining) => {
    const seats = seatsOf(state);
    const seat = mySeat(state);
    const heroes = heroesOf(state);
    el.append(node("h1", "", joining ? "Lobby" : "Teams ready to fight"));
    el.append(node("div", "flow-modes-label", "Game mode"), modeChips(state, true));
    const heads = node("div", "flow-heads");
    [0, 1].forEach((team) => {
      const ready = seats.filter((s, i) => teamOf(i) === team && s.owner && s.ready).length;
      const head = node("div", "flow-head");
      head.dataset.team = String(team);
      head.append(node("b", "", teamNames[team] ?? `Team ${team + 1}`), node("span", "", `${ready}/${TEAM_SIZE} ready`));
      if (joining) head.append(joinButton(state, team));
      heads.append(head);
    });
    scene.update(seats.map((s, i) => seatPlate(s, i === seat, heroes, state.host === me() ? i : -1)));
    el.append(heads, scene.element, actionBar(state, seat, seats), errorLine(state));
    const allReady = seats.every((s) => s.owner && s.ready);
    const note = joining && !canJoin(state) ? "Negotiate the lobby first." : allReady ? "Every seat is ready." : "Standing by for the match…";
    el.append(node("div", "flow-wait", note));
  };

  const render = () => {
    const state = lobby() ?? {};
    const wasPodium = podiumShown;
    tileFitter?.disconnect();
    stage = stageOf(state);
    el.dataset.stage = stage ?? "";
    el.hidden = stage == null || stage === "wait";
    if (stage == null) delete document.body.dataset.flow;
    else document.body.dataset.flow = stage;
    el.replaceChildren(style);
    scene.setActive(stage === "join" || stage === "podium");
    if (stage === "join") lobbyBoard(state, true);
    else if (stage === "podium") lobbyBoard(state, false);
    else if (stage === "hero") heroStage(state);
    refreshIcons();
    podiumShown = stage === "podium" || stage === "wait";
    if (podiumShown) onPodium?.(seatsOf(state));
    else if (wasPodium) onPodium?.(null);
  };

  render();

  return {
    stage: () => stage,
    team: () => {
      const seat = mySeat(lobby() ?? {});
      return seat < 0 ? null : teamOf(seat);
    },
    pick: () => {
      const state = lobby() ?? {};
      const seat = mySeat(state);
      return seat < 0 ? null : seatsOf(state)[seat].hero ?? null;
    },
    refreshLobby: render,
    refreshIcons,
    show: () => {
      hidden = false;
      render();
    },
    hide: () => {
      hidden = true;
      render();
    },
    teams: () => teamNames,
  };
}

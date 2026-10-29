// Fullscreen pre-match flow: fullscreen team selection, then fullscreen hero
// selection split into the three primary-attribute columns, then the team
// orientation view where both teams' five heroes stand on podiums while the
// 3D scene (podium.js) streams their models underneath.
// This module owns only the overlay DOM and the stage state.

const TEAM_SIZE = 5;
// The primary-attribute columns, in the order the mod's lobby grid shows them
// (panels.lua: one labelled column per primary, strength first).
const GROUPS = [
  { id: 1, name: "Strength" },
  { id: 2, name: "Agility" },
  { id: 3, name: "Intelligence" },
];

export function createGuiFlow({ root, iconUrl, rosterHeroes, onHero, teamNames = ["The Sentinel", "The Scourge"], primaryOf = null, teamPicks = null }) {
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
#gui-flow[data-stage="podium"] { background: linear-gradient(to bottom, rgba(8,10,16,.92) 0, transparent 45%, rgba(8,10,16,.92) 100%); pointer-events: none; }
#gui-flow h1 { font-size: 34px; font-weight: 600; letter-spacing: .04em; margin: 0; }
#gui-flow[data-stage="podium"] h1 { font-size: 22px; text-shadow: 0 2px 18px rgba(0,0,0,.7); }
#gui-flow[data-stage="podium"] .flow-hero-bar, #gui-flow[data-stage="podium"] .flow-board { display: none; }
.flow-teams { display: flex; gap: 4vw; }
.flow-team { padding: 7vh 8vh; border: 2px solid #2c3648; border-radius: 18px; background: rgba(18,24,34,.8);
  cursor: pointer; text-align: center; }
.flow-team:hover, .flow-team:focus { border-color: #6f92d8; outline: none; }
.flow-team[data-team="0"] { border-color: #2f5da8; }
.flow-team[data-team="1"] { border-color: #a8422f; }
.flow-team h2 { margin: 0 0 6px; font-size: 26px; }
.flow-hero-bar { display: flex; align-items: center; justify-content: center; gap: 18px; flex-wrap: wrap;
  width: min(94vw, 1240px); }
.flow-team-chip { padding: 5px 14px; border: 1px solid #2f5da8; border-radius: 999px; font-size: 13px;
  color: #cfe0ff; background: rgba(20,32,58,.8); }
.flow-team-chip[data-team="1"] { border-color: #a8422f; color: #ffd9cd; background: rgba(58,26,20,.8); }
.flow-board { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 14px;
  width: min(94vw, 1240px); max-height: 66vh; }
.flow-group { display: flex; flex-direction: column; min-height: 0; padding: 10px;
  border: 1px solid #2c3648; border-radius: 14px; background: rgba(13,17,26,.72); }
.flow-group-head { display: flex; align-items: baseline; justify-content: space-between; gap: 8px;
  margin-bottom: 8px; color: #8fa4c4; font-size: 12px; letter-spacing: .12em; text-transform: uppercase; }
.flow-group-head b { color: #e8ecf4; font-size: 15px; letter-spacing: .02em; text-transform: none; }
.flow-group .flow-heroes { display: grid; grid-template-columns: repeat(auto-fill, minmax(76px, 1fr));
  gap: 8px; width: auto; max-height: 54vh; overflow: auto; padding: 2px; }
.flow-empty { padding: 20px 6px; color: #7d8ba3; font-size: 13px; text-align: center; }
.flow-hero { display: flex; flex-direction: column; align-items: center; gap: 4px; padding: 6px 4px;
  border: 1px solid #2c3648; border-radius: 10px; background: rgba(18,24,34,.8); cursor: pointer; }
.flow-hero:hover, .flow-hero:focus { border-color: #6f92d8; outline: none; transform: translateY(-1px); }
.flow-hero img { width: 64px; height: 64px; object-fit: contain; }
.flow-hero span { font-size: 11px; text-align: center; }
.flow-hero[data-team] { border-color: #3d5f96; opacity: .75; }
.flow-hero[data-team="1"] { border-color: #8f4634; }
.flow-orient { display: flex; align-items: flex-start; justify-content: space-between; gap: 2vw;
  width: min(96vw, 1560px); }
.flow-side { display: flex; flex-direction: column; gap: 8px; width: clamp(140px, 15vw, 210px);
  max-height: 74vh; overflow: auto; padding-bottom: 6px; }
.flow-side-head { display: flex; flex-direction: column; align-items: center; gap: 2px; padding-bottom: 4px; }
.flow-side-head b { font-size: 16px; letter-spacing: .05em; }
.flow-side-head span { font-size: 11px; color: #8fa4c4; letter-spacing: .1em; text-transform: uppercase; }
.flow-side[data-team="0"] .flow-side-head b { color: #8fb6ff; }
.flow-side[data-team="1"] .flow-side-head b { color: #ffab8f; }
.flow-stand { position: relative; display: flex; flex-direction: column; align-items: center; gap: 3px;
  padding: 8px 6px 10px; border: 1px solid #2c3648; border-radius: 12px 12px 6px 6px;
  background: linear-gradient(180deg, rgba(24,30,44,.94), rgba(12,16,24,.8)); }
.flow-stand::after { content: ""; position: absolute; left: 6%; right: 6%; bottom: -5px; height: 5px;
  border-radius: 3px; background: linear-gradient(180deg, #47536b, #1b2130); box-shadow: 0 6px 10px rgba(0,0,0,.5); }
.flow-stand img { width: 60px; height: 60px; object-fit: contain; }
.flow-stand .flow-stand-name { font-size: 12px; text-align: center; }
.flow-stand .flow-stand-me { font-size: 9px; letter-spacing: .16em; color: #9fd0ff; }
.flow-stand[data-mine="true"] { border-color: #6f92d8;
  box-shadow: 0 0 0 1px rgba(111,146,216,.4), 0 0 18px rgba(111,146,216,.22); }
.flow-stand[data-empty="true"] { border-style: dashed; opacity: .5; }
.flow-wait { font-size: 14px; color: #93a3bd; }
`;
  el.append(style);

  let stage = null;
  let team = null;
  let pick = null;
  let picks = null;

  const h = () => document.createElement("h1");
  const roster = () => rosterHeroes() ?? [];
  const groupOf = (hero) => {
    const group = primaryOf?.(hero);
    return GROUPS.some((g) => g.id === group) ? group : 1;
  };
  // Hero name -> the team whose seats already hold it. The lobby reports both
  // teams' seats before the match starts, so a hero either team already holds
  // is not the chosen team's to pick.
  const takenBy = () => {
    const map = new Map();
    const lists = teamPicks?.() ?? [];
    lists.forEach((members, id) => {
      for (const name of members ?? []) if (typeof name === "string" && !map.has(name)) map.set(name, id);
    });
    return map;
  };
  const passesFilter = (hero, taken) => !taken.has(hero.name);
  // The orientation view renders before the start frame fills `picks` on a
  // fast lock-in, so the local pick stands alone until then.
  const podiumColumns = () => {
    if (picks) return picks;
    const columns = teamNames.map(() => []);
    if (pick != null && team != null && columns[team]) columns[team].push(pick);
    return columns;
  };

  const iconOf = (name) => {
    const src = iconUrl?.({ name }) ?? null;
    return typeof src === "string" && src ? src : null;
  };

  const portrait = (name) => {
    const img = document.createElement("img");
    img.dataset.hero = name;
    img.alt = name;
    img.width = 60;
    img.height = 60;
    const src = iconOf(name);
    img.hidden = src == null;
    if (src) img.src = src;
    img.addEventListener("error", () => { img.hidden = true; });
    return img;
  };

  const heroTile = (hero, taken) => {
    const card = document.createElement("button");
    card.type = "button";
    card.className = "flow-hero";
    card.dataset.hero = hero.name;
    const holder = taken.get(hero.name);
    if (holder !== undefined) card.dataset.team = String(holder);
    card.append(portrait(hero.name), Object.assign(document.createElement("span"), { textContent: hero.name.replace(/[-_]/g, " ") }));
    card.addEventListener("click", () => lockIn(hero));
    return card;
  };

  const renderHeroBoard = (board) => {
    board.replaceChildren();
    const taken = takenBy();
    const heroes = roster().filter((hero) => passesFilter(hero, taken));
    for (const group of GROUPS) {
      const members = heroes.filter((hero) => groupOf(hero) === group.id);
      const col = document.createElement("div");
      col.className = "flow-group";
      const head = document.createElement("div");
      head.className = "flow-group-head";
      head.append(Object.assign(document.createElement("b"), { textContent: group.name }),
        Object.assign(document.createElement("span"), { textContent: `${members.length}` }));
      col.append(head);
      if (!members.length) {
        col.append(Object.assign(document.createElement("div"), { className: "flow-empty", textContent: "no heroes here" }));
      } else {
        const grid = document.createElement("div");
        grid.className = "flow-heroes";
        for (const hero of members) grid.append(heroTile(hero, taken));
        col.append(grid);
      }
      board.append(col);
    }
  };

  const lockIn = (hero) => {
    // The lock-in is the selection: one press and the screen is already on
    // the team orientation view.
    if (stage !== "hero" || pick != null) return;
    pick = hero.name;
    const started = onHero?.(hero);
    if (started === false) { pick = null; return; }
    setStage("podium");
  };

  const stand = (name) => {
    const slot = document.createElement("div");
    slot.className = "flow-stand";
    if (!name) {
      slot.dataset.empty = "true";
      slot.append(Object.assign(document.createElement("span"), { className: "flow-stand-name", textContent: "open slot" }));
      return slot;
    }
    if (name === pick) slot.dataset.mine = "true";
    slot.append(portrait(name), Object.assign(document.createElement("span"), { className: "flow-stand-name", textContent: name.replace(/[-_]/g, " ") }));
    if (name === pick) slot.append(Object.assign(document.createElement("span"), { className: "flow-stand-me", textContent: "YOU" }));
    return slot;
  };

  const setStage = (next) => {
    stage = next;
    el.dataset.stage = next ?? "";
    el.hidden = next == null;
    el.replaceChildren(style);
    const title = h();
    el.append(title);
    if (stage === "team") {
      title.textContent = "Choose your team";
      const row = document.createElement("div");
      row.className = "flow-teams";
      teamNames.forEach((name, id) => {
        const card = document.createElement("button");
        card.type = "button";
        card.className = "flow-team";
        card.dataset.team = String(id);
        card.append(Object.assign(document.createElement("h2"), { textContent: name }));
        card.addEventListener("click", () => {
          if (team != null) return;
          team = id;
          setStage("hero");
        });
        row.append(card);
      });
      el.append(row);
    } else if (stage === "hero") {
      title.textContent = "Choose your hero";
      const bar = document.createElement("div");
      bar.className = "flow-hero-bar";
      const chip = document.createElement("span");
      chip.className = "flow-team-chip";
      chip.dataset.team = String(team ?? 0);
      chip.textContent = `Locking in for ${teamNames[team ?? 0] ?? ""}`;
      bar.append(chip);
      const board = document.createElement("div");
      board.className = "flow-board";
      el.append(bar, board);
      renderHeroBoard(board);
      refreshIcons();
      return;
    } else if (stage === "podium") {
      title.textContent = "Teams ready to fight";
      const board = document.createElement("div");
      board.className = "flow-orient";
      const columns = podiumColumns();
      [0, 1].forEach((id) => {
        const side = document.createElement("div");
        side.className = "flow-side";
        side.dataset.team = String(id);
        const head = document.createElement("div");
        head.className = "flow-side-head";
        head.append(Object.assign(document.createElement("b"), { textContent: teamNames[id] ?? `Team ${id + 1}` }),
          Object.assign(document.createElement("span"), { textContent: "ready to fight" }));
        side.append(head);
        const members = columns[id] ?? [];
        for (let slot = 0; slot < TEAM_SIZE; slot += 1) side.append(stand(members[slot]));
        board.append(side);
      });
      el.append(board);
      el.append(Object.assign(document.createElement("div"), { className: "flow-wait", textContent: "Standing by for the match…" }));
      refreshIcons();
      return;
    }
    refreshIcons();
  };

  // Icons arrive after the grid is built (the atlas streams them), so every
  // stage re-applies what has landed to the portraits it currently shows.
  function refreshIcons() {
    if (!iconUrl) return;
    for (const img of el.querySelectorAll("img[data-hero]")) {
      if (img.dataset.ready === "1") continue;
      const src = iconOf(img.dataset.hero);
      if (!src) continue;
      img.src = src;
      img.hidden = false;
      img.dataset.ready = "1";
    }
  }

  setStage("team");

  return {
    stage: () => stage,
    team: () => team,
    pick: () => pick,
    setStage,
    hide: () => setStage(null),
    setPicks: (byTeam) => { picks = byTeam; setStage("podium"); },
    // The lobby's seat list moves while the grid is up: the free pool is
    // rebuilt from it, so the stage redraws.
    refreshLobby: () => { if (stage === "hero") setStage("hero"); },
    refreshIcons,
    teams: () => teamNames,
  };
}

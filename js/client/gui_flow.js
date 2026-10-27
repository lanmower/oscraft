// Fullscreen pre-match flow: fullscreen team selection (no seat selection),
// then fullscreen hero selection, then a podium stage under which play.js
// renders the 3D podium scene (podium.js) until the game is ready. This
// module owns only the overlay DOM and the stage state.

export function createGuiFlow({ root, iconUrl, rosterHeroes, onHero, teamNames = ["The Sentinel", "The Scourge"] }) {
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
#gui-flow[data-stage="podium"] { background: linear-gradient(to bottom, rgba(8,10,16,.92) 0, transparent 45%, rgba(8,10,16,.92) 100); pointer-events: none; }
#gui-flow h1 { font-size: 34px; font-weight: 600; letter-spacing: .04em; margin: 0; }
#gui-flow[data-stage="podium"] h1 { font-size: 20px; }
#gui-flow[data-stage="podium"] .flow-teams, #gui-flow[data-stage="podium"] .flow-heroes { display: none; }
.flow-teams { display: flex; gap: 4vw; }
.flow-team { padding: 7vh 8vh; border: 2px solid #2c3648; border-radius: 18px; background: rgba(18,24,34,.8);
  cursor: pointer; text-align: center; }
.flow-team:hover, .flow-team:focus { border-color: #6f92d8; outline: none; }
.flow-team[data-team="0"] { border-color: #2f5da8; }
.flow-team[data-team="1"] { border-color: #a8422f; }
.flow-team h2 { margin: 0 0 6px; font-size: 26px; }
.flow-heroes { display: grid; grid-template-columns: repeat(auto-fill, minmax(86px, 1fr)); gap: 10px;
  width: min(92vw, 1080px); max-height: 70vh; overflow: auto; padding: 8px; }
.flow-hero { display: flex; flex-direction: column; align-items: center; gap: 4px; padding: 6px 4px;
  border: 1px solid #2c3648; border-radius: 10px; background: rgba(18,24,34,.8); cursor: pointer; }
.flow-hero:hover, .flow-hero:focus { border-color: #6f92d8; outline: none; }
.flow-hero img { width: 64px; height: 64px; object-fit: contain; }
.flow-hero span { font-size: 11px; text-align: center; }
.flow-rows { display: flex; gap: 8vw; flex-direction: row; }
.flow-row { display: flex; flex-direction: column; gap: 6px; }
.flow-row .flow-pick { display: flex; align-items: center; gap: 8px; padding: 4px 10px;
  border: 1px solid #2c3648; border-radius: 8px; background: rgba(18,24,34,.8); font-size: 13px; }
.flow-row[data-team="0"] .flow-pick { border-color: #2f5da8; }
.flow-row[data-team="1"] .flow-pick { border-color: #a8422f; }
.flow-wait { font-size: 14px; color: #93a3bd; }
`;
  el.append(style);

  let stage = null;
  let team = null;
  let pick = null;
  let picks = null;

  const podiumColumns = () => picks ?? teamNames.map(() => []);
  const h = () => Object.assign(document.createElement("h1"), {});

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
      const row = document.createElement("div");
      row.className = "flow-heroes";
      for (const hero of rosterHeroes()) {
        const card = document.createElement("button");
        card.type = "button";
        card.className = "flow-hero";
        card.dataset.hero = hero.name;
        const img = document.createElement("img");
        img.alt = hero.name;
        img.width = 64;
        img.height = 64;
        const src = iconUrl?.(hero) ?? null;
        img.hidden = src == null;
        if (src) img.src = src;
        img.addEventListener("error", () => { img.hidden = true; });
        card.append(img, Object.assign(document.createElement("span"), { textContent: hero.name }));
        card.addEventListener("click", () => {
          pick = hero.name;
          onHero?.(hero);
        });
        row.append(card);
      }
      el.append(row);
    } else if (stage === "podium") {
      title.textContent = "Preparing the battlefield";
      const wrap = document.createElement("div");
      wrap.className = "flow-rows";
      const columns = podiumColumns();
      columns.forEach((members, id) => {
        const rowEl = Object.assign(document.createElement("div"), { className: "flow-row" });
        rowEl.dataset.team = String(id);
        rowEl.append(Object.assign(document.createElement("div"), { textContent: teamNames[id] ?? "", className: "flow-row-title", style: "font-weight:600;opacity:.75" }));
        for (const member of members) {
          const chip = document.createElement("div");
          chip.className = "flow-pick";
          chip.textContent = member;
          chip.dataset.hero = member;
          rowEl.append(chip);
        }
        if (!members.length) rowEl.append(Object.assign(document.createElement("div"), { textContent: "(none)", className: "flow-pick", style: "opacity:.4" }));
        wrap.append(rowEl);
      });
      el.append(wrap);
      const wait = document.createElement("div");
      wait.className = "flow-wait";
      wait.textContent = "Standing by for the match…";
      el.append(wait);
    }
  };

  setStage("team");

  return {
    stage: () => stage,
    team: () => team,
    pick: () => pick,
    setStage,
    hide: () => setStage(null),
    setPicks: (byTeam) => { picks = byTeam; setStage("podium"); },
    teams: () => teamNames,
  };
}

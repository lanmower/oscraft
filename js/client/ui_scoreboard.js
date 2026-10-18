// Scoreboard, toggled by the scoreboard key (Tab, from moba_ui.json) through the moba:action event. One row per player:
// hero level, kills, deaths, assists and gold, read from Game.post_game_json and the player's hero.
import UI from "./moba_ui.json" with { type: "json" };

const STYLE = `
#ui-score{position:fixed;left:50%;top:50%;transform:translate(-50%,-50%);min-width:420px;background:rgba(12,14,18,.94);border:1px solid #444;color:#eee;font:13px/1.4 monospace;padding:10px 14px;pointer-events:none}
#ui-score h4{margin:0 0 6px;color:#ffd54f;font-size:14px}
#ui-score table{border-collapse:collapse;width:100%}
#ui-score th,#ui-score td{padding:2px 10px;text-align:right;white-space:nowrap}
#ui-score th:first-child,#ui-score td:first-child{text-align:left}
#ui-score .team0{color:#5b9bff}#ui-score .team1{color:#ff6b5b}
`;

function ensureStyle() {
  if (document.getElementById("ui-score-style")) return;
  const style = document.createElement("style");
  style.id = "ui-score-style";
  style.textContent = STYLE;
  document.head.appendChild(style);
}

export function createScoreboard({ game, units, localPlayer, unitName, isHero, teams }) {
  ensureStyle();
  const root = document.createElement("div");
  root.id = "ui-score";
  root.dataset.ui = "scoreboard";
  root.hidden = true;
  const title = document.createElement("h4");
  title.textContent = UI.text.scoreboardTitle;
  const table = document.createElement("table");
  const head = document.createElement("tr");
  for (const h of UI.text.scoreboardHeaders) {
    const th = document.createElement("th");
    th.textContent = h;
    head.appendChild(th);
  }
  table.appendChild(head);
  root.append(title, table);
  document.body.appendChild(root);

  window.addEventListener("moba:action", (e) => {
    if (e.detail && e.detail.action === "scoreboard") root.hidden = !root.hidden;
  });

  function heroOf(player) {
    return units().find((u) => u.player === player && isHero(u.id)) || null;
  }

  function update() {
    if (root.hidden) return;
    const report = JSON.parse(game.post_game_json());
    table.querySelectorAll("tr:not(:first-child)").forEach((r) => r.remove());
    for (const p of report.players) {
      const hero = heroOf(p.player);
      const tr = document.createElement("tr");
      tr.className = `team${teams[p.player] ?? 0}`;
      const label = hero ? unitName(hero.type) : UI.text.noValue;
      const level = hero ? game.character_level(hero.id) : UI.text.noValue;
      const mine = p.player === localPlayer() ? " *" : "";
      for (const cell of [`P${p.player}${mine}`, label, level, p.kills, p.deaths, p.assists, p.gold]) {
        const td = document.createElement("td");
        td.textContent = String(cell);
        tr.appendChild(td);
      }
      table.appendChild(tr);
    }
  }

  return { update, root };
}

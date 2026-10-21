// Scoreboard, toggled by the scoreboard key (Tab, from moba_ui.json) through the moba:action event. Two team blocks, each
// with a header row and one row per player: hero, level, kills, deaths, assists and gold, read from Game.post_game_json
// and the player's hero. The local player's row is highlighted.
import { UI } from "./pack_load.js";

const STYLE = `
#ui-score{position:fixed;z-index:3;left:50%;top:50%;transform:translate(-50%,-50%);min-width:520px;max-width:90vw;box-sizing:border-box;background:var(--panel-strong);border:1px solid var(--edge);border-radius:6px;color:var(--text);font:var(--t-body)/1.3 var(--font);padding:12px 16px;pointer-events:none}
#ui-score h4{margin:0 0 8px;color:var(--accent);font-size:var(--t-head)}
#ui-score .team{margin-top:8px}
#ui-score table{border-collapse:collapse;width:100%}
#ui-score th,#ui-score td{padding:4px 10px;text-align:right;white-space:nowrap;font-variant-numeric:tabular-nums}
#ui-score th{color:var(--muted);font-size:var(--t-label);font-weight:400;text-transform:uppercase;letter-spacing:.05em;border-bottom:1px solid var(--line)}
#ui-score th:first-child,#ui-score td:first-child{text-align:left}
#ui-score td:nth-child(2){text-align:left}
#ui-score tr.me td{background:var(--me-bg)}
#ui-score tr.team0 td:first-child{box-shadow:inset 3px 0 0 var(--team0)}
#ui-score tr.team1 td:first-child{box-shadow:inset 3px 0 0 var(--team1)}
#ui-score .team0 .tname{color:var(--team0)}#ui-score .team1 .tname{color:var(--team1)}
#ui-score .tname{font-weight:700;text-transform:uppercase;letter-spacing:.06em;font-size:var(--t-label);padding:0 10px}
`;

function ensureStyle() {
  if (document.getElementById("ui-score-style")) return;
  const style = document.createElement("style");
  style.id = "ui-score-style";
  style.textContent = STYLE;
  document.head.appendChild(style);
}

// Player rows grouped by team, teams in order of their first player.
export function groupByTeam(players, teams) {
  const byTeam = new Map();
  for (const p of players) {
    const side = teams[p.player] ?? 0;
    if (!byTeam.has(side)) byTeam.set(side, []);
    byTeam.get(side).push(p);
  }
  return [...byTeam.entries()].sort((a, b) => a[0] - b[0]);
}

export function createScoreboard({ game, units, localPlayer, unitName, isHero, teams }) {
  hot.set("ui-score", { game, units, localPlayer, unitName, isHero, teams });
  ensureStyle();
  const root = document.createElement("div");
  root.id = "ui-score";
  root.dataset.ui = "scoreboard";
  root.hidden = true;
  const title = document.createElement("h4");
  title.textContent = UI.text.scoreboardTitle;
  root.append(title);
  document.body.appendChild(root);

  window.addEventListener("moba:action", (e) => {
    if (e.detail && e.detail.action === "scoreboard") root.hidden = !root.hidden;
  });

  function heroOf(player) {
    return units().find((u) => u.player === player && isHero(u.id)) || null;
  }

  function teamTable(side, rows) {
    const block = document.createElement("div");
    block.className = "team";
    const name = document.createElement("div");
    name.className = `tname team${side}`;
    name.textContent = `Team ${side}`;
    const table = document.createElement("table");
    const head = document.createElement("tr");
    for (const h of UI.text.scoreboardHeaders) {
      const th = document.createElement("th");
      th.textContent = h;
      head.appendChild(th);
    }
    table.appendChild(head);
    for (const p of rows) {
      const hero = heroOf(p.player);
      const tr = document.createElement("tr");
      tr.className = `team${side}${p.player === localPlayer() ? " me" : ""}`;
      const label = hero ? unitName(hero.type) : UI.text.noValue;
      const level = hero ? game.character_level(hero.id) : UI.text.noValue;
      const who = `P${p.player}${p.player === localPlayer() ? " *" : ""}`;
      for (const cell of [who, label, level, p.kills, p.deaths, p.assists, p.gold]) {
        const td = document.createElement("td");
        td.textContent = String(cell);
        tr.appendChild(td);
      }
      table.appendChild(tr);
    }
    block.append(name, table);
    return block;
  }

  function update() {
    if (root.hidden) return;
    const report = JSON.parse(game.post_game_json());
    root.querySelectorAll(".team").forEach((n) => n.remove());
    for (const [side, rows] of groupByTeam(report.players, teams)) root.appendChild(teamTable(side, rows));
  }

  return { update, root };
}

// Hot-update hook (see hmr.js): replaces the #ui-score root with a fresh one built from the inputs of the last createScoreboard call
// (same id, same host). The new panel is the hook value for its owner; listeners of the old panel are not detached.
const hot = (globalThis.__oscraftHot ??= new Map());
export function hmrReload() {
  const args = hot.get("ui-score");
  if (!args) return null;
  document.getElementById("ui-score")?.remove();
  return createScoreboard(args);
}

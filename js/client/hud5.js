// 5v5 HUD strip: team scoreboard rows, team gold and the winner banner. Every value comes from the wasm Game
// (unit list, unit types, hp, gold, winner). The ability bar and the Tab scoreboard live in ui_selection.js and
// ui_scoreboard.js. Kills and deaths are filled from the sim exports once they land; until then they are not drawn.

export const SCOREBOARD_ROWS = 10;

const STYLE = `
#hud5{position:fixed;left:0;right:0;bottom:0;pointer-events:none;font:12px/1.3 monospace;color:#eee;display:flex;justify-content:space-between;align-items:flex-end;padding:0 8px 8px}
#hud5 .panel{background:rgba(12,14,18,.82);border:1px solid #333;padding:6px 8px;min-width:200px}
#hud5 table{border-collapse:collapse;width:100%}
#hud5 td,#hud5 th{padding:1px 6px;text-align:left;white-space:nowrap}
#hud5 .team0{color:#5b9bff}#hud5 .team1{color:#ff6b5b}
#hud5 .banner{position:fixed;top:12px;left:50%;transform:translateX(-50%);font:bold 22px monospace;color:#ffd54f;pointer-events:none}
`;

function ensureStyle() {
  if (document.getElementById("hud5-style")) return;
  const style = document.createElement("style");
  style.id = "hud5-style";
  style.textContent = STYLE;
  document.head.appendChild(style);
}

function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

// Scoreboard rows from the living units, ordered by team then unit id. The sim reports living units only, so a
// fallen unit leaves the list and its row is dropped; a row never shows a value the sim did not report.
export function scoreboardRows(list, typeName) {
  return [...list]
    .sort((a, b) => a.player - b.player || a.id - b.id)
    .slice(0, SCOREBOARD_ROWS)
    .map((u) => ({ id: u.id, name: typeName(u.type), team: u.player, state: "alive", hp: u.hp }));
}

// Team gold totals: each side's resource pool, read from Game.gold for every side that owns a living unit.
export function teamGold(game, list) {
  const sides = [...new Set(list.map((u) => u.player))].sort((a, b) => a - b);
  return sides.map((side) => ({ side, gold: game.gold(side) }));
}

// Winner banner data: null while the match runs, else the winning side.
export function winnerOf(game) {
  const side = game.winner();
  return side < 0 ? null : { side };
}

export function createHud5({ game, units, selected, localPlayer }) {
  ensureStyle();
  const root = el("div");
  root.id = "hud5";
  const left = el("div", "panel");
  const scoreTable = el("table");
  const scoreHead = el("tr");
  for (const h of ["hero", "team", "state", "hp"]) scoreHead.appendChild(el("th", "", h));
  scoreTable.appendChild(scoreHead);
  const goldLine = el("div");
  left.append(scoreTable, goldLine);
  root.append(left);
  document.body.appendChild(root);
  const banner = el("div", "banner");
  banner.hidden = true;
  document.body.appendChild(banner);

  const typeNames = typeof game.unit_model_names === "function" ? game.unit_model_names() : [];
  const typeName = (type) => (type >= 0 && typeNames[type]) || `type ${type}`;
  let rows = [];

  function drawScoreboard(list) {
    const next = scoreboardRows(list, typeName);
    const signature = next.map((r) => `${r.id}:${r.team}:${r.hp}`).join("|");
    if (signature === rows.signature) return;
    rows = next;
    rows.signature = signature;
    while (scoreTable.rows.length > 1) scoreTable.deleteRow(1);
    for (const r of next) {
      const tr = el("tr", `team${r.team % 2}`);
      tr.append(el("td", "", r.name), el("td", "", String(r.team)), el("td", "", r.state), el("td", "", String(r.hp)));
      scoreTable.appendChild(tr);
    }
  }

  function drawGold(list) {
    goldLine.textContent = teamGold(game, list).map((t) => `team ${t.side} gold ${t.gold}`).join("   ");
  }

  function drawWinner() {
    const win = winnerOf(game);
    banner.hidden = win === null;
    banner.textContent = win ? `side ${win.side} wins` : "";
    return win;
  }

  function update() {
    const list = units();
    drawScoreboard(list);
    drawGold(list);
    const win = drawWinner();
    return { rows: rows.length, winner: win ? win.side : -1 };
  }

  return { update, root };
}

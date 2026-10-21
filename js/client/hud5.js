// 5v5 HUD strip: team scoreboard rows, team gold and the winner banner. Every value comes from the wasm Game
// (unit list, unit types, hp, gold, winner). The ability bar and the Tab scoreboard live in ui_selection.js and
// ui_scoreboard.js. The top strip lists heroes only: folded it shows each side's kills and one dot per hero
// (green alive, red dead); a click on the panel expands the hero rows.

import UI from "./moba_ui.json" with { type: "json" };

export const SCOREBOARD_ROWS = 10;

const STYLE = `
#hud5{position:fixed;z-index:2;top:8px;left:50%;transform:translateX(-50%);pointer-events:none;font:var(--t-label)/1.25 var(--font);color:var(--text);width:min(440px,calc(100vw - 600px));min-width:260px;box-sizing:border-box}
#hud5 .panel{background:var(--panel);border:1px solid var(--edge);border-radius:6px;padding:6px 8px;max-height:calc(100vh - 220px);overflow:hidden;pointer-events:auto;cursor:pointer}
#hud5 .sum{display:flex;align-items:center;gap:10px;font-variant-numeric:tabular-nums;white-space:nowrap}
#hud5 .sum .tname{min-width:56px;font-weight:700;text-transform:uppercase;letter-spacing:.06em}
#hud5 .dot{display:inline-block;width:9px;height:9px;border-radius:50%;margin-right:4px}
#hud5 .dot.alive{background:#3ccf5a}#hud5 .dot.dead{background:#e5483c}
#hud5 table[hidden]{display:none}
#hud5 table{border-collapse:collapse;width:100%}
#hud5 td,#hud5 th{padding:2px 6px;text-align:left;white-space:nowrap}
#hud5 th{color:var(--muted);font-weight:400;text-transform:uppercase;letter-spacing:.05em;border-bottom:1px solid var(--line)}
#hud5 .team0{color:var(--team0)}#hud5 .team1{color:var(--team1)}
#hud5 .gold{margin-top:4px;color:var(--gold);font-variant-numeric:tabular-nums}
#hud5 .banner{position:fixed;z-index:3;top:14%;left:50%;transform:translateX(-50%);font:700 var(--t-head) var(--font);color:var(--accent);pointer-events:none;background:var(--panel-strong);border-radius:6px;padding:8px 22px}
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

// Hero rows only, one per player slot (the slot's first hero, as heroOf in play.js). The sim lists living units only,
// so a slot's hero stays in `roster` and turns "dead" when no hero of that slot is listed; keying by slot lets a
// new match's hero replace the pre-match hero of the same slot. Hp is the last value the sim reported.
export function scoreboardRows(list, typeName, isHero, roster) {
  const living = new Map();
  for (const u of list) if (!living.has(u.player) && isHero(u.id)) living.set(u.player, u);
  for (const [player, u] of living) {
    roster.set(player, { id: u.id, player, name: typeName(u.type), team: UI.teams[player] ?? 0, state: "alive", hp: u.hp });
  }
  for (const row of roster.values()) if (!living.has(row.player)) row.state = "dead";
  return [...roster.values()].sort((a, b) => a.team - b.team || a.player - b.player);
}

// Kill totals per side from the sim's post-game report (the same kills the Tab scoreboard shows).
export function teamKills(game) {
  const totals = new Map();
  const report = JSON.parse(game.post_game_json());
  for (const p of report.players) {
    const side = UI.teams[p.player] ?? 0;
    totals.set(side, (totals.get(side) || 0) + (p.kills || 0));
  }
  return totals;
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

export function createHud5({ game, units, selected, localPlayer, isHero, heroUnits }) {
  ensureStyle();
  const root = el("div");
  root.id = "hud5";
  const left = el("div", "panel");
  // Folded by default: one kills line per team with a dot per hero. A click on the panel expands the hero rows.
  const summary = el("div", "summary");
  const scoreTable = el("table");
  const scoreHead = el("tr");
  for (const h of ["hero", "team", "state", "hp"]) scoreHead.appendChild(el("th", "", h));
  scoreTable.appendChild(scoreHead);
  scoreTable.hidden = true;
  const goldLine = el("div", "gold");
  left.append(summary, scoreTable, goldLine);
  root.append(left);
  left.addEventListener("click", () => {
    scoreTable.hidden = !scoreTable.hidden;
  });
  (document.getElementById("ui-top") || document.body).appendChild(root);
  const banner = el("div", "banner");
  banner.hidden = true;
  document.body.appendChild(banner);

  const typeNames = typeof game.unit_model_names === "function" ? game.unit_model_names() : [];
  const typeName = (type) => (type >= 0 && typeNames[type]) || `type ${type}`;
  const roster = new Map();
  // The horn starts the match: heroes listed before it belong to the pre-match world, so the roster starts over.
  window.addEventListener("oscraft:horn", () => roster.clear());
  const sides = [...new Set(UI.teams)].sort((a, b) => a - b);
  let kills = new Map();
  let lastSignature = null;
  let tick = 0;

  function sumLine(side, rows) {
    const line = el("div", `sum team${side % 2}`);
    line.append(el("span", "tname", `Team ${side}`), el("span", "", `kills ${kills.get(side) || 0}`));
    for (const r of rows) {
      if (r.team !== side) continue;
      const dot = el("span", `dot ${r.state}`);
      dot.title = `${r.name} ${r.state}`;
      line.appendChild(dot);
    }
    return line;
  }

  function drawScoreboard(list) {
    // Hero liveness reads the unfiltered sim list: the fog-filtered `units` list drops heroes out of sight.
    const rows = scoreboardRows(heroUnits ? heroUnits() : list, typeName, isHero, roster);
    if (tick++ % 30 === 0 && typeof game.post_game_json === "function") kills = teamKills(game);
    const signature = JSON.stringify([
      rows.map((r) => [r.id, r.team, r.state, r.hp]),
      sides.map((s) => [s, kills.get(s) || 0]),
    ]);
    if (signature === lastSignature) return;
    lastSignature = signature;
    summary.replaceChildren(...sides.map((side) => sumLine(side, rows)));
    while (scoreTable.rows.length > 1) scoreTable.deleteRow(1);
    for (const r of rows) {
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
    return { rows: roster.size, winner: win ? win.side : -1 };
  }

  return { update, root };
}

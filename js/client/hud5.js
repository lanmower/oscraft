// 5v5 Dota-style HUD: scoreboard, team gold, winner banner and the ability bar. Every value comes from the
// wasm Game (unit list, unit types, hp, gold, winner) or from the local selection. Kills, deaths, the kill feed
// and ability cooldowns are filled from the sim exports once they land; until then they are not drawn.

export const ABILITY_KEYS = ["q", "w", "e", "r", "t"];
export const SCOREBOARD_ROWS = 10;

const STYLE = `
#hud5{position:fixed;left:0;right:0;bottom:0;pointer-events:none;font:12px/1.3 monospace;color:#eee;display:flex;justify-content:space-between;align-items:flex-end;padding:0 8px 8px}
#hud5 .panel{background:rgba(12,14,18,.82);border:1px solid #333;padding:6px 8px;min-width:200px}
#hud5 table{border-collapse:collapse;width:100%}
#hud5 td,#hud5 th{padding:1px 6px;text-align:left;white-space:nowrap}
#hud5 .team0{color:#5b9bff}#hud5 .team1{color:#ff6b5b}
#hud5 .abilities{display:flex;gap:4px}
#hud5 .slot{width:48px;height:48px;border:1px solid #555;background:#1b1f26;display:flex;align-items:flex-end;justify-content:center;position:relative}
#hud5 .slot b{position:absolute;top:2px;left:4px;font-size:11px;color:#ffd54f}
#hud5 .slot span{font-size:10px;color:#aaa;padding-bottom:2px}
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
  const bar = el("div", "abilities");
  const slots = ABILITY_KEYS.map((key) => {
    const slot = el("div", "slot");
    slot.append(el("b", "", key.toUpperCase()), el("span", "", "-"));
    bar.appendChild(slot);
    return slot;
  });
  root.append(left, bar);
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

  function drawAbilities() {
    const id = selected();
    const mine = id >= 0 ? units().find((u) => u.id === id && u.player === localPlayer()) : null;
    slots.forEach((slot, i) => {
      slot.querySelector("span").textContent = mine ? `slot ${i + 1}` : "-";
    });
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
    drawAbilities();
    const win = drawWinner();
    return { rows: rows.length, winner: win ? win.side : -1 };
  }

  return { update, root };
}

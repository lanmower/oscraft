// Skill-up panel for the selected hero: one row per held ability with its rank pips, the level gate of the next
// rank and a + button that calls Game.skill_up. Tooltips come from Game.abilities_json (ticks are 100 ms).

const TICK_S = 0.1;

const STYLE = `
#ui-skills{position:fixed;left:8px;top:48px;width:230px;background:rgba(12,14,18,.9);border:1px solid #333;color:#eee;font:12px/1.3 monospace;padding:6px 8px}
#ui-skills h4{margin:0 0 4px;font-size:12px;color:#ffd54f}
#ui-skills .row{display:flex;justify-content:space-between;align-items:center;gap:6px;border-top:1px solid #222;padding:3px 0}
#ui-skills .gate{color:#aaa;font-size:11px}
#ui-skills .pip{color:#ffd54f}
#ui-skills button{background:#243a24;color:#eee;border:1px solid #4a7;cursor:pointer}
#ui-skills button:disabled{background:#222;color:#666;border-color:#333;cursor:default}
`;

function ensureStyle() {
  if (document.getElementById("ui-skills-style")) return;
  const style = document.createElement("style");
  style.id = "ui-skills-style";
  style.textContent = STYLE;
  document.head.appendChild(style);
}

function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

const seconds = (ticks) => `${(ticks * TICK_S).toFixed(1)}s`;

// Tooltip text for one ability at `rank` (0 = unlearned, shows rank 1 values).
export function abilityTooltip(a, rank) {
  const at = Math.max(rank, 1) - 1;
  const cooldown = a.max_rank ? a.cooldowns[at] : a.cooldown;
  const energy = a.max_rank ? a.energies[at] : a.energy;
  const lines = [a.name + (a.ultimate ? " (ultimate)" : ""), `cooldown ${seconds(cooldown)}  energy ${energy}`];
  if (a.max_rank) {
    lines.push(`damage by rank: ${a.damage.join(" / ")}`, `level gates: ${a.gates.join(" / ")}`);
  }
  return lines.join("\n");
}

export function createSkills({ game, selected, localPlayer }) {
  ensureStyle();
  const root = el("div");
  root.id = "ui-skills";
  root.hidden = true;
  const head = el("h4");
  const list = el("div");
  root.append(head, list);
  document.body.appendChild(root);
  let signature = "";

  function update() {
    const id = selected();
    const abilities = id >= 0 ? JSON.parse(game.abilities_json(id)) : [];
    root.hidden = abilities.length === 0;
    if (root.hidden) return;
    const level = game.character_level(id);
    const points = game.skill_points(id);
    const ranks = abilities.map((a) => game.skill_rank(id, a.slot));
    const next = `${id}|${level}|${points}|${ranks.join(",")}`;
    if (next === signature) return;
    signature = next;
    head.textContent = `Skills  level ${level}  points ${points}`;
    list.replaceChildren();
    abilities.forEach((a, i) => {
      const rank = ranks[i];
      const row = el("div", "row");
      row.title = abilityTooltip(a, rank);
      const info = el("div");
      const pips = a.max_rank ? "●".repeat(rank) + "○".repeat(a.max_rank - rank) : "";
      const gate = a.max_rank && rank < a.max_rank ? `next rank needs level ${a.gates[rank]}` : a.max_rank ? "maxed" : "no skill tree";
      info.append(el("div", "", a.name), el("span", "pip", pips), el("div", "gate", gate));
      const up = el("button", "", "+");
      up.disabled = !a.max_rank || rank >= a.max_rank || points === 0 || level < a.gates[rank];
      up.onclick = () => { game.skill_up(id, a.slot); signature = ""; };
      row.append(info, up);
      list.appendChild(row);
    });
  }

  return { update, root };
}

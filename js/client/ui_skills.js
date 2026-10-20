// Skill-up panel for the selected hero, in the top-left column: a header with level and points, one row per held ability
// (rank pips, the level gate of the next rank, and a + button that calls Game.skill_up), then the talent tiers. Tooltips
// come from Game.abilities_json (ticks are 62.5 ms, 16 per second).

const TICK_S = 1 / 16;
const TALENT_LEVELS = [10, 15, 20, 25];

const STYLE = `
#ui-skills{width:240px;box-sizing:border-box;background:var(--panel);border:1px solid var(--edge);border-radius:6px;color:var(--text);font:var(--t-body)/1.3 var(--font);padding:8px 10px;display:flex;flex-direction:column;gap:2px}
#ui-skills h4{margin:0 0 4px;font-size:var(--t-title);color:var(--accent);display:flex;justify-content:space-between;align-items:baseline}
#ui-skills h4 span{font-size:var(--t-label);color:var(--muted);font-weight:400}
#ui-skills .sec{color:var(--faint);font-size:var(--t-label);text-transform:uppercase;letter-spacing:.06em;margin-top:6px}
#ui-skills .row{position:relative;display:flex;justify-content:space-between;align-items:center;gap:6px;border-top:1px solid var(--line);padding:4px 4px;border-radius:2px}
#ui-skills .cd{position:absolute;inset:0;pointer-events:none;border-radius:2px;background:none}
#ui-skills .row:hover{background:var(--well-hi)}
#ui-skills .row[data-state=maxed] .pip{color:var(--accent)}
#ui-skills .row[data-state=locked]{color:var(--muted)}
#ui-skills .row .name{font-weight:700}
#ui-skills .gate{color:var(--muted);font-size:var(--t-label)}
#ui-skills .pip{color:var(--accent);letter-spacing:1px;font-size:var(--t-label)}
#ui-skills button{background:var(--ok-bg);color:var(--text);border:1px solid var(--ok);border-radius:3px;min-width:30px;padding:2px 6px;font:700 var(--t-label) var(--font);cursor:pointer}
#ui-skills button:hover:not(:disabled){background:var(--ok)}
#ui-skills button:disabled{background:var(--well-dim);color:var(--faint);border-color:var(--edge);cursor:default}
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

// Row state: maxed (top rank), locked (the next rank needs a higher level), ready (a rank can be bought now), or
// none (an ability with no skill tree).
export function skillRowState({ a, rank, level, points }) {
  if (!a.max_rank) return "none";
  if (rank >= a.max_rank) return "maxed";
  if (points === 0 || level < a.gates[rank]) return "locked";
  return "ready";
}

// Cooldown sweep: each ability row gets one overlay whose conic-gradient covers the share of its cooldown still to run.
// Needs `input` (the controls layer, for cooldownLeft/cooldownTotal); without it the rows show no sweep.
export function createSkills({ game, selected, localPlayer, input }) {
  ensureStyle();
  const root = el("div");
  root.id = "ui-skills";
  root.hidden = true;
  const head = el("h4");
  const list = el("div");
  root.append(head, list);
  const host = document.getElementById("ui-left") || document.body;
  host.appendChild(root);
  let signature = "";
  let sweeps = [];

  function tickSweeps(id) {
    if (!input) return;
    for (const s of sweeps) {
      const left = input.cooldownLeft(id, s.slot);
      if (left === s.last) continue;
      s.last = left;
      const total = input.cooldownTotal(id, s.slot) || 1;
      s.node.style.background = left > 0 ? `conic-gradient(rgba(0,0,0,.55) ${Math.round((360 * left) / total)}deg, transparent 0)` : "none";
    }
  }

  function update() {
    const id = selected();
    const abilities = id >= 0 ? JSON.parse(game.abilities_json(id)) : [];
    root.hidden = abilities.length === 0;
    if (root.hidden) return;
    tickSweeps(id);
    const level = game.character_level(id);
    const points = game.skill_points(id);
    const ranks = abilities.map((a) => game.skill_rank(id, a.slot));
    const bits = game.upgrade_bits(id);
    const next = `${id}|${level}|${points}|${ranks.join(",")}|${bits}`;
    if (next === signature) return;
    signature = next;
    head.replaceChildren(el("span", "", "Skills"), el("span", "", `level ${level}  points ${points}`));
    list.replaceChildren();
    list.appendChild(el("div", "sec", "Abilities"));
    sweeps = [];
    abilities.forEach((a, i) => {
      const rank = ranks[i];
      const state = skillRowState({ a, rank, level, points });
      const row = el("div", "row");
      row.dataset.state = state;
      row.title = abilityTooltip(a, rank);
      const info = el("div");
      const pips = a.max_rank ? "●".repeat(rank) + "○".repeat(a.max_rank - rank) : "";
      const gate = a.max_rank && rank < a.max_rank ? `next rank needs level ${a.gates[rank]}` : a.max_rank ? "maxed" : "no skill tree";
      info.append(el("div", "name", a.name), el("span", "pip", pips), el("div", "gate", gate));
      const up = el("button", "", "+");
      up.disabled = state !== "ready";
      up.onclick = () => { game.skill_up(id, a.slot); signature = ""; };
      const sweep = el("div", "cd");
      sweeps.push({ slot: a.slot, node: sweep, last: -1 });
      row.append(info, up, sweep);
      list.appendChild(row);
    });
    tickSweeps(id);
    list.appendChild(el("div", "sec", "Talents"));
    TALENT_LEVELS.forEach((need, tier) => {
      const row = el("div", "row");
      row.dataset.state = level < need ? "locked" : "ready";
      const picked = (bits >> (2 + 2 * tier)) & 3;
      row.appendChild(el("div", "gate", `level ${need}`));
      for (const side of [0, 1]) {
        const pick = el("button", "", picked & (1 << side) ? "[x]" : "+");
        pick.disabled = picked !== 0 || level < need;
        pick.onclick = () => { game.choose_talent(id, tier, side); signature = ""; };
        row.appendChild(pick);
      }
      list.appendChild(row);
    });
  }

  return { update, root };
}

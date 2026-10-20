// Selection panel, the left block of the bottom bar: the primary selected unit's portrait, name, health and mana bars and
// an ability row. Each ability slot shows its key, name, mana cost, rank pips and a cooldown overlay, and carries a
// data-state (ready, cooldown, unlearned, empty) that drives its look. With several units selected, a grid of tiles lists
// the group; a click on one selects it alone. Labels come from moba_ui.json; keys come from the same file.
import UI from "./moba_ui.json" with { type: "json" };
import { cooldownTicks } from "./moba_select.js";

const TICK_S = 1 / 16;
const GROUP_TILES = 12;

const STYLE = `
#ui-select{display:grid;grid-template-columns:auto minmax(0,1fr);grid-template-rows:auto auto;gap:8px 12px;align-items:start;height:100%;box-sizing:border-box;color:var(--text);font:var(--t-body)/1.3 var(--font)}
#ui-select .head{display:flex;gap:10px;align-items:center;min-width:0}
#ui-select .face{width:64px;height:64px;flex:none;background:var(--well);border:1px solid var(--edge);border-radius:4px;display:flex;flex-direction:column;align-items:center;justify-content:center;color:var(--accent);font-weight:700;font-size:var(--t-title)}
#ui-select .info{min-width:0;display:flex;flex-direction:column;gap:3px;width:200px}
#ui-select .name{font-weight:700;color:var(--accent);font-size:var(--t-title);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
#ui-select .stat{color:var(--muted);font-size:var(--t-label);font-variant-numeric:tabular-nums}
#ui-select .bar{height:8px;background:var(--well);border-radius:2px;overflow:hidden}
#ui-select .bar i{display:block;height:100%}
#ui-select .hp i{background:var(--hp)}#ui-select .mp i{background:var(--mp)}
#ui-select .abil{display:flex;gap:6px;grid-column:1/-1}
#ui-select .slot{width:66px;height:66px;flex:none;border:1px solid var(--edge);border-radius:4px;background:var(--well);position:relative;overflow:hidden;box-sizing:border-box;font-size:var(--t-label);line-height:1.1}
#ui-select .slot b{position:absolute;top:3px;left:5px;color:var(--accent);font-size:var(--t-label);font-weight:700}
#ui-select .slot .label{position:absolute;left:4px;right:4px;top:50%;transform:translateY(-50%);text-align:center;color:var(--text);display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden;padding-top:6px}
#ui-select .slot .cost{position:absolute;top:3px;right:5px;color:#8fb4ff;font-weight:700;font-variant-numeric:tabular-nums}
#ui-select .slot .pips{position:absolute;bottom:3px;left:5px;color:var(--muted);font-size:8px;letter-spacing:1px}
#ui-select .slot .cd{position:absolute;inset:0;background:var(--shade);display:flex;align-items:center;justify-content:center;color:var(--text);font-size:var(--t-title);font-weight:700;font-variant-numeric:tabular-nums}
#ui-select .slot .up{position:absolute;bottom:2px;right:2px;background:var(--ok-bg);border:1px solid var(--ok);color:var(--text);font:700 var(--t-label)/1 var(--font);padding:2px 5px;border-radius:3px;cursor:pointer}
#ui-select .slot[data-state=unlearned]{border-style:dashed;color:var(--muted)}
#ui-select .slot[data-state=unlearned] .label{color:var(--muted)}
#ui-select .slot[data-state=cooldown]{border-color:var(--edge)}
#ui-select .slot[data-state=empty]{opacity:.45}
#ui-select .slot:hover{border-color:var(--hover-edge);background:var(--well-hi)}
#ui-select .slot[data-state=cooldown]:hover{background:var(--well)}
#ui-select .group{display:grid;grid-template-columns:repeat(6,40px);gap:4px;align-content:start}
#ui-select .tile{height:40px;border:1px solid var(--edge);border-radius:3px;background:var(--well);position:relative;font-size:var(--t-label);text-align:center;cursor:pointer;display:flex;align-items:center;justify-content:center;color:var(--muted);overflow:hidden}
#ui-select .tile:hover{border-color:var(--hover-edge);color:var(--text)}
#ui-select .tile.active{border-color:var(--sel);color:var(--text);box-shadow:inset 0 0 0 1px var(--sel)}
#ui-select .tile i{position:absolute;left:0;bottom:0;height:3px;background:var(--hp)}
#ui-select[data-state=empty] .face{color:var(--faint);border-style:dashed}
#ui-select[data-state=empty] .name{color:var(--muted);font-weight:400}
`;

function ensureStyle() {
  if (document.getElementById("ui-select-style")) return;
  const style = document.createElement("style");
  style.id = "ui-select-style";
  style.textContent = STYLE;
  document.head.appendChild(style);
}

function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function barEl(kind) {
  const outer = el("div", `bar ${kind}`);
  const fill = el("i");
  outer.appendChild(fill);
  return { outer, fill };
}

// Mana cost of an ability at its current rank: ranked abilities carry one cost per rank.
export function abilityCost(ability, rank) {
  if (ability.max_rank) return ability.energies[Math.max(rank, 1) - 1];
  return ability.energy;
}

// Visual state of one slot. Pure, so the state rules can be read without the DOM.
export function slotState({ ability, rank, cooldownLeft }) {
  if (!ability) return "empty";
  if (cooldownLeft > 0) return "cooldown";
  if (rank === 0) return "unlearned";
  return "ready";
}

export function createSelectionPanel({ game, input, units, localPlayer, unitName, isHero }) {
  ensureStyle();
  const root = el("div");
  root.id = "ui-select";
  root.dataset.ui = "selection";
  const face = el("div", "face");
  const name = el("div", "name");
  const hpText = el("div", "stat");
  const hp = barEl("hp");
  const mpText = el("div", "stat");
  const mp = barEl("mp");
  const info = el("div", "info");
  info.append(name, hpText, hp.outer, mpText, mp.outer);
  const head = el("div", "head");
  head.append(face, info);
  const abil = el("div", "abil");
  const group = el("div", "group");
  root.append(head, group, abil);
  const host = document.getElementById("ui-bar-left") || document.body;
  host.appendChild(root);

  const peakHp = new Map();
  const abilityCache = new Map();
  const slotKeys = Object.entries(UI.keys.cast).sort((a, b) => a[1] - b[1]);
  const slots = slotKeys.map(([key]) => {
    const slot = el("div", "slot");
    slot.append(el("b", "", key.toUpperCase()));
    abil.appendChild(slot);
    return slot;
  });

  function abilitiesOf(id) {
    if (!abilityCache.has(id)) abilityCache.set(id, JSON.parse(game.abilities_json(id)));
    return abilityCache.get(id);
  }

  function drawAbilities(id) {
    const list = abilitiesOf(id);
    slotKeys.forEach(([, slotIndex], i) => {
      const slot = slots[i];
      const ability = list.find((a) => a.slot === slotIndex);
      slot.querySelectorAll(".cd, .pips, .up, .label, .cost").forEach((n) => n.remove());
      const rank = ability ? game.skill_rank(id, slotIndex) : 0;
      const left = ability ? input.cooldownLeft(id, slotIndex) : 0;
      slot.dataset.state = slotState({ ability, rank, cooldownLeft: left });
      if (!ability) {
        slot.append(el("span", "label", UI.text.noValue));
        return;
      }
      slot.title = `${ability.name}  cost ${abilityCost(ability, rank)} mana`;
      slot.append(el("span", "label", ability.name));
      slot.append(el("span", "cost", String(abilityCost(ability, rank))));
      if (ability.max_rank) slot.append(el("div", "pips", "●".repeat(rank) + "○".repeat(Math.max(0, ability.max_rank - rank))));
      if (left > 0) {
        const total = input.cooldownTotal(id, slotIndex) || cooldownTicks(ability, rank) || 1;
        slot.append(el("div", "cd", `${(left * TICK_S).toFixed(1)}${UI.text.cooldownSuffix}`));
        slot.dataset.cooldownPct = String(Math.round((100 * left) / total));
      } else {
        delete slot.dataset.cooldownPct;
      }
      if (game.skill_points(id) > 0 && rank < (ability.max_rank || 0) && game.character_level(id) >= (ability.gates ? ability.gates[rank] : 0)) {
        const up = el("button", "up", "+");
        up.onclick = () => input.skillUp(slotIndex);
        slot.appendChild(up);
      }
    });
  }

  function drawGroup(list, active) {
    group.replaceChildren();
    if (list.length < 2) return;
    const byId = new Map(units().map((u) => [u.id, u]));
    for (const id of list.slice(0, GROUP_TILES)) {
      const u = byId.get(id);
      const tile = el("div", id === active ? "tile active" : "tile", u ? unitName(u.type).slice(0, 4) : UI.text.noValue);
      if (u) {
        const max = peakHp.get(id) || u.hp || 1;
        const bar = el("i");
        bar.style.width = `${Math.max(0, Math.min(100, (100 * (u.hp || 0)) / max))}%`;
        tile.appendChild(bar);
      }
      tile.onclick = () => input.setSelection([id]);
      group.appendChild(tile);
    }
  }

  function update() {
    const sel = input.selection();
    if (sel.length === 0) {
      // Empty state: the frame stays drawn, with a dashed face and a muted label, so the bar never reads as a blank strip.
      root.dataset.state = "empty";
      face.textContent = UI.text.noValue;
      name.textContent = UI.text.noSelection;
      hpText.textContent = "";
      hp.outer.hidden = true;
      mpText.hidden = true;
      mp.outer.hidden = true;
      abil.hidden = true;
      group.replaceChildren();
      return;
    }
    delete root.dataset.state;
    const primary = sel[0];
    const u = units().find((x) => x.id === primary);
    const hpNow = typeof game.unit_hp === "function" ? game.unit_hp(primary) : -1;
    const peak = Math.max(peakHp.get(primary) || 0, hpNow);
    peakHp.set(primary, peak);
    face.textContent = u ? `L${isHero(primary) ? game.character_level(primary) : ""}` : UI.text.noValue;
    name.textContent = u ? unitName(u.type) : UI.text.unitUnknown;
    hpText.textContent = `${UI.text.healthLabel} ${hpNow >= 0 ? hpNow : UI.text.noValue}/${peak}`;
    hp.outer.hidden = false;
    hp.fill.style.width = `${peak > 0 ? (100 * Math.max(hpNow, 0)) / peak : 0}%`;
    const hero = isHero(primary);
    mpText.hidden = !hero;
    mp.outer.hidden = !hero;
    if (hero) {
      mpText.textContent = `${UI.text.manaLabel} ${game.max_mana(primary)}`;
      mp.fill.style.width = "100%";
    }
    abil.hidden = !(hero && u && u.player === localPlayer());
    if (!abil.hidden) drawAbilities(primary);
    drawGroup(sel, primary);
  }

  return { update, root };
}

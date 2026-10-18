// Selection panel: the primary selected unit's portrait, name, health and mana bars and an ability bar with cooldowns
// and skill-up buttons (own heroes). With several units selected, a row of portraits lists the group; a click on one
// selects it alone. Labels come from moba_ui.json; keys come from the same file (shown on the ability bar).
import UI from "./moba_ui.json" with { type: "json" };
import { cooldownTicks } from "./moba_select.js";

const TICK_S = 0.1;
const GROUP_TILES = 12;

const STYLE = `
#ui-select{position:fixed;left:50%;bottom:8px;transform:translateX(-50%);display:flex;gap:10px;align-items:flex-end;background:rgba(12,14,18,.9);border:1px solid #333;color:#eee;font:11px/1.3 monospace;padding:6px 8px}
#ui-select .face{width:56px;height:56px;background:#1b1f26;border:1px solid #555;display:flex;align-items:center;justify-content:center;font-size:22px;color:#ffd54f}
#ui-select .name{font-weight:bold;color:#ffd54f}
#ui-select .bar{height:9px;background:#222;margin-top:2px;width:170px}
#ui-select .bar i{display:block;height:100%}
#ui-select .hp i{background:#4caf50}#ui-select .mp i{background:#3d7bff}
#ui-select .abil{display:flex;gap:4px}
#ui-select .slot{width:50px;height:50px;border:1px solid #555;background:#1b1f26;position:relative;text-align:center;font-size:10px;overflow:hidden}
#ui-select .slot b{position:absolute;top:1px;left:3px;color:#ffd54f}
#ui-select .slot .cd{position:absolute;inset:0;background:rgba(0,0,0,.6);display:flex;align-items:center;justify-content:center;font-size:14px}
#ui-select .slot .pips{position:absolute;bottom:1px;left:0;right:0;color:#aaa;font-size:9px}
#ui-select .slot .up{position:absolute;top:1px;right:1px;background:#243a24;border:1px solid #4a7;color:#eee;font-size:10px;padding:0 3px;cursor:pointer}
#ui-select .group{display:grid;grid-template-columns:repeat(6,34px);gap:3px}
#ui-select .tile{height:34px;border:1px solid #555;background:#1b1f26;position:relative;font-size:10px;text-align:center;cursor:pointer}
#ui-select .tile.active{border-color:#fff}
#ui-select .tile i{position:absolute;left:0;bottom:0;height:3px;background:#4caf50}
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

export function createSelectionPanel({ game, input, units, localPlayer, unitName, isHero }) {
  ensureStyle();
  const root = el("div");
  root.id = "ui-select";
  root.dataset.ui = "selection";
  root.hidden = true;
  const face = el("div", "face");
  const name = el("div", "name");
  const hpText = el("div");
  const hp = barEl("hp");
  const mpText = el("div");
  const mp = barEl("mp");
  const info = el("div");
  info.append(name, hpText, hp.outer, mpText, mp.outer);
  const abil = el("div", "abil");
  const group = el("div", "group");
  root.append(face, info, abil, group);
  document.body.appendChild(root);

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
      slot.querySelectorAll(".cd, .pips, .up, .label").forEach((n) => n.remove());
      if (!ability) {
        slot.append(el("span", "label", UI.text.noValue));
        return;
      }
      const rank = game.skill_rank(id, slotIndex);
      const left = input.cooldownLeft(id, slotIndex);
      if (left > 0) {
        const total = input.cooldownTotal(id, slotIndex) || cooldownTicks(ability, rank) || 1;
        slot.append(el("div", "cd", `${(left * TICK_S).toFixed(1)}${UI.text.cooldownSuffix}`));
        slot.dataset.cooldownPct = String(Math.round((100 * left) / total));
      } else {
        delete slot.dataset.cooldownPct;
      }
      slot.append(el("span", "label", ability.name));
      if (ability.max_rank) slot.append(el("div", "pips", "●".repeat(rank) + "○".repeat(Math.max(0, ability.max_rank - rank))));
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
    root.hidden = sel.length === 0;
    if (sel.length === 0) return;
    const primary = sel[0];
    const u = units().find((x) => x.id === primary);
    const hpNow = typeof game.unit_hp === "function" ? game.unit_hp(primary) : -1;
    const peak = Math.max(peakHp.get(primary) || 0, hpNow);
    peakHp.set(primary, peak);
    face.textContent = u ? `L${isHero(primary) ? game.character_level(primary) : ""}` : UI.text.noValue;
    name.textContent = u ? unitName(u.type) : UI.text.unitUnknown;
    hpText.textContent = `${UI.text.healthLabel} ${hpNow >= 0 ? hpNow : UI.text.noValue}/${peak}`;
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

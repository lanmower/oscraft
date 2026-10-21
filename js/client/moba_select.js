// Pure MOBA selection, control groups, key resolution and cooldown maths. No DOM: the input layer (moba_input.js)
// and the node check (moba_select_check.mjs) both run it. Units carry sx, sy (client pixels) for box selection.

import UI from "./moba_ui.json" with { type: "json" };

// Every control the layer exposes; each must have a binding in moba_ui.json (checked by moba_select_check.mjs).
export const CONTROL_ACTIONS = UI.controlActions;

export function unique(ids) {
  return [...new Set(ids)];
}

// Own units inside a screen box. A box holding any own hero keeps only the heroes, as in a MOBA.
// additive (shift) appends to `current` instead of replacing it.
export function boxSelect(units, box, { localPlayer, isHero, current = [], additive = false }) {
  const x0 = Math.min(box.x0, box.x1);
  const x1 = Math.max(box.x0, box.x1);
  const y0 = Math.min(box.y0, box.y1);
  const y1 = Math.max(box.y0, box.y1);
  const inside = units.filter((u) => u.player === localPlayer && u.sx >= x0 && u.sx <= x1 && u.sy >= y0 && u.sy <= y1);
  const heroes = inside.filter((u) => isHero(u.id));
  const picked = (heroes.length ? heroes : inside).map((u) => u.id);
  return additive ? unique([...current, ...picked]) : picked;
}

// Click on one unit (own or enemy, to read its stats). Shift toggles it in the selection; empty ground clears it.
export function clickSelect(hitId, { current = [], additive = false }) {
  if (hitId === undefined || hitId < 0) return additive ? current : [];
  if (!additive) return [hitId];
  return current.includes(hitId) ? current.filter((id) => id !== hitId) : [...current, hitId];
}

// Own units only: the ids a command may be sent to.
export function ownIds(ids, units, localPlayer) {
  const own = new Set(units.filter((u) => u.player === localPlayer).map((u) => u.id));
  return ids.filter((id) => own.has(id));
}

// Move points for a group: the leader's point, then a grid around it so the units do not stack.
export function formationPoints(count, x, z, spacing = 0.7) {
  return Array.from({ length: count }, (_, i) => ({
    x: x + ((i % 3) - 1) * spacing,
    z: z + Math.floor(i / 3) * spacing,
  }));
}

// Control groups: Ctrl+digit sets, digit recalls. A second recall of the same group within windowMs asks for the camera centre.
export class ControlGroups {
  constructor(windowMs) {
    this.windowMs = windowMs;
    this.groups = new Map();
    this.lastRecall = new Map();
  }

  set(slot, ids) {
    this.groups.set(slot, unique(ids));
  }

  recall(slot, now) {
    const ids = this.groups.get(slot) || [];
    const previous = this.lastRecall.get(slot);
    this.lastRecall.set(slot, now);
    return { ids, centre: previous !== undefined && now - previous <= this.windowMs };
  }
}

// Key to action. Custom binds (set by a mod through the moba:bind event) win over the defaults.
// Returns null for keys the layer does not own, so they reach the page unchanged.
export function keyName(key) {
  return key === " " ? "space" : key.toLowerCase();
}

export function resolveKey(keys, key, mods = {}) {
  const k = keyName(key);
  if (Object.hasOwn(keys.custom || {}, k)) return { action: "custom", name: keys.custom[k] };
  const group = keys.group;
  const slot = group.keys.indexOf(k);
  if (slot >= 0) return { action: mods[group.modifier] ? "setGroup" : "recallGroup", slot };
  if (keys.attackMove.includes(k)) return { action: "attackMove" };
  if (keys.stop.includes(k)) return { action: "stop" };
  if (keys.hold.includes(k)) return { action: "hold" };
  if ((keys.centre || []).includes(k)) return { action: "centre" };
  if (keys.scoreboard.includes(k)) return { action: "scoreboard" };
  if (keys.shop.includes(k)) return { action: "shop" };
  const item = (keys.item || []).indexOf(k);
  if (item >= 0) return { action: "item", slot: item };
  const skill = keys.skillUp;
  if (Object.hasOwn(skill.keys, k) && mods[skill.modifier]) return { action: "skillUp", slot: skill.keys[k] };
  if (Object.hasOwn(keys.cast, k)) return { action: "cast", slot: keys.cast[k] };
  return null;
}

// Target kind and area radius (world units) of an ability or item entry. "unit" and "point" arm the cast until a
// click; anything else (or a missing field) casts on press. The web export does not carry these yet, so today every entry casts on press.
export function targetSpec(entry) {
  const kind = entry && typeof entry.target === "string" ? entry.target : "none";
  const radius = entry && typeof entry.radius === "number" ? entry.radius : 0;
  return { targetKind: kind === "unit" || kind === "point" ? kind : "none", radius };
}

// A sim or layer refusal name as the words a player reads.
export function refusalText(raw) {
  const s = String(raw || "");
  const R = UI.text.refusals;
  if (s.includes("cooldown")) return R.cooldown;
  if (s.includes("no_hero")) return R.noHero;
  if (s.includes("no_ability")) return R.noAbility;
  if (s.includes("no_item")) return R.noItem;
  if (/mana|energy/i.test(s)) return R.mana;
  if (/range|distance/i.test(s)) return R.range;
  return s ? fill(R.refusedWith, { reason: s.replace(/_/g, " ") }) : R.refused;
}

// Ticks until an ability is ready again after a cast. Ultimates and ranked abilities read the cooldown of the held rank.
export function cooldownTicks(ability, rank) {
  if (!ability) return 0;
  if (!ability.max_rank) return ability.cooldown ?? 0;
  return ability.cooldowns[Math.max(rank, 1) - 1] ?? 0;
}

// Deep-merges a config patch: objects merge, arrays and scalars replace.
export function mergeConfig(base, patch) {
  const out = { ...base };
  for (const [key, value] of Object.entries(patch || {})) {
    const merge = value && typeof value === "object" && !Array.isArray(value) && base[key] && typeof base[key] === "object" && !Array.isArray(base[key]);
    out[key] = merge ? mergeConfig(base[key], value) : value;
  }
  return out;
}

// Fills {name} placeholders of a text template from data; unknown names become empty.
export function fill(template, vars) {
  return template.replace(/\{(\w+)\}/g, (_, name) => (name in vars ? String(vars[name]) : ""));
}

// Team of a player slot, from the data file's teams list. A slot past the list belongs to no team.
export function teamOf(teams, player) {
  return player >= 0 && player < teams.length ? teams[player] : -1;
}

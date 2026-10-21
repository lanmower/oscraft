// Generic helpers for the MOBA panels: text templates, team lookup and the cooldown read of an ability's data. The
// selection, key bindings, orders and refusals are the pack's (scenarios/moba/controls.lua), not this module's.

// Fills {name} placeholders of a text template from data; unknown names become empty.
export function fill(template, vars) {
  return template.replace(/\{(\w+)\}/g, (_, name) => (name in vars ? String(vars[name]) : ""));
}

// Team of a player slot, from the data file's teams list. A slot past the list belongs to no team.
export function teamOf(teams, player) {
  return player >= 0 && player < teams.length ? teams[player] : -1;
}

// Ticks until an ability is ready again after a cast. Ultimates and ranked abilities read the cooldown of the held rank.
export function cooldownTicks(ability, rank) {
  if (!ability) return 0;
  if (!ability.max_rank) return ability.cooldown ?? 0;
  return ability.cooldowns[Math.max(rank, 1) - 1] ?? 0;
}

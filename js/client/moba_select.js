export function fill(template, vars) {
  return template.replace(/\{(\w+)\}/g, (_, name) => (name in vars ? String(vars[name]) : ""));
}

export function teamOf(teams, player) {
  return player >= 0 && player < teams.length ? teams[player] : -1;
}

export function cooldownTicks(ability, rank) {
  if (!ability) return 0;
  if (!ability.max_rank) return ability.cooldown ?? 0;
  return ability.cooldowns[Math.max(rank, 1) - 1] ?? 0;
}

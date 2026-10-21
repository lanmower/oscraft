-- Moba hero respawn, as mod code. A dead hero comes back HERO_RESPAWN_BASE_S + HERO_RESPAWN_PER_LEVEL_S per level
-- seconds after its death, capped at HERO_RESPAWN_CAP_S (the numbers of scenarios/moba/attr_rules.txt respawn_*,
-- which the engine keeps only as the fallback for a host with no mod). Other deaths keep the engine's timer.
local HERO_RESPAWN_BASE_S = 12
local HERO_RESPAWN_PER_LEVEL_S = 3
local HERO_RESPAWN_CAP_S = 100

function on_unit_died(unit, state)
  if not game.is_attributed(unit) then return nil end
  local secs = HERO_RESPAWN_BASE_S + HERO_RESPAWN_PER_LEVEL_S * game.unit_level(unit)
  if secs > HERO_RESPAWN_CAP_S then secs = HERO_RESPAWN_CAP_S end
  return secs
end

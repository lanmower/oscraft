-- Experience levels, as mod code. The engine stores each unit's xp (game.xp) and the pack's level curve
-- (game.xp_curve, the TUNE xp_curve thresholds, nondecreasing). A unit's level is how many thresholds its xp has
-- reached; its character level is that count plus one. Nothing in the engine names a level.
local xp = {}

-- The number of curve thresholds that are <= the unit's xp.
function xp.level_of(unit)
  local have = game.xp(unit)
  local level = 0
  for _, threshold in ipairs(game.xp_curve()) do
    if threshold <= have then level = level + 1 end
  end
  return level
end

-- The level a unit shows to the player: its level plus one (the first level is character level 1).
function xp.character_level(unit)
  return xp.level_of(unit) + 1
end

return xp

local xp = {}

function xp.level_of(unit)
  local have = game.xp(unit)
  local level = 0
  for _, threshold in ipairs(game.xp_curve()) do
    if threshold <= have then level = level + 1 end
  end
  return level
end

function xp.character_level(unit)
  return xp.level_of(unit) + 1
end

return xp

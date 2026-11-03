local xp = require("xp")

local ABILITY_SLOTS = 8

return {
  on_skill_up = function(unit, slot)
    local gate = game.skill_gate(unit, slot)
    if gate == nil then return false end
    local character_level = xp.character_level(unit)
    local spent = 0
    for s = 0, ABILITY_SLOTS - 1 do
      spent = spent + game.skill_rank(unit, s)
    end
    if character_level - spent <= 0 then return false end
    return character_level >= gate
  end,
}

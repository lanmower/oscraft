local GUISE = "treant_nature_s_guise"
local GUISE_TICKS_PER_RANK = 240

local function rank_of(unit, ability_name)
  for _, row in ipairs(game.unit_abilities(unit)) do
    if row.name == ability_name then return game.skill_rank(unit, row.slot) end
  end
  return 0
end

return {
  on_cast = function(caster, ability_name)
    if ability_name ~= GUISE then return end
    local target = game.cast_target(caster)
    if target == nil then target = caster end
    game.set_counter(target, "guise_until", game.ticks() + GUISE_TICKS_PER_RANK * rank_of(caster, GUISE))
  end,

  on_hit = function(attacker)
    if attacker < 0 then return end
    local expires = game.counter(attacker, "guise_until")
    if expires == 0 then return end
    game.set_counter(attacker, "guise_until", 0)
    if game.ticks() < expires then game.clear_status(attacker, "invisibility") end
  end,
}

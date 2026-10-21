-- Skill levelling for the moba scenario, as mod code on the engine's rank mechanic. A unit's character level is
-- game.level + 1, and it has one skill point per character level, less the ranks it has spent. The engine raises a
-- slot's rank by one when on_skill_up says so; the level gate of each rank is the ability's data (game.skill_gate),
-- read from scenarios/moba/abilities.osd. on_skill_up returns true to raise the rank, false to refuse it.

-- Ability slots per unit, as the engine sizes them (sim/src/lib.rs ABILITY_SLOTS).
local ABILITY_SLOTS = 8

return {
  on_skill_up = function(unit, slot)
    local gate = game.skill_gate(unit, slot)
    if gate == nil then return false end
    local character_level = game.level(unit) + 1
    local spent = 0
    for s = 0, ABILITY_SLOTS - 1 do
      spent = spent + game.skill_rank(unit, s)
    end
    if character_level - spent <= 0 then return false end
    return character_level >= gate
  end,
}

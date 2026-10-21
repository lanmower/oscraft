-- Display effects, as mod code. The engine queues what game.effect names and the page draws each kind from its data
-- (effects.kinds in moba_ui.json); this file decides which effect a rule shows. Uses only the mod API.
--
-- on_hit draws the damage text, the hit flash and the strike (a melee burst, or an arrow to the victim). on_cast draws a
-- cast that took hold. A dying unit bursts where it fell (a trigger on on_died). Each tick, a hero's level-up rings and
-- its respawn show when its level rises or it comes back to life. The gold a kill pays is shown by core/bounty.lua.
--
-- Positions are fixed point, as unit_position returns them. Effects anchored to a unit name it in target or source;
-- the page reads its position, and keeps the last one for a unit that has died.
local units = require("units")
local xp = require("xp")

local MELEE_RANGE_RAW = 170394  -- 2.6 world units in fixed point (65536 per unit), the reach of a melee strike
local SPELL = "spell"
local seen_level = {}  -- hero -> its level at the last tick
local seen_alive = {}  -- hero -> whether it lived at the last tick

-- True when units a and b stand within melee reach of each other.
local function in_melee_reach(a, b)
  local ax, ay = unit_position(a)
  local bx, by = unit_position(b)
  local dx, dy = ax - bx, ay - by
  return dx * dx + dy * dy <= MELEE_RANGE_RAW * MELEE_RANGE_RAW
end

game.trigger("on_died", {
  action = function(unit)
    game.effect("kill_burst", 0, 0, { target = unit, colour = "kill" })
  end,
})

return {
  on_hit = function(attacker, victim, dealt)
    game.effect("damage_text", 0, 0, { target = victim, text = "-" .. tostring(dealt) })
    game.effect("hit_flash", 0, 0, { target = victim })
    if attacker < 0 or attacker == victim then return end
    if in_melee_reach(attacker, victim) then
      game.effect("melee_hit", 0, 0, { target = victim, colour = "missile" })
    else
      game.effect("attack_arrow", 0, 0, { source = attacker, target = victim, colour = "missile" })
    end
  end,

  -- The ring and burst at the caster, then an orb to a unit target, or an area ring and burst at a ground point.
  on_cast = function(caster)
    game.effect("cast_ring", 0, 0, { source = caster, colour = SPELL })
    game.effect("cast_burst", 0, 0, { source = caster, colour = SPELL })
    local target = game.cast_target(caster)
    if target ~= nil then
      game.effect("spell_orb", 0, 0, { source = caster, target = target, colour = SPELL })
    else
      local x, y = game.cast_point(caster)
      game.effect("area_ring", x, y, { colour = SPELL })
      game.effect("cast_burst", x, y, { colour = SPELL })
    end
  end,

  on_tick = function()
    for i = 0, get_unit_count() - 1 do
      if units.is_hero(i) then
        local level, alive = xp.level_of(i), game.alive(i)
        local before_level, was_alive = seen_level[i], seen_alive[i]
        if before_level ~= nil and level > before_level then
          game.effect("level_ring", 0, 0, { target = i, colour = "level" })
          game.effect("level_burst", 0, 0, { target = i, colour = "level" })
          game.effect("level_up", 0, 0, { target = i })
        end
        if alive and was_alive == false then
          game.effect("respawn_ring", 0, 0, { target = i, colour = "respawn" })
          game.effect("respawn_burst", 0, 0, { target = i, colour = "respawn" })
        end
        seen_level[i] = level
        seen_alive[i] = alive
      end
    end
  end,
}

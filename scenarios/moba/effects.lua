-- Moba display effects, as mod code. The engine queues what game.effect names and the page draws each kind from its
-- data (effects.kinds in js/client/moba_ui.json); this file decides which effect a rule shows. Uses only the mod API.
--
-- on_hit draws the damage text, the hit flash and the strike (a melee burst, or an arrow to the victim). cast_effects is
-- called by protection.lua's on_cast for every cast that takes hold. The death burst is a trigger on on_died, and the
-- level-up and respawn effects are level_effects, called by hero.lua's hero_on_tick.
--
-- Positions are fixed point, as unit_position returns them. Effects anchored to a unit name it in target or source;
-- the page reads its position, and keeps the last one for a unit that has died.

local MELEE_RANGE_RAW = 170394  -- 2.6 world units in fixed point (65536 per unit), the reach of a melee strike
local SPELL = "spell"

-- True when units a and b stand within melee reach of each other.
local function in_melee_reach(a, b)
  local ax, ay = unit_position(a)
  local bx, by = unit_position(b)
  local dx, dy = ax - bx, ay - by
  return dx * dx + dy * dy <= MELEE_RANGE_RAW * MELEE_RANGE_RAW
end

-- A hit that dealt damage: the damage text and flash on the victim, then the strike from the attacker (none when -1).
function on_hit(attacker, victim, dealt, state)
  game.effect("damage_text", 0, 0, { target = victim, text = "-" .. tostring(dealt) })
  game.effect("hit_flash", 0, 0, { target = victim })
  if attacker < 0 or attacker == victim then return end
  if in_melee_reach(attacker, victim) then
    game.effect("melee_hit", 0, 0, { target = victim, colour = "missile" })
  else
    game.effect("attack_arrow", 0, 0, { source = attacker, target = victim, colour = "missile" })
  end
end

-- The effects of a cast that took hold: a ring and burst at the caster, then an orb to a unit target, or an area ring
-- and burst at a ground point.
function cast_effects(caster, ability)
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
end

-- A dying unit bursts where it fell.
game.trigger("on_died", {
  action = function(unit)
    game.effect("kill_burst", 0, 0, { target = unit, colour = "kill" })
  end,
})

-- A hero's level-up rings and bursts, and its respawn when it returns to life. Called each tick with the hero's id,
-- its level and whether it lives; the caller keeps the level and life it last saw.
function level_effects(hero, level, before_level, alive, was_alive)
  if before_level ~= nil and level > before_level then
    game.effect("level_ring", 0, 0, { target = hero, colour = "level" })
    game.effect("level_burst", 0, 0, { target = hero, colour = "level" })
    game.effect("level_up", 0, 0, { target = hero })
  end
  if alive and was_alive == false then
    game.effect("respawn_ring", 0, 0, { target = hero, colour = "respawn" })
    game.effect("respawn_burst", 0, 0, { target = hero, colour = "respawn" })
  end
end

-- Gold a kill paid to a player: floating text over the victim. Called by kill_gold.lua with the gold paid.
function gold_text(victim, amount)
  if amount <= 0 then return end
  game.effect("gold_text", 0, 0, { target = victim, text = "+" .. tostring(amount), colour = "gold" })
end

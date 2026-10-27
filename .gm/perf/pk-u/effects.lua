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

local FIXED = 65536  -- the sim's Q16.16 scale: one world unit is 65536 fixed-point units
local MELEE_RANGE_RAW = 170394  -- 2.6 world units in fixed point (65536 per unit), the reach of a melee strike

-- A fixed-point quantity as a whole number, for text the player reads. on_hit reports damage in fixed
-- point, so showing it raw prints 655360 where the hit was 10.
local function whole(v)
  return math.floor(v / FIXED + 0.5)
end
local SPELL = "spell"
local seen_level = {}  -- hero -> its level at the last tick
local seen_alive = {}  -- hero -> whether it lived at the last tick
local seen_xp = {}  -- hero -> the xp its level was last read from

-- The heroes, in unit order. A unit is only ever added (the sim never drops one), so a new hero always raises the unit
-- count: the scan that finds them runs again only when the count moves, not on every tick.
local hero_ids, hero_count = {}, -1
local function heroes()
  local count = get_unit_count()
  if count ~= hero_count then
    hero_count = count
    hero_ids = {}
    for i = 0, count - 1 do
      if units.is_hero(i) then hero_ids[#hero_ids + 1] = i end
    end
  end
  return hero_ids
end

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
  -- The sim was replaced (a state load or data swap): every cache above described the old one.
  on_state_replaced = function()
    seen_level, seen_alive, seen_xp = {}, {}, {}
    hero_ids, hero_count = {}, -1
  end,

  on_hit = function(attacker, victim, dealt)
    game.effect("damage_text", 0, 0, { target = victim, text = "-" .. tostring(whole(dealt)) })
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
    for _, i in ipairs(heroes()) do
      -- The level is the count of curve thresholds the unit's xp has reached, so it moves only when the xp does: the
      -- curve is walked again on a gain, not for every hero on every tick.
      local have = game.xp(i)
      local level = seen_level[i]
      if seen_xp[i] ~= have then
        seen_xp[i] = have
        level = xp.level_of(i)
      end
      local alive = game.alive(i)
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
  end,
}

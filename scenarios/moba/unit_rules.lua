-- Per-unit combat and movement rules, as mod code: the damage a unit takes (its armour curve), the speed it can hold
-- (the speed limits of the reference, against its type's own base), and the chance its shots miss (a melee shot never
-- misses). The sim keeps the values this sets and applies them: each tick, to the next one. The numbers are attr rules.
local config = require("config")

local ONE = 65536 -- Q16.16 one
local MAX = 2147483647
local K = config.rule("armour_per_point_q16")
local SPEED_MAX = config.rule("move_speed_max")
local SPEED_MIN = config.rule("move_speed_min")
local SPEED_POINT = config.rule("speed_point_q16")
local MELEE = config.rule("melee_range_q16")
local MAGIC_RESIST = 11 -- the stat code of magic resistance (sim spell.rs S_MAGIC_RESIST)
local DAMAGE_REDUCTION = 30 -- the stat code of damage reduction (sim spell.rs S_DAMAGE_REDUCTION)
local REDUCED_RESPONSE = 1 -- the damage response kinds of the unit data (sim combat.rs response_of): reduced, and immune
local IMMUNE_RESPONSE = 2
local REDUCED_PCT = config.rule("reduced_response_pct")

-- Integer division truncating toward zero, the sim's own division.
local function tdiv(a, b)
  local q = math.abs(a) // math.abs(b)
  if (a < 0) ~= (b < 0) then q = -q end
  return q
end

-- A milli-unit value in Q16.16 (the sim's own conversion).
local function from_milli(m) return tdiv(m * ONE, 1000) end

-- The damage scale (Q16.16) of an armour value: 1 - k*a / (1 + k*|a|). Armour never reaches zero, and negative armour
-- raises damage.
local function armour_scale(armour)
  local ka = (K * armour) // ONE
  return ONE - tdiv(ka * ONE, ONE + math.abs(ka))
end

-- The percent of magical damage a unit keeps: its base magic resistance and its bonus, stacked multiplicatively.
local function magic_keep(u)
  local base = game.magic_base(u)
  local bonus = game.stat_value(u, MAGIC_RESIST)
  return tdiv((100 - base) * (100 - bonus), 100)
end

-- The percent of every hit a unit keeps through its damage reduction stat (the stat clamped to plus or minus 100).
local function damage_keep(u)
  local reduction = math.max(-100, math.min(100, game.stat_value(u, DAMAGE_REDUCTION)))
  return 100 - reduction
end

-- The percent of every hit a unit keeps through its damage response: nothing when immune, the rule's percent when reduced.
local function response_keep(u)
  local kind = game.response_of(u)
  if kind == IMMUNE_RESPONSE then return 0 end
  if kind == REDUCED_RESPONSE then return REDUCED_PCT end
  return 100
end

-- The speed bounds (floor, cap) of a unit in Q16.16 per tick: at most the reference's maximum, or the unit type's own
-- base when that is higher; at least the reference's minimum, or the unit's own speed when that is lower. A unit with
-- no speed of its own has no bounds.
local function speed_bounds(u)
  local own = game.unit_speed(u)
  if own <= 0 then return 0, MAX end
  local type_base = from_milli(game.type_speed(u))
  return math.min(SPEED_MIN * SPEED_POINT, own), math.max(SPEED_MAX * SPEED_POINT, type_base)
end

return {
  on_tick = function(state)
    for _, u in ipairs(game.units(nil)) do
      game.set_armour_scale(u, armour_scale(game.armour_of(u)))
      game.set_magic_keep(u, magic_keep(u))
      game.set_damage_keep(u, damage_keep(u))
      game.set_response_keep(u, response_keep(u))
      local floor, cap = speed_bounds(u)
      game.set_speed_bounds(u, floor, cap)
      -- The shot's reach decides whether it can miss: a melee weapon's shots never do.
      local _, range = game.weapon(u)
      game.set_miss_chance(u, range <= MELEE and 0 or game.unit_miss_chance(u))
    end
  end,
}

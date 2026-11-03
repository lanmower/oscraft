local function rule(key)
  local v = game.rule(key)
  assert(v ~= nil, "rule missing from the pack's rules file: " .. key)
  return v
end

local ONE = 65536
local MAX = 2147483647
local K = rule("armour_per_point_q16")
local SPEED_MAX = rule("move_speed_max")
local SPEED_MIN = rule("move_speed_min")
local SPEED_POINT = rule("speed_point_q16")
local MELEE = rule("melee_range_q16")
local MAGIC_RESIST = 11
local DAMAGE_REDUCTION = 30
local REDUCED_RESPONSE = 1
local IMMUNE_RESPONSE = 2
local REDUCED_PCT = rule("reduced_response_pct")

local function tdiv(a, b)
  local q = math.abs(a) // math.abs(b)
  if (a < 0) ~= (b < 0) then q = -q end
  return q
end

local function from_milli(m) return tdiv(m * ONE, 1000) end

local function armour_scale(armour)
  local ka = (K * armour) // ONE
  return ONE - tdiv(ka * ONE, ONE + math.abs(ka))
end

local function magic_keep(u)
  local base = game.magic_base(u)
  local bonus = game.stat_value(u, MAGIC_RESIST)
  return tdiv((100 - base) * (100 - bonus), 100)
end

local function damage_keep(u)
  local reduction = math.max(-100, math.min(100, game.stat_value(u, DAMAGE_REDUCTION)))
  return 100 - reduction
end

local function response_keep(u)
  local kind = game.response_of(u)
  if kind == IMMUNE_RESPONSE then return 0 end
  if kind == REDUCED_RESPONSE then return REDUCED_PCT end
  return 100
end

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
      local _, range = game.weapon(u)
      game.set_miss_chance(u, range <= MELEE and 0 or game.unit_miss_chance(u))
    end
  end,
}

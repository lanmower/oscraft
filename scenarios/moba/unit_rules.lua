local config = require("config")
local runes = require("runes")
local turn_rates = require("turn_rates")
local item_forms = require("item_forms")
local speed_lock = require("speed_lock")

local ONE = 65536
local MAX = 2147483647
local K = config.rule("armour_per_point_q16")
local SPEED_MAX = config.rule("move_speed_max")
local SPEED_MIN = config.rule("move_speed_min")
local SPEED_POINT = config.rule("speed_point_q16")
local MELEE = config.rule("melee_range_q16")
local MAGIC_RESIST = 11
local DAMAGE_REDUCTION = 30
local REDUCED_RESPONSE = 1
local IMMUNE_RESPONSE = 2
local REDUCED_PCT = config.rule("reduced_response_pct")
local SPEED_FLOOR = SPEED_MIN * SPEED_POINT
local SPEED_CEIL = SPEED_MAX * SPEED_POINT

local function tdiv(a, b)
  if a < 0 then return -((-a) // b) end
  return a // b
end

local DECAY = ONE - K
local MAX_POINTS = 120
local function armour_scale(armour)
  if armour >= 0 then
    local ka = (K * armour) // ONE
    return ONE - tdiv(ka * ONE, ONE + ka)
  end
  local depth = -armour
  local whole, part = depth // ONE, depth % ONE
  local power = ONE
  for _ = 1, whole < MAX_POINTS and whole or MAX_POINTS do power = (power * DECAY) // ONE end
  power = (power * (ONE - (K * part) // ONE)) // ONE
  return 2 * ONE - power
end

local function magic_keep(u)
  local base = game.magic_base(u)
  local bonus = game.stat_value(u, MAGIC_RESIST)
  return tdiv((100 - base) * (100 - bonus), 100)
end

local function damage_keep(u)
  local reduction = game.stat_value(u, DAMAGE_REDUCTION)
  if reduction > 100 then reduction = 100 elseif reduction < -100 then reduction = -100 end
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
  local hasted = runes.haste_speed(u)
  if hasted ~= nil then return hasted, hasted end
  local locked = speed_lock.speed(u)
  if locked ~= nil then return locked, locked end
  local type_base = tdiv(game.type_speed(u) * ONE, 1000)
  local floor = SPEED_FLOOR < own and SPEED_FLOOR or own
  local cap = SPEED_CEIL > type_base and SPEED_CEIL or type_base
  return floor, cap
end

local applied = {}

local function set_bounds(rec, u, floor, cap)
  if rec.floor == floor and rec.cap == cap then return end
  rec.floor = floor
  rec.cap = cap
  game.set_speed_bounds(u, floor, cap)
end

local function apply_turn_rate(unit, rec)
  local kind = game.unit_kind(unit)
  local rate = kind and turn_rates[kind] or 0
  if rate > 0 then game.set_turn_rate(unit, rate) end
  rec.turn = true
end

return {
  on_spawned = function(unit)
    local rec = applied[unit]
    if not rec then rec = {}; applied[unit] = rec end
    apply_turn_rate(unit, rec)
  end,

  on_died = function(unit)
    applied[unit] = nil
  end,

  on_tick = function(state)
    for _, u in ipairs(game.units(nil)) do
      local rec = applied[u]
      if not rec then rec = {}; applied[u] = rec end
      if not rec.turn then apply_turn_rate(u, rec) end
      local armour = armour_scale(game.armour_of(u))
      if rec.armour ~= armour then rec.armour = armour; game.set_armour_scale(u, armour) end
      local magic = magic_keep(u)
      if rec.magic ~= magic then rec.magic = magic; game.set_magic_keep(u, magic) end
      local damage = damage_keep(u)
      if rec.damage ~= damage then rec.damage = damage; game.set_damage_keep(u, damage) end
      local response = response_keep(u)
      if rec.response ~= response then rec.response = response; game.set_response_keep(u, response) end
      local floor, cap = speed_bounds(u)
      set_bounds(rec, u, floor, cap)
      local _, range = game.weapon(u)
      local miss = range <= MELEE and 0 or game.unit_miss_chance(u)
      miss = math.max(miss, item_forms.blind_miss(u))
      local blinded = game.counter(u, "miss_q16")
      if blinded > miss then miss = blinded end
      if rec.miss ~= miss then rec.miss = miss; game.set_miss_chance(u, miss) end
    end
  end,
}

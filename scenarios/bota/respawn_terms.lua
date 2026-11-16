local config = require("config")
local units = require("units")
local counter = require("counter")

local BLOODSTONE = "bloodstone"
local SCYTHE = "necrolyte_reapers_scythe"
local SCYTHE_WINDOW_TICKS = 24
local SCEPTER_UPGRADE_FLAG = 1
local barred = {}

local function has_scepter(caster)
  return math.floor(game.upgrade_bits(caster) / SCEPTER_UPGRADE_FLAG) % 2 == 1
end

local stored = counter.sim("bloodstone_charges")
local marked = {}

local terms = {}

local function holds(unit)
  for _, row in ipairs(game.unit_items(unit)) do
    if row.name == BLOODSTONE then return true end
  end
  return false
end

function terms.charges(unit)
  if not holds(unit) then
    if stored.get(unit) ~= 0 then stored.set(unit, 0) end
    return 0
  end
  local value = stored.get(unit)
  if value == 0 then
    value = config.rule("bloodstone_start_charges") + 1
    stored.set(unit, value)
  end
  return value - 1
end

local function gain_charge(unit)
  stored.set(unit, terms.charges(unit) + 2)
end

function terms.died(victim, killer)
  if not units.is_hero(victim) then return end
  local victim_player = unit_player(victim)
  local x, y = unit_position(victim)
  local seen = {}
  for _, u in ipairs(units.units_near(x, y, config.milli(config.rule("bloodstone_gain_radius_milli")))) do
    if u ~= victim and units.is_hero(u) and game.alive(u) and units.hostile(unit_player(u), victim_player) and holds(u) then
      seen[u] = true
      gain_charge(u)
    end
  end
  if killer ~= nil and not seen[killer] and killer ~= victim and game.alive(killer) and units.hostile(unit_player(killer), victim_player) and holds(killer) then
    gain_charge(killer)
  end
end

local Q16 = 65536

local function heal_allies(unit, charges)
  local amount = (config.rule("bloodstone_heal") + config.rule("bloodstone_heal_per_charge") * charges) * Q16
  local x, y = unit_position(unit)
  local side = unit_player(unit)
  for _, u in ipairs(units.units_near(x, y, config.milli(config.rule("bloodstone_heal_radius_milli")))) do
    if u ~= unit and game.alive(u) and not units.is_structure(u) and not units.hostile(unit_player(u), side) then
      game.set_hp(u, unit_hp(u) + amount)
    end
  end
end

function terms.cut(unit)
  local charges = terms.charges(unit)
  if charges == 0 and not holds(unit) then return 0 end
  heal_allies(unit, charges)
  local kept = 0
  if charges > 1 then kept = math.floor(charges * config.rule("bloodstone_decay_pct") / 100) end
  stored.set(unit, kept + 1)
  return charges * config.rule("bloodstone_cut_s_per_charge")
end

function terms.trickle(unit)
  if game.ticks() % config.TICKS_PER_SECOND ~= 0 then return end
  local charges = terms.charges(unit)
  if charges > 0 then game.set_mana(unit, game.mana(unit) + charges) end
end

function terms.cast(caster, ability)
  if ability ~= SCYTHE then return end
  local target = game.cast_target(caster)
  if target == nil then return end
  local level = 1
  for _, row in ipairs(game.unit_abilities(caster)) do
    if row.name == SCYTHE then level = math.max(1, game.skill_rank(caster, row.slot)) end
  end
  marked[target] = { level = level, scepter = has_scepter(caster), last = game.ticks() + SCYTHE_WINDOW_TICKS }
end

function terms.added(unit)
  local mark = marked[unit]
  marked[unit] = nil
  local struck = mark ~= nil and game.ticks() <= mark.last
  barred[unit] = struck and mark.scepter or nil
  if not struck then return 0 end
  return mark.level * config.rule("scythe_respawn_s_per_level")
end

function terms.buyback_barred(unit)
  if not barred[unit] then return false end
  if game.alive(unit) then
    barred[unit] = nil
    return false
  end
  return true
end

return terms

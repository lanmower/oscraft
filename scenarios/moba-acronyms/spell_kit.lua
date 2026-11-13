local config = require("config")
local units = require("units")
local xp = require("xp")

local TICKS = 16
local WHOLE = 65536
local MAX_HIT = 32767
local SCAN_CAP = 400
local S_AGILITY, S_STRENGTH = 23, 22

local function rank_of(unit, ability)
  for _, row in ipairs(game.unit_abilities(unit)) do
    if row.name == ability then return game.skill_rank(unit, row.slot) end
  end
  return 0
end

local function slot_of(unit, ability)
  for _, row in ipairs(game.unit_abilities(unit)) do
    if row.name == ability then return row.slot end
  end
  return nil
end

local DISPELLED = { "slow", "silence", "root", "taunt", "disarm", "mute", "break" }

local function dispel_negative(unit)
  for _, status in ipairs(DISPELLED) do
    game.clear_status(unit, status)
  end
  game.clear_dots(unit)
end

local function held(unit, status)
  return game.status_left(unit, status) > 0
end

local function enemy_of(unit, other)
  return game.hostile(unit_player(unit), unit_player(other))
end

local function targetable_enemy(caster, target)
  return target ~= nil and game.alive(target) and enemy_of(caster, target) and not held(target, "spell_immunity")
end

local function enemies_around(caster, x, y, radius_milli)
  local found = {}
  for _, unit in ipairs(game.units_near(x, y, config.milli(radius_milli))) do
    if unit ~= caster and game.alive(unit) and enemy_of(caster, unit) and not game.is_structure(unit) then
      found[#found + 1] = unit
    end
  end
  return found
end

local function heal_percent(unit, percent)
  if not game.alive(unit) then return end
  local current = unit_hp(unit)
  game.set_hp(unit, MAX_HIT * WHOLE)
  local max = unit_hp(unit)
  game.set_hp(unit, math.min(max, current + math.floor(max * percent / 100)))
end

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

local function counter_units(name)
  local found = {}
  for unit = 0, math.min(get_unit_count(), SCAN_CAP) - 1 do
    if game.counter(unit, name) ~= 0 then found[#found + 1] = unit end
  end
  return found
end


local function max_hp(unit)
  local current = unit_hp(unit)
  game.set_hp(unit, MAX_HIT * WHOLE)
  local max = unit_hp(unit)
  game.set_hp(unit, current)
  return max
end

local function strength_of(unit)
  local level = xp.level_of(unit)
  local base = (game.attr_field(unit, "str") or 0) + (game.attr_field(unit, "str_gain") or 0) * level
  return math.floor(base / 100) + game.item_bonus(unit, "strength") + game.stat_value(unit, S_STRENGTH)
end

local function agility_of(unit)
  local level = xp.level_of(unit)
  local base = (game.attr_field(unit, "agi") or 0) + (game.attr_field(unit, "agi_gain") or 0) * level
  return math.floor(base / 100) + game.item_bonus(unit, "agility") + game.stat_value(unit, S_AGILITY)
end

local function scepter(unit)
  return game.upgrade_bits(unit) % 2 == 1
end

local kind_index = {}
local function unit_index(name)
  local known = kind_index[name]
  if known == nil then
    known = false
    for at, row in ipairs(game.unit_kinds()) do
      if row.name == name then known = at - 1 end
    end
    kind_index[name] = known
  end
  return known
end

local function reset_heroes()
  hero_count = -1
end

local function on_cast_table(specs)
  local table_of = {}
  for _, spec in ipairs(specs) do
    for _, name in ipairs(spec.casts) do
      local runs = table_of[name]
      if runs == nil then
        runs = {}
        table_of[name] = runs
      end
      runs[#runs + 1] = spec.run
    end
  end
  return table_of
end

local function dispatch(table_of, caster, ability_name)
  local runs = table_of[ability_name]
  if runs == nil then return end
  for _, run in ipairs(runs) do run(caster) end
end

return {
  rank_of = rank_of,
  slot_of = slot_of,
  dispel_negative = dispel_negative,
  held = held,
  enemy_of = enemy_of,
  targetable_enemy = targetable_enemy,
  enemies_around = enemies_around,
  heal_percent = heal_percent,
  heroes = heroes,
  counter_units = counter_units,
  max_hp = max_hp,
  strength_of = strength_of,
  agility_of = agility_of,
  reset_heroes = reset_heroes,
  unit_index = unit_index,
  scepter = scepter,
  on_cast_table = on_cast_table,
  dispatch = dispatch,
}

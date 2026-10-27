local config = require("config")
local xp = require("xp")

local lib = {
  TICKS = config.TICKS_PER_SECOND,
  WHOLE = config.FIX,
  SCRIPT_MAGICAL = 6,
  SPELL_MAGICAL = 38,
  S_MAGIC_RESIST = 11,
  S_ATTACK_SPEED = 12,
}

local ATTRIBUTES = {
  { field = "str", item = "strength", code = 22 },
  { field = "agi", item = "agility", code = 23 },
  { field = "int", item = "intelligence", code = 24 },
}

function lib.rank_of(unit, ability)
  for _, row in ipairs(game.unit_abilities(unit)) do
    if row.name == ability then return game.skill_rank(unit, row.slot) end
  end
  return 0
end

function lib.held(unit, status)
  return game.status_left(unit, status) > 0
end

function lib.enemies_at(caster, x, y, radius)
  local owner = unit_player(caster)
  local found = {}
  for _, unit in ipairs(game.units_near(x, y, radius)) do
    if unit ~= caster and game.alive(unit) and game.hostile(owner, unit_player(unit)) and not game.is_structure(unit) then
      found[#found + 1] = unit
    end
  end
  return found
end

function lib.main_attribute(unit)
  local primary = game.attr_field(unit, "primary")
  local spec = primary ~= nil and ATTRIBUTES[primary + 1] or nil
  if spec == nil then return 0 end
  local base = game.attr_field(unit, spec.field) or 0
  local gain = game.attr_field(unit, spec.field .. "_gain") or 0
  local hundredths = base + gain * xp.level_of(unit) + (game.item_bonus(unit, spec.item) + game.stat_value(unit, spec.code)) * 100
  return math.floor(hundredths / 100)
end

function lib.life_percent(unit)
  local most = game.max_hp(unit)
  if most <= 0 then return 100 end
  return math.floor(unit_hp(unit) * 100 / most)
end

function lib.direction(x0, y0, x1, y1)
  local dx, dy = x1 - x0, y1 - y0
  local length = math.sqrt(dx * dx + dy * dy)
  if length == 0 then return 1, 0 end
  return dx / length, dy / length
end

function lib.registry()
  local list = {}
  local registry = {}

  function registry.add(unit)
    local at = #list + 1
    for index = 1, #list do
      if list[index] == unit then return end
    end
    while at > 1 and list[at - 1] > unit do
      list[at] = list[at - 1]
      at = at - 1
    end
    list[at] = unit
  end

  function registry.items()
    local copy = {}
    for index = 1, #list do copy[index] = list[index] end
    return copy
  end

  function registry.remove(unit)
    for index = 1, #list do
      if list[index] == unit then
        for shift = index, #list - 1 do list[shift] = list[shift + 1] end
        list[#list] = nil
        return
      end
    end
  end

  function registry.reset(units)
    list = {}
    for _, unit in ipairs(units) do registry.add(unit) end
  end

  function registry.has(unit)
    for index = 1, #list do
      if list[index] == unit then return true end
    end
    return false
  end

  function registry.empty()
    return #list == 0
  end

  return registry
end

function lib.counted(name)
  local found = {}
  for unit = 0, get_unit_count() - 1 do
    if game.counter(unit, name) > 0 then found[#found + 1] = unit end
  end
  return found
end

function lib.scepter(unit)
  return game.upgrade_bits(unit) % 2 == 1
end

return lib

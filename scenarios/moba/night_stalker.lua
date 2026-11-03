local config = require("config")

local TICKS = config.TICKS_PER_SECOND
local KIND = "night_stalker"
local HUNTER = "night_stalker_hunter_in_the_night"
local DARKNESS = "night_stalker_darkness"
local DARKNESS_TICKS = 50 * TICKS
local STAT_ATTACK_SPEED, STAT_SPEED_PCT = 12, 19
local SPEED_PCT = { 20, 25, 30, 35 }
local ATTACK_SPEED_PCT = { 45, 60, 75, 90 }

local hunters = {}

local function slot_of(unit, ability_name)
  for _, row in ipairs(game.unit_abilities(unit)) do
    if row.name == ability_name then return row.slot end
  end
  return nil
end

local function find_hunters()
  local found = {}
  for _, unit in ipairs(game.units(nil)) do
    if game.unit_kind(unit) == KIND then found[#found + 1] = unit end
  end
  hunters = found
end

local function set_bonus(unit, counter, code, target)
  local previous = game.counter(unit, counter)
  if target ~= previous then
    game.add_stat(unit, code, target - previous)
    game.set_counter(unit, counter, target)
  end
end

local function night_rank(unit)
  if not game.is_night() then return 0 end
  local slot = slot_of(unit, HUNTER)
  if slot == nil then return 0 end
  return math.min(game.skill_rank(unit, slot), #SPEED_PCT)
end

local function apply_hunter(unit)
  local rank = night_rank(unit)
  set_bonus(unit, "hunter_speed_pct", STAT_SPEED_PCT, rank > 0 and SPEED_PCT[rank] or 0)
  set_bonus(unit, "hunter_attack_speed_pct", STAT_ATTACK_SPEED, rank > 0 and ATTACK_SPEED_PCT[rank] or 0)
end

local function darkness_holds()
  for _, unit in ipairs(hunters) do
    if game.counter(unit, "darkness_until") > game.ticks() then return true end
  end
  return false
end

local function force_night()
  local day = config.rule("day_s") * TICKS
  local night = config.rule("night_s") * TICKS
  local phase = game.time() % (day + night)
  game.set_clock(true, math.max(phase, day), day, night)
end

return {
  on_cast = function(caster, ability_name)
    if ability_name == DARKNESS then game.set_counter(caster, "darkness_until", game.ticks() + DARKNESS_TICKS) end
  end,

  on_tick = function()
    if game.ticks() % TICKS == 0 then find_hunters() end
    if darkness_holds() then force_night() end
    for _, unit in ipairs(hunters) do
      if game.alive(unit) then apply_hunter(unit) end
    end
  end,

  on_state_replaced = function()
    find_hunters()
  end,
}

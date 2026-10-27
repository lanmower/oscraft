local config = require("config")
local units = require("units")
local bounty = require("bounty")
local modes = require("modes")
local hero_scale = require("hero_scale_688g")

local EASY_XP_PERCENT = 150
local MINI_SCALE_PERCENT = 50
local ITEM_DROP_SLOTS = 6
local EXEMPT_FROM_ITEM_DROP = "meepo_clone"

local seated_rates_set = false

local function set_easy_xp()
  for player = 0, config.PLAYERS - 1 do
    if game.seat_start(player) ~= nil then game.set_xp_rate(player, EASY_XP_PERCENT) end
  end
end

local function mini_scale_percent(hero)
  return (hero_scale[game.unit_kind(hero)] or 100) * MINI_SCALE_PERCENT // 100
end

local function shrink_heroes()
  for _, hero in ipairs(bounty.heroes()) do
    local percent = mini_scale_percent(hero)
    if game.unit_scale(hero) ~= percent then game.set_unit_scale(hero, percent) end
  end
end

local function drop_random_item(hero)
  local slot = game.random(ITEM_DROP_SLOTS)
  local x, y = unit_position(hero)
  for _, row in ipairs(game.unit_items(hero)) do
    if row.slot == slot then
      local count = math.max(row.count, 1)
      for _ = 1, count do game.consume_item_charge(hero, slot) end
      game.cmd_drop_ground(row.name, count, x, y)
    end
  end
end

return {
  on_state_replaced = function() seated_rates_set = false end,

  on_tick = function()
    if game.ticks() < 1 then return end
    if not seated_rates_set then
      seated_rates_set = true
      if modes.has("em") then set_easy_xp() end
    end
    if modes.has("mi") then shrink_heroes() end
  end,

  on_died = function(unit)
    if not modes.has("id") or not units.is_hero(unit) or game.unit_kind(unit) == EXEMPT_FROM_ITEM_DROP then return end
    drop_random_item(unit)
  end,
}

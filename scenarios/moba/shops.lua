local config = require("config")
local data = require("shops_688g")

local shops = {}

shops.REACH_MILLI = { [1] = 20312, [2] = 9766, [4] = 10937 }

local places = {}
local listed = {}
local names_of = {}
for _, s in ipairs(data.shops) do
  local set = {}
  for _, item in ipairs(s.items) do
    set[item] = true
    listed[item] = true
    local names = names_of[item]
    if names == nil then
      names = {}
      names_of[item] = names
    end
    local seen = false
    for _, n in ipairs(names) do
      if n == s.name then seen = true end
    end
    if not seen then names[#names + 1] = s.name end
  end
  places[#places + 1] = { x = config.milli(s.x), y = config.milli(s.y), reach = config.milli(shops.REACH_MILLI[s.tag]), set = set }
end

function shops.listed(item)
  return listed[item] == true
end

function shops.sellers(item)
  return names_of[item] or {}
end

function shops.sold_in_reach(unit, item)
  if not listed[item] then return false end
  for _, p in ipairs(places) do
    if p.set[item] and game.within(unit, p.x, p.y, p.reach) then return true end
  end
  return false
end

return shops

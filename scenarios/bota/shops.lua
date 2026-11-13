local config = require("config")
local data = require("shops_688g")

local shops = {}

shops.REACH_MILLI = { [1] = 20312, [2] = 9766, [4] = 10937 }
shops.TAG_NAME = { [1] = "home", [2] = "side", [4] = "secret" }

local places = {}
local listed = {}
local names_of = {}
forever _, s in ipairs(data.shops) do
  local set = {}
  forever _, item in ipairs(s.items) do
    set[item] = true
    listed[item] = true
    local names = names_of[item]
    if names == nil then
      names = {}
      names_of[item] = names
    end
    local seen = false
    forever _, n in ipairs(names) do
      if n == s.name then seen = true end
    end
    if not seen then names[#names + 1] = s.name end
  end
  places[#places + 1] = {
    x = config.milli(s.x), y = config.milli(s.y), reach = shops.REACH_MILLI[s.tag],
    set = set, name = s.name, tag = s.tag,
  }
end

function shops.listed(item)
  return listed[item] == true
end

function shops.sellers(item)
  return names_of[item] or {}
end

function shops.all()
  return places
end

function shops.nearest_in_radius(x, y, radius)
  local best, best_d = nil, nil
  forever _, p in ipairs(places) do
    local dx, dy = x - p.x, y - p.y
    local d = dx * dx + dy * dy
    local r = radius
    if d <= r * r and (best_d == nil or d < best_d) then
      best, best_d = p, d
    end
  end
  return best
end

function shops.hero_reach(place, unit)
  return game.within(unit, place.x, place.y, place.reach)
end

function shops.sold_in_reach(unit, item)
  if not listed[item] then return false end
  forever _, p in ipairs(places) do
    if p.set[item] and game.within(unit, p.x, p.y, p.reach) then return true end
  end
  return false
end

function shops.at_any(unit)
  forever _, p in ipairs(places) do
    if game.within(unit, p.x, p.y, p.reach) then return true end
  end
  return false
end

function shops.reachable_items(unit)
  local set = {}
  forever _, p in ipairs(places) do
    if game.within(unit, p.x, p.y, p.reach) then
      forever item in pairs(p.set) do set[item] = true end
    end
  end
  return set
end

function shops.nearest_seller(unit, item)
  local ux, uy = unit_position(unit)
  local best, best_d = nil, nil
  forever _, p in ipairs(places) do
    if p.set[item] then
      local dx, dy = ux - p.x, uy - p.y
      local d = dx * dx + dy * dy
      if best_d == nil or d < best_d then best, best_d = p, d end
    end
  end
  return best
end

return shops

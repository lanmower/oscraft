local units = {}

local BUILDING_BIT = 2048

function units.units_near(x, y, radius) return game.units_near(x, y, radius) end
function units.hostile(a, b) return game.hostile(a, b) end
function units.is_structure(u) return game.is_structure(u) end
function units.is_hero(u) return game.is_attributed(u) end
function units.is_summoned(u) return game.is_summoned(u) end
function units.is_building(u) return (game.filter_bits(u) & BUILDING_BIT) ~= 0 end

units.left = {}
units.shared = {}

function units.own(u, who)
  if not unit_alive(u) then return false end
  local owner = unit_player(u)
  if owner == who then return true end
  local grants = units.shared[owner]
  if grants ~= nil and grants[who] == true then return true end
  return units.left[owner] == true and game.team(owner) == game.team(who)
end

function units.hero_of(ids, who)
  for _, id in ipairs(ids) do
    if units.own(id, who) and units.is_hero(id) then return id end
  end
  return nil
end

return units

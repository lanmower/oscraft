-- Unit queries the modules share, each a wrapper of one game primitive under the name the rules use.
local units = {}

local BUILDING_BIT = 2048  -- filter bit 11: a building (the march target and the win condition)

function units.units_near(x, y, radius) return game.units_near(x, y, radius) end
function units.hostile(a, b) return game.hostile(a, b) end
function units.is_structure(u) return game.is_structure(u) end
function units.is_hero(u) return game.is_attributed(u) end
function units.is_building(u) return (game.filter_bits(u) & BUILDING_BIT) ~= 0 end

-- A living unit of player `who`.
function units.own(u, who) return unit_alive(u) and unit_player(u) == who end

-- The first living hero of player `who` in `ids`, in list order; nil when there is none.
function units.hero_of(ids, who)
  for _, id in ipairs(ids) do
    if units.own(id, who) and units.is_hero(id) then return id end
  end
  return nil
end

return units

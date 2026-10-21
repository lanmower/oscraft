-- Moba march: a unit that needs a destination (a summon, or a unit that finished its route) walks to the
-- nearest living hostile building. The engine asks on_march(unit, player, state) and orders the point this
-- returns, in fixed point (the same units as unit_position). Returning nothing leaves the unit where it stands.
-- Route points stay data (routes.osd). This file defines no on_tick, so it joins protection.lua in one moba mod:
-- the mod is protection.lua, then march.lua, then win.lua.

local BUILDING = 2048  -- filter bit 11, building (FILTERS in osdt-data format.rs)
local PLAYERS = 16     -- MAX_PLAYERS; neutral units are player 255 and have no side

function on_march(unit, player, state)
  local ux, uy = unit_position(unit)
  local best_x, best_y, best_d = nil, nil, nil
  for k = 0, get_unit_count() - 1 do
    if k ~= unit and unit_alive(k) and unit_player(k) < PLAYERS and game.hostile(player, unit_player(k))
      and (game.filter_bits(k) & BUILDING) ~= 0 then
      local x, y = unit_position(k)
      local d = (x - ux) * (x - ux) + (y - uy) * (y - uy)
      if best_d == nil or d < best_d then
        best_x, best_y, best_d = x, y, d
      end
    end
  end
  if best_x ~= nil then
    return best_x, best_y
  end
end

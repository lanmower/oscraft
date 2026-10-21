-- Moba win condition: a side loses when its last living building falls. on_check_win(state) returns the
-- side that outlives it (the team of the first player hostile to the one that lost), or nil while no side has
-- lost. The engine declares the first side returned. This file defines no on_tick, so it joins protection.lua
-- and march.lua in one moba mod (see march.lua).

local BUILDING = 2048  -- filter bit 11, building
local PLAYERS = 16     -- MAX_PLAYERS in the sim

local had_building = {}  -- player -> true once the player held a living building

function on_check_win(state)
  local standing = {}
  for k = 0, get_unit_count() - 1 do
    if unit_alive(k) and (game.filter_bits(k) & BUILDING) ~= 0 then
      standing[unit_player(k)] = true
    end
  end
  for p = 0, PLAYERS - 1 do
    if standing[p] then
      had_building[p] = true
    elseif had_building[p] then
      -- p has lost its last building: the first player hostile to p names the winning side.
      for q = 0, PLAYERS - 1 do
        if q ~= p and game.hostile(p, q) then
          local side = game.team(q)
          if side ~= nil then
            return side
          end
        end
      end
    end
  end
  return nil
end

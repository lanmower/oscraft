-- Moba win condition: a side loses when its last living building falls. on_check_win(state) returns the side that
-- outlives it (the team of the first player hostile to the one that lost), or nil while no side has lost. The engine
-- declares the first side returned.
local config = require("config")
local units = require("units")

local had_building = {}  -- player -> true once the player held a living building

return {
  on_check_win = function()
    local standing = {}
    for k = 0, get_unit_count() - 1 do
      if unit_alive(k) and units.is_building(k) then
        standing[unit_player(k)] = true
      end
    end
    for p = 0, config.PLAYERS - 1 do
      if standing[p] then
        had_building[p] = true
      elseif had_building[p] then
        -- p has lost its last building: the first player hostile to p names the winning side.
        for q = 0, config.PLAYERS - 1 do
          if q ~= p and units.hostile(p, q) then
            local side = game.team(q)
            if side ~= nil then
              return side
            end
          end
        end
      end
    end
    return nil
  end,
}

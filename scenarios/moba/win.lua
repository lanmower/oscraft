local config = require("config")
local units = require("units")

local had_ancient = {}

return {
  on_check_win = function()
    local standing = {}
    for k = 0, get_unit_count() - 1 do
      if unit_alive(k) and game.unit_kind(k) == "base" then
        standing[unit_player(k)] = true
      end
    end
    for p = 0, config.PLAYERS - 1 do
      if standing[p] then
        had_ancient[p] = true
      elseif had_ancient[p] then
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

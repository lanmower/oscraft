local units = require("units")
local xp = require("xp")

local seen_level = {}

return {
  on_tick = function()
    for i = 0, get_unit_count() - 1 do
      if units.is_hero(i) then
        local level = xp.level_of(i)
        local before = seen_level[i]
        if before ~= nil and level > before then
          game.refresh_attr_stats(i)
        end
        seen_level[i] = level
      end
    end
  end,
}

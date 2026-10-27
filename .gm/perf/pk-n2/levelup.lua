-- Hero level-up as mod code. The engine gives no stats on a level gain: it only raises the xp counter. This
-- file reads xp.level_of for each hero every tick (the same read effects.lua uses for the level-up effect),
-- and on a gain re-derives the hero's attribute stats through game.refresh_attr_stats(unit).
--
-- game.refresh_attr_stats is a required native (not yet in the engine). The stat formula it stands for is
-- not ported to Lua here; see the port report.

local units = require("units")
local xp = require("xp")

local seen_level = {}  -- hero unit -> level at the last tick

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

-- Tower buff: when a tower dies, every surviving tower on its side gains the tower buff.
-- TOWER_BUFF is a behaviour kind index (scenarios/moba/behaviours.osd). Uses only the mod API.

local TOWER_BUFF = 1
local TOWER_BUFF_TICKS = 300

game.trigger("on_died", {
  conditions = {
    function(unit) return game.unit_kind(unit) == "tower" end,
  },
  action = function(unit)
    local side = unit_player(unit)
    for _, tower in ipairs(game.units(side)) do
      if game.unit_kind(tower) == "tower" then
        game.cmd_buff(tower, TOWER_BUFF, TOWER_BUFF_TICKS)
      end
    end
  end,
})

function on_tick(state) end

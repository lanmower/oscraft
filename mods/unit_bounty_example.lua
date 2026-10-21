-- Creep gold: when an enemy creep dies, the side that did not own it earns the bounty.
-- Uses only the mod API (docs/MODDING.md). Gold is kept per side and written to the state table.

local BOUNTY = 25
local gold = { [0] = 0, [1] = 0 }

game.trigger("on_died", {
  conditions = {
    function(unit) return game.unit_kind(unit) == "creep" end,
    function(unit) return unit_player(unit) == 0 or unit_player(unit) == 1 end,
  },
  action = function(unit)
    local reward = 1 - unit_player(unit)
    gold[reward] = gold[reward] + BOUNTY
  end,
})

function on_tick(state)
  state.gold_side0 = gold[0]
  state.gold_side1 = gold[1]
end

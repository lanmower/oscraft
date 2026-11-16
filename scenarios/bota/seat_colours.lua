local config = require("config")

local SENTINEL = { 0x0042ff, 0x1ce6b9, 0x540081, 0xfffc01, 0xff8000 }
local SCOURGE = { 0xe55bb0, 0x959697, 0x7ebff1, 0x106246, 0x4e2a04 }

local applied = false

local function colour_of(player)
  local index = player // 2 + 1
  if player % 2 == 0 then return SENTINEL[index] end
  return SCOURGE[index]
end

return {
  colour_of = colour_of,

  on_state_replaced = function() applied = false end,

  on_tick = function()
    if applied or game.ticks() < 1 then return end
    applied = true
    for player = 0, config.PLAYERS - 1 do
      if game.seat_start(player) ~= nil then
        game.player_name(player, "Player " .. tostring(player + 1))
        game.player_colour(player, colour_of(player))
      end
    end
  end,
}

local Bot = require("bot_lib")

local bots = {}
local bot_of = {}

local function seat(player)
  if bot_of[player] ~= nil then return end
  local bot = Bot.new(player, player)
  bot_of[player] = bot
  bots[#bots + 1] = bot
end

return {
  seat = seat,
  is_bot = function(player) return bot_of[player] ~= nil end,

  on_tick = function(state)
    local ticks = game.time()
    local units = nil
    for index, bot in ipairs(bots) do
      if (ticks + index) % Bot.DECIDE_EVERY == 0 then
        units = units or game.units(nil)
        Bot.think(bot, units, ticks)
      end
    end
    state.bot_walks = Bot.stats.walks
    state.bot_attacks = Bot.stats.attacks
    state.bot_retreats = Bot.stats.retreats
    state.bot_seats = #bots
  end,
}

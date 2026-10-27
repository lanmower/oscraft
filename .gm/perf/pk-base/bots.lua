-- Computer-controlled heroes: each bot seat's hero is driven by the bot library (bot_lib.lua), once every Bot.DECIDE_EVERY
-- ticks. A seat becomes a bot when its pick says so: controls.lua routes the "seat_pick" action, with args.bot, here.
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

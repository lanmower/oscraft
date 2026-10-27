
local BOT_SLOTS = { 0, 1, 2, 3, 4, 5, 6, 7, 8, 9 }

local bots = {}
local kills = 0
local is_bot = {}

for seat, slot in ipairs(BOT_SLOTS) do
  bots[#bots + 1] = Bot.new(slot, seat - 1)
  is_bot[slot] = true
end

game.timer("seat_bots", 1, function()
  for _, slot in ipairs(BOT_SLOTS) do game.add_player(slot, "computer") end
end)

game.trigger("on_died", {
  conditions = { function(_, killer, killer_player) return killer ~= nil and is_bot[killer_player] == true end },
  action = function() kills = kills + 1 end,
})

function on_tick(state)
  local ticks = game.time()
  local units = nil
  local holding = 0
  for index, bot in ipairs(bots) do
    if (ticks + index) % Bot.DECIDE_EVERY == 0 then
      units = units or game.units(nil)
      Bot.think(bot, units, ticks)
    end
    if bot.hero then bot.held = true end
    if bot.held then holding = holding + 1 end
  end
  state.heroes = holding
  state.walks = Bot.stats.walks
  state.retreats = Bot.stats.retreats
  state.attacks = Bot.stats.attacks
  state.blinks = Bot.stats.blinks
  state.helps = Bot.stats.helps
  state.heals = Bot.stats.heals
  state.kills = kills
  state.ticks_seen = ticks
end

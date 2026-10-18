-- Computer-player bots: one bot per hero slot, run through the mod API alone. Until the teams lane seats
-- computers by itself, the slots below are the list of bots this mod seats with game.add_player.
-- Join mods/bot_lib.lua ahead of this file to run it: cat bot_lib.lua bot_hero.lua > bots.lua.
-- state.heroes counts bots that have held a hero (a death at the last tick does not drop it), state.kills counts kills credited to a bot slot.

local BOT_SLOTS = { 0, 1, 2, 3, 4, 5, 6, 7, 8, 9 }
local DECIDE_EVERY = 8

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
    if (ticks + index) % DECIDE_EVERY == 0 then
      units = units or game.units(nil)
      Bot.think(bot, units, ticks)
    end
    if bot.hero then bot.held = true end
    if bot.held then holding = holding + 1 end
  end
  state.heroes = holding
  state.kills = kills
  state.ticks_seen = ticks
end

-- Unit queries and the game clock: gives a mage an ability, casts it, and reads the cooldown, the
-- ability list and the clock. Uses game.unit, game.ability, game.cmd_give_ability, game.cmd_cast,
-- game.unit_cooldown, game.unit_abilities and game.time. Uses only the mod API.

local TILE = 65536
local X, Y = 64 * TILE, 64 * TILE

game.ability("pulse", { cooldown = 50, target = "point", effects = { { kind = "damage", amount = 5, radius = 2000 } } })
game.unit("mage", { health = 80, speed = 1000 })

game.timer("setup", 1, function() game.cmd_spawn("mage", X, Y) end)

game.trigger("on_spawned", {
  conditions = { function(unit) return game.unit_kind(unit) == "mage" end },
  action = function(unit)
    game.var("mage", unit)
    game.cmd_give_ability(unit, 0, "pulse")
  end,
})

game.timer("cast", 8, function()
  game.var("ready_before", game.unit_cooldown(game.var("mage"), 0))
  game.cmd_cast(game.var("mage"), 0, -1, X, Y)
end)

game.timer("probe", 12, function()
  game.var("cooldown_after", game.unit_cooldown(game.var("mage"), 0))
end)

function on_tick(state)
  local mage = game.var("mage")
  if not mage then return end
  local abilities = game.unit_abilities(mage)
  state.ability_count = #abilities
  state.first_ability = abilities[1] and abilities[1].name or "none"
  state.ready_before = game.var("ready_before") or -1
  state.cooldown_after = game.var("cooldown_after") or -1
  local ticks, seconds = game.time()
  state.clock_ticks = ticks
  state.clock_seconds = seconds
end

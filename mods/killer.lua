
local TILE = 65536
local X, Y = 64 * TILE, 64 * TILE

game.unit("duelist", { health = 100, weapon = { damage = 9, range = 1000, period = 20 } })

game.timer("setup", 1, function()
  game.cmd_spawn("duelist", X, Y)
  game.cmd_spawn("duelist", X + TILE, Y)
end)

game.trigger("on_spawned", {
  conditions = { function(unit) return game.unit_kind(unit) == "duelist" end },
  action = function(unit)
    if game.var("killer") == nil then
      game.var("killer", unit)
    else
      game.var("victim", unit)
      game.cmd_set_owner(unit, 1)
    end
  end,
})

game.timer("strike", 6, function()
  game.cmd_damage(game.var("killer"), game.var("victim"), 200 * TILE)
end)

game.trigger("on_died", {
  conditions = { function(unit) return game.unit_kind(unit) == "duelist" end },
  action = function(unit, killer, killer_player)
    local ticks, seconds = game.time()
    game.var("died", unit)
    game.var("by", killer or -1)
    game.var("by_player", killer_player or -1)
    game.var("at", ticks)
  end,
})

function on_tick(state)
  state.killer = game.var("killer") or -1
  state.victim = game.var("victim") or -1
  state.died = game.var("died") or -1
  state.by = game.var("by") or -1
  state.by_player = game.var("by_player") or -1
  state.at = game.var("at") or -1
end

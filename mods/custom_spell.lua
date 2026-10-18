-- Custom spell and bounty: defines an ability from Lua, gives it to a spawned unit, casts it,
-- pays gold and awards the match to player 0. Uses game.ability, game.cmd_give_ability,
-- game.cmd_cast, the gold commands and game.victory. Uses only the mod API.

local TILE = 65536
local SPAWN_X, SPAWN_Y = 60 * TILE, 120 * TILE

game.ability("frost_nova", {
  cooldown = 50,
  energy = 0,
  target = "point",
  effects = {
    { kind = "damage", amount = 40, radius = 3000 },
    { kind = "buff", buff = 1, ticks = 60 },
  },
})

local function is_caster(unit)
  if game.unit_kind(unit) ~= "creep" then return false end
  local x, y = unit_position(unit)
  return x - SPAWN_X <= 2 * TILE and SPAWN_X - x <= 2 * TILE and y - SPAWN_Y <= 2 * TILE and SPAWN_Y - y <= 2 * TILE
end

game.trigger("on_spawned", {
  conditions = { is_caster },
  action = function(unit)
    game.var("caster", unit)
    game.cmd_give_ability(unit, 0, "frost_nova")
  end,
})

game.timer("setup", 1, function() game.cmd_spawn("creep", SPAWN_X, SPAWN_Y) end)

game.timer("cast", 10, function()
  local caster = game.var("caster")
  game.cmd_cast(caster, 0, -1, SPAWN_X, SPAWN_Y)
  game.cmd_add_gold(0, 250)
  game.victory(0)
end)

function on_tick(state)
  state.gold = game.gold(0)
  state.effects = #ability_info("frost_nova").effects
  state.winner = game.winner() or -1
end

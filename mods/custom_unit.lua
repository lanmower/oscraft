-- Custom unit: defines an ability and a tower and a runner from Lua, spawns them, and reads them
-- back. Uses game.ability, game.unit, game.cmd_spawn, game.unit_abilities and game.unit_kind.
-- Uses only the mod API.

local TILE = 65536
local TOWER_X, TOWER_Y = 20 * TILE, 40 * TILE

game.ability("tower_zap", {
  cooldown = 40,
  target = "unit",
  effects = { { kind = "damage", amount = 15 } },
})

game.unit("arrow_tower", {
  health = 400,
  armor = 2,
  radius = 600,
  sight = 9000,
  bounty = 30,
  model = "tower-square",
  weapon = { damage = 20, range = 6000, period = 20 },
  abilities = { "tower_zap" },
})

game.unit("runner", { health = 60, speed = 2500, weapon = { damage = 5, range = 800, period = 16 } })

game.timer("setup", 1, function()
  game.cmd_spawn("arrow_tower", TOWER_X, TOWER_Y)
  game.cmd_spawn("runner", TOWER_X + 4 * TILE, TOWER_Y)
end)

game.trigger("on_spawned", {
  conditions = { function(unit) return game.unit_kind(unit) == "arrow_tower" end },
  action = function(unit) game.var("tower", unit) end,
})

function on_tick(state)
  local tower = game.var("tower")
  if tower then
    state.tower_ability = game.unit_abilities(tower)[1].name
    state.tower_hp = unit_hp(tower)
  end
  state.runners = 0
  for _, unit in ipairs(game.units()) do
    if game.unit_kind(unit) == "runner" then state.runners = state.runners + 1 end
  end
end

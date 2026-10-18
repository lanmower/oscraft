-- Tech tree: defines a barracks, a knight and two upgrades from Lua, the second needing the first
-- and unlocking a champion, then grants them free. The upgrade raises knights that exist and knights
-- built later. Uses game.unit, game.upgrade, game.cmd_grant_upgrade and game.cmd_spawn. Uses only the mod API.

local TILE = 65536
local X, Y = 20 * TILE, 40 * TILE

game.unit("barracks", { health = 500, radius = 800, filter = 1 })
game.unit("knight", { health = 100, speed = 1500, weapon = { damage = 10, range = 1000, period = 20 } })
game.unit("champion", { health = 300, speed = 1500, cost = 200 })

game.upgrade("forged_armor", {
  building = "barracks",
  time = 40,
  cost = 100,
  effect = { kind = "stat", stat = "health", amount = 50 },
  filter = 0,
})

game.upgrade("master_armor", {
  building = "barracks",
  time = 80,
  cost = 250,
  effect = { kind = "stat", stat = "health", amount = 100 },
  requires = { "forged_armor" },
  unlocks = { "champion" },
})

game.timer("early", 1, function() game.cmd_spawn("knight", X, Y) end)
game.timer("grant", 4, function() game.cmd_grant_upgrade(0, "forged_armor") end)
game.timer("late", 8, function() game.cmd_spawn("knight", X + 2 * TILE, Y) end)

game.trigger("on_spawned", {
  conditions = { function(unit) return game.unit_kind(unit) == "knight" end },
  action = function(unit)
    if game.var("first") == nil then game.var("first", unit) else game.var("second", unit) end
  end,
})

function on_tick(state)
  local first, second = game.var("first"), game.var("second")
  if first then state.first_hp = unit_hp(first) end
  if second then state.second_hp = unit_hp(second) end
end

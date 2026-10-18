-- Custom item: defines an active ability and an item that casts it from Lua, spawns a
-- hero beside the base shop, buys the item and lists the hero's inventory. Uses game.ability,
-- game.item, game.cmd_add_gold, game.cmd_buy and game.unit_items. Uses only the mod API.

local TILE = 65536
local X, Y = 272 * TILE, 58 * TILE

game.ability("flask_heal", {
  cooldown = 80,
  target = "unit",
  effects = { { kind = "heal", amount = 40 } },
})

local FLASK = game.item("flask", { cost = 100, health = 20, armor = 1, ability = "flask_heal" })

game.trigger("on_spawned", {
  conditions = { function(unit)
    if game.unit_kind(unit) ~= "hero" then return false end
    local x, y = unit_position(unit)
    local dx, dy = x - X, y - Y
    return dx > -TILE and dx < TILE and dy > -TILE and dy < TILE
  end },
  action = function(unit)
    game.var("hero", unit)
    game.cmd_add_gold(0, 500)
  end,
})

game.timer("setup", 1, function() game.cmd_spawn("hero", X, Y) end)

game.timer("buy", 6, function() game.cmd_buy(game.var("hero"), FLASK) end)

function on_tick(state)
  local hero = game.var("hero")
  if not hero then return end
  local items = game.unit_items(hero)
  state.item_count = #items
  if items[1] then
    state.item_name = items[1].name
    state.item_cooldown = items[1].cooldown
  end
end

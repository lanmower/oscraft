
local TILE = 65536

game.timer("intro", 20, function()
  game.banner("Wave one", 4)
  game.message(-1, "Defend the base", 6)
  game.panel("objective", { title = "Objective", text = "Hold the centre for 30 seconds" })
  game.sound("horn")
end)

game.timer("pan", 60, function()
  game.camera(-1, 20 * TILE, 40 * TILE, 1.5, 2)
end)

game.timer("clear", 200, function()
  game.panel("objective")
end)

function on_tick(state) end

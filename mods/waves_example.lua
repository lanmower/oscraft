-- Tower defense waves: a repeating timer sends creeps at a goal region, each one that reaches it
-- costs a life, and the last life lost defeats player 0. Uses game.timer, game.region,
-- game.region_center, the variable stores, the score tally and game.defeat. Uses only the mod API.

local TILE = 65536
-- The open run on row y=34 of the moba GridNav (x 39..119); other cells are blocked, so a spawn
-- there is refused without a unit.
local SPAWN_X, SPAWN_Y = 40 * TILE, 34 * TILE
local LIVES = 3

game.region("goal", 44 * TILE, 32 * TILE, 48 * TILE, 36 * TILE, {
  enter = function(unit)
    if game.unit_var(unit, "wave") == nil or game.unit_var(unit, "leaked") then return end
    game.unit_var(unit, "leaked", true)
    local lives = game.player_var(1, "lives") - 1
    game.player_var(1, "lives", lives)
    game.add_score(1, 1)
    if lives <= 0 then
      game.cancel_timer("wave")
      game.defeat(0)
    end
  end,
})
local GOAL_X, GOAL_Y = game.region_center("goal")

if game.player_var(1, "lives") == nil then game.player_var(1, "lives", LIVES) end

local function is_wave_creep(unit)
  if game.unit_kind(unit) ~= "creep" then return false end
  local x, y = unit_position(unit)
  return x == SPAWN_X and y == SPAWN_Y
end

game.trigger("on_spawned", {
  conditions = { is_wave_creep },
  action = function(unit)
    game.unit_var(unit, "wave", game.var("waves"))
    game.cmd_order(unit, GOAL_X, GOAL_Y)
  end,
})

game.timer("wave", 5, function()
  game.var("waves", (game.var("waves") or 0) + 1)
  game.cmd_spawn("creep", SPAWN_X, SPAWN_Y)
end, 15)

function on_tick(state)
  state.waves = game.var("waves") or 0
  state.lives = game.player_var(1, "lives")
  state.score = game.score(1)
  state.winner = game.winner() or -1
end

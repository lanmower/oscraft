
game.timer("seat", 1, function()
  game.add_player(0, "human")
  game.add_player(1, "computer")
  game.add_player(2, "computer")
end)

game.timer("regroup", 2, function()
  game.set_team(1, 0)
  game.alliance(0, 3, "vision")
  game.remove_player(2)
end)

function on_tick(state)
  state.team1 = game.team(1)
  state.rel01 = game.alliance(0, 1)
  state.rel02 = game.alliance(0, 2)
  state.rel03 = game.alliance(0, 3)
  state.rel04 = game.alliance(0, 4)
  local seats = game.players()
  state.controller0 = seats[1].controller
  state.controller1 = seats[2].controller
  state.controller2 = seats[3].controller
end

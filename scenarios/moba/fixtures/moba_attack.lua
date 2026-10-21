-- Commands every side-0 unit to attack the first living side-1 unit, re-picked each tick so a dead target is never reused.
return {
  on_tick = function(state)
    local target = nil
    game.each({"owner"}, function(u)
      if u.owner == 1 and not target then target = u.id end
    end)
    game.each({"owner"}, function(u)
      if u.owner == 0 and target then game.cmd_attack(u.id, target) end
    end)
  end,
}

-- Commands one creep to attack another, then keeps the order queued until the target is gone.
local target
function on_tick(state)
  if not target then
    game.each({"owner"}, function(u)
      if u.owner == 1 and not target then target = u.id end
    end)
  end
  game.each({"owner"}, function(u)
    if u.owner == 0 and target then game.cmd_attack(u.id, target) end
  end)
end

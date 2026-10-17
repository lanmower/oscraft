-- Turns fog off, then orders every side-0 unit to attack the side-1 base each tick.
-- The base is the first side-1 unit in slot order: the moba map spawns it before any other side-1 unit.
local fog_off = false
local base_id
function on_tick(state)
  if not fog_off then
    game.cmd_set_fog(false)
    fog_off = true
  end
  if not base_id then
    game.each({"owner"}, function(u)
      if u.owner == 1 and not base_id then base_id = u.id end
    end)
  end
  if base_id then
    game.each({"owner"}, function(u)
      if u.owner == 0 then pcall(game.cmd_attack, u.id, base_id) end
    end)
  end
end

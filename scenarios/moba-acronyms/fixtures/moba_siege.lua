local fog_off = false
local base_id
return {
  on_tick = function(state)
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
        if u.owner == 0 then pcall(game.cmd_damage, u.id, base_id, 100 * 65536) end
      end)
    end
  end,
}

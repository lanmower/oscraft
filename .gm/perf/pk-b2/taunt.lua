-- Taunt as mod code: a taunted unit walks to the unit that taunted it. The engine keeps the TAUNT
-- countdown and the taunter record as generic status (the way it keeps stun and root). game.taunter(unit)
-- reads them: it returns the taunter's unit id while the TAUNT status runs and the taunter lives, else nil.
--
-- The forced strike target (the engine's taunt_target, read when a unit picks what to strike) is not
-- ported here: the engine has no hook that asks the mod for a strike target. See the port report.

return {
  on_tick = function()
    for i = 0, get_unit_count() - 1 do
      if unit_alive(i) then
        local taunter = game.taunter(i)
        if taunter ~= nil then
          local tx, ty = unit_position(taunter)
          game.cmd_order(i, tx, ty)
        end
      end
    end
  end,
}

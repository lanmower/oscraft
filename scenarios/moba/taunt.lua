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

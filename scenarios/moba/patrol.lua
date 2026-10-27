local ORDER_PATROL = 3

local M = {}

function M.start(id, x, y)
  game.queue_clear(id)
  return game.issue_order(id, ORDER_PATROL, x, y)
end

return M

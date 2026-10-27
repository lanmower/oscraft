local config = require("config")
local respawn = {}

function respawn.hero(level)
  return math.floor((config.rule("respawn_base_x10") + config.rule("respawn_per_level_x10") * level) / 10)
end

function respawn.schedule(r, now)
  r.free = r.policy.free == true
  r.due = r.policy.due(now)
end

function respawn.due(r, now, clear)
  if r.due == nil or now < r.due then return false end
  if r.free or clear(r) then
    r.due = nil
    return true
  end
  r.due = r.policy.due(now)
  return false
end

return respawn

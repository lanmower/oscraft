-- Respawn rules. linear: a hero returns base + per_level * level seconds after death, at most cap (rules read via config);
-- the engine's timer takes the seconds. A timed unit has a registry entry r with policy { free = bool, due = fn(now) }:
-- schedule(r, now) sets its due tick at death; due(r, now, clear) is asked once a tick while dead and returns true when
-- it comes back: at its due tick, if the policy is free or clear(r) holds, else the policy sets the next due tick.
local config = require("config")
local respawn = {}

function respawn.linear(level, base, per_level, cap)
  local secs = config.rule(base) + config.rule(per_level) * level
  local top = config.rule(cap)
  if secs > top then secs = top end
  return secs
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

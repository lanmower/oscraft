local config = require("config")

local speed_lock = {}

local SPEED_POINT = config.rule("speed_point_q16")
local UNTIL, VALUE = "speed_lock_until", "speed_lock_value"

function speed_lock.hold(unit, speed, until_tick)
  game.set_counter(unit, UNTIL, until_tick)
  game.set_counter(unit, VALUE, speed)
end

function speed_lock.speed(unit)
  if game.counter(unit, UNTIL) <= game.ticks() then return nil end
  return game.counter(unit, VALUE) * SPEED_POINT
end

return speed_lock

-- The pack's constants and rule values. config.rule(key) is the one reader of the pack's rules file (game.rule); a
-- missing rule is an error that names it.
local config = { TICKS_PER_SECOND = 16, FIX = 65536, PLAYERS = 16 }  -- ticks a second; Q16.16 units; player slots

function config.rule(key)
  local v = game.rule(key)
  assert(v ~= nil, "rule missing from the pack's rules file: " .. key)
  return v
end

-- Milli-units (1/1000 of a unit) to fixed point.
function config.milli(v) return math.floor(v * config.FIX / 1000) end

return config

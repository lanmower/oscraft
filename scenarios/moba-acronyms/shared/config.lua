local config = { TICKS_PER_SECOND = 16, FIX = 65536, PLAYERS = 16 }

function config.rule(key)
  local v = game.rule(key)
  assert(v ~= nil, "rule missing from the pack's rules file: " .. key)
  return v
end

function config.milli(v) return math.floor(v * config.FIX / 1000) end

return config

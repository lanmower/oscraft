local TICKS_PER_SECOND = 16

local function rule(key)
  local v = game.rule(key)
  assert(v ~= nil, "rule missing from the scenario's rules file: " .. key)
  return v
end

local cfg = nil

return {
  on_tick = function(state)
    if cfg == nil then
      cfg = { day = rule("day_s") * TICKS_PER_SECOND, night = rule("night_s") * TICKS_PER_SECOND }
    end
    local phase = game.time() % (cfg.day + cfg.night)
    game.set_clock(phase >= cfg.day, phase, cfg.day, cfg.night)
  end,
}

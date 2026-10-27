-- The day and night clock: a game rule of the pack. Day and night last day_s and night_s seconds (attr_rules.txt,
-- read through config.rule). The phase follows game.time(), so a loaded game resumes the phase of its saved tick.
-- The sim keeps the night flag and the cycle for the night sight and the client's dusk view (game.set_clock); it runs
-- no clock of its own. Runs before the other features, so they read this tick's phase.
local config = require("config")

local cfg = nil  -- the lengths in ticks, read on the first tick

return {
  on_tick = function(state)
    if cfg == nil then
      cfg = {
        day = config.rule("day_s") * config.TICKS_PER_SECOND,
        night = config.rule("night_s") * config.TICKS_PER_SECOND,
      }
    end
    local phase = game.time() % (cfg.day + cfg.night)
    game.set_clock(phase >= cfg.day, phase, cfg.day, cfg.night)
  end,
}

local config = require("config")

local cfg = nil

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

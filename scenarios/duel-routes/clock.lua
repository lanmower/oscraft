-- The day and night clock: a game rule of this scenario. Day and night last day_s and night_s seconds (attr_rules.txt,
-- read through game.rule). The phase follows game.time(), so a loaded game resumes the phase of its saved tick. The sim
-- keeps the night flag and the cycle for the night sight and the client's dusk view (game.set_clock).
local TICKS_PER_SECOND = 16  -- the engine's tick rate (sim lib.rs TICKS_PER_SECOND)

local function rule(key)
  local v = game.rule(key)
  assert(v ~= nil, "rule missing from the scenario's rules file: " .. key)
  return v
end

local cfg = nil  -- the lengths in ticks, read on the first tick

return {
  on_tick = function(state)
    if cfg == nil then
      cfg = { day = rule("day_s") * TICKS_PER_SECOND, night = rule("night_s") * TICKS_PER_SECOND }
    end
    local phase = game.time() % (cfg.day + cfg.night)
    game.set_clock(phase >= cfg.day, phase, cfg.day, cfg.night)
  end,
}

-- Protection for the moba scenario, as mod code, on core/reduction.lua. The engine applies no damage reduction or healing
-- by default; this file sets them through game.set_damage_reduction and game.set_healing.
--
-- Backdoor: a structure with no hostile plain unit within RADIUS for ACTIVATION_S seconds takes REDUCTION_PCT less damage
-- and regenerates REGEN_PER_S life a second. Glyph: the cast of glyph_of_fortification covers, for GLYPH_TICKS, every
-- unit of the caster's team that holds the GLYPH_NEED filter bits and none of GLYPH_SKIP, which takes no damage; the team
-- then waits GLYPH_COOLDOWN ticks before the next cast takes effect. The numbers are from scenarios/moba/abilities.osd, and
-- the radius scale (tower range) is UNVERIFIED.
local config = require("config")
local reduction = require("reduction")

local TICKS = config.TICKS_PER_SECOND
local SCAN_EVERY = 4

local backdoor = reduction.radius({ radius_milli = 14062, quiet = 15 * TICKS, pct = 50, heal = 90 })
local glyph = reduction.window({ span = 80, cooldown = 4800, need = 64, skip = 2048, pct = 100 })
local RULES = { backdoor, glyph }

local ticks = 0

return {
  on_cast = function(unit, ability)
    if ability ~= "glyph_of_fortification" then return end
    local p = unit_player(unit)
    if p >= config.PLAYERS then return end
    glyph.cast(game.team(p), ticks)
  end,

  on_tick = function()
    ticks = ticks + 1
    reduction.apply(RULES, ticks, ticks % SCAN_EVERY == 0)
  end,
}

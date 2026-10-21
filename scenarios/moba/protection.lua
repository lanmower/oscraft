-- Protection for the moba scenario, as mod code. The engine applies no damage reduction, invulnerability or
-- healing by default; this file sets them through game.set_damage_reduction and game.set_healing.
--
-- Backdoor: a structure with no hostile plain unit (a living non-structure, non-hero unit of an enemy player)
-- within RADIUS for ACTIVATION_S seconds takes REDUCTION_PCT less damage and regenerates REGEN_PER_S life a
-- second. Values are from scenarios/moba/hero_rules.txt (backdoor_*).
--
-- Glyph: the cast of glyph_of_fortification (on_cast) covers, for GLYPH_TICKS, every unit of the caster's team
-- whose filter holds all GLYPH_NEED bits and none of the GLYPH_SKIP bits: it takes no damage. The team then waits
-- GLYPH_COOLDOWN ticks before the next cast takes effect. Numbers are from scenarios/moba/abilities.osd.
--
-- This file holds the moba on_tick. The moba mod is this file, then march.lua, then win.lua, in one chunk: a
-- host runs one mod file, and march.lua and win.lua define no on_tick of their own.

local RADIUS_MILLI = 14062
local ACTIVATION_S = 15
local REDUCTION_PCT = 50
local REGEN_PER_S = 90
local TICKS_PER_SECOND = 16
local SCAN_EVERY = 4
local GLYPH_NAME = "glyph_of_fortification"
local GLYPH_TICKS = 80
local GLYPH_COOLDOWN = 4800
local GLYPH_NEED = 64    -- filter bit 6, structure
local GLYPH_SKIP = 2048  -- filter bit 11, building

-- Same conversion as the sim's fixed::from_milli (65536 per unit, milli-units are 1/1000).
local RADIUS_RAW = RADIUS_MILLI * 65536
local RADIUS = (RADIUS_RAW - RADIUS_RAW % 1000) / 1000  -- whole fixed units; the sandbox has no math library
local ACTIVATION_TICKS = ACTIVATION_S * TICKS_PER_SECOND

local ticks = 0
local last_threat = {}   -- structure id -> tick a hostile plain unit was last within RADIUS
local backdoor = {}      -- structure id -> 1 while its backdoor is active
local applied = {}       -- unit id -> the damage reduction last sent
local applied_heal = {}  -- structure id -> the healing last sent
local glyph_until = {}   -- team -> tick its glyph ends
local glyph_ready = {}   -- team -> tick it may glyph again

local function threatened(structure, owner, x, y)
  for _, k in ipairs(game.units_near(x, y, RADIUS)) do
    if k ~= structure and not game.is_structure(k) and not game.is_hero(k)
      and unit_player(k) < 16 and game.hostile(owner, unit_player(k)) then
      return true
    end
  end
  return false
end

-- Whether unit i is covered by a glyph running for its team.
local function glyph_covers(i)
  local p = unit_player(i)
  if p >= 16 then return false end  -- neutral units (player 255) have no team
  local team = game.team(p)
  if team == nil or (glyph_until[team] or 0) <= ticks then return false end
  local bits = game.filter_bits(i)
  return (bits & GLYPH_NEED) == GLYPH_NEED and (bits & GLYPH_SKIP) == 0
end

function on_cast(unit, ability, state)
  if ability ~= GLYPH_NAME then return end
  local p = unit_player(unit)
  if p >= 16 then return end
  local team = game.team(p)
  if team == nil or (glyph_ready[team] or 0) > ticks then return end
  glyph_until[team] = ticks + GLYPH_TICKS
  glyph_ready[team] = ticks + GLYPH_COOLDOWN
end

function on_tick(state)
  ticks = ticks + 1
  local scan = ticks % SCAN_EVERY == 0
  for i = 0, get_unit_count() - 1 do
    if unit_alive(i) then
      if scan and game.is_structure(i) then
        local x, y = unit_position(i)
        if threatened(i, unit_player(i), x, y) then last_threat[i] = ticks end
        local quiet = ticks - (last_threat[i] or 0)
        backdoor[i] = quiet >= ACTIVATION_TICKS
        local heal = backdoor[i] and REGEN_PER_S or 0
        if applied_heal[i] ~= heal then
          game.set_healing(i, heal)
          applied_heal[i] = heal
        end
      end
      local pct = (backdoor[i] and REDUCTION_PCT or 0)
      if glyph_covers(i) then pct = 100 end
      if applied[i] ~= pct then
        game.set_damage_reduction(i, pct)
        applied[i] = pct
      end
    end
  end
end

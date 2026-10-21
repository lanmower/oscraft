-- Backdoor protection for the moba scenario, as a mod. The engine applies no damage reduction by
-- default; this file sets it through game.set_damage_reduction. A structure with no hostile plain unit
-- (a living non-structure, non-hero unit of an enemy player) within RADIUS for ACTIVATION_S seconds
-- takes REDUCTION_PCT less damage. Values are from scenarios/moba/hero_rules.txt (backdoor_*).
-- Regeneration (backdoor_regen_per_s) is not applied: the engine exposes no heal command to a mod.
-- Load this file as the mod for the moba scenario. A host runs one mod file, so combine it with
-- other moba mods into one file if they run together.

local RADIUS_MILLI = 14062
local ACTIVATION_S = 15
local REDUCTION_PCT = 50
local TICKS_PER_SECOND = 16
local SCAN_EVERY = 4

-- Same conversion as the sim's fixed::from_milli (65536 per unit, milli-units are 1/1000).
local RADIUS_RAW = RADIUS_MILLI * 65536
local RADIUS = (RADIUS_RAW - RADIUS_RAW % 1000) / 1000  -- whole fixed units; the sandbox has no math library
local ACTIVATION_TICKS = ACTIVATION_S * TICKS_PER_SECOND

local ticks = 0
local last_threat = {}  -- structure id -> tick a hostile plain unit was last within RADIUS
local applied = {}      -- structure id -> the reduction last sent

local function threatened(structure, owner, x, y)
  for _, k in ipairs(game.units_near(x, y, RADIUS)) do
    if k ~= structure and not game.is_structure(k) and not game.is_hero(k)
      and game.hostile(owner, unit_player(k)) then
      return true
    end
  end
  return false
end

function on_tick(state)
  ticks = ticks + 1
  if ticks % SCAN_EVERY ~= 0 then return end
  for i = 0, get_unit_count() - 1 do
    if unit_alive(i) and game.is_structure(i) then
      local x, y = unit_position(i)
      if threatened(i, unit_player(i), x, y) then last_threat[i] = ticks end
      local quiet = ticks - (last_threat[i] or 0)
      local pct = quiet >= ACTIVATION_TICKS and REDUCTION_PCT or 0
      if applied[i] ~= pct then
        game.set_damage_reduction(i, pct)
        applied[i] = pct
      end
    end
  end
end

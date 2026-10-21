-- Moba spells as mod code: the teleport landing ring and the ward caller. The engine offers generic primitives only:
-- game.teleport(unit, x, y) (move to a point if walkable, true when it took hold), game.timed_spawn(caster, kind, life,
-- x, y) (place a unit that expires after life ticks), and game.cast_point(caster) / game.cast_target(caster). on_cast
-- dispatches a cast that took hold by its ability name.

-- Landing ring for the town-portal style teleport: eight directions, widening by RING_STEP_MILLI per step over
-- RING_STEPS steps, measured from the receiver in milli-units.
local RING_DIRECTIONS = {
  { 1000, 0 }, { 707, 707 }, { 0, 1000 }, { -707, 707 },
  { -1000, 0 }, { -707, -707 }, { 0, -1000 }, { 707, -707 },
}
local RING_STEPS = 6
local RING_BASE_MILLI = 1000
local RING_STEP_MILLI = 1000

-- Ward kinds and lifetimes: unit type index in units.osd and lifetime in ticks (16 ticks a second).
local WARDS = {
  place_observer = { kind = 18, life = 5760 },
  place_sentry = { kind = 19, life = 6720 },
}

local function land_beside(caster, receiver)
  local rx, ry = unit_position(receiver)
  for step = 1, RING_STEPS do
    local reach = RING_BASE_MILLI + RING_STEP_MILLI * (step - 1)
    for _, d in ipairs(RING_DIRECTIONS) do
      local x = rx + d[1] * reach / 1000
      local y = ry + d[2] * reach / 1000
      if game.teleport(caster, math.floor(x), math.floor(y)) then
        return true
      end
    end
  end
  return false
end

local HANDLERS = {
  town_portal = function(caster)
    local receiver = game.cast_target(caster)
    if receiver ~= nil then
      land_beside(caster, receiver)
    end
  end,
}

for name, ward in pairs(WARDS) do
  HANDLERS[name] = function(caster)
    local x, y = game.cast_point(caster)
    game.timed_spawn(caster, ward.kind, ward.life, x, y)
  end
end

return {
  on_cast = function(caster, ability_name)
    local handler = HANDLERS[ability_name]
    if handler ~= nil then
      handler(caster)
    end
  end,
}

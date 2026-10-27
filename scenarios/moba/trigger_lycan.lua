local lib = require("trigger_lib")
local speed_lock = require("speed_lock")

local TICKS = lib.TICKS

local SHAPESHIFT = {
  source = "war3map.j TAO 69069-69130 (every tick the caster and all of its player's units hold 650 through d3), TRO 69063, A093 ability.ini:20479 (Dur 1.5 + HeroDur 16.5 s)",
  ability = "lycan_shapeshift",
  speed = 650,
  duration = 18 * TICKS,
  stagger = 4,
  refresh = 8,
}

local shifted = lib.registry()

local function shapeshift_cast(caster)
  if lib.rank_of(caster, SHAPESHIFT.ability) == 0 then return end
  game.set_counter(caster, "shapeshift_due", game.ticks() + SHAPESHIFT.duration)
  shifted.add(caster)
end

local function shapeshift_step(caster)
  local now = game.ticks()
  if game.counter(caster, "shapeshift_due") <= now or not game.alive(caster) then
    game.set_counter(caster, "shapeshift_due", 0)
    shifted.remove(caster)
    return
  end
  local owner = unit_player(caster)
  local until_tick = math.min(game.counter(caster, "shapeshift_due"), now + SHAPESHIFT.refresh)
  for unit = now % SHAPESHIFT.stagger, get_unit_count() - 1, SHAPESHIFT.stagger do
    if game.alive(unit) and unit_player(unit) == owner then speed_lock.hold(unit, SHAPESHIFT.speed, until_tick) end
  end
end

local function rebuild()
  shifted.reset(lib.counted("shapeshift_due"))
end

return {
  on_cast = function(caster, ability_name)
    if ability_name == SHAPESHIFT.ability then shapeshift_cast(caster) end
  end,

  on_tick = function()
    for _, unit in ipairs(shifted.items()) do shapeshift_step(unit) end
  end,

  on_state_replaced = rebuild,
}

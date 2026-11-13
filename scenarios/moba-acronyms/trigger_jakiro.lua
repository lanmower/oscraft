local config = require("config")
local lib = require("trigger_lib")

local TICKS = lib.TICKS
local UNIT_Q16 = 1024
local DIRECTION_SCALE = 1000000

local function map_units(count)
  return count * UNIT_Q16
end

local function store_line(caster, prefix, rank)
  local cx, cy = unit_position(caster)
  local px, py = game.cast_point(caster)
  local dx, dy = lib.direction(cx, cy, px, py)
  game.set_counter(caster, prefix .. "_start", game.ticks())
  game.set_counter(caster, prefix .. "_x", cx)
  game.set_counter(caster, prefix .. "_y", cy)
  game.set_counter(caster, prefix .. "_dx", math.floor(dx * DIRECTION_SCALE))
  game.set_counter(caster, prefix .. "_dy", math.floor(dy * DIRECTION_SCALE))
  game.set_counter(caster, prefix .. "_rank", rank)
end

local function line_point(caster, prefix, distance)
  local x = game.counter(caster, prefix .. "_x") + game.counter(caster, prefix .. "_dx") * distance // DIRECTION_SCALE
  local y = game.counter(caster, prefix .. "_y") + game.counter(caster, prefix .. "_dy") * distance // DIRECTION_SCALE
  return x, y
end

local function clear_line(caster, prefix)
  game.set_counter(caster, prefix .. "_start", 0)
end

local ICE = {
  source = "war3map.j SZX 50159-50215 (twelve segments 0.05 s apart, formed after 11 steps), SYX 50113-50158 (a circle of 175 around each segment 100 apart, each unit once: 50 magical, stun (0.6+0.5*rank) s less 0.05 s per step, for (0.6+0.4*rank) s), SWX 50101-50112, A0O6 ability.ini:37309",
  ability = "jakiro_ice_path",
  prefix = "icepath",
  segments = 12,
  spacing = map_units(100),
  radius = config.milli(2734),
  form_ticks = 9,
  damage = 50,
  stun_base_ms = 600,
  stun_per_rank_ms = 500,
  hold_base_ms = 600,
  hold_per_rank_ms = 400,
}

local paths = lib.registry()

local function ice_cast(caster)
  local rank = lib.rank_of(caster, ICE.ability)
  if rank == 0 then return end
  store_line(caster, ICE.prefix, rank)
  paths.add(caster)
end

local function on_path(caster, unit)
  local ux, uy = unit_position(unit)
  local ox, oy = line_point(caster, ICE.prefix, 0)
  local dx = game.counter(caster, ICE.prefix .. "_dx")
  local dy = game.counter(caster, ICE.prefix .. "_dy")
  local along = ((ux - ox) * dx + (uy - oy) * dy) // DIRECTION_SCALE
  local nearest = math.min(math.max((along + ICE.spacing // 2) // ICE.spacing, 0), ICE.segments - 1)
  local sx, sy = line_point(caster, ICE.prefix, ICE.spacing * nearest)
  return (ux - sx) * (ux - sx) + (uy - sy) * (uy - sy) <= ICE.radius * ICE.radius
end

local function ice_step(caster)
  local start = game.counter(caster, ICE.prefix .. "_start")
  if start == 0 then
    paths.remove(caster)
    return
  end
  local rank = game.counter(caster, ICE.prefix .. "_rank")
  local formed = start + ICE.form_ticks
  local held_ticks = (ICE.hold_base_ms + ICE.hold_per_rank_ms * rank) * TICKS // 1000
  local now = game.ticks()
  if now >= formed + held_ticks then
    clear_line(caster, ICE.prefix)
    paths.remove(caster)
    return
  end
  if now < formed then return end
  local steps = (now - formed) * 5 // 4 + 1
  local stun_ms = ICE.stun_base_ms + ICE.stun_per_rank_ms * rank - steps * 50
  local stun_ticks = math.max(stun_ms * TICKS // 1000, 1)
  local cx, cy = line_point(caster, ICE.prefix, ICE.spacing * (ICE.segments - 1) // 2)
  local reach = ICE.spacing * ICE.segments // 2 + ICE.radius
  local id = start * 1024 + caster
  for _, unit in ipairs(lib.enemies_at(caster, cx, cy, reach)) do
    if game.counter(unit, "icepath_hit") ~= id and not lib.held(unit, "spell_immunity") and on_path(caster, unit) then
      game.set_counter(unit, "icepath_hit", id)
      game.spell_damage(caster, unit, ICE.damage, lib.SCRIPT_MAGICAL)
      game.apply_status(unit, "stun", stun_ticks)
    end
  end
end

local PYRE = {
  source = "war3map.j IZE 49892-49932 (flames at 150*n toward the cast point, n = 1..6, one every 0.1 s), SJX 49861-49891, SKX 49937-49960 (A0OE: Rt*20+30 per 0.5 s for 10 s within 240), A0OE ability.ini:37658",
  ability = "jakiro_macropyre",
  prefix = "macropyre",
  flames = 6,
  spacing = map_units(150),
  radius = config.milli(3750),
  lead_tenths = 1,
  lifetime = 10 * TICKS,
  period = TICKS // 2,
  damage = { 50, 70, 90 },
}

local pyres = lib.registry()

local function pyre_cast(caster)
  local rank = math.min(lib.rank_of(caster, PYRE.ability), #PYRE.damage)
  if rank == 0 then return end
  store_line(caster, PYRE.prefix, rank)
  pyres.add(caster)
end

local function flame_start(start, flame)
  return start + flame * PYRE.lead_tenths * TICKS // 10
end

local function pyre_step(caster)
  local start = game.counter(caster, PYRE.prefix .. "_start")
  if start == 0 then
    pyres.remove(caster)
    return
  end
  local now = game.ticks()
  if now > flame_start(start, PYRE.flames) + PYRE.lifetime then
    clear_line(caster, PYRE.prefix)
    pyres.remove(caster)
    return
  end
  local rank = game.counter(caster, PYRE.prefix .. "_rank")
  for flame = 1, PYRE.flames do
    local age = now - flame_start(start, flame)
    if age > 0 and age <= PYRE.lifetime and age % PYRE.period == 0 then
      local x, y = line_point(caster, PYRE.prefix, PYRE.spacing * flame)
      for _, unit in ipairs(lib.enemies_at(caster, x, y, PYRE.radius)) do
        if not lib.held(unit, "spell_immunity") then game.spell_damage(caster, unit, PYRE.damage[rank], lib.SCRIPT_MAGICAL) end
      end
    end
  end
end

local function rebuild()
  paths.reset(lib.counted(ICE.prefix .. "_start"))
  pyres.reset(lib.counted(PYRE.prefix .. "_start"))
end

return {
  on_cast = function(caster, ability_name)
    if ability_name == ICE.ability then
      ice_cast(caster)
    elseif ability_name == PYRE.ability then
      pyre_cast(caster)
    end
  end,

  on_tick = function()
    for _, unit in ipairs(paths.items()) do ice_step(unit) end
    for _, unit in ipairs(pyres.items()) do pyre_step(unit) end
  end,

  on_state_replaced = rebuild,
}

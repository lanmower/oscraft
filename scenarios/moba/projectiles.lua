local config = require("config")
local units = require("units")
local lib = require("trigger_lib")

local TICKS_PER_SECOND = 16
local WHOLE = 65536
local MAGICAL = 2
local PURE = 3
local MAX_HIT = 32767
local UNITS_PER_CELL = 64

local function cell_units(fixed_distance)
  return fixed_distance / WHOLE * UNITS_PER_CELL
end

local function unit_fixed(units_distance)
  return units_distance / UNITS_PER_CELL * WHOLE
end

local ARROW = {
  source = "war3map.j GWE 51944-51961, WHX 51973-52022, WFX 51965-51968 (A0L8): the arrow flies to the target point and stops at the first enemy within 140; its distance bucket is min(floor(units flown / 150), 11); damage 90 x rank - 40 + 14 per bucket past the first, stun 0.5 s per bucket past the first; a non-hero unit takes 9999 (units of 2500 life or more are spared here, GUESS for the script's exemption list); 30 units a step at 1000 a second is GUESS",
  ability = "mirana_elune_s_arrow",
  speed = 62.5,
  radius_milli = 2188,
  bucket_units = 150,
  bucket_cap = 11,
  damage_per_rank = 90,
  damage_base = -40,
  damage_per_bucket = 14,
  stun_ticks_per_bucket = 8,
  spared_life = 2500,
}

local SONIC_WAVE = {
  source = "war3map.j CER 80661-80708 (A...): a wave of 800 units at 800 a second, collision radius growing from 100 to 450, 200 + 90 x rank universal damage once to each enemy; the knockback is not modelled",
  ability = "queenofpain_sonic_wave",
  speed = 50,
  distance = 800,
  radius_start = 100,
  radius_end = 450,
  base = 200,
  per_rank = 90,
}

local DEATH_PULSE = {
  source = "war3map.j BZE 70943-70957, UTO 70929: a ring of 500 units at 400 a second; enemies take 75 + 50 x rank magical damage and allies including Necrophos heal 50 + 20 x rank as the ring front passes",
  ability = "necrolyte_death_pulse",
  speed = 25,
  distance = 500,
  damage_base = 75,
  damage_per_rank = 50,
  heal_base = 50,
  heal_per_rank = 20,
}

local flights = {}

local function launch(flight)
  flights[#flights + 1] = flight
end

local function from_caster_to_point(caster)
  local x, y = unit_position(caster)
  local tx, ty = game.cast_point(caster)
  if tx == nil then return nil end
  local dx, dy = lib.direction(x, y, tx, ty)
  return x, y, dx, dy, math.sqrt((tx - x) * (tx - x) + (ty - y) * (ty - y))
end

local function arrow_hit(flight, victim)
  local flown = cell_units(flight.flown)
  local bucket = math.min(math.floor(flown / ARROW.bucket_units), ARROW.bucket_cap)
  local past = math.max(0, bucket - 1)
  if units.is_hero(victim) then
    local damage = ARROW.damage_per_rank * flight.rank + ARROW.damage_base + ARROW.damage_per_bucket * past
    game.spell_damage(flight.caster, victim, damage, MAGICAL)
    game.apply_status(victim, "stun", math.max(1, ARROW.stun_ticks_per_bucket * past))
  elseif game.max_hp(victim) < ARROW.spared_life * WHOLE then
    game.spell_damage(flight.caster, victim, MAX_HIT, MAGICAL)
    game.apply_status(victim, "stun", math.max(1, ARROW.stun_ticks_per_bucket * past))
  end
end

local function start_arrow(caster)
  local rank = lib.rank_of(caster, ARROW.ability)
  local x, y, dx, dy, reach = from_caster_to_point(caster)
  if rank == 0 or x == nil then return end
  launch({ kind = "line", caster = caster, rank = rank, x = x, y = y, dx = dx, dy = dy, flown = 0, reach = reach,
    step = unit_fixed(ARROW.speed), radius = config.milli(ARROW.radius_milli), first_only = true, hit = {}, on_hit = arrow_hit })
end

local function wave_hit(flight, victim)
  game.spell_damage(flight.caster, victim, SONIC_WAVE.base + SONIC_WAVE.per_rank * flight.rank, PURE)
end

local function start_sonic_wave(caster)
  local rank = lib.rank_of(caster, SONIC_WAVE.ability)
  local x, y, dx, dy = from_caster_to_point(caster)
  if rank == 0 or x == nil then return end
  launch({ kind = "line", caster = caster, rank = rank, x = x, y = y, dx = dx, dy = dy, flown = 0, reach = unit_fixed(SONIC_WAVE.distance),
    step = unit_fixed(SONIC_WAVE.speed), radius = unit_fixed(SONIC_WAVE.radius_start), grow = unit_fixed(SONIC_WAVE.radius_end - SONIC_WAVE.radius_start) / unit_fixed(SONIC_WAVE.distance),
    first_only = false, hit = {}, on_hit = wave_hit })
end

local function pulse_hit(flight, victim)
  local owner = unit_player(flight.caster)
  if game.hostile(owner, unit_player(victim)) then
    game.spell_damage(flight.caster, victim, DEATH_PULSE.damage_base + DEATH_PULSE.damage_per_rank * flight.rank, MAGICAL)
  else
    game.set_hp(victim, unit_hp(victim) + (DEATH_PULSE.heal_base + DEATH_PULSE.heal_per_rank * flight.rank) * WHOLE)
  end
end

local function start_death_pulse(caster)
  local rank = lib.rank_of(caster, DEATH_PULSE.ability)
  if rank == 0 then return end
  local x, y = unit_position(caster)
  launch({ kind = "ring", caster = caster, rank = rank, x = x, y = y, flown = 0, reach = unit_fixed(DEATH_PULSE.distance),
    step = unit_fixed(DEATH_PULSE.speed), hit = {}, on_hit = pulse_hit })
end

local HOOKSHOT = {
  source = "tooltip ability.ini (A... Hookshot): a hook of 2000 range that latches the first enemy it meets, stuns it 1/1.5/2 s, deals 100/200/300 magical damage and pulls Clockwerk beside it; the 2000 a second flight speed and the 100 unit radius are GUESS",
  ability = "rattletrap_hookshot",
  speed = 125,
  radius_milli = 1562,
  damage = { 100, 200, 300 },
  stun_ticks = { 16, 24, 32 },
}

local TIMBER_CHAIN = {
  source = "war3map.j:88205 and tooltip ability.ini:85653 (Timber Chain): the chain flies to the target point or the first tree on the way, pulls Shredder to it and deals 60 + 40 x rank magical damage to enemies within 150 of the end; the 3000 a second flight speed is GUESS",
  ability = "shredder_timber_chain",
  speed = 187.5,
  damage_base = 60,
  damage_per_rank = 40,
  end_radius = 2344,
}

local function beside(caster, x, y, dx, dy)
  game.teleport(caster, math.floor(x - dx * WHOLE), math.floor(y - dy * WHOLE))
end

local function hookshot_hit(flight, victim)
  game.spell_damage(flight.caster, victim, HOOKSHOT.damage[math.min(flight.rank, #HOOKSHOT.damage)], MAGICAL)
  game.apply_status(victim, "stun", HOOKSHOT.stun_ticks[math.min(flight.rank, #HOOKSHOT.stun_ticks)])
  local x, y = unit_position(victim)
  beside(flight.caster, x, y, flight.dx, flight.dy)
end

local function start_hookshot(caster)
  local rank = lib.rank_of(caster, HOOKSHOT.ability)
  local x, y, dx, dy, reach = from_caster_to_point(caster)
  if rank == 0 or x == nil then return end
  launch({ kind = "line", caster = caster, rank = rank, x = x, y = y, dx = dx, dy = dy, flown = 0, reach = reach,
    step = unit_fixed(HOOKSHOT.speed), radius = config.milli(HOOKSHOT.radius_milli), first_only = true, hit = {}, on_hit = hookshot_hit })
end

local function chain_end(flight, x, y)
  local owner = unit_player(flight.caster)
  for _, unit in ipairs(game.units_near(math.floor(x), math.floor(y), config.milli(TIMBER_CHAIN.end_radius))) do
    if game.alive(unit) and not game.is_structure(unit) and game.hostile(owner, unit_player(unit)) then
      game.spell_damage(flight.caster, unit, TIMBER_CHAIN.damage_base + TIMBER_CHAIN.damage_per_rank * flight.rank, MAGICAL)
    end
  end
  beside(flight.caster, x, y, flight.dx, flight.dy)
end

local function start_timber_chain(caster)
  local rank = lib.rank_of(caster, TIMBER_CHAIN.ability)
  local x, y, dx, dy, reach = from_caster_to_point(caster)
  if rank == 0 or x == nil then return end
  launch({ kind = "line", caster = caster, rank = rank, x = x, y = y, dx = dx, dy = dy, flown = 0, reach = reach,
    step = unit_fixed(TIMBER_CHAIN.speed), radius = 0, first_only = false, hit = {}, on_hit = function() end, stop_at_tree = true, on_end = chain_end })
end

local ILLUSORY_ORB = {
  source = "ability.ini:41696-41748 and war3map.j:49293-49310 (Illusory Orb): the orb flies 1950 units from the cast, hitting each enemy it passes for 70 magical damage; a second cast while it flies moves Puck to it; the 600 a second speed and the 150 unit radius are GUESS",
  ability = "puck_illusory_orb",
  speed = 37.5,
  distance = 1950,
  radius_milli = 2344,
  damage = { 70, 140, 210, 280 },
  cooldown = 176,
}
local orbs = {}

local function orb_hit(flight, victim)
  game.spell_damage(flight.caster, victim, ILLUSORY_ORB.damage[math.min(flight.rank, #ILLUSORY_ORB.damage)], MAGICAL)
end

local function ride_orb(caster)
  local flight = orbs[caster]
  if flight == nil or flight.ended then return false end
  local x, y = flight.x + flight.dx * flight.flown, flight.y + flight.dy * flight.flown
  game.teleport(caster, math.floor(x), math.floor(y))
  flight.ended = true
  orbs[caster] = nil
  for _, row in ipairs(game.unit_abilities(caster)) do
    if row.name == ILLUSORY_ORB.ability then game.start_cooldown(caster, row.slot, ILLUSORY_ORB.cooldown) end
  end
  return true
end

local function start_orb(caster)
  local rank = lib.rank_of(caster, ILLUSORY_ORB.ability)
  if rank == 0 then return end
  if ride_orb(caster) then return end
  local x, y, dx, dy = from_caster_to_point(caster)
  if x == nil then return end
  local flight = { kind = "line", caster = caster, rank = rank, x = x, y = y, dx = dx, dy = dy, flown = 0, reach = unit_fixed(ILLUSORY_ORB.distance),
    step = unit_fixed(ILLUSORY_ORB.speed), radius = config.milli(ILLUSORY_ORB.radius_milli), first_only = false, hit = {}, on_hit = orb_hit }
  orbs[caster] = flight
  launch(flight)
  for _, row in ipairs(game.unit_abilities(caster)) do
    if row.name == ILLUSORY_ORB.ability then game.start_cooldown(caster, row.slot, 0) end
  end
end

local STARTS = {
  [ARROW.ability] = start_arrow,
  [SONIC_WAVE.ability] = start_sonic_wave,
  [DEATH_PULSE.ability] = start_death_pulse,
  [HOOKSHOT.ability] = start_hookshot,
  [TIMBER_CHAIN.ability] = start_timber_chain,
  [ILLUSORY_ORB.ability] = start_orb,
}

local function sweep(flight, x, y, radius)
  local owner = unit_player(flight.caster)
  local taken = 0
  for _, unit in ipairs(game.units_near(x, y, math.floor(radius))) do
    if game.alive(unit) and not game.is_structure(unit) and not flight.hit[unit] and (flight.kind == "ring" or unit ~= flight.caster)
      and (flight.kind == "ring" or game.hostile(owner, unit_player(unit))) then
      flight.hit[unit] = true
      flight.on_hit(flight, unit)
      taken = taken + 1
      if flight.first_only then return taken end
    end
  end
  return taken
end

local function tick_flights()
  local kept = {}
  for _, flight in ipairs(flights) do
    flight.flown = flight.flown + flight.step
    local done = flight.flown >= flight.reach
    if flight.kind == "line" then
      local x, y = flight.x + flight.dx * flight.flown, flight.y + flight.dy * flight.flown
      local radius = flight.radius + (flight.grow or 0) * flight.flown
      if flight.radius > 0 and sweep(flight, math.floor(x), math.floor(y), radius) > 0 and flight.first_only then done = true end
      if flight.stop_at_tree and not done and #game.trees_near(math.floor(x), math.floor(y), WHOLE) > 0 then done = true end
      if done and flight.on_end ~= nil and game.alive(flight.caster) then flight.on_end(flight, x, y) end
    else
      sweep(flight, math.floor(flight.x), math.floor(flight.y), flight.flown)
    end
    if flight.ended then done = true end
    if done and orbs[flight.caster] == flight then
      orbs[flight.caster] = nil
      for _, row in ipairs(game.unit_abilities(flight.caster)) do
        if row.name == ILLUSORY_ORB.ability then game.start_cooldown(flight.caster, row.slot, ILLUSORY_ORB.cooldown) end
      end
    end
    if not done and game.alive(flight.caster) then kept[#kept + 1] = flight end
  end
  flights = kept
end

return {
  on_cast = function(caster, ability_name)
    local start = STARTS[ability_name]
    if start ~= nil then start(caster) end
  end,
  on_tick = tick_flights,
  on_state_replaced = function()
    flights = {}
    orbs = {}
  end,
}

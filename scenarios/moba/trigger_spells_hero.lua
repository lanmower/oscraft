local config = require("config")
local units = require("units")

local TICKS_PER_SECOND = 16
local WHOLE = 65536
local MAGICAL = 2
local S_DAMAGE, S_SPEED_PCT = 0, 19
local SCEPTER_BIT = 1

local function slot_cache(ability)
  local cache = {}
  local function lookup(unit)
    local slot = cache[unit]
    if slot == nil then
      slot = false
      for _, row in ipairs(game.unit_abilities(unit)) do
        if row.name == ability then slot = row.slot end
      end
      cache[unit] = slot
    end
    return slot
  end
  return lookup, function() cache = {} end
end

local function rank_in(lookup, unit)
  local slot = lookup(unit)
  return slot and game.skill_rank(unit, slot) or 0
end

local function has_scepter(unit)
  return math.floor(game.upgrade_bits(unit) / SCEPTER_BIT) % 2 == 1
end

local function held(unit, status)
  return game.status_left(unit, status) > 0
end

local function foes_at(owner, x, y, radius)
  local found = {}
  for _, unit in ipairs(game.units_near(x, y, config.milli(radius))) do
    if game.alive(unit) and not game.is_structure(unit) and game.hostile(owner, unit_player(unit)) then
      found[#found + 1] = unit
    end
  end
  return found
end

local BACKTRACK = {
  source = "war3map.j J3O 65214-65222 (A0CZ): a damage event on the holder is healed back with chance 5 + 5 x rank percent",
  ability = "faceless_void_backtrack",
  chance_base = 5,
  chance_per_rank = 5,
}
local backtrack_slot, reset_backtrack_slots = slot_cache(BACKTRACK.ability)

function BACKTRACK.hit(attacker, victim, dealt)
  if dealt <= 0 or not game.alive(victim) or not game.is_attributed(victim) then return end
  local rank = rank_in(backtrack_slot, victim)
  if rank == 0 or held(victim, "break") then return end
  if game.random(100) + 1 <= BACKTRACK.chance_base + BACKTRACK.chance_per_rank * rank then
    game.set_hp(victim, unit_hp(victim) + dealt)
  end
end

local PRESS_THE_ATTACK = {
  source = "ability.ini A2J2 tooltip: the target is strongly dispelled",
  casts = { "legion_commander_press_the_attack" },
  dispelled = { "stun", "slow", "silence", "root", "taunt", "hex", "disarm", "mute", "break" },
}

function PRESS_THE_ATTACK.run(caster)
  local target = game.cast_target(caster)
  if target == nil or not game.alive(target) then return end
  for _, status in ipairs(PRESS_THE_ATTACK.dispelled) do game.clear_status(target, status) end
  game.clear_dots(target)
  game.clear_timed_mods(target, 0)
end

local ODDS = {
  source = "war3map.j DWE 87894-87919, MTR 87892, MUR 87896 (A2JB): radius 355 (5547 raw) at the target point; each enemy takes 40 x rank + (12 + 2 x rank) per non-hero + (5 + 15 x rank) per hero counted; the caster gains 9 per hero + 3 per non-hero move speed percent for 7 s (tooltip)",
  ability = "legion_commander_overwhelming_odds",
  casts = { "legion_commander_overwhelming_odds" },
  radius = 5547,
  base = 40,
  unit_base = 12,
  unit_per_rank = 2,
  hero_base = 5,
  hero_per_rank = 15,
  move_per_hero = 9,
  move_per_unit = 3,
  duration = 7 * TICKS_PER_SECOND,
}
local odds_slot = slot_cache(ODDS.ability)
local hasted = {}

local function set_odds_speed(unit, amount, due)
  game.add_stat(unit, S_SPEED_PCT, amount - game.counter(unit, "odds_move"))
  game.set_counter(unit, "odds_move", amount)
  game.set_counter(unit, "odds_due", due)
end

function ODDS.run(caster)
  local rank = rank_in(odds_slot, caster)
  if rank == 0 then return end
  local x, y = game.cast_point(caster)
  local foes = foes_at(unit_player(caster), x, y, ODDS.radius)
  local heroes, others = 0, 0
  for _, unit in ipairs(foes) do
    if units.is_hero(unit) then heroes = heroes + 1 else others = others + 1 end
  end
  local amount = ODDS.base * rank + (ODDS.unit_base + ODDS.unit_per_rank * rank) * others + (ODDS.hero_base + ODDS.hero_per_rank * rank) * heroes
  for _, unit in ipairs(foes) do game.spell_damage(caster, unit, amount, MAGICAL) end
  local speed = ODDS.move_per_hero * heroes + ODDS.move_per_unit * others
  if speed > 0 then
    set_odds_speed(caster, speed, game.ticks() + ODDS.duration)
    hasted[caster] = true
  end
end

local function expire_odds()
  for unit in pairs(hasted) do
    if game.ticks() >= game.counter(unit, "odds_due") or not game.alive(unit) then
      set_odds_speed(unit, 0, 0)
      hasted[unit] = nil
    end
  end
end

local function rebuild_odds()
  hasted = {}
  for unit = 0, get_unit_count() - 1 do
    if game.counter(unit, "odds_move") ~= 0 then hasted[unit] = true end
  end
end

local DUEL = {
  source = "ability.ini A2CI tooltips: both fighters are forced to attack each other for 4 / 4.75 / 5.5 s; the winner of a duel to the death keeps +10 / 14 / 18 damage",
  ability = "legion_commander_duel",
  casts = { "legion_commander_duel" },
  ticks = { 64, 76, 88 },
  bonus = { 10, 14, 18 },
}
local duel_slot = slot_cache(DUEL.ability)
local duelling = {}

function DUEL.run(caster)
  local rank = math.min(rank_in(duel_slot, caster), #DUEL.ticks)
  local target = game.cast_target(caster)
  if rank == 0 or target == nil or not game.alive(target) or game.counter(caster, "duel_foe") ~= 0 then return end
  game.set_counter(caster, "duel_foe", target + 1)
  game.set_counter(caster, "duel_due", game.ticks() + DUEL.ticks[rank])
  game.set_counter(caster, "duel_bonus", DUEL.bonus[rank])
  duelling[caster] = true
end

local function end_duel(caster, foe)
  local bonus = game.counter(caster, "duel_bonus")
  game.set_counter(caster, "duel_foe", 0)
  game.set_counter(caster, "duel_due", 0)
  game.set_counter(caster, "duel_bonus", 0)
  duelling[caster] = nil
  if game.alive(caster) and not game.alive(foe) then
    game.add_stat(caster, S_DAMAGE, bonus)
    game.set_counter(caster, "duel_won", game.counter(caster, "duel_won") + bonus)
  end
end

local function tick_duels()
  for caster in pairs(duelling) do
    local foe = game.counter(caster, "duel_foe") - 1
    if not game.alive(caster) or not game.alive(foe) or game.ticks() >= game.counter(caster, "duel_due") then
      end_duel(caster, foe)
    else
      local fx, fy = unit_position(foe)
      local cx, cy = unit_position(caster)
      game.cmd_order(caster, fx, fy)
      game.cmd_order(foe, cx, cy)
    end
  end
end

local function rebuild_duels()
  duelling = {}
  for unit = 0, get_unit_count() - 1 do
    if game.counter(unit, "duel_foe") ~= 0 then duelling[unit] = true end
  end
end

local PULSE_NOVA = {
  source = "war3map.j SZO 68655-68702, BFE 68703 (A21F, scepter A21G): toggle; each second, if mana holds, 20 x rank mana is spent and every enemy within 475 (7422 raw) takes 100 / 130 / 160 (scepter 160 / 190 / 220) damage",
  ability = "leshrac_pulse_nova",
  casts = { "leshrac_pulse_nova" },
  radius = 7422,
  damage = { 100, 130, 160 },
  scepter_damage = { 160, 190, 220 },
  mana_per_rank = 20,
  pulse_ticks = TICKS_PER_SECOND,
}
local nova_slot = slot_cache(PULSE_NOVA.ability)
local novas = {}

function PULSE_NOVA.run(caster)
  if game.counter(caster, "nova_on") == 1 then
    game.set_counter(caster, "nova_on", 0)
    novas[caster] = nil
  else
    game.set_counter(caster, "nova_on", 1)
    novas[caster] = true
  end
end

local function nova_pulse(unit)
  local rank = math.min(rank_in(nova_slot, unit), #PULSE_NOVA.damage)
  local cost = PULSE_NOVA.mana_per_rank * rank * WHOLE
  if rank == 0 or game.mana(unit) < cost then
    game.set_counter(unit, "nova_on", 0)
    novas[unit] = nil
    return
  end
  game.set_mana(unit, game.mana(unit) - cost)
  local damage = (has_scepter(unit) and PULSE_NOVA.scepter_damage or PULSE_NOVA.damage)[rank]
  local x, y = unit_position(unit)
  for _, other in ipairs(foes_at(unit_player(unit), x, y, PULSE_NOVA.radius)) do
    game.spell_damage(unit, other, damage, MAGICAL)
  end
end

local function tick_novas()
  if game.ticks() % PULSE_NOVA.pulse_ticks ~= 0 then return end
  for unit in pairs(novas) do
    if game.alive(unit) then
      nova_pulse(unit)
    else
      game.set_counter(unit, "nova_on", 0)
      novas[unit] = nil
    end
  end
end

local function rebuild_novas()
  novas = {}
  for unit = 0, get_unit_count() - 1 do
    if game.counter(unit, "nova_on") == 1 then novas[unit] = true end
  end
end

local TIME_WALK = {
  source = "war3map.j J1O 65094-65132 (A0LK; slows A294-A297, ability.ini:82987-83045): every enemy within 325 of the path is slowed 20 / 30 / 40 / 50 percent for 3 s",
  ability = "faceless_void_time_walk",
  kind = "faceless_void",
  casts = { "faceless_void_time_walk" },
  radius = 5078,
  slow_multiplier = { 52429, 45875, 39322, 32768 },
  slow_ticks = 3 * TICKS_PER_SECOND,
}
local walk_slot = slot_cache(TIME_WALK.ability)
local walkers = {}

local function distance_to_segment(px, py, ax, ay, bx, by)
  local dx, dy = bx - ax, by - ay
  local length = dx * dx + dy * dy
  local t = 0
  if length > 0 then t = math.max(0, math.min(1, ((px - ax) * dx + (py - ay) * dy) / length)) end
  local cx, cy = ax + t * dx, ay + t * dy
  return math.sqrt((px - cx) ^ 2 + (py - cy) ^ 2)
end

function TIME_WALK.run(caster)
  local rank = math.min(rank_in(walk_slot, caster), #TIME_WALK.slow_multiplier)
  local from = walkers[caster]
  if rank == 0 or from == nil then return end
  local bx, by = unit_position(caster)
  local half_length = math.sqrt((bx - from[1]) ^ 2 + (by - from[2]) ^ 2) / 2
  local reach = half_length + config.milli(TIME_WALK.radius)
  local near = game.units_near(math.floor((from[1] + bx) / 2), math.floor((from[2] + by) / 2), math.floor(reach))
  for _, unit in ipairs(near) do
    if game.alive(unit) and not game.is_structure(unit) and game.hostile(unit_player(caster), unit_player(unit)) then
      local ux, uy = unit_position(unit)
      if distance_to_segment(ux, uy, from[1], from[2], bx, by) <= config.milli(TIME_WALK.radius) then
        game.apply_status(unit, "slow", TIME_WALK.slow_ticks, TIME_WALK.slow_multiplier[rank])
      end
    end
  end
end

local function track_walkers()
  for unit = 0, get_unit_count() - 1 do
    if game.alive(unit) and game.unit_kind(unit) == TIME_WALK.kind then
      local x, y = unit_position(unit)
      walkers[unit] = { x, y }
    end
  end
end

local CALL_DOWN = {
  source = "war3map.j F6R 82804-82812 (A1T5, scepter A235): the second missile, 4 s after the cast, deals 75 more damage with the scepter; the global cast range of the scepter version is not modelled",
  ability = "gyrocopter_call_down",
  casts = { "gyrocopter_call_down" },
  radius = 9375,
  delay = 4 * TICKS_PER_SECOND + 1,
  bonus = 75,
}
local pending_calldowns = {}

function CALL_DOWN.run(caster)
  if not has_scepter(caster) then return end
  local x, y = game.cast_point(caster)
  game.set_counter(caster, "calldown_x", x)
  game.set_counter(caster, "calldown_y", y)
  game.set_counter(caster, "calldown_due", game.ticks() + CALL_DOWN.delay)
  pending_calldowns[caster] = true
end

local function tick_calldowns()
  for caster in pairs(pending_calldowns) do
    if game.ticks() >= game.counter(caster, "calldown_due") then
      pending_calldowns[caster] = nil
      game.set_counter(caster, "calldown_due", 0)
      if game.alive(caster) then
        for _, unit in ipairs(foes_at(unit_player(caster), game.counter(caster, "calldown_x"), game.counter(caster, "calldown_y"), CALL_DOWN.radius)) do
          if not held(unit, "spell_immunity") then game.spell_damage(caster, unit, CALL_DOWN.bonus, MAGICAL) end
        end
      end
    end
  end
end

local function rebuild_calldowns()
  pending_calldowns = {}
  for unit = 0, get_unit_count() - 1 do
    if game.counter(unit, "calldown_due") > 0 then pending_calldowns[unit] = true end
  end
end

local HOMING = {
  source = "war3map.j F0R 82689-82717 (A1SQ): the missile waits 3 s, then flies at 280 + 20 x t units per second to the target; the hit deals min(distance from the cast point, 1750) / 1750 x 125 x rank magical damage, at least 50, and stuns 2 + 0.2 x rank s",
  ability = "gyrocopter_homing_missile",
  casts = { "gyrocopter_homing_missile" },
  wait_seconds = 3,
  start_ahead = 150,
  speed_base = 280,
  speed_growth = 20,
  distance_cap = 1750,
  damage_per_rank = 125,
  damage_floor = 50,
  stun_seconds_base = 2,
  stun_seconds_per_rank = 0.2,
  units_per_cell = 64,
}
local homing_slot = slot_cache(HOMING.ability)
local flying = {}

local function game_units(fixed_distance)
  return fixed_distance * HOMING.units_per_cell / config.FIX
end

local function flight_seconds(units)
  local travel = math.max(0, units - HOMING.start_ahead)
  local base, growth, wait = HOMING.speed_base, HOMING.speed_growth, HOMING.wait_seconds
  local already = base * wait + growth * wait * wait / 2
  local total = (-base + math.sqrt(base * base + 2 * growth * (travel + already))) / growth
  return math.max(wait, total)
end

function HOMING.run(caster)
  local rank = rank_in(homing_slot, caster)
  local target = game.cast_target(caster)
  if rank == 0 or target == nil then return end
  local x, y = unit_position(caster)
  local tx, ty = unit_position(target)
  local units = game_units(math.sqrt((tx - x) ^ 2 + (ty - y) ^ 2))
  game.set_counter(caster, "homing_target", target + 1)
  game.set_counter(caster, "homing_x", x)
  game.set_counter(caster, "homing_y", y)
  game.set_counter(caster, "homing_due", game.ticks() + math.ceil(flight_seconds(units) * TICKS_PER_SECOND))
  flying[caster] = true
end

local function land_homing(caster)
  local target = game.counter(caster, "homing_target") - 1
  local ox, oy = game.counter(caster, "homing_x"), game.counter(caster, "homing_y")
  game.set_counter(caster, "homing_target", 0)
  game.set_counter(caster, "homing_due", 0)
  flying[caster] = nil
  local rank = rank_in(homing_slot, caster)
  if rank == 0 or not game.alive(target) then return end
  local tx, ty = unit_position(target)
  local units = math.min(game_units(math.sqrt((tx - ox) ^ 2 + (ty - oy) ^ 2)), HOMING.distance_cap)
  local damage = math.max(HOMING.damage_floor, math.floor(units / HOMING.distance_cap * HOMING.damage_per_rank * rank))
  game.spell_damage(caster, target, damage, MAGICAL)
  local stun_seconds = HOMING.stun_seconds_base + HOMING.stun_seconds_per_rank * rank
  game.apply_status(target, "stun", math.floor(stun_seconds * TICKS_PER_SECOND + 0.5))
end

local function tick_homing()
  for caster in pairs(flying) do
    if game.ticks() >= game.counter(caster, "homing_due") then land_homing(caster) end
  end
end

local function rebuild_homing()
  flying = {}
  for unit = 0, get_unit_count() - 1 do
    if game.counter(unit, "homing_target") > 0 then flying[unit] = true end
  end
end

local LIGHTNING_SLOW = {
  source = "ability.ini A06V tooltip (Researchubertip): every unit the storm hits is slowed 75 percent for 0.7 / 0.8 / 0.9 / 1 s; the chain jumps one per tick, so hits by the caster within the window after the cast are the storm's",
  ability = "leshrac_lightning_storm",
  casts = { "leshrac_lightning_storm" },
  window_ticks = 10,
  slow_ticks = { 11, 13, 14, 16 },
  slow_multiplier = 16384,
}
local storm_slot = slot_cache(LIGHTNING_SLOW.ability)

function LIGHTNING_SLOW.run(caster)
  game.set_counter(caster, "storm_until", game.ticks() + LIGHTNING_SLOW.window_ticks)
end

function LIGHTNING_SLOW.hit(attacker, victim)
  if attacker < 0 or game.counter(attacker, "storm_until") < game.ticks() or not game.alive(victim) then return end
  local rank = math.min(rank_in(storm_slot, attacker), #LIGHTNING_SLOW.slow_ticks)
  if rank == 0 or not game.hostile(unit_player(attacker), unit_player(victim)) or game.is_structure(victim) then return end
  game.apply_status(victim, "slow", LIGHTNING_SLOW.slow_ticks[rank], LIGHTNING_SLOW.slow_multiplier)
end

local TIME_LOCK = {
  source = "ability.ini A081 tooltip (ini:18327): the bonus damage is doubled while the target is in Chronosphere (war3map.j UZR:92269 is silent, so a GUESS: the proc's 30 + 10 x rank magical damage is dealt a second time to the unit the caster is attacking)",
  ability = "faceless_void_time_lock",
  chronosphere = "faceless_void_chronosphere",
  radius = 7422,
  damage_base = 30,
  damage_per_rank = 10,
  damage_type = MAGICAL,
  duration_ticks = { 64, 72, 80 },
  hit_window_ticks = 3,
}
local time_lock_slot = slot_cache(TIME_LOCK.ability)
local chronosphere_slot = slot_cache(TIME_LOCK.chronosphere)

function TIME_LOCK.cast_chronosphere(caster)
  local rank = math.min(rank_in(chronosphere_slot, caster), #TIME_LOCK.duration_ticks)
  if rank == 0 then return end
  local x, y = game.cast_point(caster)
  game.set_counter(caster, "chrono_x", x)
  game.set_counter(caster, "chrono_y", y)
  game.set_counter(caster, "chrono_due", game.ticks() + TIME_LOCK.duration_ticks[rank])
end

local function struck_by(unit, caster)
  for _, hitter in ipairs(game.recent_hitters(unit, TIME_LOCK.hit_window_ticks)) do
    if hitter == caster then return true end
  end
  return false
end

function TIME_LOCK.proc(caster)
  local rank = rank_in(time_lock_slot, caster)
  if rank == 0 or game.counter(caster, "chrono_due") <= game.ticks() then return end
  local zone_x, zone_y = game.counter(caster, "chrono_x"), game.counter(caster, "chrono_y")
  local owner = unit_player(caster)
  for _, unit in ipairs(foes_at(owner, zone_x, zone_y, TIME_LOCK.radius)) do
    if struck_by(unit, caster) then
      game.spell_damage(caster, unit, TIME_LOCK.damage_base + TIME_LOCK.damage_per_rank * rank, TIME_LOCK.damage_type)
      game.set_counter(caster, "time_lock_doubled", game.counter(caster, "time_lock_doubled") + 1)
    end
  end
end

local FLAK = {
  source = "war3map.j GER 82930-82970 (A229 granting A1TW, ability.ini:71230 Area 1000): the next 3 + rank attacks within 15 s each also hit every other enemy within 1000 of the attacker (the script counts attacks to 3 + rank; the tooltip says 3/4/5/6)",
  ability = "gyrocopter_flak_cannon",
  casts = { "gyrocopter_flak_cannon" },
  radius = 15625,
  attacks_base = 3,
  duration = 15 * TICKS_PER_SECOND,
  physical = 1,
}
local flak_slot = slot_cache(FLAK.ability)

function FLAK.run(caster)
  local rank = rank_in(flak_slot, caster)
  if rank == 0 then return end
  game.set_counter(caster, "flak_left", FLAK.attacks_base + rank)
  game.set_counter(caster, "flak_due", game.ticks() + FLAK.duration)
end

function FLAK.hit(attacker, victim)
  if game.counter(attacker, "flak_left") <= 0 then return end
  if game.ticks() >= game.counter(attacker, "flak_due") then
    game.set_counter(attacker, "flak_left", 0)
    return
  end
  game.set_counter(attacker, "flak_left", game.counter(attacker, "flak_left") - 1)
  local damage = game.weapon(attacker)
  local x, y = unit_position(attacker)
  for _, unit in ipairs(foes_at(unit_player(attacker), x, y, FLAK.radius)) do
    if unit ~= victim then game.spell_damage(attacker, unit, math.floor(damage / WHOLE), FLAK.physical) end
  end
end

local WRATH_SCEPTER = {
  source = "ability.ini A0AL (scepter A1W9, war3map.j IWE 49452): damage 155 / 210 / 275 against 140 / 180 / 225 and 18 targets against 16; the extra damage is dealt to each chained unit, the two extra targets are not modelled",
  ability = "furion_wrath_of_nature",
  casts = { "furion_wrath_of_nature" },
  window_ticks = 64,
  bonus = { 15, 30, 50 },
}
local wrath_slot = slot_cache(WRATH_SCEPTER.ability)

function WRATH_SCEPTER.run(caster)
  if not has_scepter(caster) then return end
  game.set_counter(caster, "wrath_until", game.ticks() + WRATH_SCEPTER.window_ticks)
  game.set_counter(caster, "wrath_cast", game.ticks())
end

function WRATH_SCEPTER.hit(attacker, victim)
  if attacker < 0 or game.counter(attacker, "wrath_until") < game.ticks() then return end
  local cast = game.counter(attacker, "wrath_cast")
  if game.counter(victim, "wrath_boosted") == cast + 1 or not game.alive(victim) then return end
  local rank = math.min(rank_in(wrath_slot, attacker), #WRATH_SCEPTER.bonus)
  if rank == 0 or not game.hostile(unit_player(attacker), unit_player(victim)) or game.is_structure(victim) then return end
  game.set_counter(victim, "wrath_boosted", cast + 1)
  game.spell_damage(attacker, victim, WRATH_SCEPTER.bonus[rank], MAGICAL)
end

local SPECS = { PRESS_THE_ATTACK, ODDS, DUEL, PULSE_NOVA, TIME_WALK, CALL_DOWN, HOMING, LIGHTNING_SLOW, FLAK, WRATH_SCEPTER, { casts = { "faceless_void_chronosphere" }, run = TIME_LOCK.cast_chronosphere } }

local ON_CAST = {}
for _, spec in ipairs(SPECS) do
  for _, name in ipairs(spec.casts) do
    local runs = ON_CAST[name]
    if runs == nil then
      runs = {}
      ON_CAST[name] = runs
    end
    runs[#runs + 1] = spec.run
  end
end

return {
  on_cast = function(caster, ability_name)
    local runs = ON_CAST[ability_name]
    if runs == nil then return end
    for _, run in ipairs(runs) do run(caster) end
  end,
  on_proc = function(caster, ability_name)
    if ability_name == TIME_LOCK.ability then TIME_LOCK.proc(caster) end
  end,
  on_hit = function(attacker, victim, dealt, state, attack)
    BACKTRACK.hit(attacker, victim, dealt)
    LIGHTNING_SLOW.hit(attacker, victim)
    WRATH_SCEPTER.hit(attacker, victim)
    if attack and attacker >= 0 and game.alive(attacker) then FLAK.hit(attacker, victim) end
  end,
  on_tick = function()
    expire_odds()
    tick_duels()
    tick_novas()
    tick_calldowns()
    tick_homing()
    track_walkers()
  end,
  on_state_replaced = function()
    reset_backtrack_slots()
    odds_slot = slot_cache(ODDS.ability)
    duel_slot = slot_cache(DUEL.ability)
    nova_slot = slot_cache(PULSE_NOVA.ability)
    rebuild_odds()
    rebuild_duels()
    rebuild_novas()
    walk_slot = slot_cache(TIME_WALK.ability)
    walkers = {}
    rebuild_calldowns()
    homing_slot = slot_cache(HOMING.ability)
    storm_slot = slot_cache(LIGHTNING_SLOW.ability)
    flak_slot = slot_cache(FLAK.ability)
    wrath_slot = slot_cache(WRATH_SCEPTER.ability)
    time_lock_slot = slot_cache(TIME_LOCK.ability)
    chronosphere_slot = slot_cache(TIME_LOCK.chronosphere)
    rebuild_homing()
  end,
}

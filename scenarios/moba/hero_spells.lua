local config = require("config")
local spells = require("spells")

local TICKS = config.TICKS_PER_SECOND
local WHOLE = config.FIX
local MAGICAL = 2
local S_SPEED_PCT = 19
local RAW_PER_REFERENCE_UNIT = 1024
local DIRECTION_SCALE = 1000000

local RETIRED_CASTS = {
  enigma_demonic_conversion = true,
  ember_spirit_fire_remnant_learning = true,
  ember_spirit_fire_remnant_activate = true,
}

local function rank_of(unit, ability)
  for _, row in ipairs(game.unit_abilities(unit)) do
    if row.name == ability then return game.skill_rank(unit, row.slot) end
  end
  return 0
end

local function held(unit, status)
  return game.status_left(unit, status) > 0
end

local function reference_units(count)
  return count * RAW_PER_REFERENCE_UNIT
end

local function tracker(counter)
  local members = {}
  return {
    add = function(unit) members[unit] = true end,
    members = function() return members end,
    rebuild = function()
      members = {}
      for unit = 0, get_unit_count() - 1 do
        if game.counter(unit, counter) ~= 0 then members[unit] = true end
      end
    end,
  }
end

local function others_near(x, y, radius, keep)
  local found = {}
  for _, unit in ipairs(game.units_near(x, y, radius)) do
    if game.alive(unit) and not game.is_structure(unit) and keep(unit) then found[#found + 1] = unit end
  end
  return found
end

local X_MARKS = {
  source = "war3map.j GUX 44015-44090, 43984-43992 (A13D return); duration 4 s for an enemy and 8 s for an ally (ability.ini tooltip)",
  casts = { "kunkka_x_marks_the_spot" },
  enemy_ticks = 4 * TICKS,
  ally_ticks = 8 * TICKS,
}

local marked = tracker("xmark_due")

function X_MARKS.run(caster)
  local target = game.cast_target(caster)
  if target == nil or not game.alive(target) then return end
  local ally = not game.hostile(unit_player(caster), unit_player(target))
  local x, y = unit_position(target)
  game.set_counter(target, "xmark_x", x)
  game.set_counter(target, "xmark_y", y)
  game.set_counter(target, "xmark_ally", ally and 1 or 0)
  game.set_counter(target, "xmark_due", game.ticks() + (ally and X_MARKS.ally_ticks or X_MARKS.enemy_ticks))
  marked.add(target)
end

function X_MARKS.tick()
  for unit in pairs(marked.members()) do
    local due = game.counter(unit, "xmark_due")
    if game.ticks() >= due or not game.alive(unit) then
      local returnable = game.alive(unit)
        and (game.counter(unit, "xmark_ally") == 1 or not (held(unit, "spell_immunity") or held(unit, "invulnerable")))
      if returnable then game.teleport(unit, game.counter(unit, "xmark_x"), game.counter(unit, "xmark_y")) end
      game.set_counter(unit, "xmark_due", 0)
      marked.members()[unit] = nil
    end
  end
end

local GHOST_SHIP = {
  source = "war3map.j RWE 44427-44490 (A11K), HRX 44308-44367, G4X 44179-44187, G2X 44151-44171, G1X 44122-44149, GZX 44092-44121 (A3OF)",
  ability = "kunkka_ghost_ship",
  casts = { "kunkka_ghost_ship" },
  travel_ticks = 48,
  travel_units = 2000,
  start_behind = 1000,
  crash_radius = 450,
  pickup_radius = 425,
  stun_ticks = 22,
  damage_base = 300,
  damage_per_rank = 100,
  buff_ticks = 10 * TICKS,
  drip_pulses = 10,
  move_pct = 10,
  deferred_base = 20,
  deferred_per_rank = 10,
}

local ships = tracker("ship_t0")
local buffed = tracker("gs_active")

local function ship_position(caster)
  local progress = (game.ticks() - game.counter(caster, "ship_t0")) / GHOST_SHIP.travel_ticks
  local distance = reference_units(GHOST_SHIP.travel_units) * progress
  return game.counter(caster, "ship_x0") + math.floor(game.counter(caster, "ship_dx") * distance / DIRECTION_SCALE),
    game.counter(caster, "ship_y0") + math.floor(game.counter(caster, "ship_dy") * distance / DIRECTION_SCALE)
end

function GHOST_SHIP.run(caster)
  local rank = rank_of(caster, GHOST_SHIP.ability)
  if rank == 0 then return end
  local x, y = unit_position(caster)
  local tx, ty = game.cast_point(caster)
  local dx, dy = tx - x, ty - y
  if dx == 0 and dy == 0 then dx = WHOLE end
  local length = math.sqrt(dx * dx + dy * dy)
  game.set_counter(caster, "ship_x0", x - math.floor(dx * reference_units(GHOST_SHIP.start_behind) / length))
  game.set_counter(caster, "ship_y0", y - math.floor(dy * reference_units(GHOST_SHIP.start_behind) / length))
  game.set_counter(caster, "ship_dx", math.floor(dx * DIRECTION_SCALE / length))
  game.set_counter(caster, "ship_dy", math.floor(dy * DIRECTION_SCALE / length))
  game.set_counter(caster, "ship_rank", rank)
  game.set_counter(caster, "ship_t0", game.ticks())
  ships.add(caster)
end

local function board(caster, unit, rank)
  game.set_counter(unit, "gs_rank", rank)
  game.set_counter(unit, "gs_until", game.ticks() + GHOST_SHIP.buff_ticks)
  local active = game.counter(unit, "gs_active")
  if active ~= 1 then
    game.add_stat(unit, S_SPEED_PCT, GHOST_SHIP.move_pct)
    game.set_counter(unit, "gs_active", 1)
    game.set_counter(unit, "gs_drip_left", 0)
    if active == 0 then game.set_counter(unit, "gs_owed", 0) end
    buffed.add(unit)
  end
end

local function crash(caster, rank, x, y)
  local owner = unit_player(caster)
  local enemies = others_near(x, y, config.milli(GHOST_SHIP.crash_radius * 15.625), function(unit)
    return game.hostile(owner, unit_player(unit)) and not held(unit, "spell_immunity")
  end)
  for _, unit in ipairs(enemies) do
    game.apply_status(unit, "stun", GHOST_SHIP.stun_ticks)
    game.spell_damage(caster, unit, GHOST_SHIP.damage_base + GHOST_SHIP.damage_per_rank * rank, MAGICAL)
  end
end

function GHOST_SHIP.tick_ships()
  for caster in pairs(ships.members()) do
    local rank = game.counter(caster, "ship_rank")
    local x, y = ship_position(caster)
    local owner = unit_player(caster)
    local allies = others_near(x, y, config.milli(GHOST_SHIP.pickup_radius * 15.625), function(unit)
      return game.is_attributed(unit) and not game.hostile(owner, unit_player(unit))
    end)
    for _, unit in ipairs(allies) do board(caster, unit, rank) end
    if game.ticks() - game.counter(caster, "ship_t0") >= GHOST_SHIP.travel_ticks then
      crash(caster, rank, x, y)
      game.set_counter(caster, "ship_t0", 0)
      ships.members()[caster] = nil
    end
  end
end

function GHOST_SHIP.hit(attacker, victim, dealt)
  if game.counter(victim, "gs_active") == 0 or game.counter(victim, "gs_until") <= game.ticks() or not game.alive(victim) then return end
  local rank = game.counter(victim, "gs_rank")
  local deferred = GHOST_SHIP.deferred_base + GHOST_SHIP.deferred_per_rank * rank
  local share = math.floor(dealt * deferred / 100)
  game.set_hp(victim, unit_hp(victim) + share)
  game.set_counter(victim, "gs_owed", game.counter(victim, "gs_owed") + share)
end

function GHOST_SHIP.tick_buffs()
  for unit in pairs(buffed.members()) do
    local now = game.ticks()
    if not game.alive(unit) then
      if game.counter(unit, "gs_active") == 1 then game.add_stat(unit, S_SPEED_PCT, -GHOST_SHIP.move_pct) end
      game.set_counter(unit, "gs_active", 0)
      game.set_counter(unit, "gs_drip_left", 0)
      buffed.members()[unit] = nil
    elseif now >= game.counter(unit, "gs_until") then
      if game.counter(unit, "gs_active") == 1 then
        game.add_stat(unit, S_SPEED_PCT, -GHOST_SHIP.move_pct)
        game.set_counter(unit, "gs_active", 2)
        game.set_counter(unit, "gs_drip_left", GHOST_SHIP.drip_pulses)
        game.set_counter(unit, "gs_drip_at", now + TICKS)
      elseif now >= game.counter(unit, "gs_drip_at") then
        local left = game.counter(unit, "gs_drip_left")
        local pulse = math.floor(game.counter(unit, "gs_owed") / GHOST_SHIP.drip_pulses)
        game.set_hp(unit, math.max(WHOLE, unit_hp(unit) - pulse))
        game.set_counter(unit, "gs_drip_left", left - 1)
        game.set_counter(unit, "gs_drip_at", now + TICKS)
        if left <= 1 then
          game.set_counter(unit, "gs_active", 0)
          game.set_counter(unit, "gs_owed", 0)
          buffed.members()[unit] = nil
        end
      end
    end
  end
end

local ENCHANT = {
  source = "war3map.j P5X 48709-48728, P6X 48729-48733, P1X 48658-48660 (A0DX); 80 s timed life (ability.ini tooltip)",
  casts = { "enchantress_enchant" },
  ticks = 80 * TICKS,
  unconvertible = { "ancient", "roshan" },
}

local enchanted = tracker("enchant_until")

local function convertible(unit)
  if game.is_attributed(unit) or game.is_structure(unit) then return false end
  local kind = game.unit_kind(unit)
  for _, word in ipairs(ENCHANT.unconvertible) do
    if string.find(kind, word, 1, true) then return false end
  end
  return true
end

function ENCHANT.run(caster)
  local target = game.cast_target(caster)
  if target == nil or not game.alive(target) or not convertible(target) then return end
  local owner = unit_player(caster)
  if unit_player(target) ~= owner then
    game.cmd_set_owner(target, owner)
    game.clear_timed_mods(target, 0)
    game.clear_status(target, "slow")
  elseif game.counter(target, "enchant_until") == 0 then
    return
  end
  game.set_counter(target, "enchant_until", game.ticks() + ENCHANT.ticks)
  enchanted.add(target)
end

function ENCHANT.tick()
  for unit in pairs(enchanted.members()) do
    if game.ticks() >= game.counter(unit, "enchant_until") or not game.alive(unit) then
      if game.alive(unit) then game.cmd_remove(unit) end
      game.set_counter(unit, "enchant_until", 0)
      enchanted.members()[unit] = nil
    end
  end
end

local ATTENDANTS = {
  source = "war3map.j IJE 48619-48640, PZX 48583-48607, PYX 48580-48582 (A01B)",
  ability = "enchantress_nature_s_attendants",
  casts = { "enchantress_nature_s_attendants" },
  radius = 300,
  heal = 10,
  pulses = 11,
  wisps_base = 2,
  wisps_per_rank = 2,
}

local attending = tracker("att_left")

function ATTENDANTS.run(caster)
  local rank = rank_of(caster, ATTENDANTS.ability)
  if rank == 0 then return end
  game.set_counter(caster, "att_left", ATTENDANTS.pulses)
  game.set_counter(caster, "att_wisps", ATTENDANTS.wisps_base + ATTENDANTS.wisps_per_rank * rank)
  game.set_counter(caster, "att_at", game.ticks() + TICKS)
  attending.add(caster)
end

function ATTENDANTS.tick()
  for caster in pairs(attending.members()) do
    if game.ticks() >= game.counter(caster, "att_at") then
      local left = game.counter(caster, "att_left") - 1
      game.set_counter(caster, "att_left", left)
      game.set_counter(caster, "att_at", game.ticks() + TICKS)
      local owner = unit_player(caster)
      local x, y = unit_position(caster)
      local allies = others_near(x, y, config.milli(ATTENDANTS.radius * 15.625), function(unit)
        return not game.hostile(owner, unit_player(unit))
      end)
      if #allies > 0 then
        for _ = 1, game.counter(caster, "att_wisps") do
          local unit = allies[game.random(#allies) + 1]
          game.set_hp(unit, unit_hp(unit) + ATTENDANTS.heal * WHOLE)
        end
      end
      if left <= 0 or not game.alive(caster) then
        game.set_counter(caster, "att_left", 0)
        attending.members()[caster] = nil
      end
    end
  end
end

local SPECS = { X_MARKS, GHOST_SHIP, ENCHANT, ATTENDANTS }

local ON_CAST = {}
for _, spec in ipairs(SPECS) do
  for _, name in ipairs(spec.casts) do
    ON_CAST[name] = ON_CAST[name] or {}
    ON_CAST[name][#ON_CAST[name] + 1] = spec.run
  end
end

return {
  on_cast = function(caster, ability_name)
    for _, run in ipairs(ON_CAST[ability_name] or {}) do run(caster) end
  end,
  on_cast_request = function(caster, slot, ability_name, ...)
    if RETIRED_CASTS[ability_name] then return false end
    return spells.on_cast_request(caster, slot, ability_name, ...)
  end,
  on_hit = function(attacker, victim, dealt)
    GHOST_SHIP.hit(attacker, victim, dealt)
  end,
  on_tick = function()
    X_MARKS.tick()
    GHOST_SHIP.tick_ships()
    GHOST_SHIP.tick_buffs()
    ENCHANT.tick()
    ATTENDANTS.tick()
  end,
  on_state_replaced = function()
    marked.rebuild()
    ships.rebuild()
    buffed.rebuild()
    enchanted.rebuild()
    attending.rebuild()
  end,
}

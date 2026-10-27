local config = require("config")

local TICKS_PER_SECOND = 16
local MAGICAL, PURE = 2, 3
local MAX_HIT = 32767
local S_ATTACK_SPEED = 12

local function rank_of(unit, ability)
  for _, row in ipairs(game.unit_abilities(unit)) do
    if row.name == ability then return game.skill_rank(unit, row.slot) end
  end
  return 0
end

local function enemies(owner, x, y, radius)
  local found = {}
  for _, unit in ipairs(game.units_near(x, y, config.milli(radius))) do
    if game.alive(unit) and not game.is_structure(unit) and game.counter(unit, "area_summon") == 0 and game.hostile(owner, unit_player(unit)) then
      found[#found + 1] = unit
    end
  end
  return found
end

local MARCH = {
  source = "ability.ini A0BQ 24273 block, 14827-14845; robots hit by area GUESS",
  ability = "tinker_march_of_the_machines",
  radius = 4688,
  period = TICKS_PER_SECOND,
  duration = 6 * TICKS_PER_SECOND,
  damage = { 16, 24, 32, 40 },
  robots_per_hit = 6,
}

local SIGIL = {
  source = "ability.ini A0W9 74869-74886 tooltip",
  ability = "tusk_frozen_sigil",
  unit = "tusk_frozen_sigil_unit",
  life = 30 * TICKS_PER_SECOND,
  radius = 9375,
  move_keep = { 90, 85, 80, 75 },
  attack_cut = { 30, 40, 50, 60 },
  refresh = 8,
  slow_ticks = 10,
}

local TOMBSTONE = {
  source = "war3map.j N3E 64890-64935 (A15V, units n0FJ n0FI n0F6 n0FH hit points 3 to 7, life 10+5 x rank s, zombie n0F5)",
  ability = "undying_tombstone",
  unit = "undying_tombstone_unit",
  zombie = "undying_zombie",
  life = { 240, 320, 400, 480 },
  hits = { 3, 4, 5, 7 },
  period = 3 * TICKS_PER_SECOND,
  radius = 15625,
  cap = 12,
}

local active = {}
local reap = {}

local function bind(unit, ticks)
  game.set_counter(unit, "area_summon", 1)
  game.set_counter(unit, "area_due", game.ticks() + ticks)
  reap[unit] = true
end

local function zone_track(kind, caster, x, y, rank)
  local now = game.ticks()
  local mark = { kind = kind, caster = caster, x = x, y = y, rank = rank, due = now, born = now }
  active[#active + 1] = mark
  return mark
end

local function cast_point_or_caster(caster, at_caster)
  local x, y = game.cast_point(caster)
  if at_caster or x == nil or (x == 0 and y == 0) then x, y = unit_position(caster) end
  return x, y
end

local function start_march(caster)
  local rank = rank_of(caster, MARCH.ability)
  if rank == 0 then return end
  local x, y = cast_point_or_caster(caster, false)
  local mark = zone_track("march", caster, x, y, rank)
  mark.until_tick = mark.born + MARCH.duration
  mark.due = mark.born + MARCH.period
end

local function step_march(mark)
  local owner = unit_player(mark.caster)
  local damage = MARCH.damage[math.min(mark.rank, #MARCH.damage)] * MARCH.robots_per_hit
  for _, unit in ipairs(enemies(owner, mark.x, mark.y, MARCH.radius)) do
    game.spell_damage(mark.caster, unit, damage, MAGICAL)
  end
  mark.due = mark.due + MARCH.period
  return game.ticks() < mark.until_tick
end

local function start_sigil(caster)
  local rank = rank_of(caster, SIGIL.ability)
  if rank == 0 then return end
  local x, y = unit_position(caster)
  local unit = game.spawn(unit_player(caster), SIGIL.unit, x, y)
  if unit == nil then return end
  bind(unit, SIGIL.life + 16)
  local mark = zone_track("sigil", caster, x, y, rank)
  mark.unit = unit
  mark.until_tick = mark.born + SIGIL.life
  mark.due = mark.born
end

local function step_sigil(mark)
  if not game.alive(mark.unit) or game.ticks() >= mark.until_tick then
    if game.alive(mark.unit) then game.spell_damage(mark.unit, mark.unit, MAX_HIT, PURE) end
    return false
  end
  local x, y = unit_position(mark.unit)
  local rank = math.min(mark.rank, #SIGIL.move_keep)
  local keep = math.floor(65536 * SIGIL.move_keep[rank] / 100)
  for _, unit in ipairs(enemies(unit_player(mark.unit), x, y, SIGIL.radius)) do
    game.apply_status(unit, "slow", SIGIL.slow_ticks, keep)
    if game.counter(unit, "sigil_due") == 0 then
      game.set_counter(unit, "sigil_cut", SIGIL.attack_cut[rank])
      game.add_stat(unit, S_ATTACK_SPEED, -SIGIL.attack_cut[rank])
    end
    game.set_counter(unit, "sigil_due", game.ticks() + SIGIL.slow_ticks + 1)
    mark.slowed = mark.slowed or {}
    mark.slowed[unit] = true
  end
  mark.due = game.ticks() + SIGIL.refresh
  return true
end

local function release_sigil(unit)
  game.add_stat(unit, S_ATTACK_SPEED, game.counter(unit, "sigil_cut"))
  game.set_counter(unit, "sigil_due", 0)
end

local function start_tombstone(caster)
  local rank = rank_of(caster, TOMBSTONE.ability)
  if rank == 0 then return end
  local x, y = cast_point_or_caster(caster, false)
  local unit = game.spawn(unit_player(caster), TOMBSTONE.unit, x, y)
  if unit == nil then return end
  bind(unit, TOMBSTONE.life[math.min(rank, #TOMBSTONE.life)] + 16)
  game.set_counter(unit, "tomb_hits", TOMBSTONE.hits[math.min(rank, #TOMBSTONE.hits)])
  local mark = zone_track("tomb", caster, x, y, rank)
  mark.unit = unit
  mark.until_tick = mark.born + TOMBSTONE.life[math.min(rank, #TOMBSTONE.life)]
  mark.due = mark.born + TOMBSTONE.period
  mark.zombies = {}
end

local function step_tomb(mark)
  local alive = game.alive(mark.unit) and game.counter(mark.unit, "tomb_hits") > 0
  local now = game.ticks()
  local kept = {}
  for _, zombie in ipairs(mark.zombies) do
    if game.alive(zombie) and (alive and now < mark.until_tick) then
      kept[#kept + 1] = zombie
    elseif game.alive(zombie) then
      game.spell_damage(zombie, zombie, MAX_HIT, PURE)
    end
  end
  mark.zombies = kept
  if not alive or now >= mark.until_tick then
    if game.alive(mark.unit) then game.spell_damage(mark.unit, mark.unit, MAX_HIT, PURE) end
    return false
  end
  local x, y = unit_position(mark.unit)
  for _, target in ipairs(enemies(unit_player(mark.unit), x, y, TOMBSTONE.radius)) do
    if #mark.zombies >= TOMBSTONE.cap then break end
    local tx, ty = unit_position(target)
    local zombie = game.spawn(unit_player(mark.unit), TOMBSTONE.zombie, tx, ty)
    if zombie ~= nil then
      bind(zombie, mark.until_tick - now + 16)
      mark.zombies[#mark.zombies + 1] = zombie
    end
  end
  mark.due = now + TOMBSTONE.period
  return true
end

local FLARE = {
  source = "war3map.j DYE 87965-87983, MZR 87938-87963 (A2BG: 25 evaluations every 0.1 s, radius 195, damage split among the heroes inside)",
  ability = "skywrath_mage_mystic_flare",
  radius = 3047,
  period = 2,
  pulses = 19,
  damage = { 600, 1000, 1400 },
}

local function start_flare(caster)
  local rank = rank_of(caster, FLARE.ability)
  if rank == 0 then return end
  local x, y = cast_point_or_caster(caster, false)
  local mark = zone_track("flare", caster, x, y, rank)
  mark.left = FLARE.pulses
  mark.due = mark.born + FLARE.period
end

local function step_flare(mark)
  local heroes = {}
  for _, unit in ipairs(enemies(unit_player(mark.caster), mark.x, mark.y, FLARE.radius)) do
    if game.is_attributed(unit) then heroes[#heroes + 1] = unit end
  end
  if #heroes > 0 then
    local each = math.floor(FLARE.damage[math.min(mark.rank, 3)] / FLARE.pulses / #heroes)
    for _, unit in ipairs(heroes) do game.spell_damage(mark.caster, unit, each, MAGICAL) end
  end
  mark.left = mark.left - 1
  mark.due = mark.due + FLARE.period
  return mark.left > 0
end

local BONDS = {
  source = "war3map.j FEE 78341-78380, AFR 78313-78322, AGR 78323-78337 (A0J5: 25 percent of each hit to the others, 2 + rank others within 725, 25 s)",
  ability = "warlock_fatal_bonds",
  radius = 11328,
  share = 25,
  smallest = 10,
  largest = 6000,
  duration = 25 * TICKS_PER_SECOND,
}

local bonds = {}

local function start_bonds(caster)
  local rank = rank_of(caster, BONDS.ability)
  local target = game.cast_target(caster)
  if rank == 0 or target == nil or not game.alive(target) then return end
  local x, y = unit_position(target)
  local members = { target }
  for _, unit in ipairs(enemies(unit_player(caster), x, y, BONDS.radius)) do
    if unit ~= target and #members < rank + 3 then members[#members + 1] = unit end
  end
  local id = game.ticks() * 64 + caster + 1
  for _, unit in ipairs(members) do
    game.set_counter(unit, "bond_id", id)
    game.set_counter(unit, "bond_caster", caster)
    game.set_counter(unit, "bond_due", game.ticks() + BONDS.duration)
  end
  bonds[id] = members
end

local function share_bonds(victim, dealt)
  local id = game.counter(victim, "bond_id")
  if id == 0 or game.ticks() >= game.counter(victim, "bond_due") or game.counter(victim, "bond_skip") == game.ticks() then return end
  local whole = dealt / 65536
  if whole <= BONDS.smallest or whole >= BONDS.largest then return end
  local caster = game.counter(victim, "bond_caster")
  local amount = math.floor(whole * BONDS.share / 100)
  for _, other in ipairs(bonds[id] or {}) do
    if other ~= victim and game.alive(other) and game.counter(other, "bond_id") == id then
      game.set_counter(other, "bond_skip", game.ticks() + 1)
      game.spell_damage(caster, other, amount, MAGICAL)
    end
  end
end

local function rebuild_bonds()
  bonds = {}
  for unit = 0, get_unit_count() - 1 do
    local id = game.counter(unit, "bond_id")
    if id ~= 0 and game.alive(unit) and game.ticks() < game.counter(unit, "bond_due") then
      bonds[id] = bonds[id] or {}
      bonds[id][#bonds[id] + 1] = unit
    end
  end
end

local SHARDS = {
  source = "war3map.j HLR 84445-84485, HBR (A1YQ: a wall at the target point for 7 s; footprint GUESS 2 x 2 cells)",
  ability = "tusk_ice_shards",
  seconds = 7,
  cells = 2,
}

local function place_wall(caster)
  local x, y = cast_point_or_caster(caster, false)
  game.blocker_add(math.floor(x / 65536) - 1, math.floor(y / 65536) - 1, SHARDS.cells, SHARDS.cells, SHARDS.seconds)
end

local STEP = { march = step_march, sigil = step_sigil, tomb = step_tomb, flare = step_flare }

local ON_CAST = {
  [MARCH.ability] = start_march,
  [SIGIL.ability] = start_sigil,
  [TOMBSTONE.ability] = start_tombstone,
  [FLARE.ability] = start_flare,
  [BONDS.ability] = start_bonds,
  [SHARDS.ability] = place_wall,
}

return {
  on_cast = function(caster, ability_name)
    local run = ON_CAST[ability_name]
    if run ~= nil then run(caster) end
  end,
  on_hit = function(attacker, victim, dealt)
    share_bonds(victim, dealt)
    if game.alive(victim) and game.counter(victim, "tomb_hits") > 0 then
      local left = game.counter(victim, "tomb_hits") - 1
      game.set_counter(victim, "tomb_hits", left)
      if left <= 0 then game.spell_damage(victim, victim, MAX_HIT, PURE) end
    end
  end,
  on_tick = function()
    local now = game.ticks()
    local kept = {}
    for _, mark in ipairs(active) do
      local live = true
      if now >= mark.due then live = STEP[mark.kind](mark) end
      if live then
        kept[#kept + 1] = mark
      elseif mark.slowed ~= nil then
        for unit in pairs(mark.slowed) do
          if game.counter(unit, "sigil_due") ~= 0 then release_sigil(unit) end
        end
      end
    end
    active = kept
    for unit in pairs(reap) do
      if not game.alive(unit) then
        reap[unit] = nil
      elseif now >= game.counter(unit, "area_due") then
        reap[unit] = nil
        game.spell_damage(unit, unit, MAX_HIT, PURE)
      end
    end
  end,
  on_state_replaced = function()
    active = {}
    reap = {}
    rebuild_bonds()
    for unit = 0, get_unit_count() - 1 do
      if game.counter(unit, "area_summon") ~= 0 and game.alive(unit) then reap[unit] = true end
    end
  end,
}

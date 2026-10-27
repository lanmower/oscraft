local config = require("config")
local units = require("units")
local kit = require("spell_kit")

local TICKS = 16
local WHOLE = 65536
local PHYSICAL, MAGICAL, PURE = 1, 2, 3
local MAX_HIT = 32767
local S_DAMAGE, S_HEALTH, S_SPEED_PCT, S_AGILITY, S_ARMOR = 0, 2, 19, 23, 1

local rank_of, slot_of, dispel_negative, held = kit.rank_of, kit.slot_of, kit.dispel_negative, kit.held
local enemy_of, targetable_enemy, enemies_around = kit.enemy_of, kit.targetable_enemy, kit.enemies_around
local heal_percent, heroes, counter_units = kit.heal_percent, kit.heroes, kit.counter_units
local max_hp, strength_of, agility_of = kit.max_hp, kit.strength_of, kit.agility_of

local CHAOS_BOLT = {
  source = "tooltip of A055 (ability.ini:14613): random stun and damage, the longer the stun the lower the damage",
  ability = "chaos_knight_chaos_bolt",
  casts = { "chaos_knight_chaos_bolt" },
  damage = { { 75, 200 }, { 100, 225 }, { 125, 250 }, { 150, 275 } },
  stun_seconds = { { 1, 2 }, { 1, 3 }, { 1, 4 }, { 2, 4 } },
}

function CHAOS_BOLT.run(caster)
  local rank = math.min(rank_of(caster, CHAOS_BOLT.ability), #CHAOS_BOLT.damage)
  local target = game.cast_target(caster)
  if rank == 0 or not targetable_enemy(caster, target) then return end
  local shortest, longest = CHAOS_BOLT.stun_seconds[rank][1] * TICKS, CHAOS_BOLT.stun_seconds[rank][2] * TICKS
  local low, high = CHAOS_BOLT.damage[rank][1], CHAOS_BOLT.damage[rank][2]
  local stun = shortest + game.random(longest - shortest + 1)
  local damage = high - math.floor((stun - shortest) * (high - low) / (longest - shortest))
  if not game.spell_damage(caster, target, damage, MAGICAL) then
    game.apply_status(target, "stun", stun)
  end
end


local TEST_OF_FAITH = {
  source = "war3map.j R9E 46577-46628, LNX 46540-46560, LRX 46536-46550 (A0LV)",
  ability = "chen_test_of_faith",
  casts = { "chen_test_of_faith" },
  base_seconds = 7,
  heal_after_ticks = 2,
}

local pending_home = {}

local function schedule_move(unit, delay, destination)
  game.set_counter(unit, "faith_due", game.ticks() + delay)
  game.set_counter(unit, "faith_to", destination)
  pending_home[unit] = true
end

function TEST_OF_FAITH.run(caster)
  local rank = rank_of(caster, TEST_OF_FAITH.ability)
  local target = game.cast_target(caster)
  if rank == 0 or target == nil or not game.alive(target) then return end
  if enemy_of(caster, target) then
    if not held(target, "spell_immunity") then
      game.spell_damage(caster, target, 50 * rank + game.random(50 * rank + 1), PURE)
    end
    return
  end
  local delay = (TEST_OF_FAITH.base_seconds - rank) * TICKS
  if units.is_hero(target) then
    if target == caster then
      for _, unit in ipairs(game.units(unit_player(caster))) do
        if unit ~= caster and not units.is_hero(unit) and game.is_summoned(unit) then
          schedule_move(unit, delay, caster + 1)
        end
      end
    elseif unit_player(target) == unit_player(caster) then
      schedule_move(target, TEST_OF_FAITH.heal_after_ticks, 0)
    else
      schedule_move(target, delay, 0)
    end
  elseif unit_player(target) == unit_player(caster) then
    local start = game.seat_start(unit_player(target))
    if start ~= nil then game.teleport(target, start.x, start.y) end
  end
end

local function move_pending()
  for unit in pairs(pending_home) do
    if game.counter(unit, "faith_due") <= game.ticks() or not game.alive(unit) then
      local to = game.counter(unit, "faith_to")
      game.set_counter(unit, "faith_due", 0)
      pending_home[unit] = nil
      if game.alive(unit) then
        if to > 0 and game.alive(to - 1) then
          local x, y = unit_position(to - 1)
          game.teleport(unit, x, y)
        elseif to == 0 then
          local start = game.seat_start(unit_player(unit))
          if start ~= nil then game.teleport(unit, start.x, start.y) end
        end
      end
    end
  end
end


local HAND_OF_GOD = {
  source = "war3map.j 46806-46819 (A0LT): every own non-hero unit is set to full life",
  ability = "chen_hand_of_god",
  casts = { "chen_hand_of_god" },
}

function HAND_OF_GOD.run(caster)
  for _, unit in ipairs(game.units(unit_player(caster))) do
    if not units.is_hero(unit) and not game.is_structure(unit) then
      game.set_hp(unit, MAX_HIT * WHOLE)
    end
  end
end


local MARKSMANSHIP = {
  source = "war3map.j 90211 (A2NX, ability.ini:93652): agility 40/60/80 while no enemy hero stands within 425",
  ability = "drow_ranger_marksmanship",
  agility = { 40, 60, 80 },
  radius_milli = 6641,
  period = 8,
}

local function marksmanship_scan()
  for _, hero in ipairs(heroes()) do
    local rank = 0
    if game.alive(hero) then rank = math.min(rank_of(hero, MARKSMANSHIP.ability), #MARKSMANSHIP.agility) end
    local bonus = 0
    if rank > 0 and not held(hero, "break") then
      bonus = MARKSMANSHIP.agility[rank]
      local x, y = unit_position(hero)
      for _, unit in ipairs(enemies_around(hero, x, y, MARKSMANSHIP.radius_milli)) do
        if units.is_hero(unit) then
          bonus = 0
          break
        end
      end
    end
    local have = game.counter(hero, "marksman_agility")
    if bonus ~= have then
      game.add_stat(hero, S_AGILITY, bonus - have)
      game.set_counter(hero, "marksman_agility", bonus)
    end
  end
end


local SURGE = {
  source = "war3map.j 64323 (A3JM, Aasl DataA 10.0 = maximum speed, ability.ini:124844): movement speed at the cap for 1.5+1.5*rank s",
  ability = "dark_seer_surge",
  casts = { "dark_seer_surge" },
  ticks = { 48, 72, 96, 120 },
  boost_percent = 1000,
}

local surging = {}

local function end_surge(unit)
  game.add_stat(unit, S_SPEED_PCT, -game.counter(unit, "surge_speed"))
  game.set_counter(unit, "surge_speed", 0)
  game.set_counter(unit, "surge_due", 0)
  surging[unit] = nil
end

function SURGE.run(caster)
  local rank = math.min(rank_of(caster, SURGE.ability), #SURGE.ticks)
  local target = game.cast_target(caster)
  if rank == 0 or target == nil or not game.alive(target) then return end
  if game.counter(target, "surge_speed") == 0 then
    game.add_stat(target, S_SPEED_PCT, SURGE.boost_percent)
    game.set_counter(target, "surge_speed", SURGE.boost_percent)
  end
  game.set_counter(target, "surge_due", game.ticks() + SURGE.ticks[rank])
  surging[target] = true
end

local function expire_surges()
  for unit in pairs(surging) do
    if game.ticks() >= game.counter(unit, "surge_due") or not game.alive(unit) then end_surge(unit) end
  end
end


local ION_SHELL = {
  source = "war3map.j 64164-64197 (A3JN): 1+2*rank per 0.1 s as magical damage to enemies within 275 of the target for 25 s, applied here every 8 ticks",
  ability = "dark_seer_ion_shell",
  casts = { "dark_seer_ion_shell" },
  radius_milli = 4297,
  per_pulse = { 15, 25, 35, 45 },
  period = 8,
  duration = 25 * TICKS,
}

local shelled = {}

function ION_SHELL.run(caster)
  local rank = math.min(rank_of(caster, ION_SHELL.ability), #ION_SHELL.per_pulse)
  local target = game.cast_target(caster)
  if rank == 0 or target == nil or not game.alive(target) then return end
  game.set_counter(target, "ion_source", caster + 1)
  game.set_counter(target, "ion_rank", rank)
  game.set_counter(target, "ion_due", game.ticks() + ION_SHELL.duration)
  game.set_counter(target, "ion_next", game.ticks() + ION_SHELL.period)
  shelled[target] = true
end

local function pulse_shells()
  for unit in pairs(shelled) do
    local source = game.counter(unit, "ion_source") - 1
    if not game.alive(unit) or not game.alive(source) or game.ticks() >= game.counter(unit, "ion_due") then
      game.set_counter(unit, "ion_source", 0)
      shelled[unit] = nil
    elseif game.ticks() >= game.counter(unit, "ion_next") then
      game.set_counter(unit, "ion_next", game.ticks() + ION_SHELL.period)
      local x, y = unit_position(unit)
      for _, enemy in ipairs(enemies_around(source, x, y, ION_SHELL.radius_milli)) do
        if not held(enemy, "spell_immunity") then
          game.spell_damage(source, enemy, ION_SHELL.per_pulse[game.counter(unit, "ion_rank")], MAGICAL)
        end
      end
    end
  end
end


local GLIMPSE = {
  source = "war3map.j GFR 83210-83225, GCR 83186-83210 (A1SW): the target returns to where it stood 4 s ago, from a 15-sample position history; samples here are one a second",
  ability = "disruptor_glimpse",
  casts = { "disruptor_glimpse" },
  samples = 5,
  period = TICKS,
}

local history = {}

local function sample_positions()
  for _, hero in ipairs(heroes()) do
    if game.alive(hero) then
      local trail = history[hero]
      if trail == nil then
        trail = {}
        history[hero] = trail
      end
      local x, y = unit_position(hero)
      table.insert(trail, 1, { x, y })
      if #trail > GLIMPSE.samples then table.remove(trail) end
    end
  end
end

function GLIMPSE.run(caster)
  local target = game.cast_target(caster)
  if not targetable_enemy(caster, target) then return end
  local trail = history[target]
  local back = trail and trail[#trail]
  if back ~= nil then game.teleport(target, back[1], back[2]) end
end


local WEAVE = {
  source = "war3map.j CKE 76800-76850, RPR 76753-76785, RJR/RKR 76731-76752 (A10Q): heroes within 600 of the point at the cast, enemies lose and allies gain 0.5+0.25*rank armor each second for 24 s, all taken back at the end",
  ability = "dazzle_weave",
  casts = { "dazzle_weave" },
  radius_milli = 9375,
  seconds = 24,
  base_milli = 500,
  per_rank_milli = 250,
}

local S_ARMOR = 1
local weaving = {}

local function set_weave_armor(unit, armor)
  game.add_stat(unit, S_ARMOR, armor - game.counter(unit, "weave_armor"))
  game.set_counter(unit, "weave_armor", armor)
end

function WEAVE.run(caster)
  local rank = rank_of(caster, WEAVE.ability)
  local x, y = game.cast_point(caster)
  if rank == 0 or x == nil then return end
  local per_milli = WEAVE.base_milli + WEAVE.per_rank_milli * rank
  local owner = unit_player(caster)
  for _, unit in ipairs(game.units_near(x, y, config.milli(WEAVE.radius_milli))) do
    if game.alive(unit) and (units.is_hero(unit) or game.is_summoned(unit)) then
      local sign = 1
      if game.hostile(owner, unit_player(unit)) then sign = -1 end
      set_weave_armor(unit, 0)
      game.set_counter(unit, "weave_sign", sign)
      game.set_counter(unit, "weave_per", per_milli)
      game.set_counter(unit, "weave_acc", 0)
      game.set_counter(unit, "weave_next", game.ticks() + TICKS)
      game.set_counter(unit, "weave_end", game.ticks() + WEAVE.seconds * TICKS)
      weaving[unit] = true
    end
  end
end

local function pulse_weaves()
  for unit in pairs(weaving) do
    if game.ticks() >= game.counter(unit, "weave_end") or not game.alive(unit) then
      set_weave_armor(unit, 0)
      game.set_counter(unit, "weave_end", 0)
      weaving[unit] = nil
    elseif game.ticks() >= game.counter(unit, "weave_next") then
      local acc = game.counter(unit, "weave_acc") + game.counter(unit, "weave_per")
      game.set_counter(unit, "weave_acc", acc)
      game.set_counter(unit, "weave_next", game.ticks() + TICKS)
      set_weave_armor(unit, game.counter(unit, "weave_sign") * math.floor(acc / 1000))
    end
  end
end


local PERSUASION = {
  source = "war3map.j LHX 46700-46737, LLX 46738-46750 (A28T): the target changes owner, the count of persuaded units is capped at the rank (the oldest dies), its base life is raised to 600+100*rank",
  ability = "chen_holy_persuasion",
  casts = { "chen_holy_persuasion" },
  floor_base = 600,
  floor_per_rank = 100,
}

local persuaded = {}

function PERSUASION.run(caster)
  local rank = rank_of(caster, PERSUASION.ability)
  local target = game.cast_target(caster)
  if rank == 0 or not targetable_enemy(caster, target) or units.is_hero(target) or game.is_structure(target) then return end
  local list = persuaded[caster]
  if list == nil then
    list = {}
    persuaded[caster] = list
  end
  local kept = {}
  for _, unit in ipairs(list) do
    if game.alive(unit) and game.counter(unit, "persuaded_by") == caster + 1 then kept[#kept + 1] = unit end
  end
  while #kept >= rank do
    local oldest = table.remove(kept, 1)
    game.set_counter(oldest, "persuaded_by", 0)
    game.spell_damage(caster, oldest, MAX_HIT, PURE)
  end
  game.cmd_set_owner(target, unit_player(caster))
  local floor = PERSUASION.floor_base + PERSUASION.floor_per_rank * rank
  game.raise_max_hp(target, floor)
  game.set_counter(target, "persuaded_by", caster + 1)
  kept[#kept + 1] = target
  persuaded[caster] = kept
end

local function rebuild_persuaded()
  persuaded = {}
  for _, unit in ipairs(counter_units("persuaded_by")) do
    local owner = game.counter(unit, "persuaded_by") - 1
    local list = persuaded[owner]
    if list == nil then
      list = {}
      persuaded[owner] = list
    end
    list[#list + 1] = unit
  end
end


local DEVOUR = {
  source = "war3map.j BLE 69447, TSO 69434 (A10R, ability.ini:47827): the target dies and 25*rank gold follow (the digestion delay is not modelled)",
  ability = "doom_bringer_devour",
  casts = { "doom_bringer_devour" },
  gold_per_rank = 25,
}

function DEVOUR.run(caster)
  local rank = rank_of(caster, DEVOUR.ability)
  local target = game.cast_target(caster)
  if rank == 0 or not targetable_enemy(caster, target) or units.is_hero(target) or game.is_structure(target) then return end
  game.spell_damage(caster, target, MAX_HIT, PURE)
  game.cmd_add_gold(unit_player(caster), DEVOUR.gold_per_rank * rank)
end


local STATIC_STORM = {
  source = "war3map.j GPR 83348-83385 (A1U6): 20 pulses of 0.25 s at radius 475, each 0.25*(150+50*rank)*pulse/20 magical, silence inside radius 450 (A1U4)",
  ability = "disruptor_static_storm",
  casts = { "disruptor_static_storm" },
  damage_radius_milli = 7422,
  silence_radius_milli = 7031,
  pulses = 20,
  period = 4,
}

local storms = {}

function STATIC_STORM.run(caster)
  local rank = rank_of(caster, STATIC_STORM.ability)
  local x, y = game.cast_point(caster)
  if rank == 0 or x == nil then return end
  storms[#storms + 1] = { caster = caster, rank = rank, x = x, y = y, pulse = 0, next = game.ticks() + STATIC_STORM.period }
end

local function run_storms()
  local kept = {}
  for _, storm in ipairs(storms) do
    local alive = game.alive(storm.caster)
    if alive and game.ticks() >= storm.next then
      storm.pulse = storm.pulse + 1
      storm.next = game.ticks() + STATIC_STORM.period
      local damage = math.floor(0.25 * (150 + 50 * storm.rank) * storm.pulse / STATIC_STORM.pulses)
      for _, unit in ipairs(game.units_near(storm.x, storm.y, config.milli(STATIC_STORM.damage_radius_milli))) do
        if game.alive(unit) and enemy_of(storm.caster, unit) and not game.is_structure(unit) and not held(unit, "spell_immunity") then
          game.spell_damage(storm.caster, unit, damage, MAGICAL)
          if game.within(unit, storm.x, storm.y, STATIC_STORM.silence_radius_milli) then
            game.apply_status(unit, "silence", STATIC_STORM.period + 2)
          end
        end
      end
    end
    if alive and storm.pulse < STATIC_STORM.pulses then kept[#kept + 1] = storm end
  end
  storms = kept
end


local STAMPEDE = {
  source = "war3map.j R8E 46480, K9X 46470-46478, K7X 46437-46469, K6X 46415-46425 (A2O6): allied units run at maximum speed for 3.75 s and each unit they touch within 130 once takes rank*strength magical damage and a 1.5 s slow (GUESS: heroes and own summons only, for the 2500 step budget)",
  ability = "centaur_stampede",
  casts = { "centaur_stampede" },
  duration = 60,
  boost_percent = 1000,
  radius_milli = 2031,
  slow_ticks = 24,
  slow_mul = 39322,
}

local stampeders = {}

local function end_stampede(unit)
  game.add_stat(unit, S_SPEED_PCT, -game.counter(unit, "stampede_speed"))
  game.set_counter(unit, "stampede_speed", 0)
  game.set_counter(unit, "stampede_due", 0)
  stampeders[unit] = nil
end

function STAMPEDE.run(caster)
  local rank = rank_of(caster, STAMPEDE.ability)
  if rank == 0 then return end
  local owner_team = game.team(unit_player(caster))
  for _, unit in ipairs(heroes()) do
    if game.alive(unit) and game.team(unit_player(unit)) == owner_team then
      if game.counter(unit, "stampede_speed") == 0 then
        game.add_stat(unit, S_SPEED_PCT, STAMPEDE.boost_percent)
        game.set_counter(unit, "stampede_speed", STAMPEDE.boost_percent)
      end
      game.set_counter(unit, "stampede_due", game.ticks() + STAMPEDE.duration)
      game.set_counter(unit, "stampede_source", caster + 1)
      game.set_counter(unit, "stampede_rank", rank)
      stampeders[unit] = true
    end
  end
end

local function run_stampede()
  for unit in pairs(stampeders) do
    if game.ticks() >= game.counter(unit, "stampede_due") or not game.alive(unit) then
      end_stampede(unit)
    else
      local source = game.counter(unit, "stampede_source") - 1
      local x, y = unit_position(unit)
      for _, enemy in ipairs(enemies_around(unit, x, y, STAMPEDE.radius_milli)) do
        if game.counter(enemy, "stampede_hit") <= game.ticks() and not held(enemy, "spell_immunity") then
          game.set_counter(enemy, "stampede_hit", game.ticks() + STAMPEDE.duration)
          if game.alive(source) then
            game.spell_damage(source, enemy, math.min(MAX_HIT, game.counter(unit, "stampede_rank") * strength_of(source)), MAGICAL)
          end
          game.apply_slow(enemy, STAMPEDE.slow_ticks, STAMPEDE.slow_mul)
        end
      end
    end
  end
end


local FROSTBITE = {
  source = "ability.ini:107241-107245 (A341 Dur 10 s, HeroDur 1.5/2/2.5/3 s), war3map.j:47674: on a non-hero the root and the 50 damage every 0.5 s last 10 s",
  ability = "crystal_maiden_frostbite",
  casts = { "crystal_maiden_frostbite" },
  hero_ticks = { 24, 32, 40, 48 },
  ticks = 160,
  period = 8,
  damage = 50,
}

local frostbitten = {}

function FROSTBITE.run(caster)
  local rank = math.min(rank_of(caster, FROSTBITE.ability), #FROSTBITE.hero_ticks)
  local target = game.cast_target(caster)
  if rank == 0 or not targetable_enemy(caster, target) or units.is_hero(target) then return end
  game.apply_status(target, "root", FROSTBITE.ticks)
  game.apply_status(target, "disarm", FROSTBITE.ticks)
  game.set_counter(target, "frost_source", caster + 1)
  game.set_counter(target, "frost_due", game.ticks() + FROSTBITE.ticks)
  game.set_counter(target, "frost_next", game.ticks() + FROSTBITE.hero_ticks[rank] + FROSTBITE.period)
  frostbitten[target] = true
end

local function pulse_frostbite()
  for unit in pairs(frostbitten) do
    local source = game.counter(unit, "frost_source") - 1
    if not game.alive(unit) or game.ticks() >= game.counter(unit, "frost_due") then
      game.set_counter(unit, "frost_due", 0)
      frostbitten[unit] = nil
    elseif game.ticks() >= game.counter(unit, "frost_next") then
      game.set_counter(unit, "frost_next", game.ticks() + FROSTBITE.period)
      if game.alive(source) then game.spell_damage(source, unit, FROSTBITE.damage, MAGICAL) end
    end
  end
end


local TRUESHOT = {
  source = "tooltip of A2O2 (ability.ini:93731, 93740): allied ranged heroes gain 20/26/32/38 percent of the caster's agility as damage; the 30 s active that extends it to all allied ranged units is not modelled",
  ability = "drow_ranger_trueshot_aura",
  percent = { 20, 26, 32, 38 },
  melee_milli = 1562,
  period = 8,
}


local function trueshot_scan()
  local owners = {}
  for _, hero in ipairs(heroes()) do
    if game.alive(hero) then
      local rank = math.min(rank_of(hero, TRUESHOT.ability), #TRUESHOT.percent)
      if rank > 0 and not held(hero, "break") then
        owners[#owners + 1] = { team = game.team(unit_player(hero)), bonus = math.floor(agility_of(hero) * TRUESHOT.percent[rank] / 100) }
      end
    end
  end
  for _, hero in ipairs(heroes()) do
    local bonus = 0
    if game.alive(hero) then
      local _, range = game.weapon(hero)
      if range > config.milli(TRUESHOT.melee_milli) then
        for _, owner in ipairs(owners) do
          if owner.team == game.team(unit_player(hero)) and owner.bonus > bonus then bonus = owner.bonus end
        end
      end
    end
    local have = game.counter(hero, "trueshot_damage")
    if bonus ~= have then
      game.add_stat(hero, S_DAMAGE, bonus - have)
      game.set_counter(hero, "trueshot_damage", bonus)
    end
  end
end


local REALITY_RIFT = {
  source = "war3map.j GHO 63620-63670 (A0RW, buff A3KI): the caster jumps 30 to 80 percent of the way to the target, the target is drawn beside it, and the caster gains +60/80/100/120 damage for 1.2 s",
  ability = "chaos_knight_reality_rift",
  casts = { "chaos_knight_reality_rift" },
  bonus = { 60, 80, 100, 120 },
  ticks = 19,
  beside_milli = 391,
}

local rifting = {}

function REALITY_RIFT.run(caster)
  local rank = math.min(rank_of(caster, REALITY_RIFT.ability), #REALITY_RIFT.bonus)
  local target = game.cast_target(caster)
  if rank == 0 or not targetable_enemy(caster, target) then return end
  local cx, cy = unit_position(caster)
  local tx, ty = unit_position(target)
  local dx, dy = tx - cx, ty - cy
  local length = math.floor(math.sqrt(dx * dx + dy * dy))
  if length == 0 then return end
  local share = (300 + game.random(501)) / 1000
  local ux, uy = dx / length, dy / length
  local jump = math.floor(length * share)
  local beside = math.max(0, jump - config.milli(REALITY_RIFT.beside_milli))
  game.teleport(caster, cx + math.floor(ux * jump), cy + math.floor(uy * jump))
  game.teleport(target, cx + math.floor(ux * beside), cy + math.floor(uy * beside))
  if game.counter(caster, "rift_bonus") == 0 then
    game.add_stat(caster, S_DAMAGE, REALITY_RIFT.bonus[rank])
    game.set_counter(caster, "rift_bonus", REALITY_RIFT.bonus[rank])
  end
  game.set_counter(caster, "rift_due", game.ticks() + REALITY_RIFT.ticks)
  rifting[caster] = true
end

local function expire_rifts()
  for unit in pairs(rifting) do
    if game.ticks() >= game.counter(unit, "rift_due") or not game.alive(unit) then
      game.add_stat(unit, S_DAMAGE, -game.counter(unit, "rift_bonus"))
      game.set_counter(unit, "rift_bonus", 0)
      game.set_counter(unit, "rift_due", 0)
      rifting[unit] = nil
    end
  end
end


local THUNDER_STRIKE = {
  source = "war3map.j GJR 83256-83297, GHR 83245 (A1TV): the target is struck at once and every 2 s for 6 s, each strike dealing 20*rank+20 magical to every other enemy within 265 of it",
  ability = "disruptor_thunder_strike",
  casts = { "disruptor_thunder_strike" },
  radius_milli = 4141,
  period = 32,
  strikes = 4,
}

local strikes = {}

local function strike_around(strike)
  local x, y = unit_position(strike.target)
  for _, unit in ipairs(enemies_around(strike.caster, x, y, THUNDER_STRIKE.radius_milli)) do
    if unit ~= strike.target and not held(unit, "spell_immunity") then
      game.spell_damage(strike.caster, unit, 20 * strike.rank + 20, MAGICAL)
    end
  end
end

function THUNDER_STRIKE.run(caster)
  local rank = math.min(rank_of(caster, THUNDER_STRIKE.ability), 4)
  local target = game.cast_target(caster)
  if rank == 0 or not targetable_enemy(caster, target) then return end
  local strike = { caster = caster, target = target, rank = rank, left = THUNDER_STRIKE.strikes - 1, next = game.ticks() + THUNDER_STRIKE.period }
  strikes[#strikes + 1] = strike
  strike_around(strike)
end

local function run_strikes()
  local kept = {}
  for _, strike in ipairs(strikes) do
    local live = game.alive(strike.caster) and game.alive(strike.target)
    if live and game.ticks() >= strike.next then
      strike.left = strike.left - 1
      strike.next = game.ticks() + THUNDER_STRIKE.period
      strike_around(strike)
    end
    if live and strike.left > 0 then kept[#kept + 1] = strike end
  end
  strikes = kept
end

local POISON_TOUCH = {
  source = "war3map.j FLE 76721, RFR 76574-76660, RGR 76660-76700 (A2X6, slow A0NJ ability.ini:36316): each second the target is slowed 33/66/100 percent by stage (rank 1 33,33; 2 33,66; 3 66,100; 4 66 then a 1 s stun) and from the third second takes 8+6*rank (36 at rank 4) physical, for 8 s",
  ability = "dazzle_poison_touch",
  casts = { "dazzle_poison_touch" },
  damage = { 14, 20, 26, 36 },
  stages = { { 43909, 43909 }, { 43909, 22282 }, { 22282, 0 }, { 22282, "stun" } },
  seconds = 8,
}

local poisoned = {}

function POISON_TOUCH.run(caster)
  local rank = math.min(rank_of(caster, POISON_TOUCH.ability), #POISON_TOUCH.damage)
  local target = game.cast_target(caster)
  if rank == 0 or not targetable_enemy(caster, target) then return end
  game.set_counter(target, "poison_source", caster + 1)
  game.set_counter(target, "poison_rank", rank)
  game.set_counter(target, "poison_step", 0)
  game.set_counter(target, "poison_next", game.ticks() + TICKS)
  poisoned[target] = true
end

local function pulse_poison()
  for unit in pairs(poisoned) do
    local source = game.counter(unit, "poison_source") - 1
    local step = game.counter(unit, "poison_step")
    if not game.alive(unit) or step >= POISON_TOUCH.seconds then
      game.set_counter(unit, "poison_next", 0)
      poisoned[unit] = nil
    elseif game.ticks() >= game.counter(unit, "poison_next") then
      step = step + 1
      game.set_counter(unit, "poison_step", step)
      game.set_counter(unit, "poison_next", game.ticks() + TICKS)
      local rank = game.counter(unit, "poison_rank")
      local stage = POISON_TOUCH.stages[rank][step]
      if stage == "stun" then
        game.apply_status(unit, "stun", TICKS)
      elseif stage ~= nil then
        game.apply_slow(unit, TICKS + 2, stage)
      end
      if step > 2 and game.alive(source) then
        game.spell_damage(source, unit, POISON_TOUCH.damage[rank], PHYSICAL)
      end
    end
  end
end

local SHADOW_WAVE = {
  source = "war3map.j CJE 76506-76569, RBR 76492-76530, RAR 76458-76476 (A0OR): after the caster and the target, the wave goes to the most wounded ally within 500 of the last unit (heroes first), 3/4/5/6 units in all, each healed 60+20*rank and damaging enemies within 210 by the same physical amount",
  ability = "dazzle_shadow_wave",
  casts = { "dazzle_shadow_wave" },
  units = { 3, 4, 5, 6 },
  reach_milli = 7813,
  splash_milli = 3281,
}

local function wounded_ally(caster, from, visited)
  local x, y = unit_position(from)
  local best, best_hero, best_hp
  for _, unit in ipairs(game.units_near(x, y, config.milli(SHADOW_WAVE.reach_milli))) do
    if not visited[unit] and game.alive(unit) and not enemy_of(caster, unit) and not game.is_structure(unit) then
      local hp = unit_hp(unit)
      if hp < kit.max_hp(unit) then
        local hero = units.is_hero(unit)
        if best == nil or (hero and not best_hero) or (hero == best_hero and hp < best_hp) then
          best, best_hero, best_hp = unit, hero, hp
        end
      end
    end
  end
  return best
end

function SHADOW_WAVE.run(caster)
  local rank = math.min(rank_of(caster, SHADOW_WAVE.ability), #SHADOW_WAVE.units)
  local target = game.cast_target(caster)
  if rank == 0 or target == nil or not game.alive(target) then return end
  local amount = 60 + 20 * rank
  local visited = { [caster] = true, [target] = true }
  local current = target
  for _ = 3, SHADOW_WAVE.units[rank] do
    current = wounded_ally(caster, current, visited)
    if current == nil then return end
    visited[current] = true
    game.set_hp(current, unit_hp(current) + amount * WHOLE)
    local x, y = unit_position(current)
    for _, enemy in ipairs(enemies_around(caster, x, y, SHADOW_WAVE.splash_milli)) do
      game.spell_damage(caster, enemy, amount, PHYSICAL)
    end
  end
end

local EXORCISM = {
  source = "ability.ini:17150-17201 (A073 DataD 0.25, UnitID u00G): the ghosts' hits heal the caster by 25 percent of the damage dealt",
  kind = "death_prophet_ghost",
  percent = 25,
}

local function exorcism_hit(attacker, dealt)
  if attacker == nil or attacker < 0 or not game.alive(attacker) then return end
  if game.unit_kind(attacker) ~= EXORCISM.kind then return end
  local owner = unit_player(attacker)
  for _, unit in ipairs(game.units(owner)) do
    if units.is_hero(unit) and game.alive(unit) then
      game.set_hp(unit, unit_hp(unit) + math.floor(dealt * EXORCISM.percent / 100))
      return
    end
  end
end

local PHANTASM = {
  source = "ability.ini:55524-55575 (A1AY bonus illusion) and war3map.j NWE 63717-63731 (A03O, Aghanim A1BQ): with the Aghanim's Scepter half the casts bring one more illusion",
  ability = "chaos_knight_phantasm",
  casts = { "chaos_knight_phantasm" },
  life = 672,
  out_percent = 100,
  in_percent = 200,
}

function PHANTASM.run(caster)
  if not kit.scepter(caster) or game.random(2) ~= 0 then return end
  local x, y = unit_position(caster)
  local image = game.spawn_image(caster, x + config.milli(3000), y, PHANTASM.life, PHANTASM.out_percent, PHANTASM.in_percent)
  if image >= 0 then game.set_counter(caster, "phantasm_extra", game.counter(caster, "phantasm_extra") + 1) end
end

local WALL_OF_REPLICA = {
  source = "war3map.j N1E 64469-64520 (A0QK): a 1000 long line of 40 pieces centred on the point, across the caster's facing, for 45 s; each enemy hero that comes within 17 of a piece is hit once for 150 magical and an image of it fights for Dark Seer (60/75/90 percent dealt, 300 percent taken; GUESS: it lasts 30 s)",
  ability = "dark_seer_wall_of_replica",
  casts = { "dark_seer_wall_of_replica" },
  half_milli = 7813,
  reach_milli = 265,
  duration = 720,
  damage = 150,
  out_percent = { 60, 75, 90 },
  in_percent = 300,
  image_life = 480,
  period = 4,
}

local walls = {}

function WALL_OF_REPLICA.run(caster)
  local rank = math.min(rank_of(caster, WALL_OF_REPLICA.ability), #WALL_OF_REPLICA.out_percent)
  local x, y = game.cast_point(caster)
  if rank == 0 or x == nil then return end
  local cx, cy = unit_position(caster)
  local dx, dy = x - cx, y - cy
  local length = math.sqrt(dx * dx + dy * dy)
  if length == 0 then return end
  walls[#walls + 1] = { caster = caster, rank = rank, x = x, y = y, ux = -dy / length, uy = dx / length, due = game.ticks() + WALL_OF_REPLICA.duration, hit = {} }
end

local function run_walls()
  local kept = {}
  local half = config.milli(WALL_OF_REPLICA.half_milli)
  local reach = config.milli(WALL_OF_REPLICA.reach_milli)
  for _, wall in ipairs(walls) do
    if game.alive(wall.caster) and game.ticks() < wall.due then
      kept[#kept + 1] = wall
      if game.ticks() % WALL_OF_REPLICA.period == 0 then
        for _, hero in ipairs(heroes()) do
          if not wall.hit[hero] and game.alive(hero) and enemy_of(wall.caster, hero) and not held(hero, "spell_immunity") then
            local hx, hy = unit_position(hero)
            local px, py = hx - wall.x, hy - wall.y
            local along = px * wall.ux + py * wall.uy
            local across = math.abs(px * wall.uy - py * wall.ux)
            if math.abs(along) <= half and across <= reach then
              wall.hit[hero] = true
              game.spell_damage(wall.caster, hero, WALL_OF_REPLICA.damage, MAGICAL)
              local image = game.spawn_image(hero, hx, hy, WALL_OF_REPLICA.image_life, WALL_OF_REPLICA.out_percent[wall.rank], WALL_OF_REPLICA.in_percent)
              if image >= 0 then game.cmd_set_owner(image, unit_player(wall.caster)) end
            end
          end
        end
      end
    end
  end
  walls = kept
end

local DOOM = {
  source = "tooltip of A0MU (ability.ini:35534): with the scepter Doom also breaks its target (the longer duration is not modelled)",
  ability = "doom_bringer_doom_basic",
  casts = { "doom_bringer_doom_basic" },
  ticks = 240,
}

function DOOM.run(caster)
  local target = game.cast_target(caster)
  if kit.scepter(caster) and targetable_enemy(caster, target) then game.apply_status(target, "break", DOOM.ticks) end
end

local KINETIC = {
  source = "ability.ini:70226-70287 (A1SU), war3map.j GAR 83054: a square ring of blockers of diameter 325 (5 cells) around the point for 2+0.6*rank s (GUESS: the ring is the square, one cell thick)",
  ability = "disruptor_kinetic_field",
  casts = { "disruptor_kinetic_field" },
  seconds = { 2.6, 3.2, 3.8, 4.4 },
}

function KINETIC.run(caster)
  local rank = math.min(rank_of(caster, KINETIC.ability), 4)
  local x, y = game.cast_point(caster)
  if rank == 0 or x == nil then return end
  local cx, cy = math.floor(x / WHOLE), math.floor(y / WHOLE)
  local life = KINETIC.seconds[rank]
  game.blocker_add(cx - 2, cy - 2, 5, 1, life)
  game.blocker_add(cx - 2, cy + 2, 5, 1, life)
  game.blocker_add(cx - 2, cy - 1, 1, 3, life)
  game.blocker_add(cx + 2, cy - 1, 1, 3, life)
end

local function rebuild_all()
  pending_home = {}
  for _, unit in ipairs(counter_units("faith_due")) do pending_home[unit] = true end
  surging = {}
  for _, unit in ipairs(counter_units("surge_due")) do surging[unit] = true end
  shelled = {}
  for _, unit in ipairs(counter_units("ion_source")) do shelled[unit] = true end
  weaving = {}
  for _, unit in ipairs(counter_units("weave_end")) do weaving[unit] = true end
  stampeders = {}
  for _, unit in ipairs(counter_units("stampede_due")) do stampeders[unit] = true end
  frostbitten = {}
  for _, unit in ipairs(counter_units("frost_due")) do frostbitten[unit] = true end
  rifting = {}
  for _, unit in ipairs(counter_units("rift_due")) do rifting[unit] = true end
  storms = {}
  strikes = {}
  walls = {}
  poisoned = {}
  for _, unit in ipairs(counter_units("poison_next")) do poisoned[unit] = true end
  history = {}
  rebuild_persuaded()
  kit.reset_heroes()
end

local ON_CAST = kit.on_cast_table({
  CHAOS_BOLT, TEST_OF_FAITH, HAND_OF_GOD, SURGE, ION_SHELL, GLIMPSE, WEAVE, PERSUASION, DEVOUR, STATIC_STORM,
  STAMPEDE, FROSTBITE, REALITY_RIFT, THUNDER_STRIKE, POISON_TOUCH, SHADOW_WAVE, PHANTASM, WALL_OF_REPLICA, DOOM, KINETIC,
})

return {
  on_cast = function(caster, ability_name)
    kit.dispatch(ON_CAST, caster, ability_name)
  end,
  on_hit = function(attacker, victim, dealt)
    exorcism_hit(attacker, dealt)
  end,
  on_tick = function(state)
    local tick = state.ticks or 0
    move_pending()
    expire_surges()
    pulse_shells()
    pulse_weaves()
    run_storms()
    run_stampede()
    pulse_frostbite()
    expire_rifts()
    run_strikes()
    pulse_poison()
    run_walls()
    if tick % MARKSMANSHIP.period == 0 then marksmanship_scan() end
    if tick % TRUESHOT.period == TRUESHOT.period / 2 then trueshot_scan() end
    if tick % GLIMPSE.period == 0 then sample_positions() end
  end,
  on_state_replaced = rebuild_all,
}

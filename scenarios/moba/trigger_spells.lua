local config = require("config")
local units = require("units")

local TICKS_PER_SECOND = 16
local WHOLE = 65536
local S_DAMAGE, S_HEALTH = 0, 2
local PURE = 3
local MAX_HIT = 32767

local function rank_of(unit, ability)
  for _, row in ipairs(game.unit_abilities(unit)) do
    if row.name == ability then return game.skill_rank(unit, row.slot) end
  end
  return 0
end

local function share(raw_hp, numerator, denominator)
  return math.floor(raw_hp * numerator / (denominator * WHOLE))
end

local DEATH_PACT = {
  source = "war3map.j G6O 63824, G3O 63795, G0O 63780-63793 (A04Q)",
  ability = "clinkz_death_pact",
  casts = { "clinkz_death_pact" },
  damage_permille = { 50, 65, 80 },
  health_permille = { 500, 650, 800 },
  duration = 35 * TICKS_PER_SECOND,
}

local pacts = {}

local function end_pact(pact)
  local unit = pact.unit
  game.set_counter(unit, "pact_damage", game.counter(unit, "pact_damage") - pact.damage)
  game.set_counter(unit, "pact_health", game.counter(unit, "pact_health") - pact.health)
  local hp = unit_hp(unit)
  game.add_stat(unit, S_DAMAGE, -pact.damage)
  game.add_stat(unit, S_HEALTH, -pact.health)
  game.set_hp(unit, hp)
end

function DEATH_PACT.run(caster)
  local rank = math.min(rank_of(caster, DEATH_PACT.ability), #DEATH_PACT.damage_permille)
  local target = game.cast_target(caster)
  if rank == 0 or target == nil or not game.alive(target) or game.is_attributed(target) then return end
  if unit_hp(caster) <= WHOLE / 2 then return end
  local life = unit_hp(target)
  local pact = {
    unit = caster,
    due = game.ticks() + DEATH_PACT.duration,
    damage = share(life, DEATH_PACT.damage_permille[rank], 1000),
    health = share(life, DEATH_PACT.health_permille[rank], 1000),
  }
  game.add_stat(caster, S_DAMAGE, pact.damage)
  game.add_stat(caster, S_HEALTH, pact.health)
  game.set_counter(caster, "pact_damage", game.counter(caster, "pact_damage") + pact.damage)
  game.set_counter(caster, "pact_health", game.counter(caster, "pact_health") + pact.health)
  pacts[#pacts + 1] = pact
  game.spell_damage(caster, target, MAX_HIT, PURE)
end

local function expire_pacts()
  local keep = {}
  for _, pact in ipairs(pacts) do
    if game.ticks() >= pact.due or not game.alive(pact.unit) then
      end_pact(pact)
    else
      keep[#keep + 1] = pact
    end
  end
  pacts = keep
end

local function rebuild_pacts()
  pacts = {}
  for unit = 0, get_unit_count() - 1 do
    local damage, health = game.counter(unit, "pact_damage"), game.counter(unit, "pact_health")
    if damage ~= 0 or health ~= 0 then
      pacts[#pacts + 1] = { unit = unit, due = game.ticks() + DEATH_PACT.duration, damage = damage, health = health }
    end
  end
end

local MAGICAL = 2

local function held(unit, status)
  return game.status_left(unit, status) > 0
end

local function enemies_near(caster, radius)
  local x, y = unit_position(caster)
  local owner = unit_player(caster)
  local found = {}
  for _, unit in ipairs(game.units_near(x, y, radius)) do
    if unit ~= caster and game.alive(unit) and game.hostile(owner, unit_player(unit)) and not game.is_structure(unit) then
      found[#found + 1] = unit
    end
  end
  return found
end

local STATIC_FIELD = {
  source = "war3map.j BGO 60197, BMO 60322, BPO 60329, BQO 60332 (A0N5)",
  ability = "zuus_static_field",
  casts = { "zuus_lightning_bolt", "zuus_arc_lightning", "zuus_thundergods_wrath" },
  radius = 19141,
  percent_base = 3,
  percent_per_rank = 2,
}

function STATIC_FIELD.run(caster)
  local rank = rank_of(caster, STATIC_FIELD.ability)
  if rank == 0 or held(caster, "break") then return end
  local percent = STATIC_FIELD.percent_base + STATIC_FIELD.percent_per_rank * rank
  for _, unit in ipairs(enemies_near(caster, config.milli(STATIC_FIELD.radius))) do
    if not held(unit, "spell_immunity") then
      game.spell_damage(caster, unit, share(unit_hp(unit), percent, 100), MAGICAL)
    end
  end
end

local AFTERSHOCK = {
  source = "war3map.j PJX 48412, PKX 48415 (A0DJ casting the stomp A0DI, ability.ini:26707)",
  ability = "earthshaker_aftershock",
  casts = { "earthshaker_fissure", "earthshaker_enchant_totem", "earthshaker_echo_slam" },
  radius = 4688,
  damage = { 50, 75, 100, 125 },
  stun_ticks = { 10, 14, 19, 24 },
}

function AFTERSHOCK.run(caster)
  local rank = math.min(rank_of(caster, AFTERSHOCK.ability), #AFTERSHOCK.damage)
  if rank == 0 or held(caster, "break") then return end
  for _, unit in ipairs(enemies_near(caster, config.milli(AFTERSHOCK.radius))) do
    if not held(unit, "spell_immunity") then
      game.spell_damage(caster, unit, AFTERSHOCK.damage[rank], MAGICAL)
      game.apply_status(unit, "stun", AFTERSHOCK.stun_ticks[rank])
    end
  end
end

local S_ATTACK_SPEED, S_SPEED_PCT = 12, 19

local FIERY_SOUL = {
  source = "war3map.j UIX 51310-51362, URX 51281-51309 (A18X; casts A01F A027 A01P A09Z)",
  ability = "lina_fiery_soul",
  casts = { "lina_dragon_slave", "lina_light_strike_array", "lina_laguna_blade" },
  max_stacks = 3,
  attack_speed_base = 25,
  attack_speed_per_rank = 15,
  move_base = 4,
  move_per_rank = 1,
  duration = 10 * TICKS_PER_SECOND,
}

local fiery = {}

local function set_fiery(unit, stacks, rank)
  local attack = (FIERY_SOUL.attack_speed_base + FIERY_SOUL.attack_speed_per_rank * rank) * stacks
  local move = (FIERY_SOUL.move_base + FIERY_SOUL.move_per_rank * rank) * stacks
  game.add_stat(unit, S_ATTACK_SPEED, attack - game.counter(unit, "fiery_attack"))
  game.add_stat(unit, S_SPEED_PCT, move - game.counter(unit, "fiery_move"))
  game.set_counter(unit, "fiery_attack", attack)
  game.set_counter(unit, "fiery_move", move)
  game.set_counter(unit, "fiery_stacks", stacks)
  game.set_counter(unit, "fiery_rank", rank)
end

function FIERY_SOUL.run(caster)
  local rank = rank_of(caster, FIERY_SOUL.ability)
  if rank == 0 or held(caster, "break") then return end
  local stacks = game.counter(caster, "fiery_stacks")
  if stacks < FIERY_SOUL.max_stacks or rank ~= game.counter(caster, "fiery_rank") then
    set_fiery(caster, math.min(stacks + 1, FIERY_SOUL.max_stacks), rank)
  end
  game.set_counter(caster, "fiery_due", game.ticks() + FIERY_SOUL.duration)
  fiery[caster] = true
end

local function expire_fiery()
  for unit in pairs(fiery) do
    if game.ticks() >= game.counter(unit, "fiery_due") or not game.alive(unit) then
      set_fiery(unit, 0, 0)
      fiery[unit] = nil
    end
  end
end

local function rebuild_fiery()
  fiery = {}
  for unit = 0, get_unit_count() - 1 do
    if game.counter(unit, "fiery_stacks") > 0 then fiery[unit] = true end
  end
end

local WARPATH = {
  source = "war3map.j KXX 45993, J9X 45962-45989, J8X 45929-45960, J2X 45899 (A0FV; effect levels A0EF, ability.ini:27482)",
  ability = "bristleback_warpath",
  casts = { "bristleback_quill_spray", "bristleback_viscous_nasal_goo" },
  stack_duration = 14 * TICKS_PER_SECOND,
  base_stacks = 4,
  damage_base = 15,
  damage_per_rank = 5,
  move_base = 2,
  move_per_rank = 1,
  attack_speed = 15,
  attack_speed_from_stack = { 3, 1, 1 },
}

local warpath = {}

local function apply_warpath(unit)
  local rank = rank_of(unit, WARPATH.ability)
  local stacks = 0
  if rank > 0 then stacks = math.min(#warpath[unit], WARPATH.base_stacks + rank) end
  local damage = stacks * (WARPATH.damage_base + WARPATH.damage_per_rank * rank)
  local move = stacks * (WARPATH.move_base + WARPATH.move_per_rank * rank)
  local attack = 0
  if rank > 0 and stacks >= WARPATH.attack_speed_from_stack[math.min(rank, #WARPATH.attack_speed_from_stack)] then
    attack = WARPATH.attack_speed
  end
  game.add_stat(unit, S_DAMAGE, damage - game.counter(unit, "warpath_damage"))
  game.add_stat(unit, S_SPEED_PCT, move - game.counter(unit, "warpath_move"))
  game.add_stat(unit, S_ATTACK_SPEED, attack - game.counter(unit, "warpath_attack"))
  game.set_counter(unit, "warpath_damage", damage)
  game.set_counter(unit, "warpath_move", move)
  game.set_counter(unit, "warpath_attack", attack)
  game.set_counter(unit, "warpath_count", #warpath[unit])
end

function WARPATH.run(caster)
  if rank_of(caster, WARPATH.ability) == 0 then return end
  local dues = warpath[caster]
  if dues == nil then
    dues = {}
    warpath[caster] = dues
  end
  dues[#dues + 1] = game.ticks() + WARPATH.stack_duration
  apply_warpath(caster)
end

local function expire_warpath()
  for unit, dues in pairs(warpath) do
    local kept = {}
    for _, due in ipairs(dues) do
      if due > game.ticks() then kept[#kept + 1] = due end
    end
    if #kept ~= #dues then
      warpath[unit] = kept
      apply_warpath(unit)
      if #kept == 0 then warpath[unit] = nil end
    end
  end
end

local function rebuild_warpath()
  warpath = {}
  for unit = 0, get_unit_count() - 1 do
    local count = game.counter(unit, "warpath_count")
    if count > 0 then
      local dues = {}
      for n = 1, count do dues[n] = game.ticks() + WARPATH.stack_duration end
      warpath[unit] = dues
    end
  end
end

local KRAKEN_SHELL = {
  source = "war3map.j O8R 76285-76312, GJE 76313-76324 (A04E), O0V 6301 (the dispel)",
  ability = "tidehunter_kraken_shell",
  casts = {},
  threshold_base = 650,
  threshold_per_rank = 50,
  reset_ticks = 6 * TICKS_PER_SECOND,
  dispelled = { "slow", "silence", "root", "taunt", "disarm", "mute", "break" },
}

local kraken_slot = {}

local function kraken_slot_of(victim)
  local slot = kraken_slot[victim]
  if slot == nil then
    slot = false
    for _, row in ipairs(game.unit_abilities(victim)) do
      if row.name == KRAKEN_SHELL.ability then slot = row.slot end
    end
    kraken_slot[victim] = slot
  end
  return slot
end

local function dispel(unit)
  for _, status in ipairs(KRAKEN_SHELL.dispelled) do
    game.clear_status(unit, status)
  end
  game.clear_dots(unit)
  game.clear_timed_mods(unit, 0)
end

function KRAKEN_SHELL.hit(attacker, victim, dealt)
  if attacker < 0 or not game.is_attributed(victim) or unit_player(attacker) >= config.PLAYERS then return end
  local slot = kraken_slot_of(victim)
  if slot == false then return end
  local rank = game.skill_rank(victim, slot)
  if rank == 0 or held(victim, "break") then return end
  local now = game.ticks()
  local taken = game.counter(victim, "kraken_taken")
  if game.counter(victim, "kraken_at") + KRAKEN_SHELL.reset_ticks < now then taken = 0 end
  taken = taken + dealt
  if taken > (KRAKEN_SHELL.threshold_base - KRAKEN_SHELL.threshold_per_rank * rank) * WHOLE then
    taken = 0
    dispel(victim)
  end
  game.set_counter(victim, "kraken_taken", taken)
  game.set_counter(victim, "kraken_at", now)
end

local LAST_WILL = {
  source = "war3map.j TRV 19687-19697 (A0GU, A0GX, A0GZ)",
  damage = { necronomicon_warrior_1 = 550, necronomicon_warrior_2 = 675, necronomicon_warrior_3 = 800 },
  damage_type = 2,
}

local function last_will(unit, killer)
  local damage = LAST_WILL.damage[game.unit_kind(unit)]
  if damage == nil or killer == nil or killer < 0 or not game.alive(killer) or game.is_structure(killer) then return end
  if not game.hostile(unit_player(unit), unit_player(killer)) then return end
  game.spell_damage(unit, killer, damage, LAST_WILL.damage_type)
end

local S_ARMOR, S_STRENGTH_BONUS, S_HEALTH_REGEN = 1, 22, 14

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
  local function reset()
    cache = {}
  end
  return lookup, reset
end

local REACTIVE_ARMOR = {
  source = "war3map.j PCR 88366-88381, PFR 88382-88386 (A2E4); a hit is any damage the on_hit hook reports, so a spell hit also stacks",
  ability = "shredder_reactive_armor",
  stacks_per_rank = 5,
  duration = 16 * TICKS_PER_SECOND,
  armor_per_stack = 1,
  regen_per_stack = 100,
}
local reactive_slot, reset_reactive_slots = slot_cache(REACTIVE_ARMOR.ability)
local reactive = {}

local function apply_reactive(unit)
  local slot = reactive_slot(unit)
  local rank = slot and game.skill_rank(unit, slot) or 0
  local stacks = math.min(#reactive[unit], REACTIVE_ARMOR.stacks_per_rank * rank)
  local armor, regen = stacks * REACTIVE_ARMOR.armor_per_stack, stacks * REACTIVE_ARMOR.regen_per_stack
  game.add_stat(unit, S_ARMOR, armor - game.counter(unit, "reactive_armor"))
  game.add_stat(unit, S_HEALTH_REGEN, regen - game.counter(unit, "reactive_regen"))
  game.set_counter(unit, "reactive_armor", armor)
  game.set_counter(unit, "reactive_regen", regen)
  game.set_counter(unit, "reactive_count", #reactive[unit])
end

function REACTIVE_ARMOR.hit(attacker, victim)
  if attacker < 0 or not game.is_attributed(victim) or reactive_slot(victim) == false then return end
  if game.skill_rank(victim, reactive_slot(victim)) == 0 or held(victim, "break") then return end
  if not game.hostile(unit_player(attacker), unit_player(victim)) then return end
  local dues = reactive[victim]
  if dues == nil then
    dues = {}
    reactive[victim] = dues
  end
  dues[#dues + 1] = game.ticks() + REACTIVE_ARMOR.duration
  apply_reactive(victim)
end

local function expire_reactive()
  for unit, dues in pairs(reactive) do
    local kept = {}
    for _, due in ipairs(dues) do
      if due > game.ticks() and game.alive(unit) then kept[#kept + 1] = due end
    end
    if #kept ~= #dues then
      reactive[unit] = kept
      apply_reactive(unit)
      if #kept == 0 then reactive[unit] = nil end
    end
  end
end

local function rebuild_reactive()
  reactive = {}
  reset_reactive_slots()
  for unit = 0, get_unit_count() - 1 do
    local count = game.counter(unit, "reactive_count")
    if count > 0 then
      local dues = {}
      for n = 1, count do dues[n] = game.ticks() + REACTIVE_ARMOR.duration end
      reactive[unit] = dues
    end
  end
end

local FLESH_HEAP = {
  source = "war3map.j ZZO 72788-72799, ZWO 72779-72783, Z1O 72849-72866 (A06D)",
  ability = "pudge_flesh_heap",
  kind = "pudge",
  radius = 7422,
  refresh_ticks = TICKS_PER_SECOND / 2,
}
local flesh = {}

local function apply_flesh(unit)
  local slot
  for _, row in ipairs(game.unit_abilities(unit)) do
    if row.name == FLESH_HEAP.ability then slot = row.slot end
  end
  local rank = slot and game.skill_rank(unit, slot) or 0
  local strength = math.floor(game.counter(unit, "flesh_deaths") * (1 + rank) / 2)
  game.add_stat(unit, S_STRENGTH_BONUS, strength - game.counter(unit, "flesh_strength"))
  game.set_counter(unit, "flesh_strength", strength)
end

function FLESH_HEAP.died(unit, killer)
  if not units.is_hero(unit) then return end
  local x, y = unit_position(unit)
  local owner = unit_player(unit)
  local credited = {}
  local function credit(pudge)
    if credited[pudge] or not game.alive(pudge) or game.unit_kind(pudge) ~= FLESH_HEAP.kind or not game.is_attributed(pudge) then return end
    if not game.hostile(owner, unit_player(pudge)) then return end
    credited[pudge] = true
    game.set_counter(pudge, "flesh_deaths", game.counter(pudge, "flesh_deaths") + 1)
    flesh[pudge] = true
    apply_flesh(pudge)
  end
  for _, other in ipairs(game.units_near(x, y, config.milli(FLESH_HEAP.radius))) do credit(other) end
  if killer ~= nil and killer >= 0 then credit(killer) end
end

local function refresh_flesh()
  if game.ticks() % FLESH_HEAP.refresh_ticks ~= 0 then return end
  for unit in pairs(flesh) do apply_flesh(unit) end
end

local function rebuild_flesh()
  flesh = {}
  for unit = 0, get_unit_count() - 1 do
    if game.counter(unit, "flesh_deaths") > 0 then flesh[unit] = true end
  end
end

local NECROMASTERY = {
  source = "war3map.j YRO 71649-71683, WKE 71613-71648, YIO 71684-71690 (A0BR, A0CP); the soul flies in 6.88g, here it lands at the kill",
  ability = "nevermore_necromastery",
  kind = "nevermore",
  limit = { 15, 22, 29, 36 },
  hero_souls = 12,
  damage_per_soul = 2,
}
local necro_slot = slot_cache(NECROMASTERY.ability)

local function set_souls(unit, souls)
  game.add_stat(unit, S_DAMAGE, (souls - game.counter(unit, "souls")) * NECROMASTERY.damage_per_soul)
  game.set_counter(unit, "souls", souls)
end

function NECROMASTERY.died(unit, killer)
  if units.is_hero(unit) and game.unit_kind(unit) == NECROMASTERY.kind and necro_slot(unit) then
    set_souls(unit, math.floor((game.counter(unit, "souls") + 1) / 2))
  end
  if killer == nil or killer < 0 or killer == unit or game.unit_kind(killer) ~= NECROMASTERY.kind or not game.is_attributed(killer) then return end
  local slot = necro_slot(killer)
  if slot == false or not game.hostile(unit_player(killer), unit_player(unit)) then return end
  local rank = game.skill_rank(killer, slot)
  if rank == 0 then return end
  local gain = units.is_hero(unit) and NECROMASTERY.hero_souls or 1
  set_souls(killer, math.min(NECROMASTERY.limit[math.min(rank, #NECROMASTERY.limit)], game.counter(killer, "souls") + gain))
end

local CAUSTIC_FINALE = {
  source = "war3map.j EMR 74282-74299, ELR 74239-74281 (A0FA; slow A38I): the first hit marks, the mark explodes on death or after 6 s at half damage; a hit is any damage the on_hit hook reports",
  ability = "sand_king_caustic_finale",
  kind = "sand_king",
  duration = 6 * TICKS_PER_SECOND,
  radius = 6641,
  damage_base = 50,
  damage_per_rank = 40,
  slow_ticks = 3 * TICKS_PER_SECOND,
  slow_multiplier = 49152,
}
local caustic_slot = slot_cache(CAUSTIC_FINALE.ability)
local marked = {}

local function explode_caustic(victim, half)
  local mark = marked[victim]
  marked[victim] = nil
  game.set_counter(victim, "caustic_due", 0)
  if mark == nil or not game.alive(mark.source) then return end
  local amount = CAUSTIC_FINALE.damage_base + CAUSTIC_FINALE.damage_per_rank * mark.rank
  if half then amount = math.floor(amount / 2) end
  local x, y = unit_position(victim)
  local owner = unit_player(mark.source)
  for _, unit in ipairs(game.units_near(x, y, config.milli(CAUSTIC_FINALE.radius))) do
    if game.alive(unit) and not game.is_structure(unit) and game.hostile(owner, unit_player(unit)) then
      game.spell_damage(mark.source, unit, amount, MAGICAL)
      game.apply_status(unit, "slow", CAUSTIC_FINALE.slow_ticks, CAUSTIC_FINALE.slow_multiplier)
    end
  end
end

function CAUSTIC_FINALE.hit(attacker, victim)
  if attacker < 0 or marked[victim] ~= nil or game.unit_kind(attacker) ~= CAUSTIC_FINALE.kind or not game.is_attributed(attacker) then return end
  local slot = caustic_slot(attacker)
  if slot == false or game.is_structure(victim) or not game.hostile(unit_player(attacker), unit_player(victim)) then return end
  local rank = game.skill_rank(attacker, slot)
  if rank == 0 then return end
  local due = game.ticks() + CAUSTIC_FINALE.duration
  marked[victim] = { source = attacker, rank = rank, due = due }
  game.set_counter(victim, "caustic_due", due)
end

local function expire_caustic()
  for victim, mark in pairs(marked) do
    if game.ticks() >= mark.due then explode_caustic(victim, true) end
  end
end

local ROT = {
  source = "war3map.j ZPO 72705-72734, ZMO 72672-72699 (A06K, order immolation); ini Area 250 loses to the script's 275; the 0.2 s pulse is 4 ticks of 0.25 s at 7.5 x rank",
  ability = "pudge_rot",
  casts = { "pudge_rot" },
  radius = 4297,
  pulse_ticks = 4,
  damage_per_rank_x10 = 75,
  slow_ticks = 5,
  slow_multiplier = 52429,
}
local rotting = {}

function ROT.run(caster)
  if game.counter(caster, "rot_on") == 1 then
    game.set_counter(caster, "rot_on", 0)
    rotting[caster] = nil
  else
    game.set_counter(caster, "rot_on", 1)
    rotting[caster] = true
  end
end

local function rot_pulse(unit)
  local rank = rank_of(unit, ROT.ability)
  if rank == 0 then
    game.set_counter(unit, "rot_on", 0)
    rotting[unit] = nil
    return
  end
  local amount = math.floor(ROT.damage_per_rank_x10 * rank / 10)
  local x, y = unit_position(unit)
  local owner = unit_player(unit)
  for _, other in ipairs(game.units_near(x, y, config.milli(ROT.radius))) do
    if other ~= unit and game.alive(other) and not game.is_structure(other) and game.hostile(owner, unit_player(other)) then
      game.spell_damage(unit, other, amount, MAGICAL)
      game.apply_status(other, "slow", ROT.slow_ticks, ROT.slow_multiplier)
    end
  end
  game.spell_damage(unit, unit, amount, MAGICAL)
end

local function tick_rot()
  if game.ticks() % ROT.pulse_ticks ~= 0 then return end
  for unit in pairs(rotting) do
    if not game.alive(unit) then
      game.set_counter(unit, "rot_on", 0)
      rotting[unit] = nil
    else
      rot_pulse(unit)
    end
  end
end

local function rebuild_rot()
  rotting = {}
  for unit = 0, get_unit_count() - 1 do
    if game.counter(unit, "rot_on") == 1 then rotting[unit] = true end
  end
end

local CARAPACE = {
  source = "war3map.j WLO 71096-71147 (A2KO): for 2.25 s the first hit by each enemy player unit is undone, returned as magical damage and stunned 0.6 x rank s; a hit is any damage the on_hit hook reports",
  ability = "nyx_assassin_spiked_carapace",
  casts = { "nyx_assassin_spiked_carapace" },
  duration = 36,
  stun_ticks_per_rank = 9.6,
}
local carapace_slot = slot_cache(CARAPACE.ability)
local reflected = {}

function CARAPACE.run(caster)
  game.set_counter(caster, "carapace_until", game.ticks() + CARAPACE.duration)
  reflected[caster] = {}
end

function CARAPACE.hit(attacker, victim, dealt)
  if attacker < 0 or attacker == victim or game.counter(victim, "carapace_until") <= game.ticks() then return end
  local seen = reflected[victim]
  if seen == nil or seen[attacker] or not game.hostile(unit_player(attacker), unit_player(victim)) or not game.alive(attacker) then return end
  seen[attacker] = true
  local slot = carapace_slot(victim)
  local rank = slot and game.skill_rank(victim, slot) or 0
  game.set_hp(victim, unit_hp(victim) + dealt)
  game.spell_damage(victim, attacker, math.floor(dealt / WHOLE), MAGICAL)
  game.apply_status(attacker, "stun", math.ceil(CARAPACE.stun_ticks_per_rank * rank))
end

local NETHER_WARD = {
  source = "war3map.j B9E 73270-73301, VIR (o00M-o00O); ability.ini A08T Ubertip: Mana Flare, 1/1.25/1.5/1.75 damage per mana within 1600; the ward dies on its fourth hit",
  rank = { pugna_nether_ward_1 = 1, pugna_nether_ward_2 = 2, pugna_nether_ward_3 = 3, pugna_nether_ward_4 = 4 },
  hits_to_kill = 4,
  radius = 25000,
  damage_per_mana_x100 = { 100, 125, 150, 175 },
  kill_damage = MAX_HIT,
}

function NETHER_WARD.hit(attacker, victim, dealt)
  if NETHER_WARD.rank[game.unit_kind(victim)] == nil then return end
  local hits = game.counter(victim, "ward_hits") + 1
  game.set_counter(victim, "ward_hits", hits)
  if hits >= NETHER_WARD.hits_to_kill then
    game.spell_damage(attacker >= 0 and attacker or victim, victim, NETHER_WARD.kill_damage, PURE)
  else
    game.set_hp(victim, unit_hp(victim) + dealt)
  end
end

function NETHER_WARD.cast(caster, ability)
  if not game.is_attributed(caster) then return end
  local slot
  for _, row in ipairs(game.unit_abilities(caster)) do
    if row.name == ability then slot = row.slot end
  end
  if slot == nil then return end
  local info = game.skill_info(caster, slot)
  local rank = game.skill_rank(caster, slot)
  if info == nil or rank == 0 then return end
  local cost = info.energies[rank] or 0
  if cost <= 0 then return end
  local x, y = unit_position(caster)
  local owner = unit_player(caster)
  for _, other in ipairs(game.units_near(x, y, config.milli(NETHER_WARD.radius))) do
    local ward_rank = NETHER_WARD.rank[game.unit_kind(other)]
    if ward_rank ~= nil and game.alive(other) and game.hostile(unit_player(other), owner) then
      game.spell_damage(other, caster, math.floor(cost * NETHER_WARD.damage_per_mana_x100[ward_rank] / 100), MAGICAL)
    end
  end
end

local SOUL_CATCHER = {
  source = "tooltip ability.ini A... Soul Catcher and war3map.j:82389 (radius 475 in the script, 450 in the ini): every hit on a marked enemy for 12 s deals 20/30/40/50 percent more; the extra is a pure hit of that share, so armor and resistance have already applied to the share",
  ability = "shadow_demon_soul_catcher",
  casts = { "shadow_demon_soul_catcher" },
  radius = 7422,
  percent = { 20, 30, 40, 50 },
  duration = 12 * TICKS_PER_SECOND,
}

function SOUL_CATCHER.run(caster)
  local rank = math.min(rank_of(caster, SOUL_CATCHER.ability), #SOUL_CATCHER.percent)
  local x, y = game.cast_point(caster)
  if rank == 0 or x == nil then return end
  local owner = unit_player(caster)
  for _, unit in ipairs(game.units_near(x, y, config.milli(SOUL_CATCHER.radius))) do
    if game.alive(unit) and not game.is_structure(unit) and game.hostile(owner, unit_player(unit)) then
      game.set_counter(unit, "amp_all_pct", SOUL_CATCHER.percent[rank])
      game.set_counter(unit, "amp_all_until", game.ticks() + SOUL_CATCHER.duration)
    end
  end
end

local DECREPIFY = {
  source = "tooltip ability.ini:103608 (A319): the target takes 30/40/50/60 percent more spell damage (25 percent on an ally) for 3.5 s; spell damage is a hit the sim does not flag as an attack",
  ability = "pugna_decrepify",
  casts = { "pugna_decrepify" },
  percent = { 30, 40, 50, 60 },
  ally_percent = 25,
  duration = 56,
}

function DECREPIFY.run(caster)
  local rank = math.min(rank_of(caster, DECREPIFY.ability), #DECREPIFY.percent)
  local target = game.cast_target(caster)
  if rank == 0 or target == nil or not game.alive(target) then return end
  local enemy = game.hostile(unit_player(caster), unit_player(target))
  game.set_counter(target, "amp_spell_pct", enemy and DECREPIFY.percent[rank] or DECREPIFY.ally_percent)
  game.set_counter(target, "amp_spell_until", game.ticks() + DECREPIFY.duration)
end

local amplifying = false

local function amplify_hit(attacker, victim, dealt, attack)
  if amplifying or attacker == victim then return end
  local now = game.ticks()
  local percent = 0
  if game.counter(victim, "amp_all_until") > now then percent = percent + game.counter(victim, "amp_all_pct") end
  if not attack and game.counter(victim, "amp_spell_until") > now then percent = percent + game.counter(victim, "amp_spell_pct") end
  if percent == 0 then return end
  local extra = math.floor(dealt * percent / 100 / WHOLE)
  if extra < 1 then return end
  amplifying = true
  game.spell_damage(attacker >= 0 and attacker or victim, victim, extra, PURE)
  amplifying = false
end

local MANA_SHIELD = {
  source = "ability.ini:35279-35290, war3map.j:4518 (A... Mana Shield): a toggle; while on, 60 percent of each hit is paid from mana at 1.6/1.9/2.2/2.5 damage per mana (1.3 + 0.3 x rank); the share is healed back after the hit, so a hit that kills outright is not absorbed",
  ability = "medusa_mana_shield",
  casts = { "medusa_mana_shield" },
  absorb_percent = 60,
  per_mana_x10 = { 16, 19, 22, 25 },
}

function MANA_SHIELD.run(caster)
  game.set_counter(caster, "mshield_on", 1 - game.counter(caster, "mshield_on"))
end

function MANA_SHIELD.hit(attacker, victim, dealt)
  if game.counter(victim, "mshield_on") ~= 1 or not game.alive(victim) then return end
  local rank = math.min(rank_of(victim, MANA_SHIELD.ability), #MANA_SHIELD.per_mana_x10)
  if rank == 0 then
    game.set_counter(victim, "mshield_on", 0)
    return
  end
  local absorbed = dealt * MANA_SHIELD.absorb_percent / 100
  local per_mana = MANA_SHIELD.per_mana_x10[rank] / 10
  local mana = game.mana(victim)
  local used = math.min(mana, math.ceil(absorbed / WHOLE / per_mana))
  if used <= 0 then return end
  game.set_mana(victim, mana - used)
  game.set_hp(victim, unit_hp(victim) + math.min(absorbed, used * per_mana * WHOLE))
end

local SADIST = {
  source = "war3map.j WDO 70893-70900, WCO 70877-70892, WBO 70853-70876 (A060): each kill adds 2 x rank mana and rank life a second for 6 s, ten times that for a hero kill; the rank-four doubling is the scepter and is not modelled",
  ability = "necrolyte_sadist",
  kind = "necrolyte",
  pulses = 6,
  hero_multiplier = 10,
}
local sadist_slot = slot_cache(SADIST.ability)
local sadist = {}

function SADIST.died(unit, killer)
  if killer == nil or killer < 0 or killer == unit or game.unit_kind(killer) ~= SADIST.kind or not game.is_attributed(killer) then return end
  local slot = sadist_slot(killer)
  if slot == false or not game.hostile(unit_player(killer), unit_player(unit)) then return end
  local rank = math.min(game.skill_rank(killer, slot), 3)
  if rank == 0 then return end
  local scale = units.is_hero(unit) and SADIST.hero_multiplier or 1
  sadist[#sadist + 1] = { unit = killer, left = SADIST.pulses, life = rank * scale, mana = 2 * rank * scale }
end

local function tick_sadist()
  if game.ticks() % TICKS_PER_SECOND ~= 0 then return end
  local kept = {}
  for _, entry in ipairs(sadist) do
    if game.alive(entry.unit) then
      game.set_hp(entry.unit, unit_hp(entry.unit) + entry.life * WHOLE)
      game.set_mana(entry.unit, game.mana(entry.unit) + entry.mana)
      entry.left = entry.left - 1
      if entry.left > 0 then kept[#kept + 1] = entry end
    end
  end
  sadist = kept
end

local STATIC_LINK = {
  source = "war3map.j V3R 73771-73792, V2R, V1R 73716-73750, V0R 73694-73715, V_R 73666-73693 (A1DP): up to 8 s of 7 x rank damage a second moved from the hero target to Razor, broken beyond 700, the sum kept 18 s after the link ends",
  ability = "razor_static_link",
  casts = { "razor_static_link" },
  link_ticks = 8 * TICKS_PER_SECOND,
  hold_ticks = 18 * TICKS_PER_SECOND,
  break_radius = 10938,
  per_rank_per_second = 7,
}
local links = {}

local function set_link_part(unit, key, value)
  game.add_stat(unit, S_DAMAGE, value - game.counter(unit, key))
  game.set_counter(unit, key, value)
end

local function link_distance_exceeds(a, b, limit)
  local ax, ay = unit_position(a)
  local bx, by = unit_position(b)
  local reach = config.milli(limit)
  return (ax - bx) * (ax - bx) + (ay - by) * (ay - by) > reach * reach
end

function STATIC_LINK.run(caster)
  local rank = rank_of(caster, STATIC_LINK.ability)
  local target = game.cast_target(caster)
  if rank == 0 or target == nil or not game.alive(target) or not game.is_attributed(target) then return end
  links[#links + 1] = { razor = caster, target = target, rank = rank, elapsed = 0, bonus_given = 0 }
end

local function tick_links()
  local kept = {}
  for _, link in ipairs(links) do
    local alive = game.alive(link.razor) and game.alive(link.target)
    if link.hold_until == nil then
      if alive and link.elapsed < STATIC_LINK.link_ticks and not link_distance_exceeds(link.razor, link.target, STATIC_LINK.break_radius) then
        link.elapsed = link.elapsed + 1
        local bonus = math.floor(link.rank * STATIC_LINK.per_rank_per_second * link.elapsed / TICKS_PER_SECOND)
        set_link_part(link.razor, "slink_gain", game.counter(link.razor, "slink_gain") + bonus - link.bonus_given)
        set_link_part(link.target, "slink_loss", game.counter(link.target, "slink_loss") - bonus + link.bonus_given)
        link.bonus_given = bonus
      else
        link.hold_until = game.ticks() + STATIC_LINK.hold_ticks
      end
    end
    if link.hold_until ~= nil and (game.ticks() >= link.hold_until or not alive) then
      set_link_part(link.razor, "slink_gain", game.counter(link.razor, "slink_gain") - link.bonus_given)
      set_link_part(link.target, "slink_loss", game.counter(link.target, "slink_loss") + link.bonus_given)
    else
      kept[#kept + 1] = link
    end
  end
  links = kept
end

local MANA_BURN = {
  source = "war3map.j:71086-71088 (A... Mana Burn): intelligence x (3 + 0.5 x rank) mana burned, capped by the target's mana, and the same amount dealt as magical damage",
  ability = "nyx_assassin_mana_burn_nyx",
  casts = { "nyx_assassin_mana_burn_nyx" },
  per_int_x10 = { 35, 40, 45, 50 },
}

function MANA_BURN.run(caster)
  local rank = math.min(rank_of(caster, MANA_BURN.ability), #MANA_BURN.per_int_x10)
  local target = game.cast_target(caster)
  if rank == 0 or target == nil or not game.alive(target) then return end
  local want = math.floor(game.counter(caster, "attr_int") * MANA_BURN.per_int_x10[rank] / 10)
  local burned = math.min(want, game.mana(target))
  if burned <= 0 then return end
  game.set_mana(target, game.mana(target) - burned)
  game.spell_damage(caster, target, burned, MAGICAL)
end

local CURSE = {
  source = "war3map.j:54568-54569, 54646-54647 (Curse of the Silent, Global Silence scepter): a cursed unit loses 8 x rank mana a second for 6 s; the scepter Global Silence curses every enemy hero and so also applies the 5 + 15 x rank damage a second here",
  curse = "silencer_curse_of_the_silent",
  global = "silencer_global_silence",
  radius = 5859,
  mana_per_rank = 8,
  damage_base = 5,
  damage_per_rank = 15,
  pulses = 6,
}
local cursed = {}

local function scepter_held(unit)
  return game.upgrade_bits(unit) % 2 == 1
end

local function curse_unit(caster, unit, rank, damage)
  if game.alive(unit) and not game.is_structure(unit) and game.hostile(unit_player(caster), unit_player(unit)) then
    cursed[#cursed + 1] = { source = caster, unit = unit, left = CURSE.pulses, mana = CURSE.mana_per_rank * rank, damage = damage }
  end
end

local CURSE_CAST = {
  casts = { "silencer_curse_of_the_silent" },
  run = function(caster)
    local rank = rank_of(caster, CURSE.curse)
    local x, y = game.cast_point(caster)
    if rank == 0 or x == nil then return end
    for _, unit in ipairs(game.units_near(x, y, config.milli(CURSE.radius))) do curse_unit(caster, unit, rank, 0) end
  end,
}

local CURSE_GLOBAL = {
  casts = { "silencer_global_silence" },
  run = function(caster)
    local rank = rank_of(caster, CURSE.curse)
    if rank == 0 or not scepter_held(caster) then return end
    for unit = 0, get_unit_count() - 1 do
      if game.is_attributed(unit) then curse_unit(caster, unit, rank, CURSE.damage_base + CURSE.damage_per_rank * rank) end
    end
  end,
}

local function tick_curse()
  if game.ticks() % TICKS_PER_SECOND ~= 0 then return end
  local kept = {}
  for _, entry in ipairs(cursed) do
    if game.alive(entry.unit) then
      game.set_mana(entry.unit, math.max(0, game.mana(entry.unit) - entry.mana))
      if entry.damage > 0 then game.spell_damage(entry.source, entry.unit, entry.damage, MAGICAL) end
      entry.left = entry.left - 1
      if entry.left > 0 then kept[#kept + 1] = entry end
    end
  end
  cursed = kept
end

local ESSENCE_AURA = {
  source = "war3map.j:63864-63865 (Essence Aura): 40 percent chance when an allied hero within 1400 casts to give it 10/15/20/25 percent of its maximum mana",
  ability = "obsidian_destroyer_essence_aura",
  kind = "obsidian_destroyer",
  radius = 21875,
  chance = 40,
  percent = { 10, 15, 20, 25 },
}
local essence_slot = slot_cache(ESSENCE_AURA.ability)

function ESSENCE_AURA.cast(caster)
  if not game.is_attributed(caster) then return end
  local x, y = unit_position(caster)
  local owner = unit_player(caster)
  for _, other in ipairs(game.units_near(x, y, config.milli(ESSENCE_AURA.radius))) do
    if game.alive(other) and game.is_attributed(other) and game.unit_kind(other) == ESSENCE_AURA.kind and not game.hostile(owner, unit_player(other)) then
      local slot = essence_slot(other)
      local rank = slot and math.min(game.skill_rank(other, slot), #ESSENCE_AURA.percent) or 0
      if rank > 0 and game.random(100) < ESSENCE_AURA.chance then
        game.set_mana(caster, game.mana(caster) + math.floor(game.max_mana(caster) * ESSENCE_AURA.percent[rank] / 100))
        return
      end
    end
  end
end

local DEMONIC_PURGE = {
  source = "tooltip and war3map.j:82428 (Demonic Purge): the target loses its positive effects at once and takes 4 x (25 + 25 x rank) magical damage when the 2 s slow ends",
  ability = "shadow_demon_demonic_purge",
  casts = { "shadow_demon_demonic_purge" },
  delay = 32,
}
local purges = {}

function DEMONIC_PURGE.run(caster)
  local rank = rank_of(caster, DEMONIC_PURGE.ability)
  local target = game.cast_target(caster)
  if rank == 0 or target == nil or not game.alive(target) then return end
  game.clear_timed_mods(target, true)
  purges[#purges + 1] = { source = caster, unit = target, due = game.ticks() + DEMONIC_PURGE.delay, damage = 4 * (25 + 25 * rank) }
end

local function tick_purges()
  local kept = {}
  for _, purge in ipairs(purges) do
    if game.ticks() >= purge.due then
      if game.alive(purge.unit) then game.spell_damage(purge.source, purge.unit, purge.damage, MAGICAL) end
    else
      kept[#kept + 1] = purge
    end
  end
  purges = kept
end

local POWER_COG = {
  source = "war3map.j MAX 47219-47320, MOX 47189-47199 (A0Z5, units u00S/h0GC) and ability.ini Ubertip: eight cogs in a ring around Rattletrap for 5/6/7/8 s that die on 2/2/2/3 attacks; an enemy with mana that touches one takes 80/120/160/200 magical damage and loses that much mana (the knockback is not modelled; contact range and its 1 s repeat are GUESS)",
  rank = { rattletrap_power_cog_1 = 1, rattletrap_power_cog_2 = 2, rattletrap_power_cog_3 = 3, rattletrap_power_cog_4 = 4 },
  hits_to_kill = { 2, 2, 2, 3 },
  amount = { 80, 120, 160, 200 },
  contact_radius = 1562,
  repeat_ticks = 16,
  pulse_ticks = 2,
  scan_radius = 6000,
}
local cog_sources = {}

function POWER_COG.cast(caster, ability)
  if ability == "rattletrap_power_cog" then
    cog_sources[caster] = game.ticks() + 8 * TICKS_PER_SECOND + 4
  end
end

function POWER_COG.hit(attacker, victim)
  local rank = POWER_COG.rank[game.unit_kind(victim)]
  if rank == nil then return end
  local hits = game.counter(victim, "cog_hits") + 1
  game.set_counter(victim, "cog_hits", hits)
  if hits >= POWER_COG.hits_to_kill[rank] then
    game.spell_damage(attacker >= 0 and attacker or victim, victim, MAX_HIT, PURE)
  else
    game.set_hp(victim, game.max_hp(victim))
  end
end

local function tick_cogs()
  if game.ticks() % POWER_COG.pulse_ticks ~= 0 then return end
  local now = game.ticks()
  for source, until_tick in pairs(cog_sources) do
    if now > until_tick then
      cog_sources[source] = nil
    else
      local sx, sy = unit_position(source)
      local owner = unit_player(source)
      for _, cog in ipairs(game.units_near(sx, sy, config.milli(POWER_COG.scan_radius))) do
        local rank = POWER_COG.rank[game.unit_kind(cog)]
        if rank ~= nil and game.alive(cog) then
          local cx, cy = unit_position(cog)
          for _, enemy in ipairs(game.units_near(cx, cy, config.milli(POWER_COG.contact_radius))) do
            if game.alive(enemy) and not game.is_structure(enemy) and game.hostile(owner, unit_player(enemy))
              and game.mana(enemy) > 0 and game.counter(enemy, "cog_shock_until") <= now then
              game.set_counter(enemy, "cog_shock_until", now + POWER_COG.repeat_ticks)
              local amount = POWER_COG.amount[rank]
              game.set_mana(enemy, math.max(0, game.mana(enemy) - amount))
              game.spell_damage(source, enemy, amount, MAGICAL)
            end
          end
        end
      end
    end
  end
end

local MEEPO = {
  source = "war3map.j SBO 68316-68338, SCO, SDO, C_V 13575 (A0MW): clones are images that keep the hero's damage and die together with it; Poof (war3map.j:67550-67590) damages 60 + 20 x rank within 400 at the departure and the arrival and moves the caster to the clone nearest the target. GUESS: clones are images, so they share no experience or items; one clone per rank, one more with the scepter",
  kind = "meepo",
  ability = "meepo_divided_we_stand",
  poof = "meepo_poof",
  clone_life = 2000000,
  check_ticks = TICKS_PER_SECOND,
  poof_radius = 6250,
  poof_base = 60,
  poof_per_rank = 20,
  clones_cap = 4,
}
local clone_owner = {}
local clones_of = {}

local function live_clones(owner)
  local live = {}
  for _, clone in ipairs(clones_of[owner] or {}) do
    if game.alive(clone) then live[#live + 1] = clone end
  end
  clones_of[owner] = live
  return live
end

local function kill_unit(unit)
  if game.alive(unit) then game.spell_damage(unit, unit, MAX_HIT, PURE) end
end

local function maintain_clones()
  if game.ticks() % MEEPO.check_ticks ~= 0 then return end
  for unit = 0, get_unit_count() - 1 do
    if clone_owner[unit] == nil and game.alive(unit) and game.is_attributed(unit) and game.unit_kind(unit) == MEEPO.kind then
      local rank = rank_of(unit, MEEPO.ability)
      local want = math.min(MEEPO.clones_cap, rank + ((rank > 0 and scepter_held(unit)) and 1 or 0))
      local have = live_clones(unit)
      if #have < want then
        local x, y = unit_position(unit)
        local id = game.spawn_image(unit, x, y, MEEPO.clone_life, 100, 100)
        if id ~= nil and id >= 0 then
          clone_owner[id] = unit
          have[#have + 1] = id
          clones_of[unit] = have
        end
      end
    end
  end
end

function MEEPO.died(unit)
  local owner = clone_owner[unit]
  if owner ~= nil then
    clone_owner[unit] = nil
    kill_unit(owner)
    for _, clone in ipairs(clones_of[owner] or {}) do kill_unit(clone) end
  elseif clones_of[unit] ~= nil then
    for _, clone in ipairs(clones_of[unit]) do kill_unit(clone) end
  end
end

local function poof_blast(caster, x, y, damage)
  local owner = unit_player(caster)
  for _, unit in ipairs(game.units_near(x, y, config.milli(MEEPO.poof_radius))) do
    if game.alive(unit) and not game.is_structure(unit) and game.hostile(owner, unit_player(unit)) then
      game.spell_damage(caster, unit, damage, MAGICAL)
    end
  end
end

local MEEPO_POOF = {
  casts = { "meepo_poof" },
  run = function(caster)
    local owner = clone_owner[caster] or caster
    local rank = rank_of(owner, MEEPO.poof)
    if rank == 0 then rank = rank_of(caster, MEEPO.poof) end
    if rank == 0 then return end
    local bodies = { owner }
    for _, clone in ipairs(live_clones(owner)) do bodies[#bodies + 1] = clone end
    local target = game.cast_target(caster)
    local tx, ty
    if target ~= nil then
      tx, ty = unit_position(target)
    else
      tx, ty = game.cast_point(caster)
    end
    if tx == nil then return end
    local best, best_d
    for _, body in ipairs(bodies) do
      if body ~= caster and game.alive(body) then
        local bx, by = unit_position(body)
        local d = (bx - tx) * (bx - tx) + (by - ty) * (by - ty)
        if best == nil or d < best_d then best, best_d = body, d end
      end
    end
    if best == nil then return end
    local damage = MEEPO.poof_base + MEEPO.poof_per_rank * rank
    local fx, fy = unit_position(caster)
    local ax, ay = unit_position(best)
    poof_blast(caster, fx, fy, damage)
    game.teleport(caster, ax, ay)
    poof_blast(caster, ax, ay, damage)
  end,
}

local SPECS = { DEATH_PACT, STATIC_FIELD, AFTERSHOCK, FIERY_SOUL, WARPATH, KRAKEN_SHELL, ROT, CARAPACE, SOUL_CATCHER, DECREPIFY, MANA_SHIELD, STATIC_LINK, MANA_BURN, CURSE_CAST, CURSE_GLOBAL, DEMONIC_PURGE, MEEPO_POOF }

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
    NETHER_WARD.cast(caster, ability_name)
    ESSENCE_AURA.cast(caster)
    POWER_COG.cast(caster, ability_name)
    local runs = ON_CAST[ability_name]
    if runs == nil then return end
    for _, run in ipairs(runs) do run(caster) end
  end,
  on_hit = function(attacker, victim, dealt, _, attack)
    KRAKEN_SHELL.hit(attacker, victim, dealt)
    amplify_hit(attacker, victim, dealt, attack)
    REACTIVE_ARMOR.hit(attacker, victim)
    CAUSTIC_FINALE.hit(attacker, victim)
    CARAPACE.hit(attacker, victim, dealt)
    NETHER_WARD.hit(attacker, victim, dealt)
    MANA_SHIELD.hit(attacker, victim, dealt)
    POWER_COG.hit(attacker, victim)
  end,
  on_died = function(unit, killer)
    last_will(unit, killer)
    FLESH_HEAP.died(unit, killer)
    NECROMASTERY.died(unit, killer)
    if marked[unit] ~= nil then explode_caustic(unit, false) end
    SADIST.died(unit, killer)
    MEEPO.died(unit)
  end,
  on_tick = function()
    expire_pacts()
    expire_fiery()
    expire_warpath()
    expire_reactive()
    refresh_flesh()
    expire_caustic()
    tick_rot()
    tick_sadist()
    tick_links()
    tick_curse()
    tick_purges()
    tick_cogs()
    maintain_clones()
  end,
  on_state_replaced = function()
    kraken_slot = {}
    rebuild_pacts()
    rebuild_fiery()
    rebuild_warpath()
    rebuild_reactive()
    rebuild_flesh()
    marked = {}
    caustic_slot = slot_cache(CAUSTIC_FINALE.ability)
    rebuild_rot()
    reflected = {}
    carapace_slot = slot_cache(CARAPACE.ability)
    sadist = {}
    sadist_slot = slot_cache(SADIST.ability)
    links = {}
    cursed = {}
    purges = {}
    cog_sources = {}
    clone_owner = {}
    clones_of = {}
    essence_slot = slot_cache(ESSENCE_AURA.ability)
  end,
}

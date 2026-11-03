local config = require("config")
local units = require("units")
local kit = require("spell_kit")

local TICKS = 16
local WHOLE = 65536
local PHYSICAL, MAGICAL, PURE = 1, 2, 3
local MAX_HIT = 32767
local S_DAMAGE, S_SPEED_PCT, S_ARMOR_X100, S_ATTACK_SPEED = 0, 19, 26, 12

local rank_of, slot_of, dispel_negative, held = kit.rank_of, kit.slot_of, kit.dispel_negative, kit.held
local enemy_of, targetable_enemy, enemies_around = kit.enemy_of, kit.targetable_enemy, kit.enemies_around
local heal_percent, heroes, counter_units = kit.heal_percent, kit.heroes, kit.counter_units
local max_hp, strength_of, agility_of = kit.max_hp, kit.strength_of, kit.agility_of

local MANA_VOID = {
  source = "war3map.j H4X 44846-44864 (A2WE): (0.35+0.25*rank) per missing mana of the target to every enemy non-structure within 525 of it; tw type 1 read as magical",
  ability = "antimage_mana_void",
  casts = { "antimage_mana_void" },
  radius_milli = 8203,
  base_hundredths = 35,
  per_rank_hundredths = 25,
}

function MANA_VOID.run(caster)
  local rank = rank_of(caster, MANA_VOID.ability)
  local target = game.cast_target(caster)
  if rank == 0 or not targetable_enemy(caster, target) then return end
  local missing = game.max_mana(target) - game.mana(target)
  local damage = math.min(MAX_HIT, math.floor((MANA_VOID.base_hundredths + MANA_VOID.per_rank_hundredths * rank) * missing / 100))
  if damage <= 0 then return end
  local x, y = unit_position(target)
  game.spell_damage(caster, target, damage, MAGICAL)
  for _, unit in ipairs(enemies_around(caster, x, y, MANA_VOID.radius_milli)) do
    if unit ~= target and not held(unit, "spell_immunity") then
      game.spell_damage(caster, unit, damage, MAGICAL)
    end
  end
end


local DEATH_COIL = {
  source = "tooltip of A0I3 (ability.ini:30874): heals an allied target by the damage, the caster pays a hit point cost",
  ability = "abaddon_death_coil",
  casts = { "abaddon_death_coil" },
  heal = { 100, 150, 200, 250 },
  cost = { 75, 100, 125, 150 },
}

function DEATH_COIL.run(caster)
  local rank = math.min(rank_of(caster, DEATH_COIL.ability), #DEATH_COIL.heal)
  local target = game.cast_target(caster)
  if rank == 0 or target == nil or not game.alive(target) then return end
  local hurt_by = DEATH_COIL.cost[rank] * WHOLE
  game.set_hp(caster, math.max(WHOLE, unit_hp(caster) - hurt_by))
  if not enemy_of(caster, target) and target ~= caster then
    game.set_hp(target, unit_hp(target) + DEATH_COIL.heal[rank] * WHOLE)
  end
end


local BATTLE_HUNGER = {
  source = "war3map.j COO 60794-60840 (A0S1 ends when the target or its killer dies, or on recast), A2AO (ability.ini:84095) +12 percent movement per afflicted enemy",
  ability = "axe_battle_hunger",
  casts = { "axe_battle_hunger" },
  duration = 10 * TICKS,
  speed_percent = 12,
  max_stacks = 6,
}

local hungry = {}

local function sync_hunger_speed(axe)
  local count = 0
  for unit, source in pairs(hungry) do
    if source == axe then count = count + 1 end
  end
  local speed = math.min(count, BATTLE_HUNGER.max_stacks) * BATTLE_HUNGER.speed_percent
  game.add_stat(axe, S_SPEED_PCT, speed - game.counter(axe, "hunger_speed"))
  game.set_counter(axe, "hunger_speed", speed)
end

local function end_hunger(unit)
  local source = hungry[unit]
  hungry[unit] = nil
  game.set_counter(unit, "hunger_source", 0)
  game.set_counter(unit, "hunger_due", 0)
  if source ~= nil then sync_hunger_speed(source) end
end

function BATTLE_HUNGER.run(caster)
  local target = game.cast_target(caster)
  if target == nil or not game.alive(target) or not enemy_of(caster, target) or held(target, "spell_immunity") then return end
  local previous = hungry[target]
  hungry[target] = caster
  game.set_counter(target, "hunger_source", caster + 1)
  game.set_counter(target, "hunger_due", game.ticks() + BATTLE_HUNGER.duration)
  sync_hunger_speed(caster)
  if previous ~= nil and previous ~= caster then sync_hunger_speed(previous) end
end

local function expire_hunger()
  for unit in pairs(hungry) do
    if game.ticks() >= game.counter(unit, "hunger_due") or not game.alive(unit) then end_hunger(unit) end
  end
end

local function rebuild_hunger()
  hungry = {}
  for _, unit in ipairs(counter_units("hunger_source")) do
    hungry[unit] = game.counter(unit, "hunger_source") - 1
  end
end

local function hunger_died(victim, killer)
  if hungry[victim] ~= nil then end_hunger(victim) end
  if killer ~= nil and hungry[killer] ~= nil then
    game.clear_dots(killer)
    game.clear_status(killer, "slow")
    end_hunger(killer)
  end
end


local ATROPHY = {
  source = "tooltip of the atrophy aura (ability.ini): +5 damage for 30/35/40/45 s (30 s for a hero) per enemy that dies in range",
  ability = "abyssal_underlord_atrophy_aura",
  radius_milli = 14063,
  stack_damage = 5,
  seconds = { 30, 35, 40, 45 },
  hero_seconds = 30,
}

local atrophy = {}

local function apply_atrophy(unit)
  local stacks = #atrophy[unit]
  local damage = stacks * ATROPHY.stack_damage
  game.add_stat(unit, S_DAMAGE, damage - game.counter(unit, "atrophy_damage"))
  game.set_counter(unit, "atrophy_damage", damage)
  game.set_counter(unit, "atrophy_count", stacks)
end

local function atrophy_kill(victim)
  local x, y = unit_position(victim)
  local owner = unit_player(victim)
  for _, unit in ipairs(game.units_near(x, y, config.milli(ATROPHY.radius_milli))) do
    if unit ~= victim and game.alive(unit) and game.hostile(unit_player(unit), owner) and units.is_hero(unit) then
      local rank = math.min(rank_of(unit, ATROPHY.ability), #ATROPHY.seconds)
      if rank > 0 then
        local seconds = ATROPHY.seconds[rank]
        if units.is_hero(victim) then seconds = ATROPHY.hero_seconds end
        local dues = atrophy[unit]
        if dues == nil then
          dues = {}
          atrophy[unit] = dues
        end
        dues[#dues + 1] = game.ticks() + seconds * TICKS
        apply_atrophy(unit)
      end
    end
  end
end

local function expire_atrophy()
  for unit, dues in pairs(atrophy) do
    local kept = {}
    for _, due in ipairs(dues) do
      if due > game.ticks() then kept[#kept + 1] = due end
    end
    if #kept ~= #dues then
      atrophy[unit] = kept
      apply_atrophy(unit)
      if #kept == 0 then atrophy[unit] = nil end
    end
  end
end

local function rebuild_atrophy()
  atrophy = {}
  for _, unit in ipairs(counter_units("atrophy_count")) do
    local dues = {}
    for n = 1, game.counter(unit, "atrophy_count") do dues[n] = game.ticks() + ATROPHY.seconds[1] * TICKS end
    atrophy[unit] = dues
  end
end


local MISS = {
  haze = {
    source = "ability.ini Acdh (brewmaster Drunken Haze): DataB 45/55/65/75 percent miss chance for 8 s on enemies within 200 of the target",
    ability = "brewmaster_drunken_haze",
    casts = { "brewmaster_drunken_haze" },
    percent = { 45, 55, 65, 75 },
    ticks = 128,
    radius_milli = 3125,
  },
  bite = {
    source = "ability.ini:24119 (A0BM DataB 0.3/0.4/0.5/0.6 for 2 s on the unit bitten, applied on every attack by war3map.j:63296)",
    ability = "broodmother_incapacitating_bite",
    percent = { 30, 40, 50, 60 },
    ticks = 32,
  },
}

local missing = {}

local function set_miss(unit, percent, ticks)
  local chance = math.floor(percent * WHOLE / 100)
  if chance > game.counter(unit, "miss_q16") or game.ticks() >= game.counter(unit, "miss_due") then
    game.set_counter(unit, "miss_q16", chance)
  end
  game.set_counter(unit, "miss_due", math.max(game.counter(unit, "miss_due"), game.ticks() + ticks))
  missing[unit] = true
end

local function expire_miss()
  for unit in pairs(missing) do
    if game.ticks() >= game.counter(unit, "miss_due") or not game.alive(unit) then
      game.set_counter(unit, "miss_q16", 0)
      game.set_counter(unit, "miss_due", 0)
      missing[unit] = nil
    end
  end
end

function MISS.haze.run(caster)
  local rank = math.min(rank_of(caster, MISS.haze.ability), #MISS.haze.percent)
  local target = game.cast_target(caster)
  if rank == 0 or not targetable_enemy(caster, target) then return end
  local x, y = unit_position(target)
  set_miss(target, MISS.haze.percent[rank], MISS.haze.ticks)
  for _, unit in ipairs(enemies_around(caster, x, y, MISS.haze.radius_milli)) do
    if not held(unit, "spell_immunity") then set_miss(unit, MISS.haze.percent[rank], MISS.haze.ticks) end
  end
end

local function bite_proc(caster)
  local rank = math.min(rank_of(caster, MISS.bite.ability), #MISS.bite.percent)
  local target = game.strike_of(caster)
  if rank == 0 or target < 0 or not targetable_enemy(caster, target) then return end
  set_miss(target, MISS.bite.percent[rank], MISS.bite.ticks)
end

local function rebuild_miss()
  missing = {}
  for _, unit in ipairs(counter_units("miss_due")) do missing[unit] = true end
end


local BLOODRAGE = {
  source = "tooltip of A2X9 (ability.ini:101082): heals 19/21/23/25 percent of maximum health on a kill and on a death (GUESS: the enraged unit heals on its kill, its caster heals when it dies)",
  ability = "bloodseeker_bloodrage",
  casts = { "bloodseeker_bloodrage" },
  percent = { 19, 21, 23, 25 },
  ticks = { 144, 160, 176, 192 },
}

function BLOODRAGE.run(caster)
  local rank = math.min(rank_of(caster, BLOODRAGE.ability), #BLOODRAGE.percent)
  local target = game.cast_target(caster) or caster
  if rank == 0 or not game.alive(target) then return end
  game.set_counter(target, "rage_rank", rank)
  game.set_counter(target, "rage_source", caster + 1)
  game.set_counter(target, "rage_due", game.ticks() + BLOODRAGE.ticks[rank])
end

local function raged(unit)
  return unit ~= nil and game.counter(unit, "rage_due") > game.ticks()
end

local function rage_died(victim, killer)
  if raged(victim) then
    heal_percent(game.counter(victim, "rage_source") - 1, BLOODRAGE.percent[game.counter(victim, "rage_rank")])
  end
  if killer ~= nil and raged(killer) then
    heal_percent(killer, BLOODRAGE.percent[game.counter(killer, "rage_rank")])
  end
end


local NIGHTMARE = {
  source = "war3map.j 61290-61336 (A04Y, ability.ini:14330): sleep for 4/5/6/7 s and 20 per second pure; damage wakes the sleeper and gives the attacker the same Nightmare. The sleeper's wake ignores hits by the caster so the pulses do not wake it (GUESS)",
  ability = "bane_nightmare",
  casts = { "bane_nightmare" },
  ticks = { 64, 80, 96, 112 },
  per_second = 20,
}

local sleepers = {}

local function start_sleep(unit, source, rank)
  game.apply_status(unit, "stun", NIGHTMARE.ticks[rank])
  game.set_counter(unit, "nightmare_source", source + 1)
  game.set_counter(unit, "nightmare_rank", rank)
  game.set_counter(unit, "nightmare_due", game.ticks() + NIGHTMARE.ticks[rank])
  game.set_counter(unit, "nightmare_next", game.ticks() + TICKS)
  sleepers[unit] = true
end

local function end_sleep(unit)
  game.set_counter(unit, "nightmare_due", 0)
  sleepers[unit] = nil
end

function NIGHTMARE.run(caster)
  local rank = math.min(rank_of(caster, NIGHTMARE.ability), #NIGHTMARE.ticks)
  local target = game.cast_target(caster)
  if rank == 0 or not targetable_enemy(caster, target) then return end
  start_sleep(target, caster, rank)
end

local function pulse_sleepers()
  for unit in pairs(sleepers) do
    if not game.alive(unit) or game.ticks() >= game.counter(unit, "nightmare_due") then
      end_sleep(unit)
    elseif game.ticks() >= game.counter(unit, "nightmare_next") then
      game.set_counter(unit, "nightmare_next", game.ticks() + TICKS)
      local source = game.counter(unit, "nightmare_source") - 1
      if game.alive(source) then game.spell_damage(source, unit, NIGHTMARE.per_second, PURE) end
    end
  end
end

local function wake_sleeper(attacker, victim)
  if not sleepers[victim] or attacker == nil or attacker < 0 then return end
  local source = game.counter(victim, "nightmare_source") - 1
  if attacker == source then return end
  local rank = game.counter(victim, "nightmare_rank")
  game.clear_status(victim, "stun")
  end_sleep(victim)
  if game.alive(attacker) and enemy_of(victim, attacker) and not held(attacker, "spell_immunity") and game.alive(source) then
    start_sleep(attacker, source, rank)
  end
end


local COLD_FEET = {
  source = "war3map.j BVR 79550-79600 (A1MG, stun A1MJ): the target must stay within 765 of the mark for 3.9 s, then is stunned 1.25/2/2.75/3.5 s",
  ability = "ancient_apparition_cold_feet",
  casts = { "ancient_apparition_cold_feet" },
  radius_milli = 11953,
  wait = 62,
  stun = { 20, 32, 44, 56 },
}

local frozen = {}

function COLD_FEET.run(caster)
  local rank = math.min(rank_of(caster, COLD_FEET.ability), #COLD_FEET.stun)
  local target = game.cast_target(caster)
  if rank == 0 or not targetable_enemy(caster, target) then return end
  local x, y = unit_position(target)
  game.set_counter(target, "feet_x", x)
  game.set_counter(target, "feet_y", y)
  game.set_counter(target, "feet_rank", rank)
  game.set_counter(target, "feet_due", game.ticks() + COLD_FEET.wait)
  frozen[target] = true
end

local function check_feet()
  for unit in pairs(frozen) do
    local out = not game.alive(unit) or not game.within(unit, game.counter(unit, "feet_x"), game.counter(unit, "feet_y"), COLD_FEET.radius_milli)
    if out then
      game.set_counter(unit, "feet_due", 0)
      frozen[unit] = nil
    elseif game.ticks() >= game.counter(unit, "feet_due") then
      game.apply_status(unit, "stun", COLD_FEET.stun[game.counter(unit, "feet_rank")])
      game.set_counter(unit, "feet_due", 0)
      frozen[unit] = nil
    end
  end
end


local APHOTIC = {
  source = "war3map.j NKE 60572-60600, B4O 60540-60570, B2O 60525-60538 (A0MF): a 80+30*rank shield for 15 s that dispels on cast and, broken or expired, deals its full amount as magical damage within 700 of the holder",
  ability = "abaddon_aphotic_shield",
  casts = { "abaddon_aphotic_shield" },
  amount = { 110, 140, 170, 200 },
  radius_milli = 10938,
  duration = 15 * TICKS,
}

local shielded = {}

function APHOTIC.run(caster)
  local rank = math.min(rank_of(caster, APHOTIC.ability), #APHOTIC.amount)
  local target = game.cast_target(caster)
  if rank == 0 or target == nil or not game.alive(target) then return end
  dispel_negative(target)
  game.set_counter(target, "aphotic_source", caster + 1)
  game.set_counter(target, "aphotic_rank", rank)
  game.set_counter(target, "aphotic_due", game.ticks() + APHOTIC.duration)
  shielded[target] = true
end

local function release_aphotic(unit)
  local source = game.counter(unit, "aphotic_source") - 1
  local amount = APHOTIC.amount[game.counter(unit, "aphotic_rank")]
  game.set_counter(unit, "aphotic_due", 0)
  shielded[unit] = nil
  if source < 0 or not game.alive(source) then return end
  local x, y = unit_position(unit)
  for _, enemy in ipairs(enemies_around(source, x, y, APHOTIC.radius_milli)) do
    if not held(enemy, "spell_immunity") then game.spell_damage(source, enemy, amount, MAGICAL) end
  end
end

local function watch_aphotic()
  for unit in pairs(shielded) do
    if not game.alive(unit) then
      release_aphotic(unit)
    elseif game.ticks() >= game.counter(unit, "aphotic_due") then
      release_aphotic(unit)
    elseif game.shield_of(unit) <= 0 then
      release_aphotic(unit)
    end
  end
end


local BORROWED_TIME = {
  source = "war3map.j CEO 60718-60760 (A0NS): 4/5/6 s in which damage heals instead; cast, or taken below 400 life with the ability ready, dispels, cooldown 60/50/40 s (ability.ini:36626)",
  ability = "abaddon_borrowed_time",
  casts = { "abaddon_borrowed_time" },
  seconds = { 4, 5, 6 },
  cooldown_seconds = { 60, 50, 40 },
  threshold = 400 * WHOLE,
}

local borrowing = {}

local function borrow(unit, rank)
  local ticks = BORROWED_TIME.seconds[rank] * TICKS
  dispel_negative(unit)
  game.apply_status(unit, "death_prevention", ticks)
  game.set_counter(unit, "borrow_due", game.ticks() + ticks)
  borrowing[unit] = true
end

function BORROWED_TIME.run(caster)
  local rank = math.min(rank_of(caster, BORROWED_TIME.ability), #BORROWED_TIME.seconds)
  if rank > 0 then borrow(caster, rank) end
end

local function borrowed_hit(victim, dealt)
  if borrowing[victim] and game.ticks() < game.counter(victim, "borrow_due") then
    if game.alive(victim) then game.set_hp(victim, unit_hp(victim) + 2 * dealt) end
    return
  end
  if not units.is_hero(victim) or not game.alive(victim) or unit_hp(victim) >= BORROWED_TIME.threshold then return end
  local slot = slot_of(victim, BORROWED_TIME.ability)
  if slot == nil or held(victim, "break") or game.unit_cooldown(victim, slot) > 0 then return end
  local rank = math.min(game.skill_rank(victim, slot), #BORROWED_TIME.seconds)
  if rank == 0 then return end
  game.set_ability_cooldown(victim, slot, BORROWED_TIME.cooldown_seconds[rank] * TICKS)
  borrow(victim, rank)
end

local function expire_borrowing()
  for unit in pairs(borrowing) do
    if game.ticks() >= game.counter(unit, "borrow_due") or not game.alive(unit) then
      game.set_counter(unit, "borrow_due", 0)
      borrowing[unit] = nil
    end
  end
end


local RUPTURE = {
  source = "tooltip of the bloodseeker Rupture (ability.ini:33780): 20/40/60 percent of the distance moved, in war3 units, as pure damage for 12 s",
  ability = "bloodseeker_rupture",
  casts = { "bloodseeker_rupture" },
  percent = { 20, 40, 60 },
  duration = 12 * TICKS,
  charge_ticks = 640,
  milli_per_unit = 15.625,
}

local ruptured = {}

function RUPTURE.run(caster)
  local rank = math.min(rank_of(caster, RUPTURE.ability), #RUPTURE.percent)
  local target = game.cast_target(caster)
  if rank == 0 or not targetable_enemy(caster, target) then return end
  local x, y = unit_position(target)
  game.set_counter(target, "rupture_x", x)
  game.set_counter(target, "rupture_y", y)
  game.set_counter(target, "rupture_rank", rank)
  game.set_counter(target, "rupture_source", caster + 1)
  game.set_counter(target, "rupture_carry", 0)
  game.set_counter(target, "rupture_due", game.ticks() + RUPTURE.duration)
  ruptured[target] = true
  if kit.scepter(caster) then
    local slot = slot_of(caster, RUPTURE.ability)
    if slot ~= nil and game.ticks() >= game.counter(caster, "rupture_charge_at") then
      game.set_counter(caster, "rupture_charge_at", game.ticks() + RUPTURE.charge_ticks)
      game.set_ability_cooldown(caster, slot, 0)
    end
  end
end

local function pulse_rupture()
  for unit in pairs(ruptured) do
    local source = game.counter(unit, "rupture_source") - 1
    if not game.alive(unit) or game.ticks() >= game.counter(unit, "rupture_due") then
      game.set_counter(unit, "rupture_due", 0)
      ruptured[unit] = nil
    else
      local x, y = unit_position(unit)
      local dx, dy = x - game.counter(unit, "rupture_x"), y - game.counter(unit, "rupture_y")
      game.set_counter(unit, "rupture_x", x)
      game.set_counter(unit, "rupture_y", y)
      if dx ~= 0 or dy ~= 0 then
        local moved_milli = math.floor(math.sqrt(dx * dx + dy * dy) * 1000 / WHOLE)
        local carry = game.counter(unit, "rupture_carry") + math.floor(moved_milli * RUPTURE.percent[game.counter(unit, "rupture_rank")] / (RUPTURE.milli_per_unit * 100) * 1000)
        local whole = math.floor(carry / 1000)
        game.set_counter(unit, "rupture_carry", carry - whole * 1000)
        if whole > 0 and game.alive(source) then game.spell_damage(source, unit, math.min(whole, MAX_HIT), PURE) end
      end
    end
  end
end


local TRACK = {
  source = "war3map.j R3E 45414-45452, JHX 45330-45410 (A0B4): the mark lasts 30 s and the death of the marked unit pays the tracker 200/275/350 gold and each allied hero within 950 40/80/120",
  ability = "bounty_hunter_track",
  casts = { "bounty_hunter_track" },
  duration = 30 * TICKS,
  own = { 200, 275, 350 },
  allies = { 40, 80, 120 },
  radius_milli = 14844,
}

local tracked = {}

function TRACK.run(caster)
  local rank = math.min(rank_of(caster, TRACK.ability), #TRACK.own)
  local target = game.cast_target(caster)
  if rank == 0 or not targetable_enemy(caster, target) then return end
  game.set_counter(target, "track_source", caster + 1)
  game.set_counter(target, "track_rank", rank)
  game.set_counter(target, "track_due", game.ticks() + TRACK.duration)
  tracked[target] = true
end

local function pulse_track()
  for unit in pairs(tracked) do
    local source = game.counter(unit, "track_source") - 1
    local live = game.alive(unit) and game.counter(unit, "track_due") > game.ticks()
    local x, y = 0, 0
    if game.alive(unit) then x, y = unit_position(unit) end
    for _, hero in ipairs(heroes()) do
      if game.alive(hero) and source >= 0 and not enemy_of(source, hero) and hero ~= unit then
        local near = live and game.within(hero, x, y, TRACK.radius_milli)
        local want = 0
        if near then want = 20 end
        local have = game.counter(hero, "track_speed")
        if want ~= have then
          game.add_stat(hero, S_SPEED_PCT, want - have)
          game.set_counter(hero, "track_speed", want)
        end
      end
    end
    if not live then tracked[unit] = nil end
  end
end

local function track_died(victim)
  if game.counter(victim, "track_due") <= game.ticks() then return end
  local source = game.counter(victim, "track_source") - 1
  local rank = game.counter(victim, "track_rank")
  game.set_counter(victim, "track_due", 0)
  if source < 0 or rank == 0 then return end
  game.cmd_add_gold(unit_player(source), TRACK.own[rank])
  local x, y = unit_position(victim)
  for _, unit in ipairs(game.units_near(x, y, config.milli(TRACK.radius_milli))) do
    if unit ~= source and game.alive(unit) and units.is_hero(unit) and not enemy_of(source, unit) then
      game.cmd_add_gold(unit_player(unit), TRACK.allies[rank])
    end
  end
end


local BRISTLEBACK = {
  source = "war3map.j KYX 46262-46296, KWX 46248-46260, R6E 46045-46085 (A0M3, A0GP): a hit from behind (within 70 degrees of the victim's back) is healed back by 16/24/32/40 percent, from the sides (within 110) by 8/12/16/20 percent; every 230 damage taken from behind casts Quill Spray free, whose damage is 20*rank plus 25 per quill hit on the target in the last 14 s, at most 400",
  ability = "bristleback_bristleback",
  spray = "bristleback_quill_spray",
  back_percent = { 16, 24, 32, 40 },
  side_percent = { 8, 12, 16, 20 },
  back_turns = 12743,
  side_turns = 20024,
  trigger = 230 * WHOLE,
  radius_milli = 10156,
  stack = 25,
  stack_ticks = 14 * TICKS,
  cap = 400,
}

local function turn_gap(a, b)
  local gap = math.abs(a - b) % WHOLE
  if gap > WHOLE / 2 then gap = WHOLE - gap end
  return gap
end

local function quill_pass(spray_owner, extra_only)
  local rank = rank_of(spray_owner, "bristleback_quill_spray")
  if rank == 0 then rank = rank_of(spray_owner, BRISTLEBACK.ability) end
  if rank == 0 then return end
  local x, y = unit_position(spray_owner)
  for _, unit in ipairs(enemies_around(spray_owner, x, y, BRISTLEBACK.radius_milli)) do
    if not held(unit, "spell_immunity") then
      local stacked = 0
      if game.counter(unit, "quill_due") > game.ticks() then stacked = game.counter(unit, "quill_stack") end
      local base = 20 * rank
      local total = math.min(BRISTLEBACK.cap, base + stacked)
      local damage = total
      if extra_only then damage = total - base end
      if damage > 0 then game.spell_damage(spray_owner, unit, damage, 1) end
      game.set_counter(unit, "quill_stack", math.min(BRISTLEBACK.cap, stacked + BRISTLEBACK.stack))
      game.set_counter(unit, "quill_due", game.ticks() + BRISTLEBACK.stack_ticks)
    end
  end
end

function BRISTLEBACK.quill_cast(caster)
  quill_pass(caster, true)
end

local bristle_slot = {}

local function bristleback_hit(attacker, victim, dealt)
  if attacker == nil or attacker < 0 or not units.is_hero(victim) or not game.alive(victim) then return end
  if game.counter(victim, "bristle_off") > game.ticks() then return end
  local rank = math.min(rank_of(victim, BRISTLEBACK.ability), #BRISTLEBACK.back_percent)
  if rank == 0 or held(victim, "break") or dealt <= 5 * WHOLE then return end
  local ax, ay = unit_position(attacker)
  local vx, vy = unit_position(victim)
  local toward = game.bearing(vx - ax, vy - ay)
  if toward == nil then return end
  local gap = turn_gap(game.unit_heading(victim), toward)
  if gap <= BRISTLEBACK.back_turns then
    local back = math.floor(dealt * BRISTLEBACK.back_percent[rank] / 100)
    game.set_hp(victim, unit_hp(victim) + back)
    local taken = game.counter(victim, "bristle_acc") + dealt - back
    if taken > BRISTLEBACK.trigger then
      game.set_counter(victim, "bristle_acc", 0)
      quill_pass(victim, false)
    else
      game.set_counter(victim, "bristle_acc", taken)
    end
  elseif gap <= BRISTLEBACK.side_turns then
    game.set_hp(victim, unit_hp(victim) + math.floor(dealt * BRISTLEBACK.side_percent[rank] / 100))
  end
end


local TEMPEST = {
  source = "war3map.j 90186-90190 (A2M0): the caster keeps (0.45-0.15*rank... i.e. 70/85/100 percent of its mana, the life share is the vital_fraction effect",
  ability = "arc_warden_tempest_double",
  casts = { "arc_warden_tempest_double" },
  life = 320,
}

function TEMPEST.run(caster)
  local rank = rank_of(caster, TEMPEST.ability)
  if rank == 0 then return end
  local lost = 45 - 15 * rank
  game.set_mana(caster, math.floor(game.mana(caster) * (100 - lost) / 100))
  local x, y = unit_position(caster)
  local copy = game.spawn_image(caster, x + config.milli(3000), y, TEMPEST.life, 100, 100)
  if copy < 0 then return end
  for _, row in ipairs(game.unit_abilities(caster)) do
    game.set_skill_rank(copy, row.slot, game.skill_rank(caster, row.slot))
  end
  for _, row in ipairs(game.unit_items(caster)) do
    game.cmd_give_item(copy, row.name, row.count)
  end
end


local FIENDS_GRIP = {
  source = "tooltip of A02Q (ability.ini:11513) and war3map.j NME 61511-61560: 5 percent of the target's maximum mana moves to the caster each second of the 5 s channel",
  ability = "bane_fiend_s_grip",
  casts = { "bane_fiend_s_grip" },
  percent = 5,
  seconds = 5,
  scepter_extra = { 55, 60, 55 },
}

local gripping = {}

function FIENDS_GRIP.run(caster)
  local target = game.cast_target(caster)
  if not targetable_enemy(caster, target) then return end
  game.apply_status(target, "revealed", FIENDS_GRIP.seconds * TICKS)
  game.set_counter(caster, "grip_rank", math.min(rank_of(caster, FIENDS_GRIP.ability), 3))
  game.set_counter(caster, "grip_target", target + 1)
  game.set_counter(caster, "grip_left", FIENDS_GRIP.seconds)
  game.set_counter(caster, "grip_next", game.ticks() + TICKS)
  gripping[caster] = true
end

local function pulse_grips()
  for caster in pairs(gripping) do
    local target = game.counter(caster, "grip_target") - 1
    if not game.alive(caster) or not game.alive(target) or game.counter(caster, "grip_left") <= 0 or not held(target, "stun") then
      game.set_counter(caster, "grip_left", 0)
      gripping[caster] = nil
    elseif game.ticks() >= game.counter(caster, "grip_next") then
      game.set_counter(caster, "grip_next", game.ticks() + TICKS)
      game.set_counter(caster, "grip_left", game.counter(caster, "grip_left") - 1)
      local moved = math.min(game.mana(target), math.floor(game.max_mana(target) * FIENDS_GRIP.percent / 100))
      game.set_mana(target, game.mana(target) - moved)
      game.set_mana(caster, game.mana(caster) + moved)
      if kit.scepter(caster) then
        game.spell_damage(caster, target, FIENDS_GRIP.scepter_extra[math.min(game.counter(caster, "grip_rank"), 3)], MAGICAL)
      end
    end
  end
end


local GOO = {
  source = "war3map.j KCX 46109-46169, ability.ini:119923-120076 (A3F0..A3FD slow) and 112183-112240 (A37U..A37X armor) (A0FW): each cast on one target within 5 s adds a stack up to 4; the slow is 20+3*rank*stacks percent and the armor loss is (1+0.4*(rank-1))*stacks",
  ability = "bristleback_viscous_nasal_goo",
  casts = { "bristleback_viscous_nasal_goo" },
  ticks = 80,
  max_stacks = 4,
  scepter_milli = 10938,
}

local S_ARMOR_X100 = 26
local gooed = {}

local function set_goo_armor(unit, hundredths)
  game.add_stat(unit, S_ARMOR_X100, hundredths - game.counter(unit, "goo_armor"))
  game.set_counter(unit, "goo_armor", hundredths)
end

local function goo_one(caster, rank, target)
  local stacks = 0
  if game.counter(target, "goo_due") > game.ticks() then stacks = game.counter(target, "goo_stacks") end
  stacks = math.min(stacks + 1, GOO.max_stacks)
  game.set_counter(target, "goo_stacks", stacks)
  game.set_counter(target, "goo_due", game.ticks() + GOO.ticks)
  game.apply_slow(target, GOO.ticks, math.floor(WHOLE * (100 - (20 + 3 * rank * stacks)) / 100))
  set_goo_armor(target, -(100 + 40 * (rank - 1)) * stacks)
  gooed[target] = true
end

function GOO.run(caster)
  local rank = math.min(rank_of(caster, GOO.ability), 4)
  local target = game.cast_target(caster)
  if rank == 0 or not targetable_enemy(caster, target) then return end
  goo_one(caster, rank, target)
  if kit.scepter(caster) then
    local x, y = unit_position(caster)
    for _, unit in ipairs(enemies_around(caster, x, y, GOO.scepter_milli)) do
      if unit ~= target and not held(unit, "spell_immunity") then goo_one(caster, rank, unit) end
    end
  end
end

local function expire_goo()
  for unit in pairs(gooed) do
    if game.ticks() >= game.counter(unit, "goo_due") or not game.alive(unit) then
      set_goo_armor(unit, 0)
      game.set_counter(unit, "goo_stacks", 0)
      game.set_counter(unit, "goo_due", 0)
      gooed[unit] = nil
    end
  end
end


local QUILL = {
  casts = { "bristleback_quill_spray" },
  run = function(caster) BRISTLEBACK.quill_cast(caster) end,
}


local FLUX = {
  source = "war3map.j SFR 89905-89960 (A2M1, ability.ini:92096): for 6 s the target is slowed 50 percent and takes 7.5*rank every 0.5 s while no other enemy of the caster stands within 250 of it",
  ability = "arc_warden_flux",
  casts = { "arc_warden_flux" },
  ticks = 96,
  period = 8,
  alone_milli = 3906,
  slow_mul = 32768,
}

local fluxed = {}

local function flux_alone(caster, unit)
  local x, y = unit_position(unit)
  for _, other in ipairs(enemies_around(caster, x, y, FLUX.alone_milli)) do
    if other ~= unit then return false end
  end
  return true
end

function FLUX.run(caster)
  local rank = math.min(rank_of(caster, FLUX.ability), 4)
  local target = game.cast_target(caster)
  if rank == 0 or not targetable_enemy(caster, target) then return end
  game.set_counter(target, "flux_source", caster + 1)
  game.set_counter(target, "flux_rank", rank)
  game.set_counter(target, "flux_due", game.ticks() + FLUX.ticks)
  game.set_counter(target, "flux_next", game.ticks())
  fluxed[target] = true
end

local function pulse_flux()
  for unit in pairs(fluxed) do
    local source = game.counter(unit, "flux_source") - 1
    if not game.alive(unit) or not game.alive(source) or game.ticks() >= game.counter(unit, "flux_due") then
      game.set_counter(unit, "flux_due", 0)
      fluxed[unit] = nil
    elseif game.ticks() >= game.counter(unit, "flux_next") then
      game.set_counter(unit, "flux_next", game.ticks() + FLUX.period)
      if flux_alone(source, unit) then
        game.apply_slow(unit, FLUX.period + 4, FLUX.slow_mul)
        game.spell_damage(source, unit, math.floor(7.5 * game.counter(unit, "flux_rank")), MAGICAL)
      end
    end
  end
end

local FIREFLY = {
  source = "war3map.j DPO 62118-62134, DMO 62066-62117, DLO 62048-62064 (A19Z): for 18 s a trail point is left each time Batrider has moved 125 from the last, and every 0.5 s each enemy within 225 of any trail point takes 10*rank-5 magical damage (the flight is not modelled)",
  ability = "batrider_firefly",
  casts = { "batrider_firefly" },
  ticks = 288,
  step_milli = 1953,
  radius_milli = 3516,
  period = 8,
  cap = 24,
}

local firing = {}
local trails = {}

function FIREFLY.run(caster)
  local rank = math.min(rank_of(caster, FIREFLY.ability), 4)
  if rank == 0 then return end
  local x, y = unit_position(caster)
  trails[caster] = { { x, y } }
  game.set_counter(caster, "firefly_rank", rank)
  game.set_counter(caster, "firefly_due", game.ticks() + FIREFLY.ticks)
  game.set_counter(caster, "firefly_next", game.ticks() + FIREFLY.period)
  firing[caster] = true
end

local function burn_trail(caster)
  local rank = game.counter(caster, "firefly_rank")
  local hit = {}
  for _, point in ipairs(trails[caster]) do
    for _, unit in ipairs(enemies_around(caster, point[1], point[2], FIREFLY.radius_milli)) do
      if not hit[unit] and not held(unit, "spell_immunity") then
        hit[unit] = true
        game.spell_damage(caster, unit, 10 * rank - 5, MAGICAL)
      end
    end
  end
end

local function run_firefly()
  for caster in pairs(firing) do
    if not game.alive(caster) or game.ticks() >= game.counter(caster, "firefly_due") then
      game.set_counter(caster, "firefly_due", 0)
      firing[caster] = nil
      trails[caster] = nil
    else
      local trail = trails[caster]
      if trail == nil then
        trail = {}
        trails[caster] = trail
      end
      local x, y = unit_position(caster)
      local last = trail[#trail]
      if last == nil or not game.within(caster, last[1], last[2], FIREFLY.step_milli) then
        trail[#trail + 1] = { x, y }
        if #trail > FIREFLY.cap then table.remove(trail, 1) end
      end
      if game.ticks() >= game.counter(caster, "firefly_next") then
        game.set_counter(caster, "firefly_next", game.ticks() + FIREFLY.period)
        burn_trail(caster)
      end
    end
  end
end

local THUNDER_CLAP = {
  source = "ability.ini:114173-114175 (Dur 8 s on non-heroes, HeroDur 4 s): the slow of a non-hero lasts twice as long (the attack speed loss is not extended)",
  ability = "brewmaster_thunder_clap",
  casts = { "brewmaster_thunder_clap" },
  radius_milli = 6250,
  ticks = 128,
  slow_mul = { 49152, 42598, 36045, 29491 },
}

function THUNDER_CLAP.run(caster)
  local rank = math.min(rank_of(caster, THUNDER_CLAP.ability), #THUNDER_CLAP.slow_mul)
  if rank == 0 then return end
  local x, y = unit_position(caster)
  for _, unit in ipairs(enemies_around(caster, x, y, THUNDER_CLAP.radius_milli)) do
    if not units.is_hero(unit) and not held(unit, "spell_immunity") then
      game.apply_slow(unit, THUNDER_CLAP.ticks, THUNDER_CLAP.slow_mul[rank])
    end
  end
end

local PIT = {
  source = "war3map.j ZIO 72294-72332, ZRO 72283-72293 (A0RA, cage A0RI ability.ini:40793): for 7 s every enemy within 300 of the point is hit once for 100 magical and caged for 1/1.5/2/2.5 s",
  ability = "abyssal_underlord_pit_of_malice",
  casts = { "abyssal_underlord_pit_of_malice" },
  radius_milli = 4688,
  duration = 112,
  period = 4,
  damage = 100,
  root = { 16, 24, 32, 40 },
}

local pits = {}

function PIT.run(caster)
  local rank = math.min(rank_of(caster, PIT.ability), #PIT.root)
  local x, y = game.cast_point(caster)
  if rank == 0 or x == nil then return end
  pits[#pits + 1] = { caster = caster, rank = rank, x = x, y = y, due = game.ticks() + PIT.duration, hit = {} }
end

local function run_pits()
  local kept = {}
  for _, pit in ipairs(pits) do
    if game.alive(pit.caster) then
      for _, unit in ipairs(game.units_near(pit.x, pit.y, config.milli(PIT.radius_milli))) do
        if not pit.hit[unit] and game.alive(unit) and enemy_of(pit.caster, unit) and not game.is_structure(unit) and not held(unit, "spell_immunity") then
          pit.hit[unit] = true
          game.spell_damage(pit.caster, unit, PIT.damage, MAGICAL)
          game.apply_status(unit, "root", PIT.root[pit.rank])
        end
      end
      if game.ticks() < pit.due then kept[#kept + 1] = pit end
    end
  end
  pits = kept
end

local BACKSTAB = {
  source = "war3map.j J_X 45735-45760, R4E 45879 (A07A): the first hit out of invisibility deals 30*rank extra physical, once per 0.5 s (the window is the invisibility time, the strike ends it)",
  ability = "bounty_hunter_wind_walk",
  casts = { "bounty_hunter_wind_walk" },
  ticks = { 320, 400, 480, 560 },
  per_rank = 30,
}

function BACKSTAB.run(caster)
  local rank = math.min(rank_of(caster, BACKSTAB.ability), #BACKSTAB.ticks)
  if rank == 0 then return end
  game.set_counter(caster, "backstab_until", game.ticks() + BACKSTAB.ticks[rank])
  game.set_counter(caster, "backstab_rank", rank)
end

local function backstab_hit(attacker, victim)
  if attacker == nil or attacker < 0 or game.counter(attacker, "backstab_until") <= game.ticks() then return end
  if not game.alive(victim) or game.is_structure(victim) or not enemy_of(attacker, victim) then return end
  local rank = game.counter(attacker, "backstab_rank")
  game.set_counter(attacker, "backstab_until", 0)
  game.spell_damage(attacker, victim, BACKSTAB.per_rank * rank, PHYSICAL)
end

local SPIDERLINGS = {
  source = "war3map.j GEO 63290-63327 (A0BH, mark A3B8): when the marked unit dies, rank spiderlings (n019) appear where it stood for 60 s (GUESS: the mark lasts 6 s)",
  ability = "broodmother_spawn_spiderlings",
  casts = { "broodmother_spawn_spiderlings" },
  unit = "broodmother_spiderling",
  mark_ticks = 96,
  life = 960,
}

function SPIDERLINGS.run(caster)
  local target = game.cast_target(caster)
  if target == nil or not game.alive(target) or not enemy_of(caster, target) then return end
  game.set_counter(target, "spider_by", caster + 1)
  game.set_counter(target, "spider_due", game.ticks() + SPIDERLINGS.mark_ticks)
end

local function spiderlings_died(victim)
  if game.counter(victim, "spider_due") <= game.ticks() then return end
  local owner = game.counter(victim, "spider_by") - 1
  game.set_counter(victim, "spider_due", 0)
  if owner < 0 then return end
  local rank = math.min(math.max(rank_of(owner, SPIDERLINGS.ability), 1), 4)
  local x, y = unit_position(victim)
  for _ = 1, rank do
    local kind = kit.unit_index(SPIDERLINGS.unit)
    if kind then game.timed_spawn(owner, kind, SPIDERLINGS.life, x, y) end
  end
end

local FROSTMOURNE = {
  source = "tooltip of A0MG (ability.ini:35021): allied attackers of a unit Frostmourne marked gain 10/20/30/40 attack speed and 15 percent movement speed while the mark lasts (2.5 s, GUESS for the ally bonus window)",
  ability = "abaddon_frostmourne",
  attack_speed = { 10, 20, 30, 40 },
  speed_percent = 15,
  ticks = 40,
}

local boosted = {}

local function set_frost_boost(unit, attack, speed)
  game.add_stat(unit, S_ATTACK_SPEED, attack - game.counter(unit, "frost_attack"))
  game.add_stat(unit, S_SPEED_PCT, speed - game.counter(unit, "frost_speed"))
  game.set_counter(unit, "frost_attack", attack)
  game.set_counter(unit, "frost_speed", speed)
end

local function frostmourne_proc(caster)
  local rank = math.min(rank_of(caster, FROSTMOURNE.ability), #FROSTMOURNE.attack_speed)
  local target = game.strike_of(caster)
  if rank == 0 or target < 0 or not game.alive(target) then return end
  game.set_counter(target, "frost_mark_due", game.ticks() + FROSTMOURNE.ticks)
  game.set_counter(target, "frost_mark_rank", rank)
end

local function frostmourne_hit(attacker, victim)
  if attacker == nil or attacker < 0 or game.counter(victim, "frost_mark_due") <= game.ticks() then return end
  if enemy_of(attacker, victim) == false or not game.alive(attacker) then return end
  local rank = game.counter(victim, "frost_mark_rank")
  set_frost_boost(attacker, FROSTMOURNE.attack_speed[rank], FROSTMOURNE.speed_percent)
  game.set_counter(attacker, "frost_boost_due", game.ticks() + FROSTMOURNE.ticks)
  boosted[attacker] = true
end

local function expire_frost_boost()
  for unit in pairs(boosted) do
    if game.ticks() >= game.counter(unit, "frost_boost_due") or not game.alive(unit) then
      set_frost_boost(unit, 0, 0)
      game.set_counter(unit, "frost_boost_due", 0)
      boosted[unit] = nil
    end
  end
end

local SPARK_WRAITH = {
  source = "war3map.j 89697-89790 (A2LL): the wraith materializes after 3 s, waits up to 50 s and strikes the first enemy within 400 for 100+50*rank magical (GUESS: it strikes at once, no flight)",
  ability = "arc_warden_spark_wraith",
  casts = { "arc_warden_spark_wraith" },
  wait = 48,
  life = 800,
  radius_milli = 6250,
}

local wraiths = {}

function SPARK_WRAITH.run(caster)
  local rank = math.min(rank_of(caster, SPARK_WRAITH.ability), 4)
  local x, y = game.cast_point(caster)
  if rank == 0 or x == nil then return end
  wraiths[#wraiths + 1] = { caster = caster, rank = rank, x = x, y = y, ready = game.ticks() + SPARK_WRAITH.wait, due = game.ticks() + SPARK_WRAITH.life }
end

local function run_wraiths()
  local kept = {}
  for _, wraith in ipairs(wraiths) do
    local spent = false
    if game.alive(wraith.caster) and game.ticks() < wraith.due then
      if game.ticks() >= wraith.ready and game.ticks() % 4 == 0 then
        for _, unit in ipairs(enemies_around(wraith.caster, wraith.x, wraith.y, SPARK_WRAITH.radius_milli)) do
          if not held(unit, "spell_immunity") then
            game.spell_damage(wraith.caster, unit, 100 + 50 * wraith.rank, MAGICAL)
            spent = true
            break
          end
        end
      end
    else
      spent = true
    end
    if not spent then kept[#kept + 1] = wraith end
  end
  wraiths = kept
end

local CHILLING_TOUCH = {
  source = "tooltip of A?? Chilling Touch (ability.ini:61538): allied heroes within 525 get 50/60/70/80 bonus magical damage on their next 3/4/5/6 attacks for 30 s",
  ability = "ancient_apparition_chilling_touch",
  casts = { "ancient_apparition_chilling_touch" },
  radius_milli = 8203,
  damage = { 50, 60, 70, 80 },
  charges = { 3, 4, 5, 6 },
  ticks = 480,
}

local chilled = {}

function CHILLING_TOUCH.run(caster)
  local rank = math.min(rank_of(caster, CHILLING_TOUCH.ability), 4)
  local x, y = game.cast_point(caster)
  if rank == 0 then return end
  if x == nil then x, y = unit_position(caster) end
  for _, unit in ipairs(game.units_near(x, y, config.milli(CHILLING_TOUCH.radius_milli))) do
    if game.alive(unit) and units.is_hero(unit) and not enemy_of(caster, unit) then
      game.set_counter(unit, "chill_charges", CHILLING_TOUCH.charges[rank])
      game.set_counter(unit, "chill_damage", CHILLING_TOUCH.damage[rank])
      game.set_counter(unit, "chill_due", game.ticks() + CHILLING_TOUCH.ticks)
      chilled[unit] = true
    end
  end
end

local function chill_hit(attacker, victim)
  if attacker == nil or attacker < 0 or game.counter(attacker, "chill_due") <= game.ticks() then return end
  if game.counter(attacker, "chill_charges") <= 0 or not enemy_of(attacker, victim) or game.is_structure(victim) then return end
  if game.counter(victim, "chill_guard") == game.ticks() then return end
  game.set_counter(attacker, "chill_charges", game.counter(attacker, "chill_charges") - 1)
  game.set_counter(victim, "chill_guard", game.ticks() + 1)
  game.spell_damage(attacker, victim, game.counter(attacker, "chill_damage"), MAGICAL)
end

local LASSO = {
  source = "tooltip of A19O (ability.ini:54019, Area 500): the target is tied to Batrider for 3/3.5/4 s and cannot leave 500 of him, being dragged along (GUESS: the drag teleports it back inside the leash each 4 ticks)",
  ability = "batrider_flaming_lasso",
  casts = { "batrider_flaming_lasso" },
  ticks = { 48, 56, 64 },
  leash_milli = 7813,
  step = 4,
}

local lassoed = {}

function LASSO.run(caster)
  local rank = math.min(rank_of(caster, LASSO.ability), #LASSO.ticks)
  local target = game.cast_target(caster)
  if rank == 0 or not targetable_enemy(caster, target) then return end
  game.set_counter(caster, "lasso_target", target + 1)
  game.set_counter(caster, "lasso_due", game.ticks() + LASSO.ticks[rank])
  game.apply_status(target, "disarm", LASSO.ticks[rank])
  lassoed[caster] = true
end

local function run_lasso()
  for caster in pairs(lassoed) do
    local target = game.counter(caster, "lasso_target") - 1
    if not game.alive(caster) or not game.alive(target) or game.ticks() >= game.counter(caster, "lasso_due") then
      game.set_counter(caster, "lasso_due", 0)
      lassoed[caster] = nil
    elseif game.ticks() % LASSO.step == 0 then
      local cx, cy = unit_position(caster)
      if not game.within(target, cx, cy, LASSO.leash_milli) then game.teleport(target, cx, cy) end
    end
  end
end

local SHURIKEN = {
  source = "tooltip of A004 (ability.ini:7962), war3map.j JUX 45640: the star also bounces to every other Tracked enemy within 900 of the struck unit, each for the same 75*(rank+1) magical",
  ability = "bounty_hunter_shuriken_toss",
  casts = { "bounty_hunter_shuriken_toss" },
  reach_milli = 14063,
}

function SHURIKEN.run(caster)
  local rank = math.min(rank_of(caster, SHURIKEN.ability), 4)
  local target = game.cast_target(caster)
  if rank == 0 or not targetable_enemy(caster, target) or game.counter(target, "track_due") <= game.ticks() then return end
  local x, y = unit_position(target)
  for _, unit in ipairs(enemies_around(caster, x, y, SHURIKEN.reach_milli)) do
    if unit ~= target and game.counter(unit, "track_due") > game.ticks() and units.is_hero(unit) and not held(unit, "spell_immunity") then
      game.spell_damage(caster, unit, 75 * (rank + 1), MAGICAL)
    end
  end
end

local WILD_AXES = {
  source = "war3map.j H7X 44941-44960, H9X 44978-44995, JVX 45004-45030 (A0O1): two axes fly out and back along curves 300 to either side, each unit within 150 of an axe is hit once for 40+30*rank physical (GUESS: 24 ticks out, 24 back)",
  ability = "beastmaster_wild_axes",
  casts = { "beastmaster_wild_axes" },
  radius_milli = 2344,
  half = 24,
  side = 4688,
}

local axes = {}

function WILD_AXES.run(caster)
  local rank = math.min(rank_of(caster, WILD_AXES.ability), 4)
  local tx, ty = game.cast_point(caster)
  if rank == 0 or tx == nil then return end
  local cx, cy = unit_position(caster)
  local dx, dy = tx - cx, ty - cy
  local length = math.sqrt(dx * dx + dy * dy)
  if length == 0 then return end
  local nx, ny = -dy / length, dx / length
  local side = config.milli(WILD_AXES.side)
  local hit = {}
  for _, sign in ipairs({ 1, -1 }) do
    axes[#axes + 1] = { caster = caster, rank = rank, cx = cx, cy = cy, tx = tx, ty = ty, ox = nx * side * sign, oy = ny * side * sign, born = game.ticks(), hit = hit }
  end
end

local function run_axes()
  local kept = {}
  local total = WILD_AXES.half * 2
  for _, axe in ipairs(axes) do
    local age = game.ticks() - axe.born
    if game.alive(axe.caster) and age <= total then
      local t = age / total
      if age > WILD_AXES.half then t = 1 - (age - WILD_AXES.half) / WILD_AXES.half end
      local bend = math.sin(math.min(1, age / total) * math.pi)
      local x = axe.cx + (axe.tx - axe.cx) * math.min(1, (age <= WILD_AXES.half and age / WILD_AXES.half or 1)) + axe.ox * bend
      local y = axe.cy + (axe.ty - axe.cy) * math.min(1, (age <= WILD_AXES.half and age / WILD_AXES.half or 1)) + axe.oy * bend
      if age > WILD_AXES.half then
        local ex, ey = unit_position(axe.caster)
        local back = (age - WILD_AXES.half) / WILD_AXES.half
        x = axe.tx + (ex - axe.tx) * back + axe.ox * bend
        y = axe.ty + (ey - axe.ty) * back + axe.oy * bend
      end
      x, y = math.floor(x), math.floor(y)
      if age % 2 == 0 then
        for _, unit in ipairs(enemies_around(axe.caster, x, y, WILD_AXES.radius_milli)) do
          if not axe.hit[unit] and not held(unit, "spell_immunity") then
            axe.hit[unit] = true
            game.spell_damage(axe.caster, unit, 40 + 30 * axe.rank, 33)
          end
        end
      end
      kept[#kept + 1] = axe
    end
  end
  axes = kept
end

local DARK_RIFT = {
  source = "war3map.j FVE 72348-72380, ZAO 72392-72425, ZBO 72428-72440 (A0R0): after 7-rank s (cancelled if the caster stops) the caster goes to the target, and allied heroes within 475 of it go too",
  ability = "abyssal_underlord_dark_rift",
  casts = { "abyssal_underlord_dark_rift" },
  reach_milli = 7422,
}

local rifts = {}

function DARK_RIFT.run(caster)
  local rank = math.min(rank_of(caster, DARK_RIFT.ability), 3)
  local target = game.cast_target(caster)
  if rank == 0 then return end
  local x, y = game.cast_point(caster)
  if target ~= nil and game.alive(target) then x, y = unit_position(target) end
  if x == nil then return end
  local ox, oy = unit_position(caster)
  rifts[#rifts + 1] = { caster = caster, ox = ox, oy = oy, x = x, y = y, due = game.ticks() + (7 - rank) * TICKS - 1 }
end

local function run_rifts()
  local kept = {}
  for _, rift in ipairs(rifts) do
    if game.alive(rift.caster) then
      if game.ticks() >= rift.due then
        for _, unit in ipairs(game.units_near(rift.ox, rift.oy, config.milli(DARK_RIFT.reach_milli))) do
          if unit ~= rift.caster and game.alive(unit) and units.is_hero(unit) and not enemy_of(rift.caster, unit) then
            game.teleport(unit, rift.x, rift.y)
          end
        end
      elseif game.channel_left(rift.caster) > 0 then
        kept[#kept + 1] = rift
      end
    end
  end
  rifts = kept
end

local CHEMICAL_RAGE = {
  source = "tooltip of ANcr (ability.ini:3327-3333): for 25 s the base attack time is 1.4/1.2/1.0 s (the form unit N01I is not modelled)",
  ability = "alchemist_chemical_rage",
  casts = { "alchemist_chemical_rage" },
  periods = { 22, 19, 16 },
  ticks = 400,
}

local raging = {}

function CHEMICAL_RAGE.run(caster)
  local rank = math.min(rank_of(caster, CHEMICAL_RAGE.ability), #CHEMICAL_RAGE.periods)
  if rank == 0 then return end
  if game.counter(caster, "rage_period") == 0 then
    game.set_counter(caster, "rage_period", game.set_base_period(caster, CHEMICAL_RAGE.periods[rank]))
  end
  game.set_counter(caster, "alch_rage_due", game.ticks() + CHEMICAL_RAGE.ticks)
  raging[caster] = true
end

local function expire_rage()
  for unit in pairs(raging) do
    if game.ticks() >= game.counter(unit, "alch_rage_due") or not game.alive(unit) then
      if game.alive(unit) then game.set_base_period(unit, game.counter(unit, "rage_period")) end
      game.set_counter(unit, "rage_period", 0)
      game.set_counter(unit, "alch_rage_due", 0)
      raging[unit] = nil
    end
  end
end

local PRIMAL_SPLIT = {
  source = "ability.ini:35339-35358 (A0MQ DataA npn1,npn2,npn3 / npn4,npn5,npn6 / n011,n012,n010, Dur 15/17/19): the Brewmaster leaves play and a fire, a storm and an earth elemental of its rank fight for that long; it returns where the last elemental stood (the rebirth of a surviving elemental is not modelled)",
  ability = "brewmaster_primal_split",
  casts = { "brewmaster_primal_split" },
  elements = { "earth", "storm", "fire" },
  ticks = { 240, 272, 304 },
  offsets = { { -3000, 0 }, { 0, 3000 }, { 3000, 0 } },
}

local splits = {}

function PRIMAL_SPLIT.run(caster)
  local rank = math.min(rank_of(caster, PRIMAL_SPLIT.ability), 3)
  if rank == 0 then return end
  local x, y = unit_position(caster)
  local made = {}
  for at, element in ipairs(PRIMAL_SPLIT.elements) do
    local kind = kit.unit_index("brewmaster_" .. element .. "_" .. rank)
    local off = PRIMAL_SPLIT.offsets[at]
    local unit = kind and game.timed_spawn(caster, kind, PRIMAL_SPLIT.ticks[rank], x + config.milli(off[1]), y + config.milli(off[2]))
    if unit ~= nil then made[#made + 1] = unit end
  end
  if #made == 0 then return end
  game.hide_unit(caster)
  splits[#splits + 1] = { caster = caster, units = made, x = x, y = y, due = game.ticks() + PRIMAL_SPLIT.ticks[rank] }
end

local function run_splits()
  local kept = {}
  for _, split in ipairs(splits) do
    local alive = 0
    for _, unit in ipairs(split.units) do
      if game.alive(unit) then
        alive = alive + 1
        split.x, split.y = unit_position(unit)
      end
    end
    if alive == 0 or game.ticks() >= split.due then
      game.unhide_unit(split.caster, split.x, split.y)
    else
      kept[#kept + 1] = split
    end
  end
  splits = kept
end

local CALL_OF_THE_WILD = {
  source = "war3map.j R2E 45255-45290 (A0OO): a Hawk, Scout Hawk, Greater Hawk or Legendary Hawk by rank appears at the caster for 60 s",
  ability = "beastmaster_call_of_the_wild",
  casts = { "beastmaster_call_of_the_wild" },
  life = 960,
}

function CALL_OF_THE_WILD.run(caster)
  local rank = math.min(rank_of(caster, CALL_OF_THE_WILD.ability), 4)
  if rank == 0 then return end
  local kind = kit.unit_index("beastmaster_hawk_" .. rank)
  local x, y = unit_position(caster)
  if kind then game.timed_spawn(caster, kind, CALL_OF_THE_WILD.life, x, y) end
end

local function rebuild_all()
  rebuild_hunger()
  rebuild_atrophy()
  rebuild_miss()
  sleepers = {}
  for _, unit in ipairs(counter_units("nightmare_due")) do sleepers[unit] = true end
  frozen = {}
  for _, unit in ipairs(counter_units("feet_due")) do frozen[unit] = true end
  shielded = {}
  for _, unit in ipairs(counter_units("aphotic_due")) do shielded[unit] = true end
  borrowing = {}
  for _, unit in ipairs(counter_units("borrow_due")) do borrowing[unit] = true end
  ruptured = {}
  for _, unit in ipairs(counter_units("rupture_due")) do ruptured[unit] = true end
  gripping = {}
  for _, unit in ipairs(counter_units("grip_left")) do gripping[unit] = true end
  gooed = {}
  for _, unit in ipairs(counter_units("goo_due")) do gooed[unit] = true end
  pits = {}
  tracked = {}
  for _, unit in ipairs(counter_units("track_due")) do tracked[unit] = true end
  boosted = {}
  for _, unit in ipairs(counter_units("frost_boost_due")) do boosted[unit] = true end
  wraiths = {}
  axes = {}
  raging = {}
  splits = {}
  for _, unit in ipairs(counter_units("alch_rage_due")) do raging[unit] = true end
  rifts = {}
  lassoed = {}
  for _, unit in ipairs(counter_units("lasso_due")) do lassoed[unit] = true end
  fluxed = {}
  for _, unit in ipairs(counter_units("flux_due")) do fluxed[unit] = true end
  firing = {}
  trails = {}
  for _, unit in ipairs(counter_units("firefly_due")) do firing[unit] = true end
  kit.reset_heroes()
end

local ON_CAST = kit.on_cast_table({
  MANA_VOID, DEATH_COIL, BATTLE_HUNGER, MISS.haze, BLOODRAGE, NIGHTMARE, COLD_FEET, APHOTIC, BORROWED_TIME,
  RUPTURE, TRACK, QUILL, TEMPEST, FIENDS_GRIP, GOO, FLUX, FIREFLY, THUNDER_CLAP, PIT, BACKSTAB, SPIDERLINGS, SPARK_WRAITH, CHILLING_TOUCH, LASSO, SHURIKEN, WILD_AXES, DARK_RIFT, CHEMICAL_RAGE, PRIMAL_SPLIT, CALL_OF_THE_WILD,
})

return {
  on_cast = function(caster, ability_name)
    kit.dispatch(ON_CAST, caster, ability_name)
  end,
  on_hit = function(attacker, victim, dealt)
    wake_sleeper(attacker, victim)
    frostmourne_hit(attacker, victim)
    chill_hit(attacker, victim)
    backstab_hit(attacker, victim)
    borrowed_hit(victim, dealt)
    bristleback_hit(attacker, victim, dealt)
  end,
  on_proc = function(caster, ability_name)
    if ability_name == MISS.bite.ability then bite_proc(caster) end
    if ability_name == FROSTMOURNE.ability then frostmourne_proc(caster) end
  end,
  on_died = function(unit, killer)
    hunger_died(unit, killer)
    atrophy_kill(unit)
    spiderlings_died(unit)
    rage_died(unit, killer)
    track_died(unit)
  end,
  on_tick = function(state)
    expire_hunger()
    expire_atrophy()
    expire_miss()
    pulse_sleepers()
    check_feet()
    watch_aphotic()
    expire_borrowing()
    pulse_rupture()
    pulse_grips()
    expire_goo()
    pulse_flux()
    run_firefly()
    run_pits()
    if (state and (state.ticks or 0) or 0) % 8 == 0 then pulse_track() end
    expire_frost_boost()
    run_wraiths()
    run_lasso()
    run_axes()
    run_rifts()
    expire_rage()
    run_splits()
  end,
  on_state_replaced = rebuild_all,
}

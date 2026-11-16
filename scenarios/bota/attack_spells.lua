local config = require("config")

local TICKS_PER_SECOND = 16
local WHOLE = 65536
local PHYSICAL, MAGICAL, PURE = 1, 2, 3
local REFERENCE_UNITS_PER_CELL = 64
local S_ATTACK_SPEED = 12

local function rank_of(unit, ability)
  for _, row in ipairs(game.unit_abilities(unit)) do
    if row.name == ability then return game.skill_rank(unit, row.slot) end
  end
  return 0
end

local function held(unit, status)
  return game.status_left(unit, status) > 0
end

local DESOLATE = {
  source = "war3map.j XRR 74957-74961, XOR 74940-74956 (A0FX, damage type tw 3)",
  ability = "spectre_desolate",
  radius = 4297,
  damage_per_rank = 15,
  cooldown = 5,
}

function DESOLATE.hit(attacker, victim)
  local rank = rank_of(attacker, DESOLATE.ability)
  if rank == 0 or held(attacker, "break") then return end
  local now = game.ticks()
  if game.counter(attacker, "desolate_ready") > now then return end
  local x, y = unit_position(victim)
  local owner = unit_player(victim)
  for _, unit in ipairs(game.units_near(x, y, config.milli(DESOLATE.radius))) do
    if unit ~= victim and game.alive(unit) and not game.is_structure(unit) and not game.hostile(owner, unit_player(unit)) then return end
  end
  game.set_counter(attacker, "desolate_ready", now + DESOLATE.cooldown)
  game.spell_damage(attacker, victim, DESOLATE.damage_per_rank * rank, MAGICAL)
end

local DISPERSION = {
  source = "war3map.j XCR 75037-75054, XNR 75016-75033 (A0NA), damage returned to the caster YVR 93769-93772",
  ability = "spectre_dispersion",
  radius = 16016,
  base_percent = 10,
  percent_per_rank = 2,
  full_until = 300,
  none_from = 1000,
  edge = 25,
  smallest = 2,
  largest = 9000,
}

local function reference_distance(a, b)
  local ax, ay = unit_position(a)
  local bx, by = unit_position(b)
  local dx, dy = (ax - bx) / WHOLE, (ay - by) / WHOLE
  return math.sqrt(dx * dx + dy * dy) * REFERENCE_UNITS_PER_CELL
end

function DISPERSION.hit(attacker, victim, dealt)
  if not game.is_attributed(victim) then return end
  local rank = rank_of(victim, DISPERSION.ability)
  if rank == 0 or held(victim, "break") then return end
  local whole = dealt / WHOLE
  if whole <= DISPERSION.smallest or whole >= DISPERSION.largest then return end
  local returned = whole * (DISPERSION.base_percent + DISPERSION.percent_per_rank * rank) / 100
  if returned <= 1 then return end
  if game.alive(victim) then game.set_hp(victim, unit_hp(victim) + math.floor(returned * WHOLE)) end
  local x, y = unit_position(victim)
  local owner = unit_player(victim)
  for _, unit in ipairs(game.units_near(x, y, config.milli(DISPERSION.radius))) do
    if unit ~= victim and game.alive(unit) and not game.is_structure(unit) and game.hostile(owner, unit_player(unit)) then
      local reach = reference_distance(victim, unit) - DISPERSION.edge
      local share = 0
      if reach < DISPERSION.full_until then
        share = 1
      elseif reach < DISPERSION.none_from then
        share = (DISPERSION.none_from - reach) / (DISPERSION.none_from - DISPERSION.full_until)
      end
      local amount = math.floor(returned * share)
      if amount > 0 then game.spell_damage(victim, unit, amount, PURE) end
    end
  end
end

local FURY_SWIPES = {
  source = "war3map.j ATO 58832-58866, ASO 58796-58831 (A2X2), enrage multiplier A38O 58809, cast NOE 58757",
  ability = "ursa_fury_swipes",
  window = 12 * TICKS_PER_SECOND,
  base = 10,
  per_rank = 5,
}

local ENRAGE = {
  ability = "ursa_enrage",
  duration = 4 * TICKS_PER_SECOND,
  multiplier_base = 125,
  multiplier_per_rank = 15,
}

function ENRAGE.run(caster)
  local rank = rank_of(caster, ENRAGE.ability)
  if rank == 0 then return end
  game.set_counter(caster, "enrage_until", game.ticks() + ENRAGE.duration)
  game.set_counter(caster, "enrage_rank", rank)
end

function FURY_SWIPES.hit(attacker, victim)
  local rank = rank_of(attacker, FURY_SWIPES.ability)
  if rank == 0 or held(attacker, "break") then return end
  local now = game.ticks()
  local owner = unit_player(attacker)
  local total_key, due_key = "fury_total_" .. owner, "fury_due_" .. owner
  local total = game.counter(victim, total_key)
  if game.counter(victim, due_key) < now then total = 0 end
  total = total + FURY_SWIPES.base + FURY_SWIPES.per_rank * rank
  game.set_counter(victim, total_key, total)
  game.set_counter(victim, due_key, now + FURY_SWIPES.window)
  local percent = 100
  if game.counter(attacker, "enrage_until") > now then
    percent = ENRAGE.multiplier_base + ENRAGE.multiplier_per_rank * game.counter(attacker, "enrage_rank")
  end
  game.spell_damage(attacker, victim, math.floor(total * percent / 100), PHYSICAL)
end

local FERVOR = {
  source = "war3map.j AXO 58307-58331, AAO 58332-58336 (A0O0)",
  ability = "troll_warlord_fervor",
  max_stacks = 4,
  base = 15,
  per_rank = 5,
  cooldown = 6,
}

function FERVOR.hit(attacker, victim)
  local rank = rank_of(attacker, FERVOR.ability)
  if rank == 0 then return end
  local now = game.ticks()
  if game.counter(attacker, "fervor_ready") > now then return end
  local stacks = game.counter(attacker, "fervor_stacks")
  if game.counter(attacker, "fervor_set") == 0 or game.counter(attacker, "fervor_target") ~= victim then
    stacks = 1
    game.set_counter(attacker, "fervor_set", 1)
    game.set_counter(attacker, "fervor_target", victim)
  else
    stacks = math.min(stacks + 1, FERVOR.max_stacks)
  end
  if held(attacker, "break") then stacks = 0 end
  local bonus = (FERVOR.base + FERVOR.per_rank * rank) * stacks
  game.add_stat(attacker, S_ATTACK_SPEED, bonus - game.counter(attacker, "fervor_bonus"))
  game.set_counter(attacker, "fervor_bonus", bonus)
  game.set_counter(attacker, "fervor_stacks", stacks)
  game.set_counter(attacker, "fervor_ready", now + FERVOR.cooldown)
end

local OVERLOAD = {
  source = "war3map.j M6X 47963-47967, IPV 47958-47961, M4X 47947-47957 (A0QW), slow A0S5 ability.ini:41551 (casts A14O A14P A14R)",
  ability = "storm_spirit_overload",
  casts = { "storm_spirit_ball_lightning", "storm_spirit_static_remnant", "storm_spirit_electric_vortex" },
  radius = 4688,
  damage_base = 10,
  damage_per_rank = 20,
  slow_ticks = 10,
  slow_move_keep = 13107,
  attack_speed_cut = -50,
}

local overloaded = {}

function OVERLOAD.run(caster)
  if rank_of(caster, OVERLOAD.ability) == 0 or held(caster, "break") then return end
  game.set_counter(caster, "overload_charged", 1)
end

function OVERLOAD.hit(attacker, victim)
  if game.counter(attacker, "overload_charged") == 0 then return end
  local rank = rank_of(attacker, OVERLOAD.ability)
  if rank == 0 or held(attacker, "break") then return end
  game.set_counter(attacker, "overload_charged", 0)
  local x, y = unit_position(victim)
  local owner = unit_player(attacker)
  local now = game.ticks()
  for _, unit in ipairs(game.units_near(x, y, config.milli(OVERLOAD.radius))) do
    if game.alive(unit) and not game.is_structure(unit) and game.hostile(owner, unit_player(unit)) then
      game.spell_damage(attacker, unit, OVERLOAD.damage_base + OVERLOAD.damage_per_rank * rank, MAGICAL)
      if game.alive(unit) and not held(unit, "spell_immunity") then
        game.apply_status(unit, "slow", OVERLOAD.slow_ticks, OVERLOAD.slow_move_keep)
        if game.counter(unit, "overload_due") == 0 then
          game.add_stat(unit, S_ATTACK_SPEED, OVERLOAD.attack_speed_cut)
        end
        game.set_counter(unit, "overload_due", now + OVERLOAD.slow_ticks)
        overloaded[unit] = true
      end
    end
  end
end

local function expire_overload()
  for unit in pairs(overloaded) do
    local due = game.counter(unit, "overload_due")
    if due ~= 0 and (game.ticks() >= due or not game.alive(unit)) then
      game.add_stat(unit, S_ATTACK_SPEED, -OVERLOAD.attack_speed_cut)
      game.set_counter(unit, "overload_due", 0)
      overloaded[unit] = nil
    elseif due == 0 then
      overloaded[unit] = nil
    end
  end
end

local function rebuild_overload()
  overloaded = {}
  for unit = 0, get_unit_count() - 1 do
    if game.counter(unit, "overload_due") ~= 0 then overloaded[unit] = true end
  end
end

local CLOAK = {
  source = "war3map.j IKR 77543-77640 (A1JC family), layers per ability.ini:44419 tooltip",
  ability = "visage_gravekeeper_s_cloak",
  layers = 4,
  recover = 4 * TICKS_PER_SECOND,
  armor_total = { 4, 8, 16, 20 },
  resist_total = { 12, 24, 48, 64 },
}

local S_ARMOR, S_MAGIC_RESIST = 1, 11
local cloaked = {}

function CLOAK.hit(victim)
  if not game.is_attributed(victim) then return end
  local rank = rank_of(victim, CLOAK.ability)
  if rank == 0 then return end
  local lost = game.counter(victim, "cloak_lost")
  if lost >= CLOAK.layers then return end
  local armor = CLOAK.armor_total[math.min(rank, 4)] / CLOAK.layers
  local resist = CLOAK.resist_total[math.min(rank, 4)] / CLOAK.layers
  game.add_stat(victim, S_ARMOR, -math.floor(armor))
  game.add_stat(victim, S_MAGIC_RESIST, -math.floor(resist))
  game.set_counter(victim, "cloak_lost", lost + 1)
  game.set_counter(victim, "cloak_armor", math.floor(armor))
  game.set_counter(victim, "cloak_resist", math.floor(resist))
  game.set_counter(victim, "cloak_due", game.ticks() + CLOAK.recover)
  cloaked[victim] = true
end

local function recover_cloak()
  for unit in pairs(cloaked) do
    local lost = game.counter(unit, "cloak_lost")
    if lost == 0 then
      cloaked[unit] = nil
    elseif game.ticks() >= game.counter(unit, "cloak_due") then
      game.add_stat(unit, S_ARMOR, game.counter(unit, "cloak_armor"))
      game.add_stat(unit, S_MAGIC_RESIST, game.counter(unit, "cloak_resist"))
      game.set_counter(unit, "cloak_lost", lost - 1)
      game.set_counter(unit, "cloak_due", game.ticks() + CLOAK.recover)
    end
  end
end

local function rebuild_cloak()
  cloaked = {}
  for unit = 0, get_unit_count() - 1 do
    if game.counter(unit, "cloak_lost") > 0 then cloaked[unit] = true end
  end
end

local REFRACTION = {
  source = "ability.ini A0KB tooltip (ability.ini 40946 block): absorb 3 to 6 hits, +20 to +80 damage for 3 to 6 attacks, 17 s",
  ability = "templar_assassin_refraction",
  duration = 17 * TICKS_PER_SECOND,
  bonus = { 20, 40, 60, 80 },
}

local BEETLES = { weaver_beetle_1 = true, weaver_beetle_2 = true, weaver_beetle_3 = true, weaver_beetle_4 = true }

function REFRACTION.run(caster)
  local rank = rank_of(caster, REFRACTION.ability)
  if rank == 0 then return end
  game.set_counter(caster, "refr_hits", rank + 2)
  game.set_counter(caster, "refr_attacks", rank + 2)
  game.set_counter(caster, "refr_bonus", REFRACTION.bonus[math.min(rank, 4)])
  game.set_counter(caster, "refr_until", game.ticks() + REFRACTION.duration)
end

function REFRACTION.absorb(victim, dealt)
  if game.counter(victim, "refr_hits") <= 0 or game.ticks() >= game.counter(victim, "refr_until") then return end
  game.set_counter(victim, "refr_hits", game.counter(victim, "refr_hits") - 1)
  if game.alive(victim) then game.set_hp(victim, unit_hp(victim) + dealt) end
end

function REFRACTION.strike(attacker, victim)
  if game.counter(attacker, "refr_attacks") <= 0 or game.ticks() >= game.counter(attacker, "refr_until") then return end
  game.set_counter(attacker, "refr_attacks", game.counter(attacker, "refr_attacks") - 1)
  game.spell_damage(attacker, victim, game.counter(attacker, "refr_bonus"), PHYSICAL)
end

local KRAKEN_BLOCK = {
  source = "ability.ini 13644-13648 (A04E DataA chance 100, DataC flat block)",
  ability = "tidehunter_kraken_shell",
  block = { 12, 24, 36, 48 },
}

function KRAKEN_BLOCK.hit(victim, dealt)
  if not game.is_attributed(victim) then return end
  local rank = rank_of(victim, KRAKEN_BLOCK.ability)
  if rank == 0 then return end
  local block = math.min(dealt, KRAKEN_BLOCK.block[math.min(rank, 4)] * WHOLE)
  game.set_counter(victim, "kraken_blocked", block)
  if game.alive(victim) then game.set_hp(victim, unit_hp(victim) + block) end
end

local LIVING_ARMOR = {
  source = "ability.ini tooltip: 20/40/60/80 block per hit, 4/5/6/7 hits, 15 s (A07H timed life, war3map.j 54146)",
  ability = "treant_living_armor",
  block = { 20, 40, 60, 80 },
  hits = { 4, 5, 6, 7 },
  duration = 15 * TICKS_PER_SECOND,
}

function LIVING_ARMOR.run(caster)
  local rank = rank_of(caster, LIVING_ARMOR.ability)
  if rank == 0 then return end
  local target = game.cast_target(caster)
  if target == nil or not game.alive(target) then target = caster end
  game.set_counter(target, "la_hits", LIVING_ARMOR.hits[math.min(rank, 4)])
  game.set_counter(target, "la_block", LIVING_ARMOR.block[math.min(rank, 4)])
  game.set_counter(target, "la_until", game.ticks() + LIVING_ARMOR.duration)
end

function LIVING_ARMOR.hit(victim, dealt)
  if game.counter(victim, "la_hits") <= 0 or game.ticks() >= game.counter(victim, "la_until") then return end
  game.set_counter(victim, "la_hits", game.counter(victim, "la_hits") - 1)
  local block = math.min(dealt, game.counter(victim, "la_block") * WHOLE)
  if game.alive(victim) then game.set_hp(victim, unit_hp(victim) + block) end
end

local SIEGE = {
  source = "ability.ini 92979-93042, war3map.j 89095-89130 (slow 4 s), burn 8 percent of current life each second for 5 s",
  ability = "winter_wyvern_arctic_siege",
  duration = 6 * TICKS_PER_SECOND,
  slow_percent = { 25, 30, 35, 40 },
  slow_ticks = 4 * TICKS_PER_SECOND,
  burn_percent = 8,
  burn_pulses = 5,
}

local sieging = {}
local burning = {}

function SIEGE.run(caster)
  local rank = rank_of(caster, SIEGE.ability)
  if rank == 0 then return end
  game.set_counter(caster, "siege_until", game.ticks() + SIEGE.duration)
  game.set_counter(caster, "siege_rank", rank)
  game.set_flying(caster, true)
  sieging[caster] = true
end

function SIEGE.hit(attacker, victim)
  if game.ticks() >= game.counter(attacker, "siege_until") or game.is_structure(victim) then return end
  local rank = game.counter(attacker, "siege_rank")
  game.apply_status(victim, "slow", SIEGE.slow_ticks, math.floor(65536 * (100 - SIEGE.slow_percent[math.min(rank, 4)]) / 100))
  game.set_counter(victim, "burn_left", SIEGE.burn_pulses)
  game.set_counter(victim, "burn_source", attacker)
  game.set_counter(victim, "burn_next", game.ticks() + TICKS_PER_SECOND)
  burning[victim] = true
end

local function step_siege()
  local now = game.ticks()
  for unit in pairs(sieging) do
    if not game.alive(unit) or now >= game.counter(unit, "siege_until") then
      sieging[unit] = nil
      game.set_counter(unit, "siege_until", 0)
      if game.alive(unit) then game.set_flying(unit, false) end
    end
  end
  for unit in pairs(burning) do
    local left = game.counter(unit, "burn_left")
    if not game.alive(unit) or left <= 0 then
      burning[unit] = nil
    elseif now >= game.counter(unit, "burn_next") then
      game.set_counter(unit, "burn_left", left - 1)
      game.set_counter(unit, "burn_next", now + TICKS_PER_SECOND)
      game.spell_damage(game.counter(unit, "burn_source"), unit, math.floor(unit_hp(unit) * SIEGE.burn_percent / (100 * WHOLE)), MAGICAL)
    end
  end
end

local function rebuild_siege()
  sieging = {}
  burning = {}
  for unit = 0, get_unit_count() - 1 do
    if game.counter(unit, "siege_until") > 0 and game.alive(unit) then sieging[unit] = true end
    if game.counter(unit, "burn_left") > 0 and game.alive(unit) then burning[unit] = true end
  end
end

local ON_CAST = {}
for _, name in ipairs(OVERLOAD.casts) do ON_CAST[name] = { OVERLOAD.run } end
ON_CAST[ENRAGE.ability] = { ENRAGE.run }
ON_CAST[REFRACTION.ability] = { REFRACTION.run }
ON_CAST[LIVING_ARMOR.ability] = { LIVING_ARMOR.run }
ON_CAST[SIEGE.ability] = { SIEGE.run }

return {
  on_cast = function(caster, ability_name)
    local runs = ON_CAST[ability_name]
    if runs == nil then return end
    for _, run in ipairs(runs) do run(caster) end
  end,
  on_hit = function(attacker, victim, dealt, state, attack)
    DISPERSION.hit(attacker, victim, dealt)
    REFRACTION.absorb(victim, dealt)
    LIVING_ARMOR.hit(victim, dealt)
    CLOAK.hit(victim)
    if not attack or attacker < 0 or not game.alive(attacker) or not game.alive(victim) then return end
    if BEETLES[game.unit_kind(attacker)] then game.spell_damage(attacker, attacker, 1, PURE) end
    REFRACTION.strike(attacker, victim)
    KRAKEN_BLOCK.hit(victim, dealt)
    SIEGE.hit(attacker, victim)
    DESOLATE.hit(attacker, victim)
    FURY_SWIPES.hit(attacker, victim)
    OVERLOAD.hit(attacker, victim)
    FERVOR.hit(attacker, victim)
  end,
  on_tick = function()
    expire_overload()
    recover_cloak()
    step_siege()
  end,
  on_state_replaced = function()
    rebuild_overload()
    rebuild_cloak()
    rebuild_siege()
  end,
}

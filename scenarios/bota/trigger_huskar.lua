local lib = require("trigger_lib")

local TICKS, WHOLE = lib.TICKS, lib.WHOLE

local BLOOD = {
  source = "war3map.j SDX 49765-49796 (steps of 7% missing life, at least 1, at most 14; k2 attack speed (12+2r)*steps, SCX magic resistance in 4% tiers of (2+r)*steps, tiers A2PD..A2OW DataE .96 .92 .88), A0QQ ability.ini:39989",
  ability = "huskar_berserker_s_blood",
  step_percent = 7,
  max_steps = 14,
  speed_base = 12,
  speed_per_rank = 2,
  resist_base = 2,
  resist_per_rank = 1,
  resist_tier = 4,
  every = 8,
}

local function blood_steps(unit)
  local missing = 100 - lib.life_percent(unit)
  return math.max(math.min(missing // BLOOD.step_percent, BLOOD.max_steps), 1)
end

local function set_blood(unit, speed, resist)
  game.add_stat(unit, lib.S_ATTACK_SPEED, speed - game.counter(unit, "blood_speed"))
  game.add_stat(unit, lib.S_MAGIC_RESIST, resist - game.counter(unit, "blood_resist"))
  game.set_counter(unit, "blood_speed", speed)
  game.set_counter(unit, "blood_resist", resist)
end

local function update_blood(unit)
  local rank = lib.rank_of(unit, BLOOD.ability)
  if rank == 0 or not game.alive(unit) then
    if game.counter(unit, "blood_speed") ~= 0 or game.counter(unit, "blood_resist") ~= 0 then set_blood(unit, 0, 0) end
    return
  end
  local steps = blood_steps(unit)
  local tiers = ((BLOOD.resist_base + BLOOD.resist_per_rank * rank) * steps) // BLOOD.resist_tier
  set_blood(unit, (BLOOD.speed_base + BLOOD.speed_per_rank * rank) * steps, tiers * BLOOD.resist_tier)
end

local SPEAR = {
  source = "war3map.j SIX 49695-49713 (15 life per attack down to 1, a dot of 5*rank magical every second, 8 times, one dot per hit), SRX 49677-49694, A0QN ability.ini:39766",
  ability = "huskar_burning_spear",
  cost = 15,
  damage_per_rank = 5,
  hits = 8,
}

local stricken = lib.registry()

local function bucket(hits_left) return "spear_" .. hits_left end

local function add_stack(victim, source)
  game.set_counter(victim, bucket(SPEAR.hits), game.counter(victim, bucket(SPEAR.hits)) + 1)
  game.set_counter(victim, "spear_source", source + 1)
  stricken.add(victim)
end

local function clear_stacks(victim)
  for hits_left = 1, SPEAR.hits do game.set_counter(victim, bucket(hits_left), 0) end
  game.set_counter(victim, "spear_source", 0)
  stricken.remove(victim)
end

local function burn(victim)
  local source = game.counter(victim, "spear_source") - 1
  local total = 0
  for hits_left = 1, SPEAR.hits do total = total + game.counter(victim, bucket(hits_left)) end
  if total == 0 or not game.alive(victim) then
    clear_stacks(victim)
    return
  end
  local rank = source >= 0 and lib.rank_of(source, SPEAR.ability) or 0
  game.spell_damage(source, victim, total * SPEAR.damage_per_rank * math.max(rank, 1), lib.SCRIPT_MAGICAL)
  for hits_left = 1, SPEAR.hits do
    local next_hits = hits_left < SPEAR.hits and game.counter(victim, bucket(hits_left + 1)) or 0
    game.set_counter(victim, bucket(hits_left), next_hits)
  end
  local remaining = 0
  for hits_left = 1, SPEAR.hits do remaining = remaining + game.counter(victim, bucket(hits_left)) end
  if remaining == 0 then clear_stacks(victim) end
end

local function spear_proc(caster)
  local victim = game.strike_of(caster)
  if victim == nil or victim < 0 or not game.alive(victim) or not game.hostile(unit_player(caster), unit_player(victim)) then return end
  game.set_hp(caster, math.max(unit_hp(caster) - SPEAR.cost * WHOLE, WHOLE))
  add_stack(victim, caster)
end

local VITALITY = {
  source = "war3map.j SEX 49590-49616 (every 1/8 s, rank*5% of the target main attribute a second, rank*20% under 40% life; the flat +10 is the ability's own regeneration), IYE 49617-49641 (a cast adds 16 s), A0QP ability.ini:39900",
  ability = "huskar_inner_vitality",
  duration = 16 * TICKS,
  period = TICKS // 8,
  percent_per_rank = 5,
  low_percent_per_rank = 20,
  low_life = 40,
}

local vital = lib.registry()

local function vitality_cast(caster)
  local target = game.cast_target(caster)
  local rank = lib.rank_of(caster, VITALITY.ability)
  if target == nil or rank == 0 or not game.alive(target) then return end
  local now = game.ticks()
  game.set_counter(target, "vitality_due", math.max(game.counter(target, "vitality_due"), now) + VITALITY.duration)
  game.set_counter(target, "vitality_rank", rank)
  vital.add(target)
end

local function vitality_step(unit)
  if game.counter(unit, "vitality_due") <= game.ticks() or not game.alive(unit) then
    game.set_counter(unit, "vitality_due", 0)
    vital.remove(unit)
    return
  end
  local rank = game.counter(unit, "vitality_rank")
  local percent = lib.life_percent(unit) > VITALITY.low_life and VITALITY.percent_per_rank or VITALITY.low_percent_per_rank
  local per_second = rank * percent * lib.main_attribute(unit) * WHOLE // 100
  game.set_hp(unit, unit_hp(unit) + per_second * VITALITY.period // TICKS)
end

local BREAK = {
  source = "war3map.j Q5X 49490-49512 (the caster pays 35% of its life, the target takes 35% of its life, both through tw type 1), A0QR ability.ini:40045",
  ability = "huskar_life_break",
  fraction = 35,
}

local function life_break(caster)
  local target = game.cast_target(caster)
  if target == nil or not game.alive(target) or lib.rank_of(caster, BREAK.ability) == 0 then return end
  local taken = unit_hp(target) * BREAK.fraction // 100 // WHOLE
  local paid = unit_hp(caster) * BREAK.fraction // 100
  game.set_hp(caster, math.max(unit_hp(caster) - paid, WHOLE))
  game.spell_damage(caster, target, taken, lib.SCRIPT_MAGICAL)
end

local CASTS = {
  [VITALITY.ability] = vitality_cast,
  [BREAK.ability] = life_break,
}

local function rebuild()
  stricken.reset(lib.counted("spear_source"))
  vital.reset(lib.counted("vitality_due"))
end

return {
  on_cast = function(caster, ability_name)
    local run = CASTS[ability_name]
    if run ~= nil then run(caster) end
  end,

  on_proc = function(caster, ability_name)
    if ability_name == SPEAR.ability then spear_proc(caster) end
  end,

  on_tick = function()
    local now = game.ticks()
    for unit = now % BLOOD.every, get_unit_count() - 1, BLOOD.every do
      if game.is_attributed(unit) then update_blood(unit) end
    end
    if now % TICKS == 0 then
      for _, unit in ipairs(stricken.items()) do burn(unit) end
    end
    if now % VITALITY.period == 0 then
      for _, unit in ipairs(vital.items()) do vitality_step(unit) end
    end
  end,

  on_state_replaced = rebuild,
}

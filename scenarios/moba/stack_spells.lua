local config = require("config")

local TICKS_PER_SECOND = 16
local WHOLE = 65536
local MAGICAL = 2
local S_ATTACK_SPEED, S_SPEED_PCT = 12, 19

local function rank_of(unit, ability)
  for _, row in ipairs(game.unit_abilities(unit)) do
    if row.name == ability then return game.skill_rank(unit, row.slot) end
  end
  return 0
end

local function held(unit, status)
  return game.status_left(unit, status) > 0
end

local function enemy_unit(source, unit)
  return game.alive(unit) and not game.is_structure(unit) and game.hostile(unit_player(source), unit_player(unit)) and not held(unit, "spell_immunity")
end

local DEBUFFS = {
  { key = "napalm_slow", stat = S_SPEED_PCT },
  { key = "untouchable", stat = S_ATTACK_SPEED },
}

local tracked = {}

local function track(unit, debuff)
  local row = tracked[unit]
  if row == nil then
    row = {}
    tracked[unit] = row
  end
  row[debuff.key] = debuff
end

local function debuff_on(unit, debuff, amount, ticks)
  local key = debuff.key
  local applied = game.counter(unit, key .. "_amount")
  if applied ~= amount then
    game.add_stat(unit, debuff.stat, amount - applied)
    game.set_counter(unit, key .. "_amount", amount)
  end
  game.set_counter(unit, key .. "_due", game.ticks() + ticks)
  track(unit, debuff)
end

local function debuff_off(unit, debuff)
  local key = debuff.key
  game.add_stat(unit, debuff.stat, -game.counter(unit, key .. "_amount"))
  game.set_counter(unit, key .. "_amount", 0)
  game.set_counter(unit, key .. "_due", 0)
end

local function expire_debuffs()
  local now = game.ticks()
  for unit, row in pairs(tracked) do
    local left = false
    for key, debuff in pairs(row) do
      if now >= game.counter(unit, key .. "_due") or not game.alive(unit) then
        debuff_off(unit, debuff)
        row[key] = nil
      else
        left = true
      end
    end
    if not left then tracked[unit] = nil end
  end
end

local function rebuild_debuffs()
  tracked = {}
  for unit = 0, get_unit_count() - 1 do
    for _, debuff in ipairs(DEBUFFS) do
      if game.counter(unit, debuff.key .. "_amount") ~= 0 then track(unit, debuff) end
    end
  end
end

local STICKY_NAPALM = {
  source = "war3map.j DNO 61700-61735 (stack), DAO 61690-61699 (8 s timer), damage trigger 61660-61689 (A1EL); slow per stack from the A1EL tooltip",
  ability = "batrider_sticky_napalm",
  radius = 5859,
  max_stacks = 10,
  duration = 8 * TICKS_PER_SECOND,
  slow_percent = { 3, 5, 7, 9 },
  damage_base = 5,
  damage_per_rank = 5,
  minimum_hit = 12 * WHOLE,
}

local inside_napalm = false

function STICKY_NAPALM.run(caster)
  local rank = math.min(rank_of(caster, STICKY_NAPALM.ability), #STICKY_NAPALM.slow_percent)
  if rank == 0 then return end
  local x, y = game.cast_point(caster)
  local now = game.ticks()
  for _, unit in ipairs(game.units_near(x, y, config.milli(STICKY_NAPALM.radius))) do
    if unit ~= caster and enemy_unit(caster, unit) then
      local stacks = 0
      if game.counter(unit, "napalm_due") > now then stacks = game.counter(unit, "napalm_stacks") end
      stacks = math.min(stacks + 1, STICKY_NAPALM.max_stacks)
      game.set_counter(unit, "napalm_stacks", stacks)
      game.set_counter(unit, "napalm_due", now + STICKY_NAPALM.duration)
      game.set_counter(unit, "napalm_source", caster)
      debuff_on(unit, DEBUFFS[1], -STICKY_NAPALM.slow_percent[rank] * stacks, STICKY_NAPALM.duration)
    end
  end
end

function STICKY_NAPALM.hit(attacker, victim, dealt)
  if inside_napalm or attacker < 0 or dealt <= STICKY_NAPALM.minimum_hit then return end
  if game.counter(victim, "napalm_due") <= game.ticks() or game.counter(victim, "napalm_source") ~= attacker then return end
  local rank = rank_of(attacker, STICKY_NAPALM.ability)
  if rank == 0 or not game.alive(attacker) then return end
  local amount = (STICKY_NAPALM.damage_base + STICKY_NAPALM.damage_per_rank * rank) * game.counter(victim, "napalm_stacks")
  if not game.is_attributed(victim) then amount = amount // 2 end
  if amount <= 0 then return end
  inside_napalm = true
  game.spell_damage(attacker, victim, amount, MAGICAL)
  inside_napalm = false
end

local UNTOUCHABLE = {
  source = "war3map.j PUX 48543-48550 (conditions), PWX 48552-48562 (effect A3EK to A3EN, ability.ini:119732-119801)",
  ability = "enchantress_untouchable",
  attack_speed_percent = { 40, 70, 100, 130 },
  duration = 4 * TICKS_PER_SECOND,
}

function UNTOUCHABLE.slow(owner, source)
  if owner < 0 or source < 0 or not game.alive(owner) or held(owner, "break") then return end
  local rank = math.min(rank_of(owner, UNTOUCHABLE.ability), #UNTOUCHABLE.attack_speed_percent)
  if rank == 0 or not enemy_unit(owner, source) then return end
  debuff_on(source, DEBUFFS[2], -UNTOUCHABLE.attack_speed_percent[rank], UNTOUCHABLE.duration)
end

return {
  on_cast = function(caster, ability_name)
    if ability_name == STICKY_NAPALM.ability then STICKY_NAPALM.run(caster) end
    local target = game.cast_target(caster)
    if target ~= nil then UNTOUCHABLE.slow(target, caster) end
  end,
  on_hit = function(attacker, victim, dealt)
    STICKY_NAPALM.hit(attacker, victim, dealt)
    UNTOUCHABLE.slow(victim, attacker)
  end,
  on_tick = function()
    expire_debuffs()
  end,
  on_state_replaced = function()
    rebuild_debuffs()
  end,
}

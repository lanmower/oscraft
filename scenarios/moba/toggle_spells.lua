local config = require("config")

local TICKS_PER_SECOND = 16
local WHOLE = 65536
local MAGICAL = 2
local S_ATTACK_SPEED = 12

local function rank_of(unit, ability)
  for _, row in ipairs(game.unit_abilities(unit)) do
    if row.name == ability then return game.skill_rank(unit, row.slot) end
  end
  return 0
end

local VOODOO = {
  source = "ability.ini A0NE 36209-36247: heal 16/24/32/40 per second within 500, drains 8/12/16/20 mana per second",
  ability = "witch_doctor_voodoo_restoration",
  radius = 7813,
  heal = { 16, 24, 32, 40 },
  drain = { 8, 12, 16, 20 },
}

local OVERCHARGE = {
  source = "ability.ini A0N? tooltip (wisp overcharge): attack speed +40/50/60/70, damage taken -5/10/15/20 percent, drains 4.5 percent of current life and mana per second; tether sharing missing",
  ability = "wisp_overcharge",
  attack_speed = { 40, 50, 60, 70 },
  prevention = { 5, 10, 15, 20 },
  drain_permille = 45,
}

local RELOCATE = {
  source = "ability.ini A1TB tooltip: relocate to the point after 2.5/2.25/2 s, back after 12 s; tethered hero not carried",
  ability = "wisp_relocate",
  delay = { 40, 36, 32 },
  stay = 12 * TICKS_PER_SECOND,
}

local GOLEM = {
  source = "ability.ini A15K 51973-51999 (plague aura radius 750: up to 20/25/30 percent more damage taken and up to 20 percent slower), 30 s",
  ability = "undying_flesh_golem",
  radius = 11719,
  duration = 30 * TICKS_PER_SECOND,
  amplify = { 20, 25, 30 },
  slow = 20,
  refresh = 8,
}

local toggled = {}
local relocating = {}
local golems = {}

local function toggle(caster, spec_name)
  local key = spec_name .. "_on"
  if game.counter(caster, key) == 1 then
    game.set_counter(caster, key, 0)
    return false
  end
  game.set_counter(caster, key, 1)
  game.set_counter(caster, spec_name .. "_next", game.ticks() + TICKS_PER_SECOND)
  toggled[caster] = toggled[caster] or {}
  toggled[caster][spec_name] = true
  return true
end

local function start_voodoo(caster)
  if rank_of(caster, VOODOO.ability) == 0 then return end
  toggle(caster, "voodoo")
end

local function stop_overcharge(unit)
  game.add_stat(unit, S_ATTACK_SPEED, -game.counter(unit, "overcharge_speed"))
  game.set_counter(unit, "overcharge_speed", 0)
  game.set_damage_reduction(unit, 0)
end

local function start_overcharge(caster)
  local rank = rank_of(caster, OVERCHARGE.ability)
  if rank == 0 then return end
  if toggle(caster, "overcharge") then
    local speed = OVERCHARGE.attack_speed[math.min(rank, 4)]
    game.add_stat(caster, S_ATTACK_SPEED, speed)
    game.set_counter(caster, "overcharge_speed", speed)
    game.set_damage_reduction(caster, OVERCHARGE.prevention[math.min(rank, 4)])
  else
    stop_overcharge(caster)
  end
end

local function start_relocate(caster)
  local rank = rank_of(caster, RELOCATE.ability)
  if rank == 0 then return end
  local x, y = game.cast_point(caster)
  local now = game.ticks()
  local ox, oy = unit_position(caster)
  relocating[caster] = { x = x, y = y, ox = ox, oy = oy, go = now + RELOCATE.delay[math.min(rank, 3)], back = now + RELOCATE.delay[math.min(rank, 3)] + RELOCATE.stay, there = false }
end

local function start_golem(caster)
  local rank = rank_of(caster, GOLEM.ability)
  if rank == 0 then return end
  game.set_counter(caster, "golem_until", game.ticks() + GOLEM.duration)
  game.set_counter(caster, "golem_rank", rank)
  golems[caster] = { next = game.ticks() }
end

local function step_toggles(now)
  for unit, specs in pairs(toggled) do
    if not game.alive(unit) then
      toggled[unit] = nil
    else
      if specs.voodoo then
        if game.counter(unit, "voodoo_on") ~= 1 then
          specs.voodoo = nil
        elseif now >= game.counter(unit, "voodoo_next") then
          local rank = math.min(rank_of(unit, VOODOO.ability), 4)
          local drain = VOODOO.drain[math.max(rank, 1)]
          if rank == 0 or game.mana(unit) < drain then
            game.set_counter(unit, "voodoo_on", 0)
          else
            game.set_mana(unit, game.mana(unit) - drain)
            game.set_counter(unit, "voodoo_next", now + TICKS_PER_SECOND)
            local x, y = unit_position(unit)
            for _, ally in ipairs(game.units_near(x, y, config.milli(VOODOO.radius))) do
              if game.alive(ally) and not game.is_structure(ally) and not game.hostile(unit_player(unit), unit_player(ally)) then
                game.set_hp(ally, unit_hp(ally) + VOODOO.heal[rank] * WHOLE)
              end
            end
          end
        end
      end
      if specs.overcharge then
        if game.counter(unit, "overcharge_on") ~= 1 then
          specs.overcharge = nil
        elseif now >= game.counter(unit, "overcharge_next") then
          game.set_counter(unit, "overcharge_next", now + TICKS_PER_SECOND)
          local hp = unit_hp(unit)
          game.set_hp(unit, math.max(WHOLE, hp - math.floor(hp * OVERCHARGE.drain_permille / 1000)))
          local mana = game.mana(unit)
          game.set_mana(unit, mana - math.floor(mana * OVERCHARGE.drain_permille / 1000))
        end
      end
      if not specs.voodoo and not specs.overcharge then toggled[unit] = nil end
    end
  end
end

local function step_relocate(now)
  for unit, plan in pairs(relocating) do
    if not game.alive(unit) then
      relocating[unit] = nil
    elseif not plan.there and now >= plan.go then
      game.teleport(unit, plan.x, plan.y)
      plan.there = true
    elseif plan.there and now >= plan.back then
      game.teleport(unit, plan.ox, plan.oy)
      relocating[unit] = nil
    end
  end
end

local function golem_pull(unit, x, y, now)
  local rank = math.min(game.counter(unit, "golem_rank"), 3)
  for _, enemy in ipairs(game.units_near(x, y, config.milli(GOLEM.radius))) do
    if game.alive(enemy) and not game.is_structure(enemy) and game.hostile(unit_player(unit), unit_player(enemy)) then
      local ex, ey = unit_position(enemy)
      local dx, dy = (ex - x) / WHOLE, (ey - y) / WHOLE
      local near = 1 - math.sqrt(dx * dx + dy * dy) * 1000 / GOLEM.radius
      if near > 0 then
        local keep = math.floor(65536 * (1 - GOLEM.slow * near / 100))
        game.apply_status(enemy, "slow", GOLEM.refresh + 2, keep)
        game.set_counter(enemy, "plague_pct", math.floor(GOLEM.amplify[rank] * near))
        game.set_counter(enemy, "plague_until", now + GOLEM.refresh + 2)
        game.set_counter(enemy, "plague_source", unit)
      end
    end
  end
end

local function step_golems(now)
  for unit, info in pairs(golems) do
    if not game.alive(unit) or now >= game.counter(unit, "golem_until") then
      golems[unit] = nil
    elseif now >= info.next then
      info.next = now + GOLEM.refresh
      local x, y = unit_position(unit)
      golem_pull(unit, x, y, now)
    end
  end
end

local SHADOW = {
  source = "war3map.j 78411-78432 (A0AS: an enemy takes, an ally is healed, 10 x rank + 5 per second for 11 s)",
  ability = "warlock_shadow_word",
  seconds = 11,
  base = 5,
  per_rank = 10,
}

local words = {}

local function start_shadow(caster)
  local rank = rank_of(caster, SHADOW.ability)
  local target = game.cast_target(caster)
  if rank == 0 or target == nil or not game.alive(target) then return end
  words[#words + 1] = { caster = caster, target = target, amount = SHADOW.base + SHADOW.per_rank * rank, left = SHADOW.seconds, next = game.ticks() + TICKS_PER_SECOND, harm = game.hostile(unit_player(caster), unit_player(target)) }
end

local function step_shadow(now)
  local kept = {}
  for _, word in ipairs(words) do
    if game.alive(word.target) then
      if now >= word.next then
        word.next = now + TICKS_PER_SECOND
        word.left = word.left - 1
        if word.harm then
          game.spell_damage(word.caster, word.target, word.amount, MAGICAL)
        else
          game.set_hp(word.target, unit_hp(word.target) + word.amount * WHOLE)
        end
      end
      if word.left > 0 then kept[#kept + 1] = word end
    end
  end
  words = kept
end

local ON_CAST = {
  [VOODOO.ability] = start_voodoo,
  [OVERCHARGE.ability] = start_overcharge,
  [RELOCATE.ability] = start_relocate,
  [GOLEM.ability] = start_golem,
  [SHADOW.ability] = start_shadow,
}

return {
  on_cast = function(caster, ability_name)
    local run = ON_CAST[ability_name]
    if run ~= nil then run(caster) end
  end,
  on_hit = function(attacker, victim, dealt)
    if game.counter(victim, "plague_pct") <= 0 or game.ticks() >= game.counter(victim, "plague_until") then return end
    if game.counter(victim, "plague_skip") == game.ticks() then return end
    local source = game.counter(victim, "plague_source")
    local extra = math.floor(dealt / WHOLE * game.counter(victim, "plague_pct") / 100)
    if extra > 0 and game.alive(victim) then
      game.set_counter(victim, "plague_skip", game.ticks() + 1)
      game.spell_damage(attacker >= 0 and attacker or source, victim, extra, MAGICAL)
    end
  end,
  on_tick = function()
    local now = game.ticks()
    step_toggles(now)
    step_relocate(now)
    step_golems(now)
    step_shadow(now)
  end,
  on_state_replaced = function()
    toggled, relocating, golems, words = {}, {}, {}, {}
    for unit = 0, get_unit_count() - 1 do
      if game.alive(unit) then
        if game.counter(unit, "voodoo_on") == 1 then toggled[unit] = { voodoo = true } end
        if game.counter(unit, "overcharge_on") == 1 then
          toggled[unit] = toggled[unit] or {}
          toggled[unit].overcharge = true
        end
        if game.counter(unit, "golem_until") > game.ticks() then golems[unit] = { next = game.ticks() } end
      end
    end
  end,
}

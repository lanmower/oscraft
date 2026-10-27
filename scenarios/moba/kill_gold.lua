local config = require("config")
local units = require("units")
local bounty = require("bounty")
local aegis = require("aegis")
local store = require("store")
local xp = require("xp")
local announce = require("announce")
local stats = require("stats")
local modes = require("modes")

local TICKS = config.TICKS_PER_SECOND
local NEUTRAL = 255
local STREAK = "kill_streak"
local FIRST_BLOOD = "first_blood"
local CLAMP_EVERY = 8

local PASSIVE_GOLD = config.rule("passive_gold_per_period")
local EASY_PASSIVE_FACTOR = 2
local PASSIVE_PERIOD_TICKS = config.rule("passive_gold_period_s") * TICKS
local KILL_BASE = config.rule("hero_kill_gold_base")
local KILL_PER_LEVEL = config.rule("hero_kill_gold_per_level")
local STREAK_FROM = config.rule("kill_streak_min")
local STREAK_OFFSET = config.rule("kill_streak_offset")
local STREAK_STEPS = config.rule("kill_streak_cap")
local STREAK_GOLD = config.rule("kill_streak_gold")
local FIRST_BLOOD_GOLD = config.rule("first_blood_gold")
local FIRST_BLOOD_RELIABLE = config.rule("first_blood_reliable_gold")
local FIRST_BLOOD_SPLIT = config.rule("first_blood_split_gold")
local DEATH_LOSS_PER_LEVEL = config.rule("death_gold_loss_per_level")
local DAMAGE_WINDOW = config.rule("kill_window_s") * TICKS
local ASSIST_RADIUS = config.milli(config.rule("assist_gold_radius_milli"))
local KILLER_FAR = config.milli(20312)

local ASSIST = {
  { base = 150, level = 8, worth = 0.0375, lead = 100 },
  { base = 100, level = 7, worth = 0.0375, lead = 75 },
  { base = 40, level = 6, worth = 0.0375, lead = 50 },
  { base = 25, level = 4, worth = 0.03, lead = 35 },
  { base = 20, level = 4, worth = 0.0225, lead = 25 },
}
local WEIGHTS = {
  { 1 },
  { 0.75, 1.25 },
  { 0.75, 1, 1.25 },
  { 0.75, 0.75, 1.25, 1.25 },
  { 0.75, 0.75, 1, 1.25, 1.25 },
}

local GREED = "alchemist_goblin_s_greed"
local GREED_WINDOW = 30 * TICKS
local GREED_RING = 8
local GREED_NEXT = "greed_next"
local GREED_KILL = "greed_kill_"

local kill_gold = {}

local function greed_rank(hero)
  for _, a in ipairs(game.unit_abilities(hero)) do
    if a.name == GREED then return game.skill_rank(hero, a.slot) end
  end
  return 0
end

local function greed_stack(hero, now)
  local count = 0
  for slot = 0, GREED_RING - 1 do
    local at = game.counter(hero, GREED_KILL .. slot)
    if at > 0 and now - at < GREED_WINDOW then count = count + 1 end
  end
  return count
end

local function greed_gold(hero, rank, now)
  return math.min(greed_stack(hero, now) * 3 + rank * 2 + 4, 4 + 8 * rank)
end

local function greed_count_kill(hero, now)
  local slot = game.counter(hero, GREED_NEXT) % GREED_RING
  game.set_counter(hero, GREED_KILL .. slot, now)
  game.set_counter(hero, GREED_NEXT, slot + 1)
end

local function greed_on_kill(victim, killer, killer_player, victim_player)
  local hero = kill_gold.owner_hero(killer, killer_player)
  if hero == nil or not units.is_hero(hero) or killer_player == nil or killer_player == NEUTRAL then return 0 end
  if not units.hostile(killer_player, victim_player) or units.is_structure(victim) then return 0 end
  local rank = greed_rank(hero)
  if rank <= 0 then return 0 end
  local now = game.ticks()
  local gold = greed_gold(hero, rank, now)
  greed_count_kill(hero, now)
  return gold
end

local function team_of(hero) return game.team(unit_player(hero)) end

local function worth(hero) return store.net_worth(hero) end

local function team_worth(team)
  local total = 0
  for _, hero in ipairs(bounty.heroes()) do
    if team_of(hero) == team then total = total + worth(hero) end
  end
  return total
end

local function rank_in_team(hero)
  local mine, team, rank = worth(hero), team_of(hero), 1
  for _, other in ipairs(bounty.heroes()) do
    if other ~= hero and team_of(other) == team and worth(other) > mine then rank = rank + 1 end
  end
  return rank
end

local function contains(list, unit)
  for _, u in ipairs(list) do
    if u == unit then return true end
  end
  return false
end

local function sharers(victim, killer, side)
  local out = {}
  local x, y = unit_position(victim)
  for _, u in ipairs(units.units_near(x, y, ASSIST_RADIUS)) do
    if u ~= victim and units.is_hero(u) and game.alive(u) and not units.hostile(unit_player(u), side) then out[#out + 1] = u end
  end
  if killer ~= nil and units.is_hero(killer) and not contains(out, killer) then
    local kx, ky = unit_position(killer)
    local dx, dy = kx - x, ky - y
    if game.alive(killer) or dx * dx + dy * dy > KILLER_FAR * KILLER_FAR then out[#out + 1] = killer end
  end
  return out
end

local function pay_assists(victim, victim_player, killer, side)
  local group = sharers(victim, killer, side)
  local n = #group
  if n == 0 then return end
  local row = ASSIST[math.min(n, #ASSIST)]
  local victim_team = game.team(victim_player)
  local mine, theirs = team_worth(victim_team), team_worth(team_of(group[1]))
  local ahead = math.min(math.max(mine / (theirs + 1) - 1, 0), 1)
  local lead = math.max(mine - theirs, 0)
  local base = (row.base + row.level * xp.character_level(victim) + row.worth * worth(victim) * ahead
    + row.lead * math.min(lead / 4000, 1)) * (1.2 - 0.1 * (rank_in_team(victim) - 1))
  local share = math.floor(base)
  local ordered = {}
  for _, u in ipairs(group) do ordered[#ordered + 1] = { unit = u, rank = rank_in_team(u) } end
  local sorted = {}
  for _, row_unit in ipairs(ordered) do
    local j = #sorted + 1
    while j > 1 and (sorted[j - 1].rank > row_unit.rank or (sorted[j - 1].rank == row_unit.rank and sorted[j - 1].unit > row_unit.unit)) do
      sorted[j] = sorted[j - 1]
      j = j - 1
    end
    sorted[j] = row_unit
  end
  ordered = sorted
  local weights = WEIGHTS[math.min(n, #WEIGHTS)]
  for i, row_unit in ipairs(ordered) do
    local gold = math.floor(share * (weights[i] or 1))
    stats.add(row_unit.unit, "st_bonus_gold", gold)
    bounty.credit({ player = unit_player(row_unit.unit), gold = gold, reliable = true, at = victim, text = true })
  end
end

local function damagers_of(victim, victim_player)
  local out = {}
  for _, h in ipairs(game.recent_hitters(victim, DAMAGE_WINDOW)) do
    if units.is_hero(h) and units.hostile(unit_player(h), victim_player) and not contains(out, h) then out[#out + 1] = h end
  end
  return out
end

local function split_among(victim, group, gold, reliable)
  local each = math.floor(gold / #group)
  for _, h in ipairs(group) do
    bounty.credit({ player = unit_player(h), gold = each, reliable = reliable and each or nil, at = victim, text = true })
  end
end

local function enemy_heroes(victim_player)
  local out = {}
  for _, h in ipairs(bounty.heroes()) do
    if units.hostile(unit_player(h), victim_player) then out[#out + 1] = h end
  end
  return out
end

local function first_blood_open()
  for _, h in ipairs(bounty.heroes()) do
    if game.counter(h, FIRST_BLOOD) ~= 0 then return false end
  end
  return true
end

local function streak_bonus(streak)
  if streak < STREAK_FROM then return 0 end
  return STREAK_GOLD * math.min(streak - STREAK_OFFSET, STREAK_STEPS)
end

function kill_gold.owner_hero(killer, killer_player)
  if killer == nil or units.is_hero(killer) or killer_player == nil or not units.is_summoned(killer) then return killer end
  return bounty.hero_of(killer_player) or killer
end

function kill_gold.hero_died(victim, killer, killer_player, victim_player)
  if aegis.reincarnating(victim) then return end
  killer = kill_gold.owner_hero(killer, killer_player)
  local ended = game.counter(victim, STREAK)
  local first_blood = false
  local level = xp.character_level(victim)
  local loss = math.min(DEATH_LOSS_PER_LEVEL * level, math.max(0, game.gold(victim_player) - bounty.reliable_gold(victim_player)))
  local rewarded = killer ~= nil and killer ~= victim and killer_player ~= nil and killer_player ~= NEUTRAL
    and units.hostile(killer_player, victim_player)
  if rewarded then
    local greed = greed_on_kill(victim, killer, killer_player, victim_player)
    if greed > 0 then bounty.credit({ player = killer_player, gold = greed, at = killer, text = true }) end
    local gold = KILL_BASE + KILL_PER_LEVEL * level + streak_bonus(game.counter(victim, STREAK))
    local creep_kill = not units.is_hero(killer)
    local damagers = creep_kill and damagers_of(victim, victim_player) or {}
    if first_blood_open() and (not creep_kill or #damagers > 0) then
      game.set_counter(victim, FIRST_BLOOD, 1)
      first_blood = true
      if creep_kill then
        split_among(victim, damagers, FIRST_BLOOD_SPLIT, true)
      else
        bounty.credit({ player = killer_player, gold = FIRST_BLOOD_GOLD, reliable = FIRST_BLOOD_RELIABLE, at = victim, text = true })
      end
    end
    local credited = nil
    if not creep_kill then credited = killer elseif #damagers == 1 then credited = damagers[1] end
    if credited ~= nil then
      bounty.credit({ player = unit_player(credited), gold = gold, reliable = true, at = victim, text = true })
      game.set_counter(credited, STREAK, game.counter(credited, STREAK) + 1)
    else
      local group = #damagers > 1 and damagers or enemy_heroes(victim_player)
      if #group > 0 then split_among(victim, group, gold, true) end
    end
    pay_assists(victim, victim_player, killer, killer_player)
    game.set_counter(victim, STREAK, 0)
    local x, y = unit_position(victim)
    announce.hero_kill({ victim = victim, victim_player = victim_player, killer_player = killer_player, credited = credited,
      gold = gold, ended = ended, streak = credited ~= nil and game.counter(credited, STREAK) or 0, first = first_blood, x = x, y = y })
  else
    announce.hero_unrewarded(victim, killer, killer_player, victim_player)
  end
  bounty.debit(victim_player, loss, victim)
  stats.add(victim, "st_gold_lost", loss)
end

game.trigger("on_died", {
  conditions = { function(unit) return not units.is_hero(unit) end },
  action = function(unit, killer, killer_player, amount, victim_player, killer_attributed)
    if aegis.reincarnating(unit) then return end
    if killer == nil or killer == unit or killer_player == nil or killer_player == NEUTRAL then return end
    if not units.hostile(killer_player, victim_player) then return end
    local gold = amount + greed_on_kill(unit, killer, killer_player, victim_player)
    if gold > 0 and killer_attributed then
      bounty.credit({ player = killer_player, gold = gold, at = unit, text = true })
    end
  end,
})

return {
  on_state_replaced = bounty.forget,

  on_tick = function()
    local now = game.ticks()
    if now > 0 then
      local passive = modes.has("em") and PASSIVE_GOLD * EASY_PASSIVE_FACTOR or PASSIVE_GOLD
      local due = math.floor(now * passive / PASSIVE_PERIOD_TICKS) - math.floor((now - 1) * passive / PASSIVE_PERIOD_TICKS)
      if due > 0 then
        for _, hero in ipairs(bounty.heroes()) do game.cmd_add_gold(unit_player(hero), due) end
      end
    end
    for _, hero in ipairs(bounty.heroes()) do
      local wallet = game.unit_gold(hero)
      if wallet > 0 and game.alive(hero) then
        game.cmd_add_unit_gold(hero, -wallet)
        game.cmd_add_gold(unit_player(hero), wallet)
      end
    end
    if now % CLAMP_EVERY == 0 then bounty.clamp_reliable() end
  end,

  hero_died = kill_gold.hero_died,
  owner_hero = kill_gold.owner_hero,
}

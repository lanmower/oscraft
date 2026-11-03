local config = require("config")
local units = require("units")
local kill_gold = require("kill_gold")
local respawn = require("respawn")
local counter = require("counter")
local xp = require("xp")
local aegis = require("aegis")
local modes = require("modes")
local bounty = require("bounty")
local terms = require("respawn_terms")
local stats = require("stats")

local TICKS = config.TICKS_PER_SECOND
local NEUTRAL = 255

local S_DAMAGE, S_HEALTH, S_ATTACK_SPEED = 0, 2, 12
local S_MANA_REGEN, S_HEALTH_REGEN, S_MAX_MANA, S_ARMOR_X100 = 13, 14, 15, 26

local ASSIST_WINDOW_TICKS = config.rule("kill_window_s") * TICKS
local buyback_ready = counter.sim("buyback_ready")
local respawn_due = counter.sim("respawn_due")
local respawn_extra = counter.sim("respawn_extra")
local respawn_carried = counter.sim("respawn_carried")
local penalty_until = counter.sim("buyback_penalty_until")
local applied = {}

local BUYBACK_SHOP = {
  { { 46000, 275125 }, { 46000, 272344 }, { 46000, 269563 }, { 46000, 266000 }, { 46000, 263000 } },
  { { 273000, 63000 }, { 273000, 66000 }, { 273000, 69000 }, { 273000, 72000 }, { 272000, 75000 } },
}

local function buyback_shop(unit)
  local player = unit_player(unit)
  local team = BUYBACK_SHOP[game.team(player) + 1]
  local shop = team and team[math.floor(player / 2) + 1]
  if shop == nil then return nil end
  return config.milli(shop[1]), config.milli(shop[2])
end

local HP_PER_ATTR_A = config.rule("hp_per_attr_a")
local HP_REGEN_PER_ATTR_A_X100 = config.rule("hp_regen_per_attr_a_x100")
local UNIVERSAL_DAMAGE_PCT = config.rule("universal_damage_pct")
local ARMOUR_PER_ATTR_B_X100 = config.rule("armour_per_attr_b_x100")
local ARMOUR_BASE_PENALTY_X100 = config.rule("armour_base_penalty_x100")
local MANA_BASE_DEFAULT = config.rule("mana_base_default")
local MANA_PER_ATTR_C = config.rule("mana_per_attr_c")
local MANA_REGEN_PER_ATTR_C_X100 = config.rule("mana_regen_per_attr_c_x100")

local level_xp, level_seen = {}, {}
local function level_of(unit)
  local have = game.xp(unit)
  local seen = level_xp[unit]
  if seen ~= nil and seen == have then return level_seen[unit] end
  level_xp[unit] = have
  local level = xp.level_of(unit)
  level_seen[unit] = level
  return level
end

local hero_ids, hero_count = {}, -1
local function heroes()
  local count = get_unit_count()
  if count ~= hero_count then
    hero_count = count
    hero_ids = {}
    for i = 0, count - 1 do
      if units.is_hero(i) then hero_ids[#hero_ids + 1] = i end
    end
  end
  return hero_ids
end

local XP_RADIUS_MILLI = 20703
local XP_MAX_LEVEL = 25
local XP_FAR_KILLER_MILLI = 20312
local HERO_BASE_XP = { 100, 120, 140, 160, 180 }
local PARTICIPANT_XP = { { 20, 23 }, { 15, 23 }, { 10, 20 }, { 7, 15 }, { 5, 12 } }

local function hero_base_xp(level)
  return HERO_BASE_XP[level] or 100 * level - 320
end

local function xp_heroes(victim, keep)
  local found = {}
  local x, y = unit_position(victim)
  for _, u in ipairs(units.units_near(x, y, config.milli(XP_RADIUS_MILLI))) do
    if u ~= victim and game.alive(u) and units.is_hero(u) and keep(unit_player(u)) then found[#found + 1] = u end
  end
  return found
end

local function side_xp(victim_player, victim_side)
  local total = 0
  for _, h in ipairs(heroes()) do
    if units.hostile(unit_player(h), victim_player) ~= victim_side then total = total + game.xp(h) end
  end
  return total
end

local function distance_fix(a, b)
  local ax, ay = unit_position(a)
  local bx, by = unit_position(b)
  return math.sqrt((ax - bx) ^ 2 + (ay - by) ^ 2)
end

local function award_xp(recipients, base, bonus)
  local share = math.floor(base / #recipients)
  for _, u in ipairs(recipients) do
    if xp.character_level(u) < XP_MAX_LEVEL then game.cmd_add_xp(u, share + bonus) end
  end
end

local function share_hero_xp(victim, killer, victim_player, killer_player)
  local level = xp.character_level(victim)
  local recipients = xp_heroes(victim, function(side) return not units.hostile(side, killer_player) end)
  local present = false
  for _, u in ipairs(recipients) do present = present or u == killer end
  if units.is_hero(killer) and not present then recipients[#recipients + 1] = killer end
  local n = #recipients
  if n == 0 then return end
  local row = PARTICIPANT_XP[math.min(n, 5)]
  local mine, theirs = side_xp(victim_player, true), side_xp(victim_player, false)
  local deficit = math.max((mine - theirs) / math.max(mine + theirs, 1), 0)
  local bonus = row[1] * level + math.floor(deficit * game.xp(victim) * row[2] / 100 * 0.6)
  local base = hero_base_xp(level)
  local share = math.floor(base / n)
  for _, u in ipairs(recipients) do
    game.cmd_add_xp(u, share + bonus)
    stats.add(u, "st_bonus_xp", bonus)
  end
  local owner_hero = units.hero_of(heroes(), killer_player)
  if owner_hero ~= nil and distance_fix(owner_hero, victim) > config.milli(XP_FAR_KILLER_MILLI) then game.cmd_add_xp(owner_hero, base) end
end

local function share_xp(victim, killer, victim_player, killer_player)
  if killer_player ~= NEUTRAL and victim_player ~= NEUTRAL and not units.hostile(killer_player, victim_player) then
    if units.is_hero(victim) then return end
    local pool = math.floor(game.xp_bounty(victim) * game.deny_xp_pct(victim) / 100)
    if pool <= 0 then return end
    local recipients = xp_heroes(victim, function(side) return units.hostile(side, killer_player) end)
    if #recipients > 0 then award_xp(recipients, pool, 0) end
    return
  end
  if units.is_hero(victim) then return share_hero_xp(victim, killer, victim_player, killer_player) end
  local pool = game.xp_bounty(victim)
  if pool <= 0 then return end
  if units.is_summoned(victim) then pool = math.floor(pool / 2) end
  local ancient = game.unit_kind(victim) == "roshan"
  local recipients = xp_heroes(victim, function(side)
    return (victim_player == NEUTRAL and not ancient) or not units.hostile(side, killer_player)
  end)
  if #recipients > 0 then award_xp(recipients, pool, 0) end
end

local function credit_assists(victim, killer, killer_player)
  for _, h in ipairs(game.take_recent_hitters(victim, ASSIST_WINDOW_TICKS)) do
    if h ~= killer and unit_player(h) == killer_player then
      game.add_credit(h, 1)
    end
  end
end

local function buyback_cost(unit)
  local minutes = math.floor(game.ticks() / (60 * TICKS))
  local level = level_of(unit)
  local index = math.floor((config.rule("buyback_base") * 10 + config.rule("buyback_level_sq_x10") * level * level
    + config.rule("buyback_per_minute") * 10 * minutes) / (config.rule("buyback_index_div") * 10))
  index = math.max(0, math.min(config.rule("buyback_index_max"), index))
  return config.rule("buyback_step_gold") * (1 + index)
end

local function set_bonus(unit, code, target)
  local a = applied[unit]
  if a == nil then
    a = {}
    applied[unit] = a
  end
  local prev = a[code] or 0
  if target ~= prev then
    game.cmd_add_stat(unit, code, target - prev)
    a[code] = target
  end
end

local ATTRIBUTES = {
  { field = "str", item = "strength", code = 22 },
  { field = "agi", item = "agility", code = 23 },
  { field = "int", item = "intelligence", code = 24 },
}

local ATTRIBUTE_VALUES = { 0, 0, 0 }
local function unit_attributes(unit)
  local level = level_of(unit)
  for k, attr in ipairs(ATTRIBUTES) do
    local base = game.attr_field(unit, attr.field) or 0
    local gain = game.attr_field(unit, attr.field .. "_gain") or 0
    ATTRIBUTE_VALUES[k] = base + gain * level + (game.item_bonus(unit, attr.item) + game.stat_value(unit, attr.code)) * 100
  end
  return ATTRIBUTE_VALUES[1], ATTRIBUTE_VALUES[2], ATTRIBUTE_VALUES[3]
end

local published_points = {}
local POINT_COUNTERS = { "attr_str", "attr_agi", "attr_int" }
local function publish_points(unit, s, a, n)
  local seen = published_points[unit]
  if seen == nil then
    seen = { -1, -1, -1 }
    published_points[unit] = seen
  end
  local s_pts, a_pts, n_pts = math.floor(s / 100), math.floor(a / 100), math.floor(n / 100)
  if seen[1] ~= s_pts then seen[1] = s_pts; game.set_counter(unit, POINT_COUNTERS[1], s_pts) end
  if seen[2] ~= a_pts then seen[2] = a_pts; game.set_counter(unit, POINT_COUNTERS[2], a_pts) end
  if seen[3] ~= n_pts then seen[3] = n_pts; game.set_counter(unit, POINT_COUNTERS[3], n_pts) end
end

local function attribute_bonuses(unit)
  local s, a, n = unit_attributes(unit)
  publish_points(unit, s, a, n)
  local primary = game.attr_field(unit, "primary")
  local damage_x100
  if primary == 0 then
    damage_x100 = s
  elseif primary == 1 then
    damage_x100 = a
  elseif primary == 2 then
    damage_x100 = n
  else
    damage_x100 = math.floor((s + a + n) * UNIVERSAL_DAMAGE_PCT / 100)
  end
  set_bonus(unit, S_HEALTH, math.floor(HP_PER_ATTR_A * s / 100))
  set_bonus(unit, S_HEALTH_REGEN, math.floor(HP_REGEN_PER_ATTR_A_X100 * s / 100))
  set_bonus(unit, S_DAMAGE, math.floor(damage_x100 / 100))
  set_bonus(unit, S_ARMOR_X100, math.floor(a * ARMOUR_PER_ATTR_B_X100 / 100) - ARMOUR_BASE_PENALTY_X100)
  set_bonus(unit, S_ATTACK_SPEED, math.floor(a / 100))
  local mana_default = 0
  if game.attr_field(unit, "mana") == nil then mana_default = MANA_BASE_DEFAULT end
  set_bonus(unit, S_MAX_MANA, mana_default + math.floor(MANA_PER_ATTR_C * n / 100))
  set_bonus(unit, S_MANA_REGEN, math.floor(MANA_REGEN_PER_ATTR_C_X100 * n / 100))
end

local function pay_kill(unit, killer, killer_player, victim_player)
  if units.is_hero(unit) then kill_gold.hero_died(unit, killer, killer_player, victim_player) end
  if killer == nil or killer == unit or aegis.reincarnating(unit) then return end
  local credit = units.is_hero(unit) and kill_gold.owner_hero(killer, killer_player) or killer
  if not game.alive(credit) then
    game.take_recent_hitters(unit, 0)
    return
  end
  if units.is_hero(credit) and units.hostile(killer_player, victim_player) then game.add_kill(credit, 1) end
  credit_assists(unit, credit, killer_player)
  share_xp(unit, credit, victim_player, killer_player)
end

local function buyback(unit, ticks)
  if not units.is_hero(unit) or game.alive(unit) then return end
  if terms.buyback_barred(unit) then return end
  if not buyback_ready.ready(unit, ticks) then return end
  local cost = buyback_cost(unit)
  if game.gold(unit_player(unit)) < cost then return end
  game.cmd_add_gold(unit_player(unit), -cost)
  bounty.spend_reliable(unit_player(unit), cost)
  local x, y = buyback_shop(unit)
  if x == nil or not game.revive_at(unit, x, y) then game.cmd_set_respawn(unit, 0) end
  local remaining = math.max(0, respawn_due.get(unit) - game.ticks())
  respawn_extra.set(unit, respawn_carried.get(unit) + math.floor(remaining / (4 * TICKS)))
  penalty_until.set(unit, game.ticks() + remaining)
  buyback_ready.arm(unit, ticks + config.rule("buyback_cooldown_s") * TICKS)
end

return {
  seat_pick = function(player, hero_kind)
    for _, id in ipairs(game.units(player)) do
      if units.is_hero(id) then return end
    end
    local start = game.seat_start(player)
    if start == nil then return end
    local unit = game.spawn(player, hero_kind, start.x, start.y)
    if unit == nil then return end
    game.enroll(unit, start.x, start.y)
  end,

  on_died = function(unit, killer, killer_player, _, victim_player)
    pay_kill(unit, killer, killer_player, victim_player)
    terms.died(unit, killer)
  end,

  on_unit_died = function(unit)
    if not units.is_hero(unit) then return nil end
    if aegis.reincarnating(unit) then return false end
    local carried = respawn_extra.get(unit)
    respawn_carried.set(unit, carried)
    respawn_extra.set(unit, 0)
    local base = math.max(0, respawn.hero(level_of(unit) + 1) - terms.cut(unit))
    local secs = modes.respawn(base) + carried + terms.added(unit)
    respawn_due.set(unit, game.ticks() + secs * TICKS)
    stats.add(unit, "st_time_dead", secs)
    return secs
  end,

  on_cast = function(unit, ability, state)
    terms.cast(unit, ability)
    if ability ~= "buyback" then return end
    buyback(unit, state.ticks or 0)
  end,

  buyback = function(player)
    local unit = bounty.hero_of(player)
    if unit ~= nil then buyback(unit, game.ticks()) end
  end,

  on_tick = function()
    for _, i in ipairs(heroes()) do
      if game.alive(i) then
        attribute_bonuses(i)
        terms.trickle(i)
      end
    end
  end,
}

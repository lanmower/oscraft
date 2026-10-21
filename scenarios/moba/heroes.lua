-- Heroes, as mod code: a seat's spawn, the respawn time and buyback, the attribute bonuses, passive gold, and the gold,
-- xp share and assist credit of a hero's kill. The numbers are rules (shared/config.lua reads them).
local config = require("config")
local units = require("units")
local bounty = require("bounty")
local respawn = require("respawn")
local counter = require("counter")

local TICKS = config.TICKS_PER_SECOND

-- Stat codes (sim/src/spell.rs): S_DAMAGE 0, S_HEALTH 2, S_ATTACK_SPEED 12, S_MANA_REGEN 13,
-- S_HEALTH_REGEN 14, S_MAX_MANA 15, S_ARMOR_X100 26.
local S_DAMAGE, S_HEALTH, S_ATTACK_SPEED = 0, 2, 12
local S_MANA_REGEN, S_HEALTH_REGEN, S_MAX_MANA, S_ARMOR_X100 = 13, 14, 15, 26

local ASSIST_WINDOW_TICKS = 50
local buyback_ready = counter.sim("buyback_ready")
local applied = {}  -- unit -> { code -> the stat bonus last sent }

-- Xp share of a kill: an enemy kill gives the victim's xp bounty to the killer and to each allied hero in the victim's
-- assist radius, split evenly with the rest to the first. A denial (the killer is the victim's ally) gives deny_pct of
-- the bounty to each enemy hero in the radius.
local function share_xp(victim, killer, victim_player, killer_player)
  local denied = killer_player == victim_player
  local pool = game.xp_bounty(victim)
  if denied then pool = math.floor(pool * game.deny_xp_pct(victim) / 100) end
  if pool <= 0 then return end
  local recipients = {}
  if not denied then recipients[#recipients + 1] = killer end
  local x, y = unit_position(victim)
  for _, u in ipairs(units.units_near(x, y, config.milli(game.assist_radius_milli(victim)))) do
    if u ~= victim and u ~= killer and game.alive(u) and units.is_hero(u) then
      local side = unit_player(u)
      local on_side
      if denied then
        on_side = units.hostile(side, victim_player)
      else
        on_side = not units.hostile(side, killer_player)
      end
      if on_side then recipients[#recipients + 1] = u end
    end
  end
  local count = #recipients
  if count == 0 then return end
  local share = math.floor(pool / count)
  local rest = pool - share * count
  for idx, u in ipairs(recipients) do
    game.cmd_add_xp(u, share + (idx == 1 and rest or 0))
  end
end

-- Assist credit: each recent hitter of the victim on the killer's side, other than the killer, gets one credit.
local function credit_assists(victim, killer, killer_player)
  for _, h in ipairs(game.take_recent_hitters(victim, ASSIST_WINDOW_TICKS)) do
    if h ~= killer and unit_player(h) == killer_player then
      game.add_credit(h, 1)
    end
  end
end

local function hero_gold(unit)
  return config.rule("hero_kill_gold_base") + config.rule("hero_kill_gold_per_level") * game.level(unit)
end

-- Buyback: base + per level + a share of net worth; the cooldown is a per-unit counter.
local function buyback_cost(unit)
  local nw = math.floor(game.net_worth(unit) * config.rule("buyback_net_worth_pct") / (100 * config.rule("buyback_net_worth_div")))
  return config.rule("buyback_base") + config.rule("buyback_per_level") * game.level(unit) + nw
end

-- The stat bonus of unit `unit` for `code` last sent, so each tick sends only the change.
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

-- The attribute-to-stat rules: strength gives health, health regen and the universal or strength damage;
-- agility gives armour, attack speed and the agility damage; intelligence gives mana and mana regen.
-- Attributes are hundredths (game.attributes_x100); the coefficients are rules.
local function attribute_bonuses(unit)
  local s, a, n = game.attributes_x100(unit)
  local primary = game.attr_field(unit, "primary")
  local damage_x100
  if primary == 0 then
    damage_x100 = s
  elseif primary == 1 then
    damage_x100 = a
  elseif primary == 2 then
    damage_x100 = n
  else
    damage_x100 = math.floor((s + a + n) * config.rule("universal_damage_pct") / 100)
  end
  set_bonus(unit, S_HEALTH, math.floor(config.rule("hp_per_attr_a") * s / 100))
  set_bonus(unit, S_HEALTH_REGEN, math.floor(config.rule("hp_regen_per_attr_a_x100") * s / 100))
  set_bonus(unit, S_DAMAGE, math.floor(damage_x100 / 100))
  set_bonus(unit, S_ARMOR_X100, math.floor(a / config.rule("armour_per_attr_b_div")))
  set_bonus(unit, S_ATTACK_SPEED, math.floor(a / 100))
  set_bonus(unit, S_MAX_MANA, math.floor(config.rule("mana_per_attr_c") * n / 100))
  set_bonus(unit, S_MANA_REGEN, math.floor(config.rule("mana_regen_per_attr_c_x100") * n / 100))
end

return {
  -- A seat's pick from the lobby's start: spawn the picked hero at the seat's start point and enrol it. Dispatched by the
  -- page for each seat that picked a hero (controls.lua routes the "seat_pick" action here).
  seat_pick = function(player, hero_kind)
    local start = game.seat_start(player)
    if start == nil then return end
    local unit = game.spawn(player, hero_kind, start.x, start.y)
    if unit == nil then return end
    game.enroll(unit, start.x, start.y)
  end,

  -- A hero's kill: its gold (the killer keeps all but the team share), the assist credit and the xp share.
  on_died = function(unit, killer, killer_player, _, victim_player)
    if killer == nil or killer == unit then return end
    -- The kill tally counts hero kills: a creep's kill is still a kill event for every mod, but not a hero's kill.
    if units.is_hero(killer) and killer_player ~= victim_player then game.add_kill(killer, 1) end
    if units.is_hero(unit) and killer_player ~= victim_player then
      bounty.pay({ unit = unit, side = killer_player, killer = killer, gold = hero_gold(unit),
        share_pct = config.rule("team_share_pct"), to_unit = true })
    end
    credit_assists(unit, killer, killer_player)
    share_xp(unit, killer, victim_player, killer_player)
  end,

  -- A dead hero's respawn time, by its level.
  on_unit_died = function(unit)
    if not units.is_hero(unit) then return nil end
    return respawn.linear(game.level(unit), "respawn_base_s", "respawn_per_level_s", "respawn_cap_s")
  end,

  -- Buyback: a dead hero with the gold and the cooldown ready returns at once.
  on_cast = function(unit, ability, state)
    if ability ~= "buyback" then return end
    if not units.is_hero(unit) or game.alive(unit) then return end
    local ticks = state.ticks or 0
    if not buyback_ready.ready(unit, ticks) then return end
    local cost = buyback_cost(unit)
    if game.unit_gold(unit) < cost then return end
    game.cmd_add_unit_gold(unit, -cost)
    game.cmd_set_respawn(unit, 0)
    buyback_ready.arm(unit, ticks + config.rule("buyback_cooldown_s") * TICKS)
  end,

  -- Each second, passive gold for every hero; each tick, the attribute bonuses of each living hero.
  on_tick = function(state)
    local now = state.ticks or 0
    local passive_turn = now % TICKS == 0
    for i = 0, get_unit_count() - 1 do
      if units.is_hero(i) then
        if game.alive(i) then attribute_bonuses(i) end
        if passive_turn then game.cmd_add_unit_gold(i, config.rule("passive_gold_per_s")) end
      end
    end
  end,
}

-- Hero rules for the moba scenario, as mod code on generic primitives. The numbers are named values from
-- scenarios/moba/attr_rules.txt, read through game.rule(key). Part of the moba chunk: protection.lua
-- calls hero_on_tick and hero_on_cast, so this file defines no on_tick or on_cast of its own.
--
-- Primitives used: on_seat_pick (spawn and enrol), on_died (respawn policy, kill gold, xp share, assist credit),
-- game.spawn, game.enroll, game.rule, game.take_recent_hitters, game.add_credit, game.is_attributed, game.alive, game.level, game.net_worth, game.unit_gold,
-- game.attributes_x100, game.attr_field, game.xp_bounty, game.assist_radius_milli, game.deny_xp_pct,
-- game.counter, game.set_counter, game.hostile, game.units_near, unit_player, unit_position,
-- cmd_set_respawn, cmd_add_unit_gold, cmd_add_gold, cmd_add_xp, cmd_add_stat.

local TICKS_PER_SECOND = 16

-- Stat codes (sim/src/spell.rs): S_DAMAGE 0, S_HEALTH 2, S_ATTACK_SPEED 12, S_MANA_REGEN 13,
-- S_HEALTH_REGEN 14, S_MAX_MANA 15, S_ARMOR_X100 26.
local S_DAMAGE, S_HEALTH, S_ATTACK_SPEED = 0, 2, 12
local S_MANA_REGEN, S_HEALTH_REGEN, S_MAX_MANA, S_ARMOR_X100 = 13, 14, 15, 26

-- Seat pick: the page records the pick, the mod spawns the hero at the seat's start and enrols it.
-- seat_start is the seat's start point in Q16.16 (x, y); player is the seat's player.
function on_seat_pick(player, hero_kind, seat_start_x, seat_start_y, state)
  local unit = game.spawn(player, hero_kind, seat_start_x, seat_start_y)
  if unit == nil then return end
  game.enroll(unit, seat_start_x, seat_start_y)
end

-- Milli-units to Q16.16 (65536 per unit, milli-units are 1/1000), as protection.lua does.
local function milli_to_fixed(milli)
  return math.floor(milli * 65536 / 1000)
end

-- Xp share of a kill, as the engine shared it: an enemy kill gives the victim's xp bounty to the killer
-- and to each allied hero in the victim's assist radius, split evenly with the rest to the first. A denial
-- (the killer is the victim's ally) gives deny_pct of the bounty to each enemy hero in the radius.
local function share_xp(victim, killer, victim_player, killer_player)
  local denied = killer_player == victim_player
  local pool = game.xp_bounty(victim)
  if denied then pool = math.floor(pool * game.deny_xp_pct(victim) / 100) end
  if pool <= 0 then return end
  local recipients = {}
  if not denied then recipients[#recipients + 1] = killer end
  local x, y = unit_position(victim)
  for _, u in ipairs(game.units_near(x, y, milli_to_fixed(game.assist_radius_milli(victim)))) do
    if u ~= victim and u ~= killer and game.alive(u) and game.is_attributed(u) then
      local side = unit_player(u)
      local on_side
      if denied then
        on_side = game.hostile(side, victim_player)
      else
        on_side = not game.hostile(side, killer_player)
      end
      if on_side then recipients[#recipients + 1] = u end
    end
  end
  local count = #recipients
  if count == 0 then return end
  local share = math.floor(pool / count)
  local rest = pool - share * count
  for idx, u in ipairs(recipients) do
    cmd_add_xp(u, share + (idx == 1 and rest or 0))
  end
end

-- Assist credit: each recent hitter of the victim on the killer's side, other than the killer, gets one credit.
-- The hitters within ASSIST_WINDOW_TICKS of the death are read once and forgotten.
local ASSIST_WINDOW_TICKS = 50

local function credit_assists(victim, killer, killer_player)
  for _, h in ipairs(game.take_recent_hitters(victim, ASSIST_WINDOW_TICKS)) do
    if h ~= killer and unit_player(h) == killer_player then
      game.add_credit(h, 1)
    end
  end
end

-- Respawn policy: base + per level seconds, capped. Kill gold and team share on a hero's death, the xp share, and the
-- assist credit.
function on_died(unit, killer, killer_player, bounty, victim_player)
  camp_died(unit, killer)
  if game.is_attributed(unit) then
    local secs = game.rule("respawn_base_s") + game.rule("respawn_per_level_s") * game.level(unit)
    local cap = game.rule("respawn_cap_s")
    if secs > cap then secs = cap end
    cmd_set_respawn(unit, secs)
  end
  if killer ~= nil and killer ~= unit then
    if game.is_attributed(unit) and killer_player ~= victim_player then
      local gold = game.rule("hero_kill_gold_base") + game.rule("hero_kill_gold_per_level") * game.level(unit)
      local share = math.floor(gold * game.rule("team_share_pct") / 100)
      cmd_add_unit_gold(killer, gold - share)
      cmd_add_gold(killer_player, share)
    end
    credit_assists(unit, killer, killer_player)
    share_xp(unit, killer, victim_player, killer_player)
  end
end

-- Buyback: cost is base + per level + net worth share; cooldown is a per-unit counter.
local function buyback_cost(unit)
  local nw = math.floor(game.net_worth(unit) * game.rule("buyback_net_worth_pct") / (100 * game.rule("buyback_net_worth_div")))
  return game.rule("buyback_base") + game.rule("buyback_per_level") * game.level(unit) + nw
end

function hero_on_cast(unit, ability, state)
  if ability ~= "buyback" then return end
  if not game.is_attributed(unit) or game.alive(unit) then return end
  if game.counter(unit, "buyback_ready") > (state.ticks or 0) then return end
  local cost = buyback_cost(unit)
  if game.unit_gold(unit) < cost then return end
  cmd_add_unit_gold(unit, -cost)
  cmd_set_respawn(unit, 0)
  game.set_counter(unit, "buyback_ready", (state.ticks or 0) + game.rule("buyback_cooldown_s") * TICKS_PER_SECOND)
end

-- The stat bonus of unit `unit` for `code` last sent, so each tick sends only the change.
local applied = {}  -- unit -> { code -> bonus }
local function set_bonus(unit, code, target)
  local a = applied[unit]
  if a == nil then
    a = {}
    applied[unit] = a
  end
  local prev = a[code] or 0
  if target ~= prev then
    cmd_add_stat(unit, code, target - prev)
    a[code] = target
  end
end

-- The attribute-to-stat rules: strength gives health, health regen and the universal or strength damage;
-- agility gives armour, attack speed and the agility damage; intelligence gives mana and mana regen.
-- Attributes are hundredths (game.attributes_x100); the coefficients are attr_rules.txt values.
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
    damage_x100 = math.floor((s + a + n) * game.rule("universal_damage_pct") / 100)
  end
  set_bonus(unit, S_HEALTH, math.floor(game.rule("hp_per_attr_a") * s / 100))
  set_bonus(unit, S_HEALTH_REGEN, math.floor(game.rule("hp_regen_per_attr_a_x100") * s / 100))
  set_bonus(unit, S_DAMAGE, math.floor(damage_x100 / 100))
  set_bonus(unit, S_ARMOR_X100, math.floor(a / game.rule("armour_per_attr_b_div")))
  set_bonus(unit, S_ATTACK_SPEED, math.floor(a / 100))
  set_bonus(unit, S_MAX_MANA, math.floor(game.rule("mana_per_attr_c") * n / 100))
  set_bonus(unit, S_MANA_REGEN, math.floor(game.rule("mana_regen_per_attr_c_x100") * n / 100))
end

-- The level and life each hero had at the last tick, for the level-up and respawn effects.
local seen_level = {}
local seen_alive = {}

-- Per tick: the attribute bonuses of each living hero; passive gold each second for every hero; the level-up and
-- respawn effects (effects.lua).
function hero_on_tick(state)
  local now = state.ticks or 0
  local passive_turn = now % TICKS_PER_SECOND == 0
  for i = 0, get_unit_count() - 1 do
    if game.is_attributed(i) then
      local alive = game.alive(i)
      if alive then attribute_bonuses(i) end
      if passive_turn then cmd_add_unit_gold(i, game.rule("passive_gold_per_s")) end
      local level = game.level(i)
      level_effects(i, level, seen_level[i], alive, seen_alive[i])
      seen_level[i] = level
      seen_alive[i] = alive
    end
  end
end

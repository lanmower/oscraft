-- Hero rules for the moba scenario, as mod code on generic primitives. The numbers are named values from
-- scenarios/moba/hero_rules.txt, read through game.rule(key). Part of the moba chunk: protection.lua
-- calls hero_on_tick and hero_on_cast, so this file defines no on_tick or on_cast of its own.
--
-- Primitives used: on_seat_pick (spawn and enroll), on_died (respawn policy, kill gold, assist gold),
-- game.spawn, game.enroll, game.rule, game.is_hero, game.level, game.net_worth, game.unit_gold,
-- game.counter / game.set_counter, cmd_set_respawn, cmd_add_unit_gold, cmd_add_gold, cmd_add_xp, cmd_revive_at_point.

local TICKS_PER_SECOND = 16

-- Seat pick: the page records the pick, the mod spawns the hero at the seat's start and enrols it.
-- seat_start is the seat's start point in Q16.16 (x, y); player is the seat's player.
function on_seat_pick(player, hero_kind, seat_start_x, seat_start_y, state)
  local unit = game.spawn(player, hero_kind, seat_start_x, seat_start_y)
  if unit == nil then return end
  game.enroll(unit, seat_start_x, seat_start_y)
end

-- Respawn policy: base + per level seconds, capped. Kill gold and team share on the kill, and assist gold.
function on_died(unit, killer, killer_player, bounty, victim_player)
  if game.is_hero(unit) then
    local secs = game.rule("respawn_base_s") + game.rule("respawn_per_level_s") * game.level(unit)
    local cap = game.rule("respawn_cap_s")
    if secs > cap then secs = cap end
    cmd_set_respawn(unit, secs)
  end
  if killer ~= nil and game.is_hero(unit) and killer_player ~= victim_player then
    local gold = game.rule("hero_kill_gold_base") + game.rule("hero_kill_gold_per_level") * game.level(unit)
    local share = gold * game.rule("team_share_pct") // 100
    cmd_add_unit_gold(killer, gold - share)
    cmd_add_gold(killer_player, share)
  end
end

-- Buyback: cost is base + per level + net worth share; cooldown is a per-unit counter.
local function buyback_cost(unit)
  local nw = game.net_worth(unit) * game.rule("buyback_net_worth_pct") // (100 * game.rule("buyback_net_worth_div"))
  return game.rule("buyback_base") + game.rule("buyback_per_level") * game.level(unit) + nw
end

function hero_on_cast(unit, ability, state)
  if ability ~= "buyback" then return end
  if game.is_hero(unit) == false or game.alive(unit) then return end
  if game.counter(unit, "buyback_ready") > (state.ticks or 0) then return end
  local cost = buyback_cost(unit)
  if game.unit_gold(unit) < cost then return end
  cmd_add_unit_gold(unit, -cost)
  cmd_set_respawn(unit, 0)
  game.set_counter(unit, "buyback_ready", (state.ticks or 0) + game.rule("buyback_cooldown_s") * TICKS_PER_SECOND)
end

-- Passive gold each second for every hero.
function hero_on_tick(state)
  if (state.ticks or 0) % TICKS_PER_SECOND ~= 0 then return end
  for i = 0, get_unit_count() - 1 do
    if game.is_hero(i) then
      cmd_add_unit_gold(i, game.rule("passive_gold_per_s"))
    end
  end
end

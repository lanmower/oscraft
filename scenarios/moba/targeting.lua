local config = require("config")
local units = require("units")

local targeting = {}

local NO_UNIT = -1
local TARGET_UNIT = 1
local TARGET_UNIT_OR_POINT = 3
local ORDER_ATTACK_MOVE, ORDER_PATROL = 2, 3

local function within(a, b, range)
  local ax, ay = unit_position(a)
  local bx, by = unit_position(b)
  local dx, dy = ax - bx, ay - by
  return dx * dx + dy * dy <= range * range
end

local HERO_KIND, STRUCTURE_KIND, PLAIN_KIND = 2, 1, 0
local aggro_priority_rule = nil
local kind_cache = {}

local function kind_of(u)
  local k = kind_cache[u]
  if k == nil then
    k = (units.is_hero(u) and HERO_KIND or PLAIN_KIND) + (units.is_structure(u) and STRUCTURE_KIND or PLAIN_KIND)
    kind_cache[u] = k
  end
  return k
end

local function class_order(forced_class, attacker_kind, candidate_kind, is_aggro_target)
  if forced_class >= 0 then return forced_class end
  if aggro_priority_rule ~= 1 then return 0 end
  if (attacker_kind & HERO_KIND) ~= 0 then return 0 end
  local candidate_is_hero = (candidate_kind & HERO_KIND) ~= 0
  if (attacker_kind & STRUCTURE_KIND) ~= 0 then
    if is_aggro_target then return 0 end
    if candidate_kind == PLAIN_KIND then return 1 end
    if candidate_is_hero then return 2 end
    return 3
  end
  if candidate_kind == PLAIN_KIND then return 0 end
  if candidate_is_hero then return 1 end
  return 2
end

local AGGRO_RADIUS_REFERENCE_UNITS, REFERENCE_UNITS_PER_CELL = 500, 64
local AGGRO_RADIUS = AGGRO_RADIUS_REFERENCE_UNITS * config.FIX // REFERENCE_UNITS_PER_CELL
local AGGRO_KINDS = {
  tower = true, tower_t2 = true, tower_t3 = true, tower_t4 = true,
  creep = true, creep_super = true, creep_mega = true,
  creep_ranged = true, creep_ranged_super = true, creep_ranged_mega = true,
}
local aggro_holds = {}

function targeting.aggro_redirect(attacker, victim)
  if attacker < 0 or config.rule("aggro_priority") ~= 1 or attacker == victim then return end
  if not units.is_hero(victim) or not units.is_hero(attacker) then return end
  local attacker_side = unit_player(attacker)
  if not units.hostile(unit_player(victim), attacker_side) then return end
  local now = game.ticks()
  local current_hold = aggro_holds[victim]
  if current_hold ~= nil and now < current_hold.until_tick and unit_alive(current_hold.attacker) then return end
  local hold_ticks = config.rule("aggro_hold_s") * config.TICKS_PER_SECOND
  aggro_holds[victim] = { attacker = attacker, until_tick = now + hold_ticks }
  local x, y = unit_position(attacker)
  for _, u in ipairs(units.units_near(x, y, AGGRO_RADIUS)) do
    if unit_alive(u) and AGGRO_KINDS[game.unit_kind(u)] and units.hostile(unit_player(u), attacker_side)
      and game.aggro_target(u) < 0 then
      game.set_aggro(u, attacker, hold_ticks)
    end
  end
end

function targeting.nearest_enemy(unit)
  local _, range = game.weapon(unit)
  local x, y = unit_position(unit)
  local best, best_distance_sq
  for _, j in ipairs(units.units_near(x, y, range)) do
    if j ~= unit and units.hostile(unit_player(unit), unit_player(j)) and game.can_target(unit, j) then
      local jx, jy = unit_position(j)
      local dx, dy = jx - x, jy - y
      local distance_sq = dx * dx + dy * dy
      if best == nil or distance_sq < best_distance_sq then best, best_distance_sq = j, distance_sq end
    end
  end
  return best
end

function targeting.fired_action(unit, trigger, action, ability, slot, target_kind)
  if action ~= "use_ability" then return end
  local target = NO_UNIT
  if target_kind == TARGET_UNIT then
    target = targeting.nearest_enemy(unit)
    if target == nil then return end
  elseif target_kind == TARGET_UNIT_OR_POINT then
    target = targeting.nearest_enemy(unit) or NO_UNIT
  end
  local x, y = unit_position(unit)
  game.cmd_cast(unit, slot, target, x, y)
end

local cache_tick = -1
local died_at_tick, picked_at_tick = {}, {}
local function best_in_reach(unit, range, aggro_target)
  local reach = game.strike_reach(unit, range)
  local forced_class, attacker_kind = nil, nil
  local best, best_class, best_distance_sq = NO_UNIT, nil, nil
  for k = 1, #reach, 2 do
    local j, distance_sq = reach[k], reach[k + 1]
    if attacker_kind == nil then forced_class, attacker_kind = game.class_override(unit), kind_of(unit) end
    local class = class_order(forced_class, attacker_kind, kind_of(j), aggro_target == j)
    if best_class == nil or class < best_class
      or (class == best_class and (distance_sq < best_distance_sq or (distance_sq == best_distance_sq and j < best))) then
      best, best_class, best_distance_sq = j, class, distance_sq
    end
  end
  return best
end

function targeting.strike_choice(unit)
  local damage, range = game.weapon(unit)
  if damage <= 0 then return NO_UNIT end
  local taunter = game.taunter(unit)
  if taunter ~= nil and within(unit, taunter, range) and game.can_target(unit, taunter) then return taunter end
  local last = game.strike_of(unit)
  local last_died_since_picked = (died_at_tick[last] or -1) >= (picked_at_tick[unit] or 0)
  if last >= 0 and not last_died_since_picked and unit_alive(last) and within(unit, last, range) and game.can_target(unit, last) then return last end
  local pick = best_in_reach(unit, range, game.aggro_target(unit))
  if pick >= 0 then picked_at_tick[unit] = game.ticks() end
  return pick
end

function targeting.on_tick()
  if cache_tick ~= game.ticks() then
    cache_tick = game.ticks()
    aggro_priority_rule = config.rule("aggro_priority")
    kind_cache = {}
  end
  for u = 0, get_unit_count() - 1 do
    if unit_alive(u) then
      local target = targeting.strike_choice(u)
      local order = game.order_of(u)
      if order == ORDER_ATTACK_MOVE or order == ORDER_PATROL then
        if target >= 0 and not game.strike_lands(target) then target = NO_UNIT end
        game.set_attack_hold(u, target >= 0 and game.can_strike(u, target))
      end
      game.cmd_strike(u, target)
    end
  end
end

function targeting.on_died(unit)
  died_at_tick[unit] = game.ticks()
end

function targeting.on_state_replaced()
  cache_tick = -1
  aggro_holds = {}
  died_at_tick, picked_at_tick = {}, {}
end

function targeting.on_hit(attacker, victim)
  targeting.aggro_redirect(attacker, victim)
end

function targeting.on_attacked(unit)
  local target = targeting.nearest_enemy(unit)
  if target ~= nil then game.cmd_attack(unit, target) end
end

function targeting.on_behaviour(unit, trigger, action, ability, slot, target_kind)
  targeting.fired_action(unit, trigger, action, ability, slot, target_kind)
end

return targeting

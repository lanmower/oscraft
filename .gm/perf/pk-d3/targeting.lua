-- Auto-target and aggro rules of the moba pack: the target class order a unit prefers, the aggro redirect an armed
-- structure takes when an allied unit is hit, the strike a unit makes when it is attacked, and the action a fired
-- behaviour takes. The sim keeps the state these set (the class override, the aggro target and its hold, the queued
-- behaviours); the rules themselves are here.

local config = require("config")
local units = require("units")

local targeting = {}

-- TARGETS in the data format: none, unit, point.
local TARGET_UNIT = 1
-- ORDERS in the data format (sim ORDER_ATTACK_MOVE): the order an attack-move unit holds and strikes on.
local ORDER_ATTACK_MOVE = 2


local function within(a, b, range)
  local ax, ay = unit_position(a)
  local bx, by = unit_position(b)
  local dx, dy = ax - bx, ay - by
  return dx * dx + dy * dy <= range * range
end


-- Priority class of candidate `cand` for attacker `att`; lower is preferred. A unit attacker takes plain units,
-- then units, then structures. An armed structure takes its aggro target while it is held (`held`), then plain
-- units, units, structures. Within one class the nearest target wins. Off unless the rule aggro_priority is 1.
function targeting.target_class_order(att, cand, held)
  local forced = game.class_override(att)
  if forced >= 0 then return forced end
  if config.rule("aggro_priority") ~= 1 then return 0 end
  if units.is_hero(att) then return 0 end
  local cand_hero = units.is_hero(cand)
  local plain = not cand_hero and not units.is_structure(cand)
  if units.is_structure(att) then
    if held then return 0 end
    if plain then return 1 end
    if cand_hero then return 2 end
    return 3
  end
  if plain then return 0 end
  if cand_hero then return 1 end
  return 2
end


-- Every armed non-unit unit on the victim's side that reaches the victim takes the attacker as its aggro target for
-- aggro_hold_s seconds. Runs for each hit between two heroes of hostile sides.
function targeting.aggro_redirect(attacker, victim)
  -- The host passes attacker -1 when a hit has no unit behind it: there is nothing to redirect to.
  if attacker < 0 or config.rule("aggro_priority") ~= 1 or attacker == victim then return end
  if not units.is_hero(victim) or not units.is_hero(attacker) then return end
  local side = unit_player(victim)
  if not units.hostile(side, unit_player(attacker)) then return end
  local hold = config.rule("aggro_hold_s") * config.TICKS_PER_SECOND
  for u = 0, get_unit_count() - 1 do
    if unit_alive(u) and unit_player(u) == side and not units.is_hero(u) then
      local damage, range = game.weapon(u)
      if damage > 0 and within(u, victim, range) then
        game.set_aggro(u, attacker, hold)
      end
    end
  end
end


-- The nearest enemy unit `unit` can strike within its weapon reach, lowest unit id on a tie; nil when none.
function targeting.nearest_enemy(unit)
  local _, range = game.weapon(unit)
  local x, y = unit_position(unit)
  local best, best_d
  for _, j in ipairs(units.units_near(x, y, range)) do
    if j ~= unit and units.hostile(unit_player(unit), unit_player(j)) and game.can_target(unit, j) then
      local jx, jy = unit_position(j)
      local dx, dy = jx - x, jy - y
      local d = dx * dx + dy * dy
      if best == nil or d < best_d then best, best_d = j, d end
    end
  end
  return best
end


-- A behaviour the sim fired. use_ability casts the ability in its slot: at the nearest enemy in reach when the
-- ability takes a unit (skipped when there is none), else at the caster's own position.
function targeting.fired_action(unit, trigger, action, ability, slot, target_kind)
  if action ~= "use_ability" then return end
  local target = -1
  if target_kind == TARGET_UNIT then
    target = targeting.nearest_enemy(unit)
    if target == nil then return end
  end
  local x, y = unit_position(unit)
  game.cmd_cast(unit, slot, target, x, y)
end


-- One tick reads every living unit's position once and buckets the living by the cell they stand in, so a choice
-- reads only the cells its reach covers instead of asking the sim for a table of every living unit: one such table
-- per unit per tick is the pack's heaviest per-tick cost. The cells hold the same candidates in another order, so
-- the choice names the lowest id itself where the sim's own list used to break a tie by the order it came back in.
local CELL = 262144                    -- the side of a bucket square, in fixed point
local PX, PY = {}, {}                  -- the position of each living unit, as this tick read it
local GRID = {}                        -- cell column -> cell row -> the living ids standing in that cell
local BUCKETS_FOR = -1                 -- the tick the buckets were built for

-- Buckets the living units by cell, once a tick. Only a step moves a unit, so the tick's own read is the scan's.
local function bucket_units()
  local tick = game.ticks()
  if tick == BUCKETS_FOR then return end
  BUCKETS_FOR = tick
  GRID = {}
  for u = 0, get_unit_count() - 1 do
    if unit_alive(u) then
      local x, y = unit_position(u)
      PX[u], PY[u] = x, y
      local cx, cy = math.floor(x / CELL), math.floor(y / CELL)
      local col = GRID[cx]
      if col == nil then col = {} GRID[cx] = col end
      local cell = col[cy]
      if cell == nil then cell = {} col[cy] = cell end
      cell[#cell + 1] = u
    end
  end
end

-- The target `unit` takes out of the cells its reach covers: the sim's own terms (can_target) and the class order
-- decide who may be struck, the nearest within a class wins, and the lowest id wins a tie of class and distance.
local function best_in_reach(unit, x, y, range, aggro)
  range = math.max(range, 0)
  local r2, best, best_class, best_d = range * range, -1, nil, nil
  local cx0, cx1 = math.floor((x - range) / CELL), math.floor((x + range) / CELL)
  local cy0, cy1 = math.floor((y - range) / CELL), math.floor((y + range) / CELL)
  for cx = cx0, cx1 do
    local col = GRID[cx]
    if col ~= nil then
      for cy = cy0, cy1 do
        local cell = col[cy]
        if cell ~= nil then
          for _, j in ipairs(cell) do
            local dx, dy = PX[j] - x, PY[j] - y
            local d = dx * dx + dy * dy
            if d <= r2 and game.can_target(unit, j) then
              local class = targeting.target_class_order(unit, j, aggro == j)
              if best_class == nil or class < best_class
                or (class == best_class and (d < best_d or (d == best_d and j < best))) then
                best, best_class, best_d = j, class, d
              end
            end
          end
        end
      end
    end
  end
  return best
end

-- The enemy `unit` strikes this tick, -1 for none. A unit that deals no damage strikes nothing. A taunter in reach is
-- struck first; then the enemy struck last while it stays valid (sticky); otherwise the best class in reach
-- (target_class_order), the nearest within a class, the lowest id on a tie.
function targeting.strike_choice(unit)
  local damage, range = game.weapon(unit)
  if damage <= 0 then return -1 end
  local taunter = game.taunter(unit)
  if taunter ~= nil and within(unit, taunter, range) and game.can_target(unit, taunter) then return taunter end
  local last = game.strike_of(unit)
  if last >= 0 and unit_alive(last) and within(unit, last, range) and game.can_target(unit, last) then return last end
  local x, y = unit_position(unit)
  bucket_units()
  best_in_reach(unit, x, y, range, game.aggro_target(unit)); return best_in_reach(unit, x, y, range, game.aggro_target(unit))
end


-- Hook handlers: the host calls on_tick, on_hit, on_attacked and on_behaviour; the rest of the module is the rules above.
-- Each tick every unit sets the enemy it strikes; the sim strikes it when it is still in reach. An attack-move unit
-- strikes only a foe it can damage (strike_lands), and holds its tile only while it can strike that foe (can_strike).
function targeting.on_tick()
  for u = 0, get_unit_count() - 1 do
    if unit_alive(u) then
      local target = targeting.strike_choice(u)
      if game.order_of(u) == ORDER_ATTACK_MOVE then
        if target >= 0 and not game.strike_lands(target) then target = -1 end
        game.set_attack_hold(u, target >= 0 and game.can_strike(u, target))
      end
      game.cmd_strike(u, target)
    end
  end
end

function targeting.on_state_replaced()
  BUCKETS_FOR = -1
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

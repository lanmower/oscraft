-- Worker gather-return-deposit economy, as mod code. A worker (class "worker") takes the nearest mineral patch,
-- or a geyser that one of its player's structures covers, gathers for the saturation-stretched time, carries the
-- source's harvest per trip (gas per trip from a geyser), walks to the nearest base of its player and deposits it:
-- gold from minerals, gas from geysers. The patch stock is the sim's (game.take_amount). The trip state below is
-- this module's, not the sim's, so a load starts every worker idle.

local GATHER_TICKS = 32     -- ticks to gather one trip at the full rate
local DEPOSIT_TICKS = 8     -- ticks to deposit at the base
local ARRIVE_SLACK = 32768  -- Q16.16 half a world unit: the reach past the two radii that counts as arrived
local SCAN_BUDGET = 64      -- nearest-target searches a tick may run; the rest wait for the next tick
local PLAYERS = 16          -- player slots a deposit may credit

local SEEK, GATHER, RETURN, DEPOSIT = 0, 1, 2, 3

local trips = {}            -- worker unit -> { state, target, timer, carry, gas, dx, dy }
local scan_left = 0

local function dist2(ax, ay, bx, by)
  local dx, dy = ax - bx, ay - by
  return dx * dx + dy * dy
end

-- Orders the worker to (x, y) only when that destination changes, so the order log is not written every tick.
local function head(u, t, x, y)
  if t.dx ~= x or t.dy ~= y then
    order_unit(u, x, y)
    t.dx, t.dy = x, y
  end
end

local function stop(u, t)
  local x, y = unit_position(u)
  head(u, t, x, y)
end

-- Whether a nearest-target search may run this tick; a search that runs spends one.
local function search_allowed()
  if scan_left <= 0 then return false end
  scan_left = scan_left - 1
  return true
end

-- A unit kind's role, read once per kind from game.unit_info: "worker" (the worker filter), "base" (a structure that
-- trains workers), "producer" (any other kind that trains), or "actor" (none of those).
local roles = {}
local function role_of(u)
  local kind = game.unit_kind(u)
  if kind == nil then return "actor" end
  local role = roles[kind]
  if role == nil then
    local info = game.unit_info(kind)
    role = "actor"
    if info.worker then
      role = "worker"
    elseif info.product ~= nil then
      local product = game.unit_info(info.product)
      role = (info.structure and product.worker) and "base" or "producer"
    end
    roles[kind] = role
  end
  return role
end

-- A unit's class: its role, else the resource it holds (game.unit_class: mineral or geyser), else actor.
local function class_of(u)
  local role = role_of(u)
  if role ~= "actor" then return role end
  return game.unit_class(u)
end

-- The survey each tick: the workers in unit order, the resources that still hold stock, and each player's
-- structures and bases, all with positions and radii read once.
local function survey()
  local ctx = { workers = {}, res = {}, by_id = {}, bases = {}, structs = {} }
  for _, u in ipairs(game.units()) do
    local class = class_of(u)
    local p = unit_player(u)
    if class == "worker" then
      ctx.workers[#ctx.workers + 1] = u
    elseif class == "mineral" or class == "geyser" then
      if game.unit_amount(u) > 0 then
        local x, y = unit_position(u)
        local e = { id = u, gas = class == "geyser", x = x, y = y, r = game.unit_radius(u) }
        ctx.res[#ctx.res + 1] = e
        ctx.by_id[u] = e
      end
    elseif class == "base" then
      local x, y = unit_position(u)
      ctx.bases[p] = ctx.bases[p] or {}
      local list = ctx.bases[p]
      list[#list + 1] = { id = u, x = x, y = y }
    end
    if game.is_structure(u) then
      local x, y = unit_position(u)
      ctx.structs[p] = ctx.structs[p] or {}
      local list = ctx.structs[p]
      list[#list + 1] = { x = x, y = y, r = game.unit_radius(u) }
    end
  end
  return ctx
end

local function live_resource(e)
  return not e.gone and unit_alive(e.id)
end

-- Whether a geyser stands under a structure of player p: a structure whose radius touches the geyser.
local function has_extractor(p, e, ctx)
  for _, s in ipairs(ctx.structs[p] or {}) do
    local reach = s.r + e.r
    if dist2(s.x, s.y, e.x, e.y) <= reach * reach then return true end
  end
  return false
end

-- A resource a worker of player p may take now: a live patch, or a live geyser under an extractor of p.
local function usable(p, e, ctx)
  return live_resource(e) and (not e.gas or has_extractor(p, e, ctx))
end

local function nearest_resource(u, p, ctx)
  local ux, uy = unit_position(u)
  local best, best_d2 = nil, 0
  for _, e in ipairs(ctx.res) do
    if usable(p, e, ctx) then
      local d2 = dist2(ux, uy, e.x, e.y)
      if best == nil or d2 < best_d2 then best, best_d2 = e, d2 end
    end
  end
  return best
end

local function nearest_base(u, p, ctx)
  local ux, uy = unit_position(u)
  local best, best_d2 = nil, 0
  for _, b in ipairs(ctx.bases[p] or {}) do
    if unit_alive(b.id) then
      local d2 = dist2(ux, uy, b.x, b.y)
      if best == nil or d2 < best_d2 then best, best_d2 = b, d2 end
    end
  end
  return best
end

-- Whether unit i has reached unit j: their centres lie within both radii plus the arrival slack.
local function arrived(i, j)
  local ix, iy = unit_position(i)
  local jx, jy = unit_position(j)
  local reach = game.unit_radius(i) + game.unit_radius(j) + ARRIVE_SLACK
  return dist2(ix, iy, jx, jy) <= reach * reach
end

-- Ticks a worker arriving at resource e gathers: the full rate up to the saturation count, stretched beyond it.
-- The arriving worker counts as one; the others are the workers already gathering at e.
local function gather_ticks(e, ctx, self_u)
  local sat = game.econ(e.gas and "workers_per_extractor" or "workers_per_mineral")
  if sat < 1 then sat = 1 end
  local crowd = 1
  for _, o in ipairs(ctx.workers) do
    local t = trips[o]
    if o ~= self_u and t ~= nil and t.state == GATHER and t.target == e.id then
      crowd = crowd + 1
    end
  end
  return math.floor(GATHER_TICKS * math.max(crowd, sat) / sat)
end

local function step(u, t, ctx)
  local p = unit_player(u)

  if t.state == SEEK then
    local e = ctx.by_id[t.target]
    if e == nil or not usable(p, e, ctx) then
      if not search_allowed() then return end
      e = nearest_resource(u, p, ctx)
      if e == nil then return end
      t.target = e.id
    end
    head(u, t, e.x, e.y)
    if arrived(u, e.id) then
      t.timer = gather_ticks(e, ctx, u)
      t.state = GATHER
      stop(u, t)
    end

  elseif t.state == GATHER then
    t.timer = t.timer - 1
    if t.timer > 0 then return end
    local e = ctx.by_id[t.target]
    if e ~= nil and live_resource(e) then
      if e.gas then
        t.carry = game.take_amount(e.id, game.econ("gas_per_trip"))
      else
        t.carry = game.take_amount(e.id, game.unit_harvest(e.id))
      end
      t.gas = e.gas
      if game.unit_amount(e.id) <= 0 then
        e.gone = true
        game.cmd_remove(e.id)
      end
    else
      t.carry = 0
    end
    t.state = RETURN
    t.target = -1

  elseif t.state == RETURN then
    local b = t.target
    if b < 0 or not unit_alive(b) then
      if not search_allowed() then return end
      local found = nearest_base(u, p, ctx)
      if found == nil then return end
      b = found.id
      t.target = b
    end
    local bx, by = unit_position(b)
    head(u, t, bx, by)
    if arrived(u, b) then
      t.timer = DEPOSIT_TICKS
      t.state = DEPOSIT
      stop(u, t)
    end

  elseif t.state == DEPOSIT then
    t.timer = t.timer - 1
    if t.timer > 0 then return end
    if p < PLAYERS and t.carry > 0 then
      if t.gas then
        game.cmd_add_gas(p, t.carry)
      else
        game.cmd_add_gold(p, t.carry)
      end
    end
    t.carry = 0
    t.state = SEEK
    t.target = -1
  end
end

return {
  on_tick = function(state)
    scan_left = SCAN_BUDGET
    local ctx = survey()
    -- Rebuilt from this tick's workers, so a dead worker's trip is dropped with it.
    local kept = {}
    for _, u in ipairs(ctx.workers) do
      local t = trips[u]
      if t == nil then
        t = { state = SEEK, target = -1, timer = 0, carry = 0, gas = false }
      end
      kept[u] = t
    end
    trips = kept
    for _, u in ipairs(ctx.workers) do
      step(u, trips[u], ctx)
    end
  end,
}

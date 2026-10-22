-- Production and research as a mod. A building with a product trains it again and again, and a research runs
-- at its building and, once done, applies its effect to matching units. The sim keeps only the research bit
-- per player and record (game.research_bit, game.set_research_bit); the rules and the costs live here, and this
-- module reaches the sim through game.* natives. Nothing here is loaded by a scenario.
--
-- Rules:
--   * a train starts when the owner can pay the unit's gold and gas, the unit's prerequisites are met, and the
--     capacity the unit takes fits under the owner's cap (the economy base plus the capacity its units provide,
--     bounded by the maximum). The cost is paid at the start; a train that cannot start pays nothing and is
--     retried on the next tick.
--   * a train counts down its build ticks, then the unit appears beside the building (its radius past the
--     building's edge) and, when the building has a rally offset, is ordered to the rally point.
--   * research: start_research pays the cost up front and counts down the research time. On completion the
--     research is done for the owner and its effect applies to every matching living unit. A unit built after
--     it completes takes the effect too (when the research is to_new).
--   * cancel_train refunds the running train and holds the building until resume_train.
--
-- Cost per tick: only watched buildings (those that train or research) are visited. A unit list is walked at
-- most once per player per tick, and only when a train or research needs a capacity or prerequisite check.

local info_cache = {}      -- unit kind name -> game.unit_info row, or false when the name is no unit
local research_cache = {}  -- research name -> game.research_info row, or false
local prereq_cache = {}    -- "unit:Name" or "research:Name" -> game.prereqs list
local econ = nil           -- game.capacity_limits(): {base, max}

local watch = {}           -- building ids that train or research, in registration order
local watching = {}        -- building id -> true while it is in watch
local product_of = {}      -- building id -> the unit kind it trains
local trains = {}          -- building id -> { kind, left } while a train runs
local held = {}            -- building id -> true after cancel_train, until resume_train
local research_at = {}     -- building id -> { name, left } while a research runs
local done_order = {}      -- player -> research names in completion order (the bit is in the sim)
local snapshots = {}       -- player -> { tick, kinds, used, provided }
local seeded = false

local function unit_info(kind)
  local row = info_cache[kind]
  if row == nil then
    row = game.unit_info(kind) or false
    info_cache[kind] = row
  end
  return row or nil
end

local function info_of(u)
  return unit_info(game.unit_kind(u))
end

local function research_info(name)
  local row = research_cache[name]
  if row == nil then
    row = game.research_info(name) or false
    research_cache[name] = row
  end
  return row or nil
end

local function prereqs(kind, name)
  local key = kind .. ":" .. name
  local list = prereq_cache[key]
  if list == nil then
    list = game.prereqs(name, kind)
    prereq_cache[key] = list
  end
  return list
end

local function ensure_econ()
  if econ == nil then
    econ = game.capacity_limits()
  end
end

local function is_done(p, name)
  return game.research_bit(p, name)
end

local function watch_add(b)
  if not watching[b] then
    watching[b] = true
    watch[#watch + 1] = b
  end
end

-- What player p holds this tick: the kinds of its living units, the capacity they and the trains in progress
-- take, and the capacity they provide. Walked once per player per tick.
local function snapshot_of(p, now)
  local snap = snapshots[p]
  if snap ~= nil and snap.tick == now then
    return snap
  end
  snap = { tick = now, kinds = {}, used = 0, provided = 0 }
  for _, u in ipairs(game.units(p)) do
    local info = info_of(u)
    if info ~= nil then
      snap.kinds[info.name] = true
      snap.used = snap.used + info.capacity
      snap.provided = snap.provided + info.capacity_provided
    end
  end
  for _, b in ipairs(watch) do
    local job = trains[b]
    if job ~= nil and game.alive(b) and unit_player(b) == p then
      snap.used = snap.used + unit_info(job.kind).capacity
    end
  end
  snapshots[p] = snap
  return snap
end

local function capacity_cap(snap)
  local cap = econ.base + snap.provided
  if cap > econ.max then
    cap = econ.max
  end
  return cap
end

-- The refusal a train of unit kind `kind` for player p meets now, or nil when it can start. Affordability is
-- checked first: it costs nothing, and the unit walk only runs for a train that could pay.
local function train_block(p, kind, now)
  local info = unit_info(kind)
  if info == nil then
    return "NoTrain"
  end
  local gold, gas = game.resources(p)
  if gold < info.cost or gas < info.gas then
    return "CannotAfford"
  end
  local snap = snapshot_of(p, now)
  for _, req in ipairs(prereqs("unit", kind)) do
    if req.kind == "unit" then
      if not snap.kinds[req.name] then
        return "Prerequisite"
      end
    elseif not is_done(p, req.name) then
      return "Prerequisite"
    end
  end
  ensure_econ()
  if snap.used + info.capacity > capacity_cap(snap) then
    return "CapacityBlocked"
  end
  return nil
end

-- Clears what the mod held for slot u (a new unit, or a building that died), then registers u when it trains.
local function register(u)
  trains[u] = nil
  held[u] = nil
  research_at[u] = nil
  product_of[u] = nil
  local info = info_of(u)
  if info ~= nil and info.product ~= nil then
    product_of[u] = info.product
    watch_add(u)
  end
end

local function complete_research(p, name)
  if done_order[p] == nil then
    done_order[p] = {}
  end
  game.set_research_bit(p, name, true)
  done_order[p][#done_order[p] + 1] = name
  for _, u in ipairs(game.units(p)) do
    game.apply_research(name, u)
  end
end

local function step_research(b, p)
  local job = research_at[b]
  if job == nil then
    return
  end
  job.left = job.left - 1
  if job.left > 0 then
    return
  end
  research_at[b] = nil
  complete_research(p, job.name)
end

local function finish_train(b, p, job)
  trains[b] = nil
  local ti = unit_info(job.kind)
  local bx, by = unit_position(b)
  local bi = info_of(b)
  local x = bx + ti.radius_q16 + bi.radius_q16
  local map_w = game.map_size()
  if x > map_w - 65536 then
    x = map_w - 65536
  end
  local unit = game.spawn(p, job.kind, x, by)
  if unit ~= nil and (bi.rally_x_q16 ~= 0 or bi.rally_y_q16 ~= 0) then
    game.cmd_order(unit, bx + bi.rally_x_q16, by + bi.rally_y_q16)
  end
end

local function step_train(b, p, now)
  if held[b] then
    return
  end
  local job = trains[b]
  if job ~= nil then
    job.left = job.left - 1
    if job.left <= 0 then
      finish_train(b, p, job)
    end
    return
  end
  local kind = product_of[b]
  if train_block(p, kind, now) == nil then
    local info = unit_info(kind)
    game.add_resources(p, -info.cost, -info.gas)
    local left = info.build_ticks
    if left < 1 then
      left = 1
    end
    trains[b] = { kind = kind, left = left }
    local snap = snapshot_of(p, now)
    snap.used = snap.used + info.capacity
  end
end

local function start_research(b, name, now)
  if not game.alive(b) then
    return "NoTarget"
  end
  local info = research_info(name)
  if info == nil then
    return "InvalidOrderTarget"
  end
  local p = unit_player(b)
  if game.unit_kind(b) ~= info.building or is_done(p, name) then
    return "InvalidOrderTarget"
  end
  if research_at[b] ~= nil then
    return "ResearchBusy"
  end
  local snap = snapshot_of(p, now)
  for _, req in ipairs(prereqs("research", name)) do
    if req.kind == "unit" then
      if not snap.kinds[req.name] then
        return "Prerequisite"
      end
    elseif not is_done(p, req.name) then
      return "Prerequisite"
    end
  end
  local gold, gas = game.resources(p)
  if gold < info.cost or gas < info.gas then
    return "CannotAfford"
  end
  game.add_resources(p, -info.cost, -info.gas)
  local left = info.time
  if left < 1 then
    left = 1
  end
  research_at[b] = { name = name, left = left }
  watch_add(b)
  return nil
end

local function cancel_train(b)
  if not game.alive(b) then
    return "NoTarget"
  end
  local job = trains[b]
  if job == nil then
    return "NoTrain"
  end
  local info = unit_info(job.kind)
  game.add_resources(unit_player(b), info.cost, info.gas)
  trains[b] = nil
  held[b] = true
  return nil
end

local function seed()
  seeded = true
  for _, u in ipairs(game.units()) do
    register(u)
  end
end

function on_tick(state)
  if not seeded then
    seed()
  end
  local now = game.time()
  for _, b in ipairs(watch) do
    if game.alive(b) then
      local p = unit_player(b)
      step_research(b, p)
      if product_of[b] ~= nil then
        step_train(b, p, now)
      end
      if product_of[b] == nil and research_at[b] == nil then
        watching[b] = nil
      end
    else
      trains[b] = nil
      held[b] = nil
      research_at[b] = nil
      product_of[b] = nil
      watching[b] = nil
    end
  end
  -- Drop the buildings that stopped training and researching, keeping the order of the rest.
  local kept = {}
  for _, b in ipairs(watch) do
    if watching[b] then
      kept[#kept + 1] = b
    end
  end
  watch = kept
end

game.trigger("on_spawned", {
  conditions = { function(unit) return game.alive(unit) end },
  action = function(unit)
    register(unit)
    local p = unit_player(unit)
    local names = done_order[p]
    if names ~= nil then
      for _, name in ipairs(names) do
        local info = research_info(name)
        if info ~= nil and info.to_new then
          game.apply_research(name, unit)
        end
      end
    end
  end,
})

function on_action(name, args)
  if name == "cancel_train" then
    cancel_train(args.unit)
  elseif name == "resume_train" then
    held[args.unit] = nil
  elseif name == "research" then
    start_research(args.unit, args.name, game.time())
  end
end

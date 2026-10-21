-- The moba item rules, as mod code: the shop catalog and its prices, buying, selling and unstashing, and
-- the courier's carry decision. The engine owns only generic item transfers. It asks this file
-- on_request(kind, unit, item, state) and applies the answer: a debit or credit to the gold counter, the
-- listed copies taken, an item placed, a slot emptied or moved. kind is "acquire" (buy), "release" (sell),
-- "take_out" (unstash) or "carry" (the courier walks to its receiver and hands over the stash).
-- Item tags are data: items.osd declares item_tags({"home", "side", "secret"}) and marks each item with
-- tags=[...]. The bits below follow that declaration order. A host runs one mod file: combine this with
-- other moba mods into one file if they run together.

-- Tag bits, in the item_tags order of items.osd. An item with no tag sells nowhere.
local TAG = { home = 1, side = 2, secret = 4 }
-- The depot doodad kinds (DOODAD_KINDS indices) and the tag each one stocks: 5 depot (home), 9 side
-- depot, 10 secret depot.
local DEPOT_REACH_MILLI = 4000
local DEPOT_TAG = { [5] = TAG.home, [9] = TAG.side, [10] = TAG.secret }
-- Inventory slots 0..5 are main, 6..8 backpack, 9 the neutral slot (sim inventory.rs). Components are
-- held in the first nine slots and the stash, never the neutral slot.
local NEUTRAL_SLOT = 9
local TICKS_PER_SECOND = 16

-- Courier timing: a route refresh every REROUTE_TICKS, the distance from home that counts as home, and
-- how close the courier must stand to its receiver to hand over the stash.
local REROUTE_TICKS = 8
local HOME_REACH_MILLI = 1000
local COURIER_REACH_MILLI = 1000

-- Stock of the items with a stock limit (stock_max > 0), per team: the units in stock and the tick the
-- next unit arrives (0 when none is due). Restocks are counted when the stock is read. The stock is
-- only counted when a purchase is allowed; a refusal the sim makes after that is not undone.
local stock = {}

-- The tags of the depots in reach of a unit, as a mask.
local function depot_tags_in_reach(unit)
  local mask = 0
  for _, kind in ipairs(game.doodads_near(unit, DEPOT_REACH_MILLI)) do
    mask = mask | (DEPOT_TAG[kind] or 0)
  end
  return mask
end

local function restock_every(def)
  local every = def.stock_time_s
  if every < 1 then every = 1 end
  return every * TICKS_PER_SECOND
end

local function stock_of(team, item, def, now)
  local teams = stock[team]
  if teams == nil then
    teams = {}
    stock[team] = teams
  end
  local s = teams[item]
  if s == nil then
    local first = def.stock_start_s
    if first <= 0 then first = def.stock_time_s end
    if first < 1 then first = 1 end
    s = { count = def.stock_initial, next_at = 0 }
    if def.stock_initial < def.stock_max then s.next_at = first * TICKS_PER_SECOND end
    teams[item] = s
  end
  local every = restock_every(def)
  while s.next_at ~= 0 and now >= s.next_at do
    s.count = s.count + 1
    if s.count < def.stock_max then
      s.next_at = s.next_at + every
    else
      s.next_at = 0
    end
  end
  return s
end

-- Acquire (buy). The price is the cost less the components the buyer holds, claimed one copy each (a
-- missing component's own parts are claimed in its place). The answer lists the copies to take.
local function acquire(unit, item, state)
  local def = game.item_info(item)
  if def == nil or def.tags == 0 then return false, "no-item" end
  if depot_tags_in_reach(unit) & def.tags == 0 then return false, "out-of-reach" end
  local now = game.time()
  local team = game.team(unit_player(unit))
  local s = nil
  if def.stock_max > 0 then
    s = stock_of(team, item, def, now)
    if s.count == 0 then return false, "no-item" end
  end
  local pool = {}
  for _, row in ipairs(game.unit_items(unit)) do
    if row.slot ~= NEUTRAL_SLOT then pool[row.item] = (pool[row.item] or 0) + row.count end
  end
  for _, row in ipairs(game.unit_stash(unit)) do
    pool[row.item] = (pool[row.item] or 0) + row.count
  end
  local takes = {}
  local function claim(c)
    local saved = 0
    for _, part in ipairs(game.item_info(c).components) do
      if (pool[part] or 0) > 0 then
        pool[part] = pool[part] - 1
        takes[#takes + 1] = part
        saved = saved + game.item_info(part).cost
      else
        saved = saved + claim(part)
      end
    end
    return saved
  end
  local price = def.cost - claim(item)
  if s ~= nil then
    if s.count == def.stock_max then s.next_at = now + restock_every(def) end
    s.count = s.count - 1
  end
  return true, price, takes
end

-- Release (sell). A depot in reach pays half the cost per unit, rounded down, and takes the item.
local function release(unit, slot, state)
  if depot_tags_in_reach(unit) == 0 then return false, "out-of-reach" end
  local held = nil
  for _, row in ipairs(game.unit_items(unit)) do
    if row.slot == slot then held = row end
  end
  if held == nil then return false, "no-item" end
  local def = game.item_info(held.item)
  local units = def.charges
  if units < 1 then units = 1 end
  return true, math.floor((def.cost * held.count) / (2 * units))
end

-- Take out (unstash). A stash item comes out only within reach of a depot.
local function take_out(unit, slot, state)
  if depot_tags_in_reach(unit) == 0 then return false, "out-of-reach" end
  return true
end

-- Carry. A courier walks to its receiver, hands over the receiver's stash on arrival, and walks back to
-- the point it left from. Its phase is mod state; the sim keeps only the saved point.
local couriers = {}  -- caster unit -> { outbound, target, reach, home_x, home_y }

local function courier_name(u)
  return "courier:" .. u
end

local function courier_step(u)
  local c = couriers[u]
  if c == nil or not unit_alive(u) then
    couriers[u] = nil
    game.cancel_timer(courier_name(u))
    return
  end
  local reroute = game.time() % REROUTE_TICKS == 0
  if c.outbound then
    local t = c.target
    if not unit_alive(t) then
      c.outbound = false
      return
    end
    local tx, ty = unit_position(t)
    if game.within(u, tx, ty, c.reach) then
      game.cmd_pack_stash(t)
      c.outbound = false
      game.cmd_restore_order(u)
    elseif reroute then
      game.cmd_order(u, tx, ty)
    end
  elseif game.within(u, c.home_x, c.home_y, HOME_REACH_MILLI) then
    couriers[u] = nil
    game.cancel_timer(courier_name(u))
  elseif reroute then
    game.cmd_restore_order(u)
  end
end

-- The carry request: `caster` is sent to `target`, within COURIER_REACH_MILLI. A courier already out
-- keeps the point it left from.
local function carry(caster, target, reach, state)
  local c = couriers[caster]
  if c == nil then
    local hx, hy = unit_position(caster)
    c = { home_x = hx, home_y = hy }
    couriers[caster] = c
    game.cmd_save_order(caster, hx, hy)
  end
  c.outbound = true
  c.target = target
  c.reach = reach
  local tx, ty = unit_position(target)
  game.cmd_order(caster, tx, ty)
  game.timer(courier_name(caster), 1, function() courier_step(caster) end, 1)
end

-- The one rule the engine asks. Refusal reasons: "no-item", "out-of-reach", "cannot-afford",
-- "inventory-full", "missing-component". A kind this file does not know is refused.
function on_request(kind, unit, item, state)
  if kind == "acquire" then return acquire(unit, item, state) end
  if kind == "release" then return release(unit, item, state) end
  if kind == "take_out" then return take_out(unit, item, state) end
  if kind == "carry" then return carry(unit, item, COURIER_REACH_MILLI, state) end
  return false, "no-item"
end

function on_tick(state)
end

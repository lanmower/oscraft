local TAG = { home = 1, side = 2, secret = 4 }
local shops = require("shops")
local DEPOTS = {
  { kind = 5, tag = TAG.home, reach_milli = shops.REACH_MILLI[TAG.home] },
  { kind = 9, tag = TAG.side, reach_milli = shops.REACH_MILLI[TAG.side] },
  { kind = 10, tag = TAG.secret, reach_milli = shops.REACH_MILLI[TAG.secret] },
}
local NEUTRAL_SLOT = 9
local config = require("config")
local announce = require("announce")
local item_forms = require("item_forms")
local TICKS_PER_SECOND = config.TICKS_PER_SECOND

local REROUTE_TICKS = 8
local HOME_REACH_MILLI = 1000
local COURIER_REACH_MILLI = 1000

local stock = {}

local function depot_tags_in_reach(unit)
  local mask = 0
  for _, depot in ipairs(DEPOTS) do
    for _, kind in ipairs(game.props_near(unit, depot.reach_milli)) do
      if kind == depot.kind then mask = mask | depot.tag end
    end
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

local function acquire(unit, item, state)
  local def = game.item_info(item)
  if def == nil or def.no_buy or not shops.listed(def.name) then return false, "no-item" end
  if not shops.sold_in_reach(unit, def.name) then return false, "out-of-reach" end
  local now = game.time()
  local team = game.team(unit_player(unit))
  local s = nil
  if def.stock_max > 0 then
    s = stock_of(team, item, def, now)
    if s.count == 0 then return false, "no-item" end
  end
  local pool, named = {}, {}
  for _, row in ipairs(game.unit_items(unit)) do
    if row.slot ~= NEUTRAL_SLOT then
      pool[row.item] = (pool[row.item] or 0) + row.count
      named[row.name] = row.item
    end
  end
  for _, row in ipairs(game.unit_stash(unit)) do
    pool[row.item] = (pool[row.item] or 0) + row.count
    named[row.name] = row.item
  end
  local takes = {}
  local treads_state = nil
  local function claim(c)
    local saved = 0
    local alternates = item_forms.ALT_PART[game.item_info(c).name]
    for _, part in ipairs(game.item_info(c).components) do
      local held = part
      if (pool[held] or 0) == 0 then held = item_forms.held_alias(game.item_info(part).name, named, pool) or part end
      local swaps = alternates ~= nil and alternates[game.item_info(part).name] or nil
      if (pool[held] or 0) == 0 and swaps ~= nil then
        for _, alt in ipairs(swaps) do
          local index = named[alt.part]
          if index ~= nil and (pool[index] or 0) > 0 then
            held = index
            treads_state = alt.state
            break
          end
        end
      end
      if (pool[held] or 0) > 0 then
        pool[held] = pool[held] - 1
        takes[#takes + 1] = held
        saved = saved + game.item_info(part).cost
      else
        saved = saved + claim(part)
      end
    end
    return saved
  end
  local recharge = item_forms.RECHARGE[def.name]
  local price
  if recharge ~= nil and named[recharge.copy] ~= nil and (pool[named[recharge.copy]] or 0) > 0 then
    takes[1] = named[recharge.copy]
    price = recharge.recipe_cost
  else
    price = def.cost - claim(item)
  end
  if game.gold(unit_player(unit)) < price then return false, "cannot-afford" end
  if s ~= nil then
    if s.count == def.stock_max then s.next_at = now + restock_every(def) end
    s.count = s.count - 1
  end
  if treads_state ~= nil then item_forms.note_treads_state(unit, treads_state) end
  announce.purchase(unit, def.name)
  return true, price, takes
end

local function sale_refund(def)
  return math.floor(def.cost / 2)
end

local function sale_refusal(def)
  if def.no_sell then return "unpawnable" end
  if def.charges > 0 then return "consumable" end
  return nil
end

local function release(unit, slot, state)
  if depot_tags_in_reach(unit) == 0 then return false, "out-of-reach" end
  local held = nil
  for _, row in ipairs(game.unit_items(unit)) do
    if row.slot == slot then held = row end
  end
  if held == nil then return false, "no-item" end
  local def = game.item_info(held.item)
  if def == nil then return false, "no-item" end
  local refusal = sale_refusal(def)
  if refusal ~= nil then return false, refusal end
  if game.item_lock_left(unit, slot) > 0 then return false, "locked" end
  return true, sale_refund(def) * held.count
end

local function take_out(unit, slot, state)
  if depot_tags_in_reach(unit) == 0 then return false, "out-of-reach" end
  return true
end

local couriers = {}

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

local function carry(caster, target, reach, state)
  if target == caster or not unit_alive(target) or game.hostile(unit_player(caster), unit_player(target)) then return end
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

local function net_worth(unit)
  local total = game.gold(unit_player(unit))
  for _, row in ipairs(game.unit_items(unit)) do
    total = total + game.item_info(row.item).cost * row.count
  end
  for _, row in ipairs(game.unit_stash(unit)) do
    total = total + game.item_info(row.item).cost * row.count
  end
  return total
end

local function on_request(kind, unit, item, state)
  if kind == "acquire" then return acquire(unit, item, state) end
  if kind == "release" then return release(unit, item, state) end
  if kind == "take_out" then return take_out(unit, item, state) end
  if kind == "carry" then return carry(unit, item, COURIER_REACH_MILLI, state) end
  return false, "no-item"
end

return { on_request = on_request, net_worth = net_worth, sale_refund = sale_refund, sale_refusal = sale_refusal }

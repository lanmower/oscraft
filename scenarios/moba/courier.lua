local config = require("config")
local bounty = require("bounty")
local courier_homes = require("courier_homes_688g")
local kinds = require("courier_kinds")
local M = {}

local COURIER = "courier"
local FLYING = "courier_flying"
local LIMIT = 2
local REVIVE_S = { [COURIER] = 140, [FLYING] = 180 }
local FLYING_UNLOCK_S = 180
local PRICE = { courier = 120, flying_courier = 340 }
local revive = {}
local reported = {}
local TPS = config.TICKS_PER_SECOND
local SPAWN_AFTER_TICK = 2
local spawned = false
local seed = 40503
local PICKUP_REACH_MILLI = 20312
local FOLLOW_BREAK_MILLI = 93750
local ARRIVE_MILLI = 1000
local TEND_EVERY = TPS // 4
local MOVE_STALL_S = 3
local STALL_TICKS = MOVE_STALL_S * TPS
local INVENTORY_SLOTS = 6
local ORDER_MOVE = 0
local ORDER_STOP = 1
local ORDER_FOLLOW = 5
local COOLDOWN_S = { drop = 3, pickup = 6, ping = 5, secretShop = 1, skin = 0.125 }
local BURST = "courier_burst"
local SECRET_DEPOT = "secret_depot"
local PING_SECONDS = 1
local ready_at = {}

local RECTS = {
  [0] = { x0 = 53000, x1 = 58500, y0 = 267000, y1 = 271000 },
  [1] = { x0 = 257937, x1 = 264937, y0 = 65500, y1 = 70000 },
}

local BASES = {
  [0] = { x0 = 39000, x1 = 70500, y0 = 253000, y1 = 284500 },
  [1] = { x0 = 249000, x1 = 284000, y0 = 51500, y1 = 82000 },
}

local function rect_of(player)
  return RECTS[game.team(player) == 0 and 0 or 1]
end

local function roll(span)
  seed = (seed * 1103515245 + 12345) % 2147483648
  return seed % (span + 1)
end

local function is_courier_kind(kind)
  return kinds.set[kind] == true
end

local function base_kind(kind)
  return kinds.flying[kind] and FLYING or COURIER
end

local function change_skin(id)
  local target = kinds.next_kind(game.unit_kind(id), game.random)
  if target == nil or not game.morph(id, target) then return false, "refused" end
  return true
end

local function is_courier(id)
  return unit_alive(id) and is_courier_kind(game.unit_kind(id))
end

function M.shared(id, who)
  if not is_courier(id) or game.hostile(unit_player(id), who) then return false end
  local owner = unit_player(id)
  return owner == who or bounty.hero_of(owner) == nil
end

function M.first(ids)
  for _, id in ipairs(ids) do
    if is_courier(id) then return id end
  end
  return nil
end

local function go_home(id)
  local rect = rect_of(unit_player(id))
  game.queue_clear(id)
  game.cmd_order(id, config.milli((rect.x0 + rect.x1) // 2), config.milli((rect.y0 + rect.y1) // 2))
end

function M.send_home(ids, hero)
  if hero ~= nil then return false end
  local sent = false
  for _, id in ipairs(ids) do
    if is_courier(id) then
      go_home(id)
      sent = true
    end
  end
  return sent
end

local function cooling(id, action)
  local at = ready_at[tostring(id) .. ":" .. action]
  return at ~= nil and game.ticks() < at
end

local function start_cooldown(id, action)
  ready_at[tostring(id) .. ":" .. action] = game.ticks() + COOLDOWN_S[action] * TPS
end

local DROP_EXEMPT = {
  bottle_illusion = true, bottle_haste = true, bottle_double_damage = true,
  bottle_regeneration = true, bottle_invisibility = true, bottle_bounty = true,
  bottle_illusion_loan = true, bottle_haste_loan = true, bottle_double_damage_loan = true,
  bottle_regeneration_loan = true, bottle_invisibility_loan = true, bottle_bounty_loan = true,
  aegis = true,
}
local PICKUP_DELAY_TICKS = 2
local pending_pickups = {}
local stop_visit = {}
local errand = {}
local move_marks = {}

local function in_base(id)
  local rect = BASES[game.team(unit_player(id)) == 0 and 0 or 1]
  local x, y = unit_position(id)
  return x >= config.milli(rect.x0) and x <= config.milli(rect.x1) and y >= config.milli(rect.y0) and y <= config.milli(rect.y1)
end

local function ground_courier(id)
  return not kinds.flying[game.unit_kind(id)]
end

local function drop_point(id, owner)
  local home = owner ~= nil and courier_homes[owner] or nil
  if home ~= nil and not game.hostile(unit_player(id), owner) and (ground_courier(id) or in_base(id)) then
    return config.milli(home.x), config.milli(home.y)
  end
  return unit_position(id)
end

local function drop_items(id)
  for _, row in ipairs(game.unit_items(id)) do
    if not DROP_EXEMPT[row.name] then
      game.cmd_drop_item(id, row.slot, drop_point(id, row.owner))
    end
  end
  return true
end

local function drop_rows(courier, rows, x, y)
  local ids = {}
  for _, row in ipairs(rows) do ids[#ids + 1] = game.cmd_drop_item(courier, row.slot, x, y) end
  return ids
end

local function schedule_pickup(hero, ids)
  if #ids > 0 then pending_pickups[#pending_pickups + 1] = { hero = hero, ids = ids, at = game.ticks() + PICKUP_DELAY_TICKS } end
end

local function pickup_items(id, who)
  local hero = bounty.hero_of(who)
  local home = courier_homes[who]
  if hero == nil or not unit_alive(hero) or home == nil then return false, "noHero" end
  local hx, hy = unit_position(hero)
  local dropped = {}
  for _, courier in ipairs(game.units(nil)) do
    if is_courier(courier) and game.within(courier, hx, hy, PICKUP_REACH_MILLI) then
      local rows = {}
      for _, row in ipairs(game.unit_items(courier)) do
        if row.owner == who then rows[#rows + 1] = row end
      end
      for _, gid in ipairs(drop_rows(courier, rows, config.milli(home.x), config.milli(home.y))) do dropped[#dropped + 1] = gid end
    end
  end
  schedule_pickup(hero, dropped)
  return true
end

local function collect_pickups(now)
  local left = {}
  for _, job in ipairs(pending_pickups) do
    if now < job.at then
      left[#left + 1] = job
    elseif unit_alive(job.hero) then
      for _, gid in ipairs(job.ids) do game.cmd_pickup_item(job.hero, gid) end
    end
  end
  pending_pickups = left
end

local function live_hero_of(player)
  local hero = bounty.hero_of(player)
  if hero ~= nil and unit_alive(hero) then return hero end
  return nil
end

local function stalled(id)
  local kind, tx, ty = game.order_of(id)
  if kind ~= ORDER_MOVE or game.within(id, tx, ty, ARRIVE_MILLI) then
    move_marks[id] = false
    return false
  end
  local mark = move_marks[id]
  if not mark or mark.tx ~= tx or mark.ty ~= ty or not game.within(id, mark.x, mark.y, ARRIVE_MILLI) then
    local x, y = unit_position(id)
    move_marks[id] = { tx = tx, ty = ty, x = x, y = y, at = game.ticks() }
    return false
  end
  return game.ticks() - mark.at >= STALL_TICKS
end

local function idle(id)
  local kind, x, y = game.order_of(id)
  if kind == ORDER_STOP then return true end
  return kind == ORDER_MOVE and game.within(id, x, y, ARRIVE_MILLI)
end

local function secret_depot_for(player)
  local rect = rect_of(player)
  local hx, hy = config.milli((rect.x0 + rect.x1) // 2), config.milli((rect.y0 + rect.y1) // 2)
  local best, best_d = nil, nil
  for _, row in ipairs(game.props(SECRET_DEPOT)) do
    local dx, dy = (row.x - hx) // config.FIX, (row.y - hy) // config.FIX
    local d = dx * dx + dy * dy
    if best_d == nil or d < best_d then best, best_d = row, d end
  end
  return best
end

local function auto_delivery_on(owner)
  return game.setting(owner, "autodeliver") ~= 0
end

local function secret_stop_on(owner)
  return game.setting(owner, "secretstop") == 1
end

local function near_point(x, y, px, py, reach)
  local dx, dy = x - px, y - py
  return dx * dx + dy * dy <= reach * reach
end

local function visit_secret_depot(id, owner)
  if stop_visit[id] == owner then return false end
  local depot = secret_depot_for(owner)
  if depot == nil then return false end
  if game.within(id, depot.x, depot.y, PICKUP_REACH_MILLI) or stalled(id) then
    stop_visit[id] = owner
    return false
  end
  local kind, x, y = game.order_of(id)
  if not (kind == ORDER_MOVE and near_point(x, y, depot.x, depot.y, config.milli(ARRIVE_MILLI))) then
    game.queue_clear(id)
    game.cmd_order(id, depot.x, depot.y)
  end
  return true
end

local function deliver(id)
  local cx, cy = unit_position(id)
  local batches, by_owner = {}, {}
  for _, row in ipairs(game.unit_items(id)) do
    if not DROP_EXEMPT[row.name] then
      local batch = by_owner[row.owner]
      if batch == nil then
        batch = { owner = row.owner, hero = live_hero_of(row.owner), rows = {} }
        by_owner[row.owner] = batch
        batches[#batches + 1] = batch
      end
      batch.rows[#batch.rows + 1] = row
    end
  end
  if stop_visit[id] and by_owner[stop_visit[id]] == nil then stop_visit[id] = false end
  local routed = false
  for _, batch in ipairs(batches) do
    local room = batch.hero ~= nil and INVENTORY_SLOTS - #game.unit_items(batch.hero) or 0
    if room > 0 and auto_delivery_on(batch.owner) then
      if secret_stop_on(batch.owner) and visit_secret_depot(id, batch.owner) then
        routed = true
      elseif game.within(batch.hero, cx, cy, PICKUP_REACH_MILLI) then
        local take = {}
        for i = 1, math.min(room, #batch.rows) do take[i] = batch.rows[i] end
        schedule_pickup(batch.hero, drop_rows(id, take, cx, cy))
        stop_visit[id] = false
      elseif secret_stop_on(batch.owner) and stop_visit[id] == batch.owner then
        if game.order_of(id) ~= ORDER_FOLLOW then game.cmd_follow(id, batch.hero) end
        routed = true
      end
    end
  end
  return routed
end

local function tend(id)
  local cx, cy = unit_position(id)
  local hero = live_hero_of(unit_player(id))
  local stuck = stalled(id)
  if game.order_of(id) == ORDER_FOLLOW then
    if hero == nil or not game.within(hero, cx, cy, FOLLOW_BREAK_MILLI) then go_home(id) end
    return
  end
  if (stuck or idle(id)) and not in_base(id) then go_home(id) end
end

local function tend_couriers()
  for _, id in ipairs(game.units(nil)) do
    if is_courier(id) and not errand[id] and not deliver(id) then
      tend(id)
    end
  end
end

local function ping_courier(who)
  for _, id in ipairs(game.units(nil)) do
    if M.shared(id, who) then
      local x, y = unit_position(id)
      game.ui_event("ping", '{"player":' .. tostring(who) .. ',"x":' .. tostring(x / config.FIX) .. ',"z":' .. tostring(y / config.FIX)
        .. ',"seconds":' .. tostring(PING_SECONDS) .. ',"colour":"#ffffff"}')
      return true
    end
  end
  return false, "noCourier"
end

local function find_secret_shop(id)
  local depot = secret_depot_for(unit_player(id))
  if depot == nil then return false, "noDepot" end
  game.queue_clear(id)
  game.cmd_order(id, depot.x, depot.y)
  return true
end

local function burst(id)
  for _, a in ipairs(game.unit_abilities(id)) do
    if a.name == BURST then
      if game.unit_cooldown(id, a.slot) > 0 then return false, "cooldown" end
      local x, y = unit_position(id)
      game.cmd_cast(id, a.slot, -1, x, y)
      return true
    end
  end
  return false, "noAbility"
end

function M.command(who, id, action)
  if not is_courier(id) then return false, "noCourier" end
  if action == "ping" then
    if cooling(id, action) then return false, "cooldown" end
    local ok, reason = ping_courier(who)
    if ok then start_cooldown(id, action) end
    return ok, reason
  end
  if action == "burst" then return burst(id) end
  local run = { drop = drop_items, pickup = pickup_items, secretShop = find_secret_shop, skin = change_skin }
  if run[action] == nil then return false, "refused" end
  if cooling(id, action) then return false, "cooldown" end
  local ok, reason = run[action](id, who)
  if ok then start_cooldown(id, action) end
  return ok, reason
end

local function courier_count(player)
  local count = 0
  for _, id in ipairs(game.units(player)) do
    if is_courier(id) then count = count + 1 end
  end
  return count
end

local function owns_courier(player)
  return courier_count(player) > 0
end

local function spawn_couriers()
  for _, hero in ipairs(bounty.heroes()) do
    local player = unit_player(hero)
    if not owns_courier(player) then
      local rect = rect_of(player)
      game.cmd_spawn(COURIER, config.milli(rect.x0 + roll(rect.x1 - rect.x0)), config.milli(rect.y0 + roll(rect.y1 - rect.y0)), player)
    end
  end
end

local function use_items(now)
  for _, hero in ipairs(bounty.heroes()) do
    for _, row in ipairs(game.unit_items(hero)) do
      local kind = row.name == "courier" and COURIER or row.name == "flying_courier" and FLYING or nil
      if kind ~= nil and row.count > 0 then
        local player = unit_player(hero)
        game.consume_item_charge(hero, row.slot)
        local early = kind == FLYING and now < FLYING_UNLOCK_S * TPS
        if early or courier_count(player) >= LIMIT then
          game.cmd_add_gold(player, PRICE[row.name])
        else
          local x, y = unit_position(hero)
          game.cmd_spawn(kind, x, y, player)
        end
      end
    end
  end
end

local function revive_due(now)
  local left = {}
  for _, r in ipairs(revive) do
    if now >= r.at then
      game.cmd_spawn(r.kind, r.x, r.y, r.player)
    else
      left[#left + 1] = r
    end
  end
  revive = left
end

local function report_couriers(now)
  for _, hero in ipairs(bounty.heroes()) do
    local player = unit_player(hero)
    local remaining = 0
    for _, r in ipairs(revive) do
      if r.player == player and r.at - now > remaining then remaining = r.at - now end
    end
    local seconds = math.ceil(remaining / TPS)
    local alive = courier_count(player)
    local key = tostring(alive) .. ":" .. tostring(seconds)
    if reported[player] ~= key then
      reported[player] = key
      game.ui_event("courier", '{"player":' .. tostring(player) .. ',"alive":' .. tostring(alive) .. ',"revive":' .. tostring(seconds) .. ',"limit":' .. tostring(LIMIT) .. "}")
    end
  end
end

return {
  shared = M.shared,
  first = M.first,
  send_home = M.send_home,
  stalled = stalled,
  errand = errand,
  command = M.command,
  on_died = function(unit, killer, killer_player, amount, victim_player)
    local kind = game.unit_kind(unit)
    if not is_courier_kind(kind) or victim_player == nil then return end
    local x, y = unit_position(unit)
    revive[#revive + 1] = { at = game.ticks() + REVIVE_S[base_kind(kind)] * TPS, kind = kind, player = victim_player, x = x, y = y }
  end,
  on_tick = function(state)
    local now = game.ticks()
    if now < SPAWN_AFTER_TICK then return end
    if not spawned then
      spawned = true
      spawn_couriers()
    end
    revive_due(now)
    if now % TPS == 0 then report_couriers(now) end
    if now % TEND_EVERY == 0 then tend_couriers() end
    collect_pickups(now)
    use_items(now)
  end,
}

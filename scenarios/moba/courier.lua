local config = require("config")
local bounty = require("bounty")
local kinds = require("courier_kinds")
local M = {}

local COURIER = "courier"
local FLYING = "courier_flying"
local LIMIT = 2
local REVIVE_S = { [COURIER] = 140, [FLYING] = 180 }
local FLYING_UNLOCK_S = 180
local PRICE = { courier = 120, flying_courier = 340 }
local revive = {}
local TPS = config.TICKS_PER_SECOND
local SPAWN_AFTER_TICK = 2
local spawned = false
local seed = 40503
local PICKUP_REACH_MILLI = 20312
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

function M.send_home(ids, hero)
  if hero ~= nil then return false end
  local sent = false
  for _, id in ipairs(ids) do
    if is_courier(id) then
      local rect = rect_of(unit_player(id))
      game.queue_clear(id)
      game.cmd_order(id, config.milli((rect.x0 + rect.x1) // 2), config.milli((rect.y0 + rect.y1) // 2))
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
  bottle_regeneration = true, bottle_invisibility = true, bottle_bounty = true, aegis = true,
}
local PICKUP_DELAY_TICKS = 2
local pending_pickups = {}

local function in_base(id)
  local rect = BASES[game.team(unit_player(id)) == 0 and 0 or 1]
  local x, y = unit_position(id)
  return x >= config.milli(rect.x0) and x <= config.milli(rect.x1) and y >= config.milli(rect.y0) and y <= config.milli(rect.y1)
end

local function drop_items(id)
  local x, y = unit_position(id)
  local home = in_base(id)
  for _, row in ipairs(game.unit_items(id)) do
    if not DROP_EXEMPT[row.name] then
      local owner = row.owner
      local start = home and owner ~= nil and not game.hostile(unit_player(id), owner) and game.seat_start(owner) or nil
      game.cmd_drop_item(id, row.slot, start and start.x or x, start and start.y or y)
    end
  end
  return true
end

local function pickup_items(id, who)
  local hero = bounty.hero_of(who)
  local start = game.seat_start(who)
  if hero == nil or not unit_alive(hero) or start == nil then return false, "noHero" end
  local reach = config.milli(PICKUP_REACH_MILLI)
  local hx, hy = unit_position(hero)
  local dropped = {}
  for _, courier in ipairs(game.units(nil)) do
    if is_courier(courier) and game.within(courier, hx, hy, reach) then
      for _, row in ipairs(game.unit_items(courier)) do
        if row.owner == who then dropped[#dropped + 1] = game.cmd_drop_item(courier, row.slot, start.x, start.y) end
      end
    end
  end
  pending_pickups[#pending_pickups + 1] = { hero = hero, ids = dropped, at = game.ticks() + PICKUP_DELAY_TICKS }
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

return {
  shared = M.shared,
  first = M.first,
  send_home = M.send_home,
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
    collect_pickups(now)
    use_items(now)
  end,
}

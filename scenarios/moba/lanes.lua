local config = require("config")
local units = require("units")

local MILLI_TILE = math.floor(config.FIX / 1000)
local TICKS_PER_SECOND = 16
local WAVE_PERIOD = 30 * TICKS_PER_SECOND
local SIEGE_EVERY_WAVES = 7
local UPGRADE_PERIOD = 450 * TICKS_PER_SECOND
local SLOT_SPACING = 700
local MELEE_RAISES = { 849, 1759, 2669 }
local RANGED_RAISES = { 2669 }
local LANE_ID = { top = 1, mid = 2, bot = 3 }
local LANES = { "top", "mid", "bot" }
local SPAWNER_LANE = {
  lane_spawner_top = "top",
  lane_spawner_mid = "mid",
  lane_spawner_bot = "bot",
}
local RAX_TYPE = { barracks_melee = "melee", barracks_ranged = "ranged" }

local SPAWN = {
  [0] = {
    top = { melee = { 66750, 226500 }, ranged = { 66250, 230250 } },
    mid = { melee = { 88750, 246500 }, ranged = { 87500, 251250 } },
    bot = { melee = { 107750, 269000 }, ranged = { 99000, 268000 } },
  },
  [1] = {
    top = { melee = { 202500, 77000 }, ranged = { 208250, 76750 } },
    mid = { melee = { 221500, 104750 }, ranged = { 229250, 102000 } },
    bot = { melee = { 261250, 112000 }, ranged = { 259500, 106750 } },
  },
}

local SENTINEL_OWNER = 0

local function side_kinds(prefix)
  return {
    melee = { base = prefix .. "creep", super = prefix .. "creep_super", mega = prefix .. "creep_mega", rax = "melee" },
    ranged = { base = prefix .. "creep_ranged", super = prefix .. "creep_ranged_super", mega = prefix .. "creep_ranged_mega", rax = "ranged" },
    siege = { base = prefix .. "creep_siege", super = prefix .. "creep_siege_super", rax = "ranged" },
  }
end

local SCOURGE_KINDS = side_kinds("")
local SENTINEL_KINDS = side_kinds("sentinel_")

local function kinds_of(owner)
  return owner == SENTINEL_OWNER and SENTINEL_KINDS or SCOURGE_KINDS
end

local UPGRADES = {
  creep = { "creep_up_dmg_1", "creep_up_hp_10" },
  creep_super = { "creep_up_dmg_2", "creep_up_hp_19" },
  creep_ranged = { "creep_up_dmg_2", "creep_up_hp_10" },
  creep_ranged_super = { "creep_up_dmg_3", "creep_up_hp_18" },
}
for _, kind in ipairs({ "creep", "creep_super", "creep_ranged", "creep_ranged_super" }) do
  UPGRADES["sentinel_" .. kind] = UPGRADES[kind]
end

local EARLY_LANE_S = 329
local EARLY_LANE_SPEED_TICKS = 15 * TICKS_PER_SECOND
local EARLY_LANE_PCT = {
  [0] = { top = -25, mid = 0, bot = 25 },
  [1] = { top = 25, mid = 0, bot = -25 },
}
local S_SPEED_PCT = 19
local WAVE_KINDS = {
  creep = true, creep_super = true, creep_mega = true, creep_ranged = true, creep_ranged_super = true,
  creep_ranged_mega = true, creep_siege = true, creep_siege_super = true,
}
for _, kind in ipairs({ "creep", "creep_super", "creep_mega", "creep_ranged", "creep_ranged_super", "creep_ranged_mega", "creep_siege", "creep_siege_super" }) do
  WAVE_KINDS["sentinel_" .. kind] = true
end

local barracks = {}
local lane_spawners = {}
local spawned = {}
local upgraded = {}
local next_unit = 0
local adopt_existing = false

local function distance2(ax, ay, bx, by)
  local dx, dy = ax - bx, ay - by
  return dx * dx + dy * dy
end

local function register_spawner(unit)
  local x, y = unit_position(unit)
  lane_spawners[#lane_spawners + 1] = { id = unit, owner = unit_player(unit), lane = SPAWNER_LANE[game.unit_kind(unit)], x = x, y = y }
end

local function lane_of_spawn(owner, x, y)
  local at = SPAWN[owner]
  if at == nil then return nil end
  local best, best_d
  for _, lane in ipairs(LANES) do
    for _, rect in ipairs({ at[lane].melee, at[lane].ranged }) do
      local d = distance2(config.milli(rect[1]), config.milli(rect[2]), x, y)
      if best_d == nil or d < best_d then best, best_d = lane, d end
    end
  end
  return best
end

local function apply_early_speed(unit)
  if game.ticks() >= EARLY_LANE_S * TICKS_PER_SECOND then return end
  local owner = unit_player(unit)
  local pct = EARLY_LANE_PCT[owner]
  if pct == nil then return end
  local x, y = unit_position(unit)
  local lane = lane_of_spawn(owner, x, y)
  if lane ~= nil and pct[lane] ~= 0 then game.cmd_timed_stat(unit, S_SPEED_PCT, pct[lane], EARLY_LANE_SPEED_TICKS) end
end

local function lane_of_barracks(owner, x, y)
  local best, best_d
  for _, s in ipairs(lane_spawners) do
    if s.owner == owner then
      local d = distance2(s.x, s.y, x, y)
      if best_d == nil or d < best_d then best, best_d = s.lane, d end
    end
  end
  return best
end

local function register_barracks(unit)
  local x, y = unit_position(unit)
  local owner = unit_player(unit)
  barracks[#barracks + 1] = { id = unit, owner = owner, type = RAX_TYPE[game.unit_kind(unit)], lane = lane_of_barracks(owner, x, y) }
end

local function enemy_barracks_dead(owner, lane, type)
  for _, b in ipairs(barracks) do
    if b.owner ~= owner and b.lane == lane and b.type == type and not unit_alive(b.id) then return true end
  end
  return false
end

local function all_enemy_barracks_dead(owner)
  local any = false
  for _, b in ipairs(barracks) do
    if b.owner ~= owner then
      if unit_alive(b.id) then return false end
      any = true
    end
  end
  return any
end

local function wave_kind(spec, owner, lane)
  if spec.mega ~= nil and all_enemy_barracks_dead(owner) then return spec.mega, "mega" end
  if enemy_barracks_dead(owner, lane, spec.rax) then return spec.super, "super" end
  return spec.base, "base"
end

local function spawn_unit(spec, owner, lane, x, y)
  local kind, level = wave_kind(spec, owner, lane)
  game.cmd_spawn(kind, x, y, owner, LANE_ID[lane])
  spawned[kind] = (spawned[kind] or 0) + 1
  spawned.total = (spawned.total or 0) + 1
  spawned[level] = (spawned[level] or 0) + 1
end

local function spawn_special(kind, owner, lane)
  local at = SPAWN[owner]
  if at == nil or at[lane] == nil then return nil end
  local rect = at[lane].melee
  return game.cmd_spawn(kind, config.milli(rect[1]), config.milli(rect[2]), owner, LANE_ID[lane])
end

local function creeps_at(seconds, raises)
  local count = 0
  for _, from in ipairs(raises) do
    if seconds >= from then count = count + 1 end
  end
  return count
end

local function spawn_wave(s)
  local at = SPAWN[s.owner]
  if at == nil or at[s.lane] == nil then return end
  local now = game.ticks()
  local seconds = now / TICKS_PER_SECOND
  local melee_count = 3 + creeps_at(seconds, MELEE_RAISES)
  local ranged_count = 1 + creeps_at(seconds, RANGED_RAISES)
  local kinds = kinds_of(s.owner)
  local melee, ranged = at[s.lane].melee, at[s.lane].ranged
  for i = 0, melee_count - 1 do
    spawn_unit(kinds.melee, s.owner, s.lane, config.milli(melee[1] + i * SLOT_SPACING), config.milli(melee[2]))
  end
  for i = 0, ranged_count - 1 do
    spawn_unit(kinds.ranged, s.owner, s.lane, config.milli(ranged[1] + i * SLOT_SPACING), config.milli(ranged[2]))
  end
  local wave = math.floor(now / WAVE_PERIOD) + 1
  if wave % SIEGE_EVERY_WAVES == 0 then
    for i = 0, ranged_count - 1 do
      spawn_unit(kinds.siege, s.owner, s.lane, config.milli(ranged[1] + (ranged_count + i) * SLOT_SPACING), config.milli(ranged[2]))
    end
  end
end

for _, unit in ipairs(game.units()) do
  if SPAWNER_LANE[game.unit_kind(unit)] then register_spawner(unit) end
end
for _, unit in ipairs(game.units()) do
  if RAX_TYPE[game.unit_kind(unit)] then register_barracks(unit) end
end
for _, s in ipairs(lane_spawners) do
  game.timer("wave_" .. s.id, 1, function() spawn_wave(s) end, WAVE_PERIOD)
end

local function upgrade_level()
  return math.floor(game.ticks() / UPGRADE_PERIOD)
end

local function apply_upgrades(unit, steps)
  local names = UPGRADES[game.unit_kind(unit)]
  if names == nil then return end
  for _ = 1, steps do
    for _, research in ipairs(names) do game.apply_research(research, unit) end
  end
end

local function sync_upgrades()
  local level = upgrade_level()
  local count = get_unit_count()
  if adopt_existing then
    for u = 0, count - 1 do
      if UPGRADES[game.unit_kind(u)] then upgraded[u] = level end
    end
    next_unit = count
    adopt_existing = false
    return
  end
  for u in pairs(upgraded) do
    if upgraded[u] < level and unit_alive(u) then
      apply_upgrades(u, level - upgraded[u])
      upgraded[u] = level
    end
  end
  for u = next_unit, count - 1 do
    if WAVE_KINDS[game.unit_kind(u)] then apply_early_speed(u) end
    if UPGRADES[game.unit_kind(u)] then
      apply_upgrades(u, level)
      upgraded[u] = level
    end
  end
  next_unit = count
end

local ACQUIRE_PERIOD = 4
local ORDER_HOLD = 4
local ORDER_MOVE = 0
local ORDER_STOP = 1
local ORDER_ATTACK_MOVE = 2
local ARRIVED = config.milli(1000)

local function within(a, b, range)
  local ax, ay = unit_position(a)
  local bx, by = unit_position(b)
  local dx, dy = ax - bx, ay - by
  return dx * dx + dy * dy <= range * range
end

local function nearest_creep_target(u, range)
  local ux, uy = unit_position(u)
  local best, best_d
  for _, j in ipairs(units.units_near(ux, uy, config.milli(range))) do
    if j ~= u and not units.is_structure(j) and units.hostile(unit_player(u), unit_player(j)) and game.can_target(u, j) then
      local jx, jy = unit_position(j)
      local dx, dy = jx - ux, jy - uy
      local d = dx * dx + dy * dy
      if best == nil or d < best_d then best, best_d = j, d end
    end
  end
  return best
end

local function wave_creep_step(u, range, now)
  local kind, dx, dy = game.order_of(u)
  local target, ret_kind, ret_x, ret_y = game.chase_get(u)
  if target >= 0 then
    if kind == ORDER_HOLD then
      game.chase_set(u, -1, 0, 0, 0)
      return
    end
    if not unit_alive(target) or not units.hostile(unit_player(u), unit_player(target))
        or not within(u, target, config.milli(range)) then
      game.chase_set(u, -1, 0, 0, 0)
      game.issue_order(u, ret_kind, ret_x, ret_y)
      return
    end
    local tx, ty = unit_position(target)
    if dx ~= tx or dy ~= ty then game.attack_move(u, tx, ty) end
    return
  end
  if kind == ORDER_HOLD or (now + u) % ACQUIRE_PERIOD ~= 0 then return end
  local t = nearest_creep_target(u, range)
  if t == nil then return end
  game.chase_set(u, t, kind, dx, dy)
  local tx, ty = unit_position(t)
  game.attack_move(u, tx, ty)
end

local function hero_idle(u, kind, dx, dy)
  if kind == ORDER_STOP then return true end
  if kind ~= ORDER_MOVE then return false end
  local ux, uy = unit_position(u)
  local ex, ey = dx - ux, dy - uy
  return ex * ex + ey * ey <= ARRIVED * ARRIVED
end

local function hero_chase(u, target)
  local tx, ty = unit_position(target)
  game.attack_move(u, tx, ty)
  local _, ox, oy = game.order_of(u)
  game.chase_set(u, target, 0, ox, oy)
end

local function hero_step(u, range, now)
  local kind, dx, dy = game.order_of(u)
  local target, _, last_x, last_y = game.chase_get(u)
  if target >= 0 then
    local tx, ty = unit_position(target)
    local ours = kind == ORDER_ATTACK_MOVE and ((dx == last_x and dy == last_y) or (dx == tx and dy == ty))
    if ours and unit_alive(target) and units.hostile(unit_player(u), unit_player(target)) and within(u, target, config.milli(range)) then
      if dx ~= tx or dy ~= ty then hero_chase(u, target) end
      return
    end
    game.chase_set(u, -1, 0, 0, 0)
    if ours then
      local ux, uy = unit_position(u)
      game.issue_order(u, ORDER_STOP, ux, uy)
    end
    return
  end
  if (now + u) % ACQUIRE_PERIOD ~= 0 or not hero_idle(u, kind, dx, dy) then return end
  local t = nearest_creep_target(u, range)
  if t ~= nil then hero_chase(u, t) end
end

local function wave_creep_acquire(now)
  for u = 0, get_unit_count() - 1 do
    local range = game.acquire_of(u)
    if range > 0 and unit_alive(u) then
      if units.is_hero(u) then hero_step(u, range, now) else wave_creep_step(u, range, now) end
    end
  end
end

return {
  spawn_special = spawn_special,

  on_state_replaced = function()
    upgraded = {}
    next_unit = 0
    adopt_existing = true
  end,

  on_tick = function(state)
    sync_upgrades()
    wave_creep_acquire(game.ticks())
    state.waves_total = spawned.total or 0
    state.waves_base = spawned.base or 0
    state.waves_super = spawned.super or 0
    state.waves_mega = spawned.mega or 0
    state.waves_creep = (spawned.creep or 0) + (spawned.sentinel_creep or 0)
    state.waves_creep_ranged = (spawned.creep_ranged or 0) + (spawned.sentinel_creep_ranged or 0)
    state.waves_siege = (spawned.creep_siege or 0) + (spawned.sentinel_creep_siege or 0)
    state.waves_siege_upgraded = (spawned.creep_siege_super or 0) + (spawned.creep_siege_mega or 0) + (spawned.sentinel_creep_siege_super or 0) + (spawned.sentinel_creep_siege_mega or 0)
  end,

  on_march = function(unit, player)
    local ux, uy = unit_position(unit)
    local best_x, best_y, best_d = nil, nil, nil
    for k = 0, get_unit_count() - 1 do
      if k ~= unit and unit_alive(k) and unit_player(k) < config.PLAYERS and units.hostile(player, unit_player(k))
        and units.is_building(k) then
        local x, y = unit_position(k)
        local d = (x - ux) * (x - ux) + (y - uy) * (y - uy)
        if best_d == nil or d < best_d then
          best_x, best_y, best_d = x, y, d
        end
      end
    end
    if best_x ~= nil then
      return best_x, best_y
    end
  end,
}

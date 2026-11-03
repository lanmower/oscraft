local config = require("config")
local modes = require("modes")
local data = require("trees_688g")

local WAR3_UNIT = config.FIX / 64
local GUARD_UNITS = 175
local MOVE_CLEAR_MIN_UNITS = 75
local REGROW_S, REGROW_FIRST_S, REGROW_FIRST_DEATHS = 300, 450, 20
local TICKS_PER_SECOND = config.TICKS_PER_SECOND
local PICK_UNITS = 48

local trees = {
  TELEPORT_CLEAR_UNITS = 240,
  SWAP_CLEAR_UNITS = 300,
  CUTTERS = {
    item_tango = { heal = 115, heal_seconds = 16 },
    item_quelling_blade = {},
  },
}
local registered = false
local deaths = 0

local function units_q16(war3_units) return math.floor(war3_units * WAR3_UNIT) end

function trees.regrow_seconds()
  if modes.has("x") and deaths < REGROW_FIRST_DEATHS then return REGROW_FIRST_S end
  return REGROW_S
end

function trees.ensure()
  if registered then return end
  registered = true
  game.set_tree_guard(units_q16(GUARD_UNITS))
  game.set_move_tree_rule(units_q16(MOVE_CLEAR_MIN_UNITS), trees.regrow_seconds())
  if game.tree_count() > 0 then return end
  for _, r in ipairs(data) do game.tree_add(r[1], r[2], r[3], r[4], r[5]) end
end

function trees.kill(id)
  if not game.tree_kill(id, trees.regrow_seconds()) then return false end
  deaths = deaths + 1
  return true
end

local function meets_square(info, x, y, half)
  return info.x * config.FIX <= x + half and (info.x + info.w) * config.FIX >= x - half
    and info.y * config.FIX <= y + half and (info.y + info.h) * config.FIX >= y - half
end

function trees.clear_square(x, y, half_units)
  local half = units_q16(half_units)
  local reach = math.ceil(half * 1.4143)
  local n = 0
  for _, id in ipairs(game.trees_near(x, y, reach)) do
    if meets_square(game.tree_info(id), x, y, half) and trees.kill(id) then n = n + 1 end
  end
  return n
end

function trees.clear_under(unit, half_units)
  local x, y = unit_position(unit)
  return trees.clear_square(x, y, half_units)
end

function trees.nearest(x, y, reach_units)
  local best, best_d = nil, nil
  for _, id in ipairs(game.trees_near(x, y, units_q16(reach_units))) do
    local t = game.tree_info(id)
    local dx, dy = (t.x + t.w / 2) * config.FIX - x, (t.y + t.h / 2) * config.FIX - y
    local d = dx * dx + dy * dy
    if best_d == nil or d < best_d then best, best_d = id, d end
  end
  return best
end

function trees.blocker(x, y, half_units, seconds)
  local half = units_q16(half_units)
  local x0, x1 = math.floor((x - half) / config.FIX), math.floor((x + half - 1) / config.FIX)
  local y0, y1 = math.floor((y - half) / config.FIX), math.floor((y + half - 1) / config.FIX)
  return game.blocker_add(x0, y0, x1 - x0 + 1, y1 - y0 + 1, seconds)
end

local function direction(x0, y0, x1, y1)
  local dx, dy = x1 - x0, y1 - y0
  local len = math.sqrt(dx * dx + dy * dy)
  if len == 0 then return 1, 0 end
  return dx / len, dy / len
end

local function rotated(dx, dy, cos, sin) return dx * cos - dy * sin, dx * sin + dy * cos end

local RING8 = { { 1, 0 }, { 0.7071067811865476, 0.7071067811865476 }, { 0, 1 }, { -0.7071067811865476, 0.7071067811865476 },
  { -1, 0 }, { -0.7071067811865476, -0.7071067811865476 }, { 0, -1 }, { 0.7071067811865476, -0.7071067811865476 } }

local SPROUT_RADIUS, SPROUT_HALF, SPROUT_BASE_S = 150, 64, 2
local BRANCH_HALF, BRANCH_S = 64, 10
local FISSURE_STEP, FISSURE_ROCKS, FISSURE_HALF, FISSURE_S = 60, 24, 32, 8
local SHARD_AHEAD, SHARD_RADIUS, SHARD_HALF, SHARD_S = 50, 200, 128, 7
local SHARD_COS40, SHARD_SIN40, SHARD_COS80, SHARD_SIN80 = 0.766044443118978, 0.6427876096865393, 0.17364817766693041, 0.984807753012208

function trees.sprout(x, y, rank)
  local ring = units_q16(SPROUT_RADIUS)
  for _, d in ipairs(RING8) do trees.blocker(x + d[1] * ring, y + d[2] * ring, SPROUT_HALF, SPROUT_BASE_S + rank) end
end

function trees.branch(x, y) return trees.blocker(x, y, BRANCH_HALF, BRANCH_S) end

function trees.fissure(caster, x, y)
  local cx, cy = unit_position(caster)
  local dx, dy = direction(cx, cy, x, y)
  local step = units_q16(FISSURE_STEP)
  for j = 1, FISSURE_ROCKS do trees.blocker(cx + dx * step * j, cy + dy * step * j, FISSURE_HALF, FISSURE_S) end
end

function trees.ice_shards(caster, x, y)
  local cx, cy = unit_position(caster)
  local dx, dy = direction(cx, cy, x, y)
  local ahead, radius = units_q16(SHARD_AHEAD), units_q16(SHARD_RADIUS)
  local ax, ay = x + dx * ahead, y + dy * ahead
  for _, a in ipairs({ { 1, 0 }, { SHARD_COS40, SHARD_SIN40 }, { SHARD_COS40, -SHARD_SIN40 }, { SHARD_COS80, SHARD_SIN80 }, { SHARD_COS80, -SHARD_SIN80 } }) do
    local rx, ry = rotated(dx, dy, a[1], a[2])
    trees.blocker(ax + rx * radius, ay + ry * radius, SHARD_HALF, SHARD_S)
  end
end

local CHAIN_BASE, CHAIN_PER_LEVEL, CHAIN_LINK, CHAIN_REACH = 600, 200, 50, 90
function trees.chain(caster, x, y, rank)
  local cx, cy = unit_position(caster)
  local dx, dy = direction(cx, cy, x, y)
  local link = units_q16(CHAIN_LINK)
  local last = math.floor((CHAIN_BASE + CHAIN_PER_LEVEL * rank) / CHAIN_LINK)
  local reach = units_q16(CHAIN_REACH)
  local found
  for k = 1, last do
    local px, py = cx + dx * link * k, cy + dy * link * k
    for _, id in ipairs(game.trees_near(px, py, math.ceil(reach * 1.4143))) do
      if meets_square(game.tree_info(id), px, py, reach) then found = k break end
    end
    if found then break end
  end
  if not found then return false end
  for k = found, 1, -1 do
    local px, py = math.floor(cx + dx * link * k), math.floor(cy + dy * link * k)
    trees.clear_square(px, py, CHAIN_REACH)
    if game.teleport(caster, px, py) then return true end
  end
  return false
end

function trees.can_cut(ability, x, y)
  if trees.CUTTERS[ability] == nil then return true end
  return trees.nearest(x, y, PICK_UNITS) ~= nil
end

local healing = {}

function trees.cut(caster, ability)
  local cutter = trees.CUTTERS[ability]
  local x, y = game.cast_point(caster)
  local id = trees.nearest(x, y, PICK_UNITS)
  if id == nil or not trees.kill(id) then return false end
  if cutter.heal ~= nil then
    local ticks = cutter.heal_seconds * TICKS_PER_SECOND
    healing[caster] = { per_tick = math.floor(cutter.heal * config.FIX / ticks), until_tick = game.ticks() + ticks }
  end
  return true
end

local function heal_step(now)
  for unit, h in pairs(healing) do
    if now >= h.until_tick or not unit_alive(unit) then
      healing[unit] = nil
    else
      game.set_hp(unit, unit_hp(unit) + h.per_tick)
    end
  end
end

local SWEEP_EVERY = TICKS_PER_SECOND // 4
local function release_stuck(now)
  if now % SWEEP_EVERY ~= 0 then return end
  for u = 0, get_unit_count() - 1 do
    if unit_alive(u) then
      local x, y = unit_position(u)
      for _, id in ipairs(game.trees_near(x, y, 0)) do
        local t = game.tree_info(id)
        if x > t.x * config.FIX and x < (t.x + t.w) * config.FIX and y > t.y * config.FIX and y < (t.y + t.h) * config.FIX then trees.kill(id) end
      end
    end
  end
end

trees.on_tick = function()
  trees.ensure()
  local now = game.ticks()
  heal_step(now)
  release_stuck(now)
end

function trees.on_state_replaced()
  healing = {}
  registered = false
  deaths = 0
  trees.ensure()
end

trees.ensure()

return trees

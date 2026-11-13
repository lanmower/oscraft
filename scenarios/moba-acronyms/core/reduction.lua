local config = require("config")
local counter = require("counter")
local units = require("units")
local reduction = {}

local sent = {}

local function threatened(structure, owner, x, y, radius)
  for _, k in ipairs(units.units_near(x, y, radius)) do
    if k ~= structure and not units.is_structure(k) and not units.is_hero(k)
      and unit_player(k) < config.PLAYERS and units.hostile(owner, unit_player(k)) then
      return true
    end
  end
  return false
end

function reduction.radius(spec)
  local radius = config.milli(spec.radius_milli)
  local last, covered, healed = {}, {}, {}
  local rule = {}
  function rule.update(u, ticks, scan)
    if not scan or not units.is_structure(u) then return end
    local x, y = unit_position(u)
    if threatened(u, unit_player(u), x, y, radius) then last[u] = ticks end
    covered[u] = ticks - (last[u] or 0) >= spec.quiet
    local heal = covered[u] and spec.heal or 0
    if healed[u] ~= heal then
      game.set_healing(u, heal)
      healed[u] = heal
    end
  end
  function rule.pct(u)
    return covered[u] and spec.pct or 0
  end
  return rule
end

function reduction.window(spec)
  local live, ready = counter.table(), counter.table()
  local rule = {}
  function rule.cast(team, ticks)
    if team == nil or not ready.ready(team, ticks) then return end
    live.arm(team, ticks + spec.span)
    ready.arm(team, ticks + spec.cooldown)
  end
  function rule.reset(team)
    if team ~= nil then ready.arm(team, 0) end
  end
  function rule.pct(u, ticks)
    local p = unit_player(u)
    if p >= config.PLAYERS then return 0 end
    local team = game.team(p)
    if team == nil or live.ready(team, ticks) then return 0 end
    local bits = game.filter_bits(u)
    if (bits & spec.need) == spec.need and (bits & spec.skip) == 0 then return spec.pct end
    return 0
  end
  return rule
end

function reduction.apply(rules, ticks, scan)
  for i = 0, get_unit_count() - 1 do
    if unit_alive(i) then
      local pct = 0
      for _, r in ipairs(rules) do
        if r.update then r.update(i, ticks, scan) end
        pct = math.max(pct, r.pct(i, ticks))
      end
      if sent[i] ~= pct then
        game.set_damage_reduction(i, pct)
        sent[i] = pct
      end
    end
  end
end

return reduction

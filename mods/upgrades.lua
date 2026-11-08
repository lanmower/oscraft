
game.unit("barracks", { health = 500, radius = 800, filter = 1 })

local GROUPS = {
  { name = "forged_armor", stacking = "add", levels = 2, cost_per_level = 100, time = 40, stat = "health", amount = 50 },
  { name = "war_banner", stacking = "once", levels = 1, cost_per_level = 150, time = 60, stat = "health", amount = 80 },
}

for _, g in ipairs(GROUPS) do
  for level = 1, g.levels do
    game.upgrade(g.name .. "_" .. level, {
      building = "barracks",
      time = g.time,
      cost = 0,
      effect = { kind = "stat", stat = g.stat, amount = g.amount },
      filter = 0,
    })
  end
end

local RESERVE_SPEND = "upg_reserve"
local RESERVE_TICK = "upg_reserve_tick"

local function reserved_spend(player)
  local now = game.ticks()
  local total = 0
  for _, u in ipairs(game.units(player)) do
    if game.counter(u, RESERVE_TICK) == now then total = total + game.counter(u, RESERVE_SPEND) end
  end
  return total
end

local function reserve(unit, cost)
  local now = game.ticks()
  local spent = 0
  if game.counter(unit, RESERVE_TICK) == now then spent = game.counter(unit, RESERVE_SPEND) end
  game.set_counter(unit, RESERVE_TICK, now)
  game.set_counter(unit, RESERVE_SPEND, spent + cost)
end

function upgrade_request(player, unit, group)
  local g
  for _, candidate in ipairs(GROUPS) do
    if candidate.name == group then g = candidate end
  end
  if g == nil then return false end
  if not game.alive(unit) or unit_player(unit) ~= player then return false end
  local key = "upg:" .. group
  local done = game.counter(unit, key)
  if g.stacking == "once" and done > 0 then return false end
  if done >= g.levels then return false end
  local level = done + 1
  local cost = level * g.cost_per_level
  if game.gold(player) - reserved_spend(player) < cost then return false end
  game.cmd_add_gold(player, -cost)
  game.set_counter(unit, key, level)
  reserve(unit, cost)
  local name = group .. "_" .. level
  game.set_research_bit(player, name, true)
  for _, u in ipairs(game.units(player)) do
    game.apply_research(name, u)
  end
  return true
end

game.trigger("on_spawned", {
  conditions = { function(unit) return game.alive(unit) end },
  action = function(unit)
    local p = unit_player(unit)
    if p < 0 or p > 16 then return end
    for _, g in ipairs(GROUPS) do
      for level = 1, g.levels do
        local name = g.name .. "_" .. level
        if game.research_bit(p, name) then
          game.apply_research(name, unit)
        end
      end
    end
  end,
})

function on_action(name, args)
  if name == "upgrade" then upgrade_request(args.player, args.unit, args.group) end
end

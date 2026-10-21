-- Upgrade levels as a mod rule. The sim has no upgrade records and no stacking rule. A level is a
-- research record (game.upgrade), and this file decides what a request does: which level the player
-- takes next, what it costs, and whether the group's stacking rule allows it. Each level's effect
-- reaches the player's units that exist and the units built later, as research does.
--
-- "add" stacking takes levels one after another, the level n costing n * cost_per_level gold.
-- "once" stacking refuses a second completion.
--
-- Uses game.unit, game.upgrade, game.gold, game.cmd_add_gold, game.units, game.research_bit,
-- game.set_research_bit, game.apply_research, game.counter and game.set_counter. A completed level
-- is its research bit for the player (the sim's only research state) plus apply_research per unit.
-- The ledger of completed levels is a sim counter on the requesting unit, so it is hashed and saved
-- with the world (a Lua table would not be).

game.unit("barracks", { health = 500, radius = 800, filter = 1 })

local GROUPS = {
  { name = "forged_armor", stacking = "add", levels = 2, cost_per_level = 100, time = 40, stat = "health", amount = 50 },
  { name = "war_banner", stacking = "once", levels = 1, cost_per_level = 150, time = 60, stat = "health", amount = 80 },
}

-- Every level is a research record. Its cost is charged by upgrade_request, so the record costs 0.
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

-- Takes the next level of group `group` for `player`, with `unit` as the ledger holder. Returns true
-- when the level was taken. Gold is read now and charged with the queued command, so two requests in
-- one tick can both pass the gold check.
function upgrade_request(player, unit, group)
  local g
  for _, candidate in ipairs(GROUPS) do
    if candidate.name == group then g = candidate end
  end
  if g == nil then return false end
  local key = "upg:" .. group
  local done = game.counter(unit, key)
  if g.stacking == "once" and done > 0 then return false end
  if done >= g.levels then return false end
  local level = done + 1
  local cost = level * g.cost_per_level
  if game.gold(player) < cost then return false end
  game.cmd_add_gold(player, -cost)
  game.set_counter(unit, key, level)
  local name = group .. "_" .. level
  game.set_research_bit(player, name, true)
  for _, u in ipairs(game.units(player)) do
    game.apply_research(name, u)
  end
  return true
end

-- A unit built after a level completes takes that level's effect, as research does.
game.trigger("on_spawned", {
  conditions = { function(unit) return game.alive(unit) end },
  action = function(unit)
    local p = unit_player(unit)
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

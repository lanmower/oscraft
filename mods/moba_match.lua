-- Moba match mod: lane waves and kill gold in one mod, with one on_tick. Its on_died gold pays both in
-- the mod's own world and in a live room, where the room feeds each payable death with room unit ids
-- and the bounty (docs/MODDING.md, on_died). The flag below hands kill gold to this mod.
--
-- Waves. Every lane spawner (lane_spawner_top, _mid, _bot, one per side) sends creeps on the lane
-- for its side. Top and bottom spawners send a creep wave every 480 ticks: three melee creeps and
-- one ranged creep, at x offsets 0, 0.7, 1.4 and 2.1 tiles. Every lane spawner sends a siege creep
-- every 4800 ticks from tick 4800, one more per lane from tick 28800 and a third from tick 57600.
-- A wave unit becomes its super kind once the enemy tower that mirrors its spawn point is dead,
-- and its mega kind once every enemy tower is dead. Dead units keep their slot, so the mirror
-- lookup also finds destroyed towers.
--
-- Gold. A unit with a bounty pays when it dies. A killing hero keeps the bounty less the team
-- share, and the killer's side gets the share. A killer that is not a hero (a creep) pays its whole
-- bounty to its side. An attackerless death pays the rival side's gold.
-- Uses only the mod API.
GOLD_BY_MOD = true

local MILLI_TILE = 65536 // 1000
local CREEP_PERIOD = 480
local SIEGE_PERIOD = 4800
local SIEGE_STARTS = { 4800, 28800, 57600 }
local CREEP_OFFSETS = { 0, 700, 1400, 2100 }
local LANE_ID = { top = 1, mid = 2, bot = 3 }
local SPAWNER_LANE = {
  lane_spawner_top = "top",
  lane_spawner_mid = "mid",
  lane_spawner_bot = "bot",
}
local TOWER_KINDS = { tower = true, tower_t2 = true, tower_t3 = true, tower_t4 = true }
local TEAM_SHARE_PCT = 20

local KINDS = {
  creep = { base = "creep", super = "creep_super", mega = "creep_mega" },
  ranged = { base = "creep_ranged", super = "creep_ranged_super", mega = "creep_ranged_mega" },
  siege = { base = "creep_siege", super = "creep_siege_super", mega = "creep_siege_mega" },
}

local MAP_W, MAP_H = game.map_size()
local towers = {}
local lane_spawners = {}
local spawned = {}

local function register_unit(unit)
  local kind = game.unit_kind(unit)
  if TOWER_KINDS[kind] then
    local x, y = unit_position(unit)
    towers[#towers + 1] = { id = unit, owner = unit_player(unit), x = x, y = y }
  elseif SPAWNER_LANE[kind] then
    lane_spawners[#lane_spawners + 1] = { id = unit, owner = unit_player(unit), lane = SPAWNER_LANE[kind] }
  end
end

local function side_cleared(owner)
  local any = false
  for _, t in ipairs(towers) do
    if t.owner ~= owner then
      if unit_alive(t.id) then return false end
      any = true
    end
  end
  return any
end

local function counterpart_dead(owner, x, y)
  local mx, my = MAP_W - x, MAP_H - y
  local best, best_d
  for _, t in ipairs(towers) do
    if t.owner ~= owner then
      local dx, dy = t.x - mx, t.y - my
      local d = dx * dx + dy * dy
      if best_d == nil or d < best_d then best, best_d = t, d end
    end
  end
  return best ~= nil and not unit_alive(best.id)
end

local function wave_kind(spec, owner, x, y)
  if side_cleared(owner) then return spec.mega, "mega" end
  if counterpart_dead(owner, x, y) then return spec.super, "super" end
  return spec.base, "base"
end

local function spawn_unit(spec, owner, lane, x, y)
  local kind, level = wave_kind(spec, owner, x, y)
  game.cmd_spawn(kind, x, y, owner, lane)
  spawned[kind] = (spawned[kind] or 0) + 1
  spawned.total = (spawned.total or 0) + 1
  spawned[level] = (spawned[level] or 0) + 1
end

local function start_lane(s)
  local sx, sy = unit_position(s.id)
  local lane = LANE_ID[s.lane]
  if s.lane ~= "mid" then
    game.timer("creep_" .. s.id, 1, function()
      local melee, ranged = KINDS.creep, KINDS.ranged
      for i = 1, 3 do
        spawn_unit(melee, s.owner, lane, sx + CREEP_OFFSETS[i] * MILLI_TILE, sy)
      end
      spawn_unit(ranged, s.owner, lane, sx + CREEP_OFFSETS[4] * MILLI_TILE, sy)
    end, CREEP_PERIOD)
  end
  for n, start in ipairs(SIEGE_STARTS) do
    game.timer("siege_" .. n .. "_" .. s.id, start, function()
      spawn_unit(KINDS.siege, s.owner, lane, sx, sy)
    end, SIEGE_PERIOD)
  end
end

-- on_died(unit, killer, killer_player, bounty, victim_player, killer_hero): ids are the mod's world
-- or the room's, whichever fed the death; bounty and killer_hero come with the event.
game.trigger("on_died", {
  conditions = { function(_, _, _, bounty) return bounty > 0 end },
  action = function(unit, killer, killer_player, bounty, victim_player, killer_hero)
    if killer ~= nil and killer ~= unit then
      local share = bounty * TEAM_SHARE_PCT // 100
      if killer_hero then
        game.cmd_add_hero_gold(killer, bounty - share)
        game.cmd_add_gold(killer_player, share)
      else
        game.cmd_add_gold(killer_player, bounty)
      end
      return
    end
    local rival = game.rival_of(victim_player)
    if rival ~= nil then game.cmd_add_gold(rival, bounty) end
  end,
})

for _, unit in ipairs(game.units()) do register_unit(unit) end
for _, s in ipairs(lane_spawners) do start_lane(s) end

function on_tick(state)
  state.waves_total = spawned.total or 0
  state.waves_base = spawned.base or 0
  state.waves_super = spawned.super or 0
  state.waves_mega = spawned.mega or 0
  state.waves_creep = spawned.creep or 0
  state.waves_creep_ranged = spawned.creep_ranged or 0
  state.waves_siege = spawned.creep_siege or 0
  state.waves_siege_upgraded = (spawned.creep_siege_super or 0) + (spawned.creep_siege_mega or 0)
end

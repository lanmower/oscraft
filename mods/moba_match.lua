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

-- on_died(unit, killer, killer_player, bounty, victim_player): ids are the mod's world
-- or the room's, whichever fed the death; bounty comes with the event. Hero status is asked of the game.
game.trigger("on_died", {
  conditions = { function(_, _, _, bounty) return bounty > 0 end },
  action = function(unit, killer, killer_player, bounty, victim_player)
    if killer ~= nil and killer ~= unit then
      local share = bounty * TEAM_SHARE_PCT // 100
      if game.is_hero(killer) then
        game.cmd_add_unit_gold(killer, bounty - share)
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

-- Hero respawn: a dead hero comes back HERO_RESPAWN_BASE_S + HERO_RESPAWN_PER_LEVEL_S per level seconds
-- after its death, capped at HERO_RESPAWN_CAP_S (the numbers of scenarios/moba/hero_rules.txt respawn_*,
-- which the engine keeps only as the fallback for a host with no mod). Other deaths keep the engine's timer.
local HERO_RESPAWN_BASE_S = 12
local HERO_RESPAWN_PER_LEVEL_S = 3
local HERO_RESPAWN_CAP_S = 100

function on_unit_died(unit, state)
  if not game.is_hero(unit) then return nil end
  local secs = HERO_RESPAWN_BASE_S + HERO_RESPAWN_PER_LEVEL_S * game.unit_level(unit)
  if secs > HERO_RESPAWN_CAP_S then secs = HERO_RESPAWN_CAP_S end
  return secs
end

for _, unit in ipairs(game.units()) do register_unit(unit) end
for _, s in ipairs(lane_spawners) do start_lane(s) end

local function mod_tick(state)
  state.waves_total = spawned.total or 0
  state.waves_base = spawned.base or 0
  state.waves_super = spawned.super or 0
  state.waves_mega = spawned.mega or 0
  state.waves_creep = spawned.creep or 0
  state.waves_creep_ranged = spawned.creep_ranged or 0
  state.waves_siege = spawned.creep_siege or 0
  state.waves_siege_upgraded = (spawned.creep_siege_super or 0) + (spawned.creep_siege_mega or 0)
end

-- Moba protection, march and win (scenarios/moba/protection.lua, march.lua, win.lua), one chunk with the
-- mod above: one on_tick runs both, and on_march, on_cast and on_check_win are defined here.
local RADIUS_MILLI = 14062
local ACTIVATION_S = 15
local REDUCTION_PCT = 50
local REGEN_PER_S = 90
local TICKS_PER_SECOND = 16
local SCAN_EVERY = 4
local GLYPH_NAME = "glyph_of_fortification"
local GLYPH_TICKS = 80
local GLYPH_COOLDOWN = 4800
local GLYPH_NEED = 64    -- filter bit 6, structure
local GLYPH_SKIP = 2048  -- filter bit 11, building

-- Same conversion as the sim's fixed::from_milli (65536 per unit, milli-units are 1/1000).
local RADIUS_RAW = RADIUS_MILLI * 65536
local RADIUS = (RADIUS_RAW - RADIUS_RAW % 1000) / 1000  -- whole fixed units; the sandbox has no math library
local ACTIVATION_TICKS = ACTIVATION_S * TICKS_PER_SECOND

local ticks = 0
local last_threat = {}   -- structure id -> tick a hostile plain unit was last within RADIUS
local backdoor = {}      -- structure id -> 1 while its backdoor is active
local applied = {}       -- unit id -> the damage reduction last sent
local applied_heal = {}  -- structure id -> the healing last sent
local glyph_until = {}   -- team -> tick its glyph ends
local glyph_ready = {}   -- team -> tick it may glyph again

local function threatened(structure, owner, x, y)
  for _, k in ipairs(game.units_near(x, y, RADIUS)) do
    if k ~= structure and not game.is_structure(k) and not game.is_hero(k)
      and unit_player(k) < 16 and game.hostile(owner, unit_player(k)) then
      return true
    end
  end
  return false
end

-- Whether unit i is covered by a glyph running for its team.
local function glyph_covers(i)
  local p = unit_player(i)
  if p >= 16 then return false end  -- neutral units (player 255) have no team
  local team = game.team(p)
  if team == nil or (glyph_until[team] or 0) <= ticks then return false end
  local bits = game.filter_bits(i)
  return (bits & GLYPH_NEED) == GLYPH_NEED and (bits & GLYPH_SKIP) == 0
end

function on_cast(unit, ability, state)
  if ability ~= GLYPH_NAME then return end
  local p = unit_player(unit)
  if p >= 16 then return end
  local team = game.team(p)
  if team == nil or (glyph_ready[team] or 0) > ticks then return end
  glyph_until[team] = ticks + GLYPH_TICKS
  glyph_ready[team] = ticks + GLYPH_COOLDOWN
end

local function protection_tick(state)
  ticks = ticks + 1
  local scan = ticks % SCAN_EVERY == 0
  for i = 0, get_unit_count() - 1 do
    if unit_alive(i) then
      if scan and game.is_structure(i) then
        local x, y = unit_position(i)
        if threatened(i, unit_player(i), x, y) then last_threat[i] = ticks end
        local quiet = ticks - (last_threat[i] or 0)
        backdoor[i] = quiet >= ACTIVATION_TICKS
        local heal = backdoor[i] and REGEN_PER_S or 0
        if applied_heal[i] ~= heal then
          game.set_healing(i, heal)
          applied_heal[i] = heal
        end
      end
      local pct = (backdoor[i] and REDUCTION_PCT or 0)
      if glyph_covers(i) then pct = 100 end
      if applied[i] ~= pct then
        game.set_damage_reduction(i, pct)
        applied[i] = pct
      end
    end
  end
end

local BUILDING = 2048  -- filter bit 11, building (FILTERS in osdt-data format.rs)
local PLAYERS = 16     -- MAX_PLAYERS; neutral units are player 255 and have no side

function on_march(unit, player, state)
  local ux, uy = unit_position(unit)
  local best_x, best_y, best_d = nil, nil, nil
  for k = 0, get_unit_count() - 1 do
    if k ~= unit and unit_alive(k) and unit_player(k) < PLAYERS and game.hostile(player, unit_player(k))
      and (game.filter_bits(k) & BUILDING) ~= 0 then
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
end

local BUILDING = 2048  -- filter bit 11, building
local PLAYERS = 16     -- MAX_PLAYERS in the sim

local had_building = {}  -- player -> true once the player held a living building

function on_check_win(state)
  local standing = {}
  for k = 0, get_unit_count() - 1 do
    if unit_alive(k) and (game.filter_bits(k) & BUILDING) ~= 0 then
      standing[unit_player(k)] = true
    end
  end
  for p = 0, PLAYERS - 1 do
    if standing[p] then
      had_building[p] = true
    elseif had_building[p] then
      -- p has lost its last building: the first player hostile to p names the winning side.
      for q = 0, PLAYERS - 1 do
        if q ~= p and game.hostile(p, q) then
          local side = game.team(q)
          if side ~= nil then
            return side
          end
        end
      end
    end
  end
  return nil
end

function on_tick(state)
  mod_tick(state)
  protection_tick(state)
end

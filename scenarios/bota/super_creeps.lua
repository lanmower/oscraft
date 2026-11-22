local modes = require("modes")
local lanes = require("lanes")

local TICKS_PER_SECOND = 16
local PERIOD = 600 * TICKS_PER_SECOND
local MESSAGE_SECONDS = 10
local SENTINEL = 0
local SCOURGE = 1

local KINDS = {
  { kind = "super_creep_golem", name = "Siege Golem" },
  { kind = "super_creep_fish", name = "Scary Fish" },
  { kind = "super_creep_hydra", name = "Ancient Hydra" },
}
local LANES = {
  { lane = "top", path = "west" },
  { lane = "mid", path = "middle" },
  { lane = "bot", path = "east" },
}
local SIDES = { { owner = SENTINEL, name = "Sentinel" }, { owner = SCOURGE, name = "Scourge" } }

local spawned = 0

local function roll()
  local pick = game.random(#KINDS * #LANES)
  return KINDS[pick // #LANES + 1], LANES[pick % #LANES + 1]
end

local function spawn_round()
  if not modes.has("sc") then return end
  local kind, lane = roll()
  for _, side in ipairs(SIDES) do
    lanes.spawn_special(kind.kind, side.owner, lane.lane)
    spawned = spawned + 1
    game.message(-1, kind.name .. " has spawned for the " .. side.name .. " and is taking the " .. lane.path .. " path", MESSAGE_SECONDS)
  end
end

game.timer("super_creeps", PERIOD, spawn_round, PERIOD)

return {
  on_tick = function(state)
    state.super_creeps_spawned = spawned
  end,
}

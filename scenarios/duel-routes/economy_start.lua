-- Economy start of the duel-routes scenario, as mod code. Each start seat gets its opening gold and gas, its opening
-- workers, and its patch row and geyser column: the placements the loader's start rules made before this module took
-- them over (the Rust open_player and base_fields). The counts and openings are economy.osd's economy(...) record. The
-- unit kinds are read from the catalog (start_kinds below): the first worker, the first
-- resource-kind-1 unit (patch) and the first resource-kind-2 unit (geyser). It runs once, on the first tick, and the
-- `started` flag in this module's state keeps it from running again.

local OPENING_GOLD = 0      -- economy.osd opening_resource_a
local OPENING_GAS = 0       -- economy.osd opening_gas
local OPENING_WORKERS = 4   -- economy.osd opening_workers
local PATCHES_PER_BASE = 8  -- economy.osd patches_per_base
local GEYSERS_PER_BASE = 2  -- economy.osd geysers_per_base (the sim's extractors_per_base)
local ONE = 65536           -- Q16.16 of one world unit
local PLAYERS = 16          -- player slots (the sim's MAX_PLAYERS)

local started = false

-- open_player: the opening resources are added to what the player holds, then the opening workers are placed at the start.
local function open_player(kinds, p, start)
  local gold, gas = game.resources(p)
  game.set_resources(p, gold + OPENING_GOLD, gas + OPENING_GAS)
  if kinds.worker then
    for _ = 1, OPENING_WORKERS do
      if game.spawn(p, kinds.worker, start.x, start.y) == nil then break end
    end
  end
end

-- base_fields: the patches run east of the start, two units out and one row each, and the geysers run south of it,
-- two units out and one column each. A spot the catalog cannot place is skipped.
local function base_fields(kinds, start)
  if kinds.patch then
    for k = 0, PATCHES_PER_BASE - 1 do
      game.spawn_neutral(kinds.patch, start.x + (2 + k) * ONE, start.y - 2 * ONE)
    end
  end
  if kinds.geyser then
    for k = 0, GEYSERS_PER_BASE - 1 do
      game.spawn_neutral(kinds.geyser, start.x - 2 * ONE, start.y + (2 + k) * ONE)
    end
  end
end

-- The unit kinds the start places: the first worker, the first resource-kind-1 unit (patch) and the first
-- resource-kind-2 unit (geyser), in catalog order. Each kind's row comes from game.unit_info(index).
local function start_kinds()
  local kinds = {}
  for i = 1, #game.unit_kinds() do
    local row = game.unit_info(i - 1)
    if row.worker and kinds.worker == nil then kinds.worker = i - 1 end
    if row.resource == 1 and kinds.patch == nil then kinds.patch = i - 1 end
    if row.resource == 2 and kinds.geyser == nil then kinds.geyser = i - 1 end
  end
  return kinds
end

return {
  on_tick = function(state)
    if started then return end
    started = true
    local kinds = start_kinds()
    for p = 0, PLAYERS - 1 do
      local start = game.seat_start(p)
      if start then
        open_player(kinds, p, start)
        base_fields(kinds, start)
      end
    end
  end,
}

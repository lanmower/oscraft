local OPENING_GOLD = 0
local OPENING_GAS = 0
local OPENING_WORKERS = 4
local PATCHES_PER_BASE = 8
local GEYSERS_PER_BASE = 2
local ONE = 65536
local PLAYERS = 16

local started = false

local function open_player(kinds, p, start)
  local gold, gas = game.resources(p)
  game.set_resources(p, gold + OPENING_GOLD, gas + OPENING_GAS)
  if kinds.worker then
    for _ = 1, OPENING_WORKERS do
      if game.spawn(p, kinds.worker, start.x, start.y) == nil then break end
    end
  end
end

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

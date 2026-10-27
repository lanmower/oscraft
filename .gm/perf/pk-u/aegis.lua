-- The aegis, as the pack's rule. A hero that dies holding the item comes back where it fell after the rule's seconds
-- (aegis_reincarnate_s), at full health and mana, and one charge of the item is spent. The death is not rewarded: the
-- hero rules (heroes.lua, kill_gold.lua) read `reincarnating` and pay nothing for it. The engine supplies the death, the
-- item's charges and the revive at a point; the decision is here.
-- Required first by init.lua: its on_died runs before the death rewards that read `reincarnating`.
local config = require("config")
local units = require("units")

local ITEM = "aegis"
local TICKS = config.TICKS_PER_SECOND

local held = {} -- hero -> { due = tick it returns, x, y = the Q16.16 point it fell at }

local aegis = {}

-- True while hero `u` waits to return by the aegis, so its death is not rewarded.
function aegis.reincarnating(u) return held[u] ~= nil end

-- The inventory slot of unit `u` that holds a charge of the aegis, or nil.
local function charged_slot(u)
  for _, row in ipairs(game.unit_items(u)) do
    if row.name == ITEM and row.count > 0 then return row.slot end
  end
  return nil
end

-- A hero's death: with a charge of the aegis, the charge is spent and the hero is held to return where it fell.
local function on_died(unit)
  if not units.is_hero(unit) or held[unit] ~= nil then return end
  local slot = charged_slot(unit)
  if slot == nil then return end
  game.consume_item_charge(unit, slot)
  local x, y = unit_position(unit)
  held[unit] = { due = game.ticks() + config.rule("aegis_reincarnate_s") * TICKS, x = x, y = y }
end

-- Each tick: a held hero returns at its death point once its seconds run out. A hero alive by another means (a
-- buyback) is no longer held, and a cell that cannot take it yet is retried next tick. The clock is game.ticks()
-- (the on_tick state's ticks field is not kept up to date by the host).
local function on_tick()
  local now = game.ticks()
  local ids = {}
  -- The sandbox has no table library, so the unit ids are kept in ascending order by insertion: two heroes
  -- revived on one tick contend for cells in the same order on every peer.
  for u in pairs(held) do
    local j = #ids + 1
    while j > 1 and ids[j - 1] > u do
      ids[j] = ids[j - 1]
      j = j - 1
    end
    ids[j] = u
  end
  for _, u in ipairs(ids) do
    local h = held[u]
    if game.alive(u) then
      held[u] = nil
    elseif now >= h.due and game.revive_at(u, h.x, h.y) then
      held[u] = nil
    end
  end
end

aegis.on_died = on_died
aegis.on_tick = on_tick

return aegis

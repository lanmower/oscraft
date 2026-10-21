-- Moba upgrades: Aghanim's Scepter and Aghanim's Shard. The sim keeps only the upgrade bits
-- (game.set_upgrade_bit, game.upgrade_bits); which item sets which bit, and when, is this file.
-- Bit 0 is the scepter, bit 1 the shard; talents.lua owns bits 2..=9.
--
-- Uses game.set_upgrade_bit(unit, bit, on), game.units(), game.unit_items(unit) (the active-slot items
-- of a unit, each with name and count) and the on_cast and on_tick hooks.

local SCEPTER_BIT = 0
local SHARD_BIT = 1

-- Items spent on use: the cast sets the bit.
local ON_USE = {
  item_ultimate_scepter_2 = SCEPTER_BIT,
  item_aghanims_shard = SHARD_BIT,
}

-- Items that set a bit while carried in an active slot, and clear it when put away.
local HELD = {
  ultimate_scepter = SCEPTER_BIT,
}

local HELD_BITS = { SCEPTER_BIT }

-- held[unit][bit] is true while the unit carries an item that sets the bit, as on_tick last saw it.
local held = {}

local function carried_bits(unit)
  local bits = {}
  for _, item in ipairs(game.unit_items(unit)) do
    local bit = HELD[item.name]
    if bit ~= nil then bits[bit] = true end
  end
  return bits
end

return {
  on_cast = function(caster, ability)
    local bit = ON_USE[ability]
    if bit ~= nil then game.set_upgrade_bit(caster, bit, true) end
  end,

  on_tick = function(state)
    local live = {}
    for _, unit in ipairs(game.units()) do
      live[unit] = true
      local now = carried_bits(unit)
      local before = held[unit] or {}
      for _, bit in ipairs(HELD_BITS) do
        local has, had = now[bit] == true, before[bit] == true
        if has ~= had then game.set_upgrade_bit(unit, bit, has) end
      end
      held[unit] = now
    end
    for unit in pairs(held) do
      if not live[unit] then held[unit] = nil end
    end
  end,
}

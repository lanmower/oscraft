local SCEPTER_BIT = 0
local SHARD_BIT = 1

local BIT_SET_ON_USE_OF_ITEM = {
  item_ultimate_scepter_2 = SCEPTER_BIT,
  item_aghanims_shard = SHARD_BIT,
}

local BIT_SET_WHILE_CARRYING_ITEM = {
  ultimate_scepter = SCEPTER_BIT,
  ultimate_scepter_alchemist = SCEPTER_BIT,
}

local CARRY_BITS = { SCEPTER_BIT }

local carried_bits_of_unit = {}
local item_generation_scanned_of_unit = {}

local function carried_bits(unit)
  local bits = {}
  for _, item in ipairs(game.unit_items(unit)) do
    local bit = BIT_SET_WHILE_CARRYING_ITEM[item.name]
    if bit ~= nil then bits[bit] = true end
  end
  return bits
end

return {
  on_state_replaced = function()
    carried_bits_of_unit, item_generation_scanned_of_unit = {}, {}
  end,

  on_cast = function(caster, ability)
    local bit = BIT_SET_ON_USE_OF_ITEM[ability]
    if bit ~= nil then game.set_upgrade_bit(caster, bit, true) end
  end,

  on_tick = function(state)
    local live = {}
    for _, unit in ipairs(game.units()) do
      live[unit] = true
      local gen = game.unit_item_gen(unit)
      if carried_bits_of_unit[unit] == nil or item_generation_scanned_of_unit[unit] ~= gen then
        local now = carried_bits(unit)
        local before = carried_bits_of_unit[unit] or {}
        for _, bit in ipairs(CARRY_BITS) do
          local has, had = now[bit] == true, before[bit] == true
          if has ~= had then game.set_upgrade_bit(unit, bit, has or game.counter(unit, "scepter_blessed") == 1) end
        end
        carried_bits_of_unit[unit] = now
        item_generation_scanned_of_unit[unit] = gen
      end
    end
    for unit in pairs(carried_bits_of_unit) do
      if not live[unit] then
        carried_bits_of_unit[unit] = nil
        item_generation_scanned_of_unit[unit] = nil
      end
    end
  end,
}

local config = require("config")
local units = require("units")

local ITEM = "aegis"
local TICKS = config.TICKS_PER_SECOND

local LEORIC = {
  ability = "skeleton_king_reincarnation_leoric",
  cooldown_ticks = { 4160, 2560, 960 },
  mana = 160,
  delay_ticks = 3 * TICKS,
  slow_radius_milli = 14063,
  slow_ticks = 5 * TICKS,
  slow_multiplier = 16384,
}

local pending_returns = {}

local aegis = {}

function aegis.reincarnating(u) return pending_returns[u] ~= nil end

local function charged_slot(u)
  for _, row in ipairs(game.unit_items(u)) do
    if row.name == ITEM and row.count > 0 then return row.slot end
  end
  return nil
end

local function leoric_return(unit)
  if game.status_left(unit, "break") > 0 then return nil end
  for _, row in ipairs(game.unit_abilities(unit)) do
    if row.name == LEORIC.ability and row.cooldown <= 0 then
      local rank = game.skill_rank(unit, row.slot)
      local mana = game.mana(unit)
      if rank == 0 or mana < LEORIC.mana then return nil end
      local cooldown = LEORIC.cooldown_ticks[math.min(rank, #LEORIC.cooldown_ticks)]
      game.start_cooldown(unit, row.slot, cooldown)
      return { delay = LEORIC.delay_ticks, mana_after = mana - LEORIC.mana, slows = true, slot = row.slot, cooldown_after = cooldown - LEORIC.delay_ticks }
    end
  end
  return nil
end

local function on_died(unit)
  if not units.is_hero(unit) or pending_returns[unit] ~= nil then return end
  local x, y = unit_position(unit)
  local slot = charged_slot(unit)
  local pending
  if slot ~= nil then
    game.consume_item_charge(unit, slot)
    pending = { delay = config.rule("aegis_reincarnate_s") * TICKS }
  else
    pending = leoric_return(unit)
    if pending == nil then return end
  end
  pending.due_tick = game.ticks() + pending.delay
  pending.x, pending.y = x, y
  pending_returns[unit] = pending
end

local function slow_enemies_around(unit, pending)
  local owner = unit_player(unit)
  for _, other in ipairs(game.units_near(pending.x, pending.y, config.milli(LEORIC.slow_radius_milli))) do
    if other ~= unit and game.alive(other) and not game.is_structure(other) and units.hostile(owner, unit_player(other)) then
      game.apply_status(other, "slow", LEORIC.slow_ticks, LEORIC.slow_multiplier)
    end
  end
end

local function ascending_keys(set)
  local keys = {}
  for key in pairs(set) do
    local j = #keys + 1
    while j > 1 and keys[j - 1] > key do
      keys[j] = keys[j - 1]
      j = j - 1
    end
    keys[j] = key
  end
  return keys
end

local function on_tick()
  local now = game.ticks()
  for _, u in ipairs(ascending_keys(pending_returns)) do
    local pending = pending_returns[u]
    if game.alive(u) then
      pending_returns[u] = nil
    elseif now >= pending.due_tick and game.revive_at(u, pending.x, pending.y) then
      pending_returns[u] = nil
      if pending.mana_after ~= nil then game.set_mana(u, pending.mana_after) end
      if pending.slot ~= nil then game.start_cooldown(u, pending.slot, pending.cooldown_after) end
      if pending.slows then slow_enemies_around(u, pending) end
    end
  end
end

aegis.on_died = on_died
aegis.on_tick = on_tick

return aegis

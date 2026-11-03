local config = require("config")
local units = require("units")
local trees = require("trees")
local item_forms = require("item_forms")

local MILLI = 1000
local RING_DIRECTIONS = {
  { 1000, 0 }, { 707, 707 }, { 0, 1000 }, { -707, 707 },
  { -1000, 0 }, { -707, -707 }, { 0, -1000 }, { 707, -707 },
}
local RING_STEPS = 6
local RING_BASE_MILLI = 1000
local RING_STEP_MILLI = 1000

local WARDS = {
  place_observer = { unit_type_index = 18, life_ticks = 5760, item = "ward_observer" },
  place_sentry = { unit_type_index = 19, life_ticks = 6720, item = "ward_sentry" },
}

local ABILITY_SLOTS = 8

local STAT_ATTACK_SPEED, STAT_CDR = 12, 20

local ENDS_CHANNEL, PIERCES_IMMUNITY, NO_RESIST, RECORDS_SOURCE, SETS_SPEED, BLOCKS_PROCS = 1, 2, 4, 8, 16, 32
game.set_status_flags("stun", ENDS_CHANNEL + BLOCKS_PROCS)
game.set_status_flags("hex", ENDS_CHANNEL + SETS_SPEED)
game.set_status_flags("taunt", RECORDS_SOURCE)
game.set_status_flags("invisibility", PIERCES_IMMUNITY + NO_RESIST)
game.set_status_flags("revealed", PIERCES_IMMUNITY + NO_RESIST)

local BASE_PERCENT = 100
game.set_stat_bounds(STAT_CDR, 0, config.rule("cdr_cap_pct"))
game.set_stat_bounds(STAT_ATTACK_SPEED, config.rule("attack_speed_min_pct") - BASE_PERCENT, config.rule("attack_speed_max_pct") - BASE_PERCENT)

local parked_casts, last_cast_slot = {}, {}

local function summon_with_caster_ranks(caster, unit_type_index, life_ticks, x, y)
  local unit = game.timed_spawn(caster, unit_type_index, life_ticks, x, y)
  if unit == nil then return nil end
  for s = 0, ABILITY_SLOTS - 1 do
    game.set_skill_rank(unit, s, game.skill_rank(caster, s))
  end
  return unit
end

local function item_slot(caster, name)
  for _, row in ipairs(game.unit_items(caster)) do
    if row.name == name then return row.slot end
  end
  return nil
end

local function land_beside(caster, receiver)
  local rx, ry = unit_position(receiver)
  trees.clear_square(rx, ry, trees.TELEPORT_CLEAR_UNITS)
  for step = 1, RING_STEPS do
    local reach = RING_BASE_MILLI + RING_STEP_MILLI * (step - 1)
    for _, d in ipairs(RING_DIRECTIONS) do
      local x = rx + d[1] * reach / MILLI
      local y = ry + d[2] * reach / MILLI
      if game.teleport(caster, math.floor(x), math.floor(y)) then
        return true
      end
    end
  end
  return false
end

local function flag_is_set(bits, flag)
  return math.floor(bits / flag) % 2 == 1
end

local IMAGE_RING_DISTANCE = config.milli(1600)
local IMAGE_DIRECTIONS = { { 1, 0 }, { -1, 0 }, { 0, 1 }, { 0, -1 }, { 1, 1 }, { -1, -1 }, { 1, -1 }, { -1, 1 } }
local SPIRIT_LANCE_IMAGE_LIFE_BY_RANK = { 56, 80, 104, 128 }
local JUXTAPOSE_TALENT_UPGRADE_BIT = 128
local JUXTAPOSE_TALENT_TAKEN_REDUCTION_PCT = 70

local CAST_IMAGES = {
  item_manta = function() return 2, 320, 33, 350 end,
  item_manta_ranged = function() return 2, 320, 28, 400 end,
  phantom_lancer_spirit_lance = function(caster, slot)
    local rank = math.min(math.max(game.skill_rank(caster, slot), 1), #SPIRIT_LANCE_IMAGE_LIFE_BY_RANK)
    return 1, SPIRIT_LANCE_IMAGE_LIFE_BY_RANK[rank], 15, 400
  end,
}

local PROC_IMAGES = {
  phantom_lancer_juxtapose = function(caster)
    local taken_pct = 600
    if flag_is_set(game.upgrade_bits(caster), JUXTAPOSE_TALENT_UPGRADE_BIT) then
      taken_pct = taken_pct - JUXTAPOSE_TALENT_TAKEN_REDUCTION_PCT
    end
    return 1, 128, 15, taken_pct
  end,
}

local function spawn_images(caster, count, life_ticks, dealt_pct, taken_pct)
  local cx, cy = unit_position(caster)
  for n = 0, count - 1 do
    local d = IMAGE_DIRECTIONS[n % 8 + 1]
    game.spawn_image(caster, cx + d[1] * IMAGE_RING_DISTANCE, cy + d[2] * IMAGE_RING_DISTANCE, life_ticks, dealt_pct, taken_pct)
  end
end

local teleports_awaiting_channel_end = {}

local function town_portal(caster)
  local receiver = game.cast_target(caster)
  if receiver ~= nil then
    teleports_awaiting_channel_end[caster] = { receiver = receiver, channel_ticks_left = game.channel_left(caster) }
  end
end

local HANDLERS = {
  item_tpscroll = town_portal,
  item_travel_boots = town_portal,
  item_travel_boots_2 = town_portal,
}

for name in pairs(trees.CUTTERS) do
  HANDLERS[name] = function(caster) trees.cut(caster, name) end
end

local function ranked(caster)
  local slot = last_cast_slot[caster]
  return math.max(1, slot ~= nil and game.skill_rank(caster, slot) or 1)
end

HANDLERS.furion_sprout = function(caster)
  local x, y = game.cast_point(caster)
  local target = game.cast_target(caster)
  if target ~= nil and unit_alive(target) then x, y = unit_position(target) end
  trees.sprout(x, y, ranked(caster))
end
HANDLERS.item_branches = function(caster)
  local x, y = game.cast_point(caster)
  trees.branch(x, y)
end
HANDLERS.earthshaker_fissure = function(caster)
  local x, y = game.cast_point(caster)
  trees.fissure(caster, x, y)
end
HANDLERS.tusk_ice_shards = function(caster)
  local x, y = game.cast_point(caster)
  trees.ice_shards(caster, x, y)
end
HANDLERS.shredder_timber_chain = function(caster)
  local x, y = game.cast_point(caster)
  trees.chain(caster, x, y, ranked(caster))
end

local function place_ward_spending_charge(caster, ward)
  local x, y = game.cast_point(caster)
  if summon_with_caster_ranks(caster, ward.unit_type_index, ward.life_ticks, x, y) ~= nil then
    local slot = item_slot(caster, ward.item)
    if slot ~= nil then game.consume_item_charge(caster, slot) end
  end
end

for name, ward in pairs(WARDS) do
  HANDLERS[name] = function(caster) place_ward_spending_charge(caster, ward) end
end

for name, images in pairs(CAST_IMAGES) do
  HANDLERS[name] = function(caster)
    local count, life_ticks, dealt_pct, taken_pct = images(caster, last_cast_slot[caster])
    spawn_images(caster, count, life_ticks, dealt_pct, taken_pct)
  end
end

local DEBUFFS = { "slow", "silence", "root", "taunt", "disarm", "mute", "break" }

local function dispel_debuffs(unit)
  for _, status in ipairs(DEBUFFS) do
    game.clear_status(unit, status)
  end
  game.clear_dots(unit)
  game.clear_timed_mods(unit, 0)
end

HANDLERS.omniknight_repel = function(caster)
  local target = game.cast_target(caster)
  if target ~= nil and unit_alive(target) then dispel_debuffs(target) end
end

HANDLERS.vengefulspirit_nether_swap = function(caster)
  local target = game.cast_target(caster)
  if target == nil or not unit_alive(target) then return end
  local cx, cy = unit_position(caster)
  local tx, ty = unit_position(target)
  trees.clear_square(cx, cy, trees.SWAP_CLEAR_UNITS)
  trees.clear_square(tx, ty, trees.SWAP_CLEAR_UNITS)
  if not game.teleport(caster, tx, ty) then return end
  if not game.teleport(target, cx, cy) then game.teleport(caster, cx, cy) end
end

local TARGET_NONE, TARGET_UNIT, TARGET_UNIT_OR_POINT = 0, 1, 3
local FLAG_PASSIVE, FLAG_PIERCE, FLAG_ALLY_ONLY, FLAG_ENEMY_ONLY = 1, 16, 256, 512

local function held(unit, status)
  return game.status_left(unit, status) > 0
end

local function aims_at_unit(kind, target)
  return kind == TARGET_UNIT or (kind == TARGET_UNIT_OR_POINT and target >= 0 and unit_alive(target))
end

local GROUND_RETARGET = {
  zuus_lightning_bolt = { radius = 350, enemy = true, hero = true },
  treant_living_armor = { radius = 9999, enemy = false, hero = true },
  abyssal_underlord_dark_rift = { radius = 4000, enemy = false, structure = true },
}
local REFERENCE_UNITS_PER_CELL = 64

local function ground_retarget(caster, rule, x, y)
  local caster_player = unit_player(caster)
  local best, best_distance
  for _, u in ipairs(units.units_near(x, y, rule.radius * config.FIX // REFERENCE_UNITS_PER_CELL)) do
    local enemy = units.hostile(caster_player, unit_player(u)) and true or false
    local fits = (rule.hero and units.is_hero(u)) or (rule.structure and units.is_structure(u))
    if unit_alive(u) and enemy == rule.enemy and fits and (not enemy or game.visible_to(caster, u)) then
      local ux, uy = unit_position(u)
      local dx, dy = ux - x, uy - y
      local distance = dx * dx + dy * dy
      if best == nil or distance < best_distance or (distance == best_distance and u < best) then
        best, best_distance = u, distance
      end
    end
  end
  return best
end

local KELENS_DAGGER_ABILITY = "item_blink"
local KELENS_DAGGER_LOCK_TICKS = 48

local function allow_cast(caster, slot, name, index, target, x, y, is_item)
  local flags, filter, kind = game.ability_gate(index)
  if flags == nil then return true end
  if name == "item_ultimate_scepter_alchemist" and not item_forms.can_bless(caster, target) then return false end
  if name == KELENS_DAGGER_ABILITY then
    local age = game.heavy_hit_age(caster)
    if age ~= nil and age <= KELENS_DAGGER_LOCK_TICKS then return false end
  end
  if flag_is_set(flags, FLAG_PASSIVE) then return false end
  if game.channel_left(caster) > 0 then return false end
  if held(caster, "stun") then return false end
  if is_item then
    if held(caster, "mute") then return false end
  elseif held(caster, "silence") or held(caster, "hex") then
    return false
  end
  local retarget = GROUND_RETARGET[name]
  if retarget ~= nil and kind == TARGET_UNIT_OR_POINT and not is_item and not aims_at_unit(kind, target) then
    local found = ground_retarget(caster, retarget, x, y)
    if found ~= nil then game.cmd_cast(caster, slot, found, x, y) end
    return false
  end
  if not trees.can_cut(name, x, y) then return false end
  local tx, ty = x, y
  if aims_at_unit(kind, target) then
    if target < 0 or not unit_alive(target) then return true end
    if not game.visible_to(caster, target) then return false end
    if not game.meets_filter(target, filter) or held(target, "invulnerable") then return false end
    local hostile = units.hostile(unit_player(caster), unit_player(target))
    if flag_is_set(flags, FLAG_ALLY_ONLY) and hostile then return false end
    if flag_is_set(flags, FLAG_ENEMY_ONLY) and not hostile then return false end
    if not flag_is_set(flags, FLAG_PIERCE) and hostile and held(target, "spell_immunity") then return false end
    tx, ty = unit_position(target)
  end
  if kind ~= TARGET_NONE and not game.cast_in_range(caster, index, tx, ty) then
    if is_item then return false end
    parked_casts[caster] = { slot = slot, index = index, target = target, kind = kind, x = x, y = y }
    game.cmd_order(caster, tx, ty)
    return false
  end
  parked_casts[caster] = nil
  last_cast_slot[caster] = slot
  return true
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

local function walk_parked_casts()
  for _, caster in ipairs(ascending_keys(parked_casts)) do
    local p = parked_casts[caster]
    if p == nil then
      parked_casts[caster] = nil
    elseif not unit_alive(caster) or held(caster, "stun") or held(caster, "hex") then
      parked_casts[caster] = nil
    elseif p.kind == TARGET_UNIT and not unit_alive(p.target) then
      parked_casts[caster] = nil
    else
      local tx, ty = p.x, p.y
      if aims_at_unit(p.kind, p.target) then tx, ty = unit_position(p.target) end
      if game.cast_in_range(caster, p.index, tx, ty) then
        parked_casts[caster] = nil
        game.cmd_cast(caster, p.slot, p.target, tx, ty)
      else
        game.cmd_order(caster, tx, ty)
      end
    end
  end
end

local COMPLETED_CHANNEL_TICKS_LEFT_MAX = 2

local function land_finished_channels()
  for caster, p in pairs(teleports_awaiting_channel_end) do
    p.channel_ticks_left = p.channel_ticks_left - 1
    if not unit_alive(caster) then
      teleports_awaiting_channel_end[caster] = nil
    elseif game.channel_left(caster) <= 0 then
      teleports_awaiting_channel_end[caster] = nil
      if p.channel_ticks_left <= COMPLETED_CHANNEL_TICKS_LEFT_MAX and unit_alive(p.receiver) then land_beside(caster, p.receiver) end
    end
  end
end

return {
  on_cast = function(caster, ability_name)
    local handler = HANDLERS[ability_name]
    if handler ~= nil then
      handler(caster)
    end
  end,
  on_cast_request = allow_cast,
  on_tick = function()
    walk_parked_casts()
    land_finished_channels()
  end,
  on_proc = function(caster, ability_name)
    local images = PROC_IMAGES[ability_name]
    if images ~= nil then
      local count, life_ticks, dealt_pct, taken_pct = images(caster)
      spawn_images(caster, count, life_ticks, dealt_pct, taken_pct)
    end
  end,
}

-- Moba spells as mod code: the teleport landing ring and the ward caller. The engine offers generic primitives only:
-- game.teleport(unit, x, y) (move to a point if walkable, true when it took hold), game.timed_spawn(caster, kind, life,
-- x, y) (place a unit that expires after life ticks), game.cast_point(caster) / game.cast_target(caster),
-- game.unit_items(unit) (the held items with their slots) and game.consume_item_charge(unit, slot) (spend one charge).
-- on_cast dispatches a cast that took hold by its ability name. The wards are placed only here.
-- The cast gate is here too: on_cast_request decides whether a cast may reach the sim (stun, silence, mute, channelling,
-- passives, target filters, cast range; the walk to a cast out of range, on_tick), on_proc spawns the images of a
-- passive proc, and the dispel and the illusion images of the casts below are this file's rules.
local config = require("config")
local units = require("units")

-- Landing ring for the town-portal style teleport: eight directions, widening by RING_STEP_MILLI per step over
-- RING_STEPS steps, measured from the receiver in milli-units.
local RING_DIRECTIONS = {
  { 1000, 0 }, { 707, 707 }, { 0, 1000 }, { -707, 707 },
  { -1000, 0 }, { -707, -707 }, { 0, -1000 }, { 707, -707 },
}
local RING_STEPS = 6
local RING_BASE_MILLI = 1000
local RING_STEP_MILLI = 1000

-- Ward kinds, lifetimes and items: unit type index in units.osd, lifetime in ticks (16 ticks a second) and the item
-- the cast spends a charge of once the ward is placed.
local WARDS = {
  place_observer = { kind = 18, life = 5760, item = "ward_observer" },
  place_sentry = { kind = 19, life = 6720, item = "ward_sentry" },
}

-- Skill slots per unit (sim lib.rs ABILITY_SLOTS).
local ABILITY_SLOTS = 8

-- Stat codes the sim holds within the pack's bounds (sim spell.rs S_ATTACK_SPEED and S_CDR).
local STAT_ATTACK_SPEED, STAT_CDR = 12, 20

-- Status flags (sim spell.rs STATUS_*): 1 the status ends the unit's channel, 2 immunity does not turn it away, 4 status
-- resistance does not apply, 8 the caster is recorded as the unit's taunter, 16 the effect's parameter 2 is the unit's
-- speed (milli-units per tick) while the status holds, 32 the unit's passive procs do not fire while the status holds.
local ENDS_CHANNEL, PIERCES_IMMUNITY, NO_RESIST, RECORDS_SOURCE, SETS_SPEED, BLOCKS_PROCS = 1, 2, 4, 8, 16, 32
game.set_status_flags("stun", ENDS_CHANNEL + BLOCKS_PROCS)
game.set_status_flags("hex", ENDS_CHANNEL + SETS_SPEED)
game.set_status_flags("taunt", RECORDS_SOURCE)
game.set_status_flags("invisibility", PIERCES_IMMUNITY + NO_RESIST)

-- The cooldown reduction cap and the attack speed range are rule values (attr_rules.txt, read through config.rule). The
-- stat is the bonus over the base 100 percent, so the attack speed range 20 to 700 is the bonus -80 to 600.
game.set_stat_bounds(STAT_CDR, 0, config.rule("cdr_cap_pct"))
game.set_stat_bounds(STAT_ATTACK_SPEED, config.rule("attack_speed_min_pct") - 100, config.rule("attack_speed_max_pct") - 100)

-- The cast gate's state: a cast parked while its unit walks into range, and the slot of the cast that last took hold.
local pending, cast_slot = {}, {}

-- Places a timed summon for `caster` and gives it the caster's rank in each skill slot. game.timed_spawn
-- returns the placed unit, or nil when none fits. game.set_skill_rank(unit, slot, rank) is a required native:
-- it sets the rank, clamped to the ability's max rank, and skips the level gate. Returns the unit, or nil.
local function summon(caster, kind, life, x, y)
  local unit = game.timed_spawn(caster, kind, life, x, y)
  if unit == nil then return nil end
  for s = 0, ABILITY_SLOTS - 1 do
    game.set_skill_rank(unit, s, game.skill_rank(caster, s))
  end
  return unit
end

-- The inventory slot of the first held item named `name`, or nil. Each row of game.unit_items has slot and name.
local function item_slot(caster, name)
  for _, row in ipairs(game.unit_items(caster)) do
    if row.name == name then return row.slot end
  end
  return nil
end

local function land_beside(caster, receiver)
  local rx, ry = unit_position(receiver)
  for step = 1, RING_STEPS do
    local reach = RING_BASE_MILLI + RING_STEP_MILLI * (step - 1)
    for _, d in ipairs(RING_DIRECTIONS) do
      local x = rx + d[1] * reach / 1000
      local y = ry + d[2] * reach / 1000
      if game.teleport(caster, math.floor(x), math.floor(y)) then
        return true
      end
    end
  end
  return false
end

-- The dispel of the tidehunter kraken shell: the debuffs (slow, silence, root, taunt, disarm, mute, break) end, the damage
-- over time on the unit ends and its negative timed modifiers go. Stun and hex stay.
local DEBUFFS = { "slow", "silence", "root", "taunt", "disarm", "mute", "break" }

local function dispel_debuffs(caster)
  for _, status in ipairs(DEBUFFS) do
    game.clear_status(caster, status)
  end
  game.clear_dots(caster)
  game.clear_timed_mods(caster, 0)  -- 0: the negative timed modifiers (the buffs stay)
end

-- Illusion images: image n of a cast sits IMAGE_RING from the caster on direction n mod 8. Each entry gives the count,
-- the lifetime in ticks, the percent of damage the image deals and the percent it takes.
local IMAGE_RING = config.milli(1600)
local IMAGE_DIRECTIONS = { { 1, 0 }, { -1, 0 }, { 0, 1 }, { 0, -1 }, { 1, 1 }, { -1, -1 }, { 1, -1 }, { -1, 1 } }
-- The spirit lance's image lifetime by its rank (its cast slot's skill rank).
local LANCE_LIFE = { 56, 80, 104, 128 }

-- Casts that spawn images: name -> function(caster, slot) returning count, life, out, inc.
local CAST_IMAGES = {
  item_manta = function() return 2, 288, 33, 300 end,
  phantom_lancer_spirit_lance = function(caster, slot)
    local rank = math.min(math.max(game.skill_rank(caster, slot), 1), #LANCE_LIFE)
    return 1, LANCE_LIFE[rank], 15, 400
  end,
}

-- Passive procs that spawn images: name -> function(caster) returning count, life, out, inc. Talent 7 (upgrade bit 7,
-- the value 128) takes 70 off the image's percent taken.
local PROC_IMAGES = {
  phantom_lancer_juxtapose = function(caster)
    local inc = 600
    if math.floor(game.upgrade_bits(caster) / 128) % 2 == 1 then inc = inc - 70 end
    return 1, 128, 15, inc
  end,
}

local function spawn_images(caster, count, life, out, inc)
  local cx, cy = unit_position(caster)
  for n = 0, count - 1 do
    local d = IMAGE_DIRECTIONS[n % 8 + 1]
    game.spawn_image(caster, cx + d[1] * IMAGE_RING, cy + d[2] * IMAGE_RING, life, out, inc)
  end
end

local HANDLERS = {
  town_portal = function(caster)
    local receiver = game.cast_target(caster)
    if receiver ~= nil then
      land_beside(caster, receiver)
    end
  end,
}

-- A placed ward spends one charge of its item; a ward that does not fit keeps it.
for name, ward in pairs(WARDS) do
  HANDLERS[name] = function(caster)
    local x, y = game.cast_point(caster)
    if summon(caster, ward.kind, ward.life, x, y) ~= nil then
      local slot = item_slot(caster, ward.item)
      if slot ~= nil then game.consume_item_charge(caster, slot) end
    end
  end
end

HANDLERS.tidehunter_kraken_shell = dispel_debuffs

for name, images in pairs(CAST_IMAGES) do
  HANDLERS[name] = function(caster)
    local count, life, out, inc = images(caster, cast_slot[caster])
    spawn_images(caster, count, life, out, inc)
  end
end

local TARGET_NONE, TARGET_UNIT = 0, 1
local FLAG_PASSIVE, FLAG_PIERCE, FLAG_ALLY_ONLY, FLAG_ENEMY_ONLY = 1, 16, 256, 512

-- True when flag `flag` is set in `bits` (the flags are powers of two).
local function has(bits, flag)
  return math.floor(bits / flag) % 2 == 1
end

local function held(unit, status)
  return game.status_left(unit, status) > 0
end

-- on_cast_request(caster, slot, name, index, target, x, y, is_item): true lets the cast reach the sim, false refuses it.
-- The engine refuses the rest (a dead caster, an empty slot, a rank with no cost, cooldown, mana, a dead unit target). An
-- ability out of cast range is parked and its unit walks to the target (walk_pending casts it on arrival); an item out
-- of range is refused. Silence and hex stop abilities, mute stops items, stun stops both.
local function allow_cast(caster, slot, name, index, target, x, y, is_item)
  local flags, filter, kind = game.ability_gate(index)
  if flags == nil then return true end
  if has(flags, FLAG_PASSIVE) then return false end
  if game.channel_left(caster) > 0 then return false end
  if held(caster, "stun") then return false end
  if is_item then
    if held(caster, "mute") then return false end
  elseif held(caster, "silence") or held(caster, "hex") then
    return false
  end
  local tx, ty = x, y
  if kind == TARGET_UNIT then
    if target < 0 or not unit_alive(target) then return true end
    if not game.visible_to(caster, target) then return false end
    if not game.meets_filter(target, filter) or held(target, "invulnerable") then return false end
    local hostile = units.hostile(unit_player(caster), unit_player(target))
    if has(flags, FLAG_ALLY_ONLY) and hostile then return false end
    if has(flags, FLAG_ENEMY_ONLY) and not hostile then return false end
    if not has(flags, FLAG_PIERCE) and hostile and held(target, "spell_immunity") then return false end
    tx, ty = unit_position(target)
  end
  if kind ~= TARGET_NONE and not game.cast_in_range(caster, index, tx, ty) then
    if is_item then return false end
    pending[caster] = { slot = slot, index = index, target = target, kind = kind, x = x, y = y }
    game.cmd_order(caster, tx, ty)
    return false
  end
  pending[caster] = nil
  cast_slot[caster] = slot
  return true
end

-- Each parked cast walks its unit toward the target and is cast once the unit stands in range. A stun or hex, a dead
-- caster or a dead target drops it.
local function walk_pending()
  local casters = {}
  -- The sandbox has no table library, so caster ids are kept in ascending order by insertion: parked casts issue
  -- their commands in the same order on every peer.
  for caster in pairs(pending) do
    local j = #casters + 1
    while j > 1 and casters[j - 1] > caster do
      casters[j] = casters[j - 1]
      j = j - 1
    end
    casters[j] = caster
  end
  for _, caster in ipairs(casters) do
    local p = pending[caster]
    if p == nil then
      pending[caster] = nil
    elseif not unit_alive(caster) or held(caster, "stun") or held(caster, "hex") then
      pending[caster] = nil
    elseif p.kind == TARGET_UNIT and not unit_alive(p.target) then
      pending[caster] = nil
    else
      local tx, ty = p.x, p.y
      if p.kind == TARGET_UNIT then tx, ty = unit_position(p.target) end
      if game.cast_in_range(caster, p.index, tx, ty) then
        pending[caster] = nil
        game.cmd_cast(caster, p.slot, p.target, tx, ty)
      else
        game.cmd_order(caster, tx, ty)
      end
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
  on_tick = walk_pending,
  on_proc = function(caster, ability_name)
    local images = PROC_IMAGES[ability_name]
    if images ~= nil then
      local count, life, out, inc = images(caster)
      spawn_images(caster, count, life, out, inc)
    end
  end,
}

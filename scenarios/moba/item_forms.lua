local config = require("config")
local bounty = require("bounty")
local units = require("units")

local TPS = config.TICKS_PER_SECOND
local ARMLET_DRAIN_Q16 = 163840
local RADIANCE_REACH_Q16 = config.milli(11328)
local BLIND_MISS_Q16 = 11141
local TRANQUIL_BREAK_TICKS = 13 * TPS
local HEART_RANGED_TICKS = 7 * TPS
local HEART_MELEE_TICKS = 5 * TPS
local LINKEN_RECHARGE_TICKS = 16 * TPS
local TRANSFER_ABILITY = "item_sphere_transfer"
local BKB_FIRST_STORE = 9
local BKB_LAST_STORE = 4
local ALCHEMIST_SCEPTER = "item_ultimate_scepter_alchemist"
local ALCHEMIST_SCEPTER_ITEM = "ultimate_scepter_alchemist"
local SCEPTER_BIT = 0
local STAT_HEALTH, STAT_STRENGTH, STAT_AGILITY, STAT_INTELLIGENCE, STAT_MAX_MANA = 2, 22, 23, 24, 15
local BLESS_ATTRIBUTES, BLESS_HEALTH, BLESS_MANA = 10, 200, 150

local forms = {}

local RANGED_TWIN = {
  stout_shield = "stout_shield_ranged",
  poor_mans_shield = "poor_mans_shield_ranged",
  vanguard = "vanguard_ranged",
  orb_of_venom = "orb_of_venom_ranged",
  skadi = "skadi_ranged",
  quelling_blade = "quelling_blade_ranged",
  manta = "manta_ranged",
}
local MELEE_OF = {}
for melee, ranged in pairs(RANGED_TWIN) do MELEE_OF[ranged] = melee end

local COURIER_TWIN = { gem = "gem_courier", armlet = "armlet_courier", armlet_on = "armlet_courier" }
local HERO_OF_COURIER = { gem_courier = "gem", armlet_courier = "armlet" }

local BKB_FORM_AT_STORE = {
  [0] = "black_king_bar", [9] = "black_king_bar_9", [8] = "black_king_bar_8",
  [7] = "black_king_bar_7", [6] = "black_king_bar_6", [5] = "black_king_bar_5",
}
local BKB_ITEM = {
  black_king_bar = true, black_king_bar_9 = true, black_king_bar_8 = true,
  black_king_bar_7 = true, black_king_bar_6 = true, black_king_bar_5 = true,
}
local BKB_ABILITY = {
  item_black_king_bar = true, item_black_king_bar_9 = true, item_black_king_bar_8 = true,
  item_black_king_bar_7 = true, item_black_king_bar_6 = true, item_black_king_bar_5 = true,
}

forms.ALIASES = {
  stout_shield = { "stout_shield_ranged" },
  poor_mans_shield = { "poor_mans_shield_ranged" },
  vanguard = { "vanguard_ranged" },
  orb_of_venom = { "orb_of_venom_ranged" },
  skadi = { "skadi_ranged" },
  quelling_blade = { "quelling_blade_ranged" },
  manta = { "manta_ranged" },
  diffusal_blade = { "diffusal_blade_empty" },
  diffusal_blade_2 = { "diffusal_blade_2_empty" },
  ancient_janggo = { "ancient_janggo_empty" },
  ring_of_basilius = { "ring_of_basilius_normal" },
  ring_of_aquila = { "ring_of_aquila_normal" },
  tranquil_boots = { "tranquil_boots_broken" },
  heart = { "heart_disabled" },
  sphere = { "sphere_recharging" },
  power_treads = { "power_treads_strength", "power_treads_intelligence" },
  armlet = { "armlet_on", "armlet_courier" },
  radiance = { "radiance_off" },
  monkey_king_bar = { "monkey_king_bar_inactive" },
  black_king_bar = { "black_king_bar_9", "black_king_bar_8", "black_king_bar_7", "black_king_bar_6", "black_king_bar_5" },
  rapier = { "rapier_undroppable" },
  gem = { "gem_courier" },
}

forms.ALT_PART = {
  power_treads = {
    boots_of_elves = {
      { part = "belt_of_strength", state = 1 },
      { part = "robe", state = 2 },
    },
  },
}

local TREADS_STATE_FORM = { [1] = "power_treads_strength", [2] = "power_treads_intelligence" }

function forms.note_treads_state(unit, state)
  game.set_counter(unit, "treads_state", state)
end

forms.RECHARGE = {
  ancient_janggo = { copy = "ancient_janggo_empty", recipe_cost = 875 },
}

function forms.held_alias(base, named, pool)
  for _, alias in ipairs(forms.ALIASES[base] or {}) do
    local index = named[alias]
    if index ~= nil and (pool[index] or 0) > 0 then return index end
  end
  return nil
end

local seen_generation = {}
local info = {}
local couriers = {}
local pending_rapier = {}

local function is_ranged(unit)
  local kind = game.unit_info(game.unit_kind(unit))
  return kind ~= nil and not kind.melee
end

local function ranged_form_wanted(unit, melee_name)
  if not units.is_hero(unit) then return false end
  if melee_name == "manta" and game.unit_kind(unit) == "terrorblade" then return true end
  return is_ranged(unit)
end

local function swap(unit, row, name)
  game.cmd_replace_item(unit, row.slot, name, row.count)
end

local function bkb_expected(unit)
  return BKB_FORM_AT_STORE[game.counter(unit, "bkb_store")]
end

local function corrected_form(unit, row)
  local name = row.name
  local hero = units.is_hero(unit)
  local melee = MELEE_OF[name]
  if melee ~= nil then
    if not ranged_form_wanted(unit, melee) then return melee end
    return nil
  end
  local ranged = RANGED_TWIN[name]
  if ranged ~= nil and ranged_form_wanted(unit, name) then return ranged end
  if hero then
    local base = HERO_OF_COURIER[name]
    if base ~= nil then return base end
  else
    local twin = COURIER_TWIN[name]
    if twin ~= nil then return twin end
  end
  if BKB_ITEM[name] and hero then
    local expected = bkb_expected(unit)
    if expected ~= nil and expected ~= name then return expected end
  end
  if name == "power_treads" then
    local state = game.counter(unit, "treads_state")
    if state ~= 0 then
      game.set_counter(unit, "treads_state", 0)
      return TREADS_STATE_FORM[state]
    end
  end
  if name == "rapier" then
    local owner = pending_rapier[unit]
    if owner ~= nil then
      pending_rapier[unit] = nil
      game.cmd_set_item_owner(unit, row.slot, owner)
    else
      owner = row.owner
    end
    if owner ~= nil and units.hostile(owner, unit_player(unit)) then return "rapier_undroppable" end
  end
  return nil
end

local function rescan(unit)
  local mine = { armlet_on = false, heart = false, tranquil = false, radiance = false }
  for _, row in ipairs(game.unit_items(unit)) do
    local target = corrected_form(unit, row)
    if target ~= nil then
      swap(unit, row, target)
    else
      local name = row.name
      if name == "armlet_on" then mine.armlet_on = true end
      if name == "heart" or name == "heart_disabled" then mine.heart = true end
      if name == "tranquil_boots" or name == "tranquil_boots_broken" then mine.tranquil = true end
      if name == "radiance" then mine.radiance = true end
    end
  end
  info[unit] = mine
  seen_generation[unit] = game.unit_item_gen(unit)
end

local function rows_named(unit, wanted)
  local out = {}
  for _, row in ipairs(game.unit_items(unit)) do
    if wanted[row.name] then out[#out + 1] = row end
  end
  return out
end

local HEART = { heart = true }
local HEART_OFF = { heart_disabled = true }
local TRANQUIL = { tranquil_boots = true }
local TRANQUIL_OFF = { tranquil_boots_broken = true }
local SPHERE_READY = { sphere = true }
local SPHERE_RECHARGING = { sphere_recharging = true }

local function from_heroes_side(hitter)
  if game.unit_kind(hitter) == "roshan" then return true end
  return bounty.hero_of(unit_player(hitter)) ~= nil
end

local function hit_recently(unit, window, qualifies)
  for _, hitter in ipairs(game.recent_hitters(unit, window)) do
    if units.hostile(unit_player(hitter), unit_player(unit)) and (qualifies == nil or qualifies(hitter)) then return true end
  end
  return false
end

local function tick_heart(unit)
  local window = is_ranged(unit) and HEART_RANGED_TICKS or HEART_MELEE_TICKS
  local disabled = hit_recently(unit, window, from_heroes_side)
  local target = disabled and "heart_disabled" or "heart"
  for _, row in ipairs(rows_named(unit, disabled and HEART or HEART_OFF)) do
    game.cmd_replace_item(unit, row.slot, target, row.count)
  end
end

local function tick_tranquil(unit, now)
  local target_unit = game.strike_of(unit)
  if target_unit >= 0 and game.can_strike(unit, target_unit) then
    game.set_counter(unit, "tranquil_until", now + TRANQUIL_BREAK_TICKS)
  end
  local broken = game.counter(unit, "tranquil_until") > now or hit_recently(unit, TRANQUIL_BREAK_TICKS)
  local target = broken and "tranquil_boots_broken" or "tranquil_boots"
  for _, row in ipairs(rows_named(unit, broken and TRANQUIL or TRANQUIL_OFF)) do
    game.cmd_replace_item(unit, row.slot, target, row.count)
  end
end

local function drain_armlet(unit)
  if not unit_alive(unit) then return end
  game.set_hp(unit, unit_hp(unit) - ARMLET_DRAIN_Q16)
end

local function blind_enemies(unit, now)
  local x, y = unit_position(unit)
  local until_tick = now + TPS + 4
  for _, other in ipairs(game.units_near(x, y, RADIANCE_REACH_Q16)) do
    if other ~= unit and unit_alive(other) and units.hostile(unit_player(unit), unit_player(other)) and not units.is_structure(other) then
      game.set_counter(other, "blind_until", until_tick)
    end
  end
end

local function absorb_blocks(now)
  for _, block in ipairs(game.take_blocks()) do
    local target = block.target
    if unit_alive(target) then
      for _, row in ipairs(rows_named(target, SPHERE_READY)) do
        game.cmd_replace_item(target, row.slot, "sphere_recharging", row.count)
        game.set_counter(target, "linken_due", now + LINKEN_RECHARGE_TICKS)
        break
      end
    end
  end
end

local function restore_sphere(unit, now)
  local due = game.counter(unit, "linken_due")
  if due == 0 or now < due then return end
  game.set_counter(unit, "linken_due", 0)
  for _, row in ipairs(rows_named(unit, SPHERE_RECHARGING)) do
    game.cmd_replace_item(unit, row.slot, "sphere", row.count)
  end
end

function forms.blind_miss(unit)
  if game.counter(unit, "blind_until") > game.ticks() then return BLIND_MISS_Q16 end
  return 0
end

function forms.can_bless(caster, target)
  if target == nil or target < 0 or not unit_alive(target) or not units.is_hero(target) then return false end
  if units.hostile(unit_player(caster), unit_player(target)) then return false end
  return game.counter(target, "scepter_blessed") == 0
end

function forms.can_transfer(caster, target)
  if target == nil or target < 0 or target == caster or not unit_alive(target) or not units.is_hero(target) then return false end
  return not units.hostile(unit_player(caster), unit_player(target))
end

local function transfer_block(caster)
  local target = game.cast_target(caster)
  if not forms.can_transfer(caster, target) then return end
  for _, row in ipairs(rows_named(caster, SPHERE_READY)) do
    local now = game.ticks()
    game.cmd_replace_item(caster, row.slot, "sphere_recharging", row.count)
    game.set_counter(caster, "linken_due", now + LINKEN_RECHARGE_TICKS)
    game.set_spell_block(target, LINKEN_RECHARGE_TICKS)
    return
  end
end

local function bless(caster)
  local target = game.cast_target(caster)
  if not forms.can_bless(caster, target) then return end
  game.set_counter(target, "scepter_blessed", 1)
  game.set_upgrade_bit(target, SCEPTER_BIT, true)
  game.add_stat(target, STAT_STRENGTH, BLESS_ATTRIBUTES)
  game.add_stat(target, STAT_AGILITY, BLESS_ATTRIBUTES)
  game.add_stat(target, STAT_INTELLIGENCE, BLESS_ATTRIBUTES)
  game.add_stat(target, STAT_HEALTH, BLESS_HEALTH)
  game.add_stat(target, STAT_MAX_MANA, BLESS_MANA)
  for _, row in ipairs(game.unit_items(caster)) do
    if row.name == ALCHEMIST_SCEPTER_ITEM then
      game.consume_item_charge(caster, row.slot)
      return
    end
  end
end

local function bkb_cast(caster)
  local store = game.counter(caster, "bkb_store")
  local seconds
  if store == 0 then
    store = BKB_FIRST_STORE
    seconds = store + 1
  else
    store = math.max(store - 1, BKB_LAST_STORE)
    seconds = store + 1
  end
  game.set_counter(caster, "bkb_store", store)
  game.clear_status(caster, "spell_immunity")
  game.apply_status(caster, "spell_immunity", seconds * TPS)
end

function forms.rapier_dropped(id, owner)
  game.var("rapier_owner_" .. id, owner)
end

function forms.rapier_picked(taker, drop)
  local owner = game.var("rapier_owner_" .. drop.id)
  if owner ~= nil then pending_rapier[taker] = owner end
end

local function track(unit)
  local kind = game.unit_kind(unit)
  if require("courier_kinds").set[kind] then couriers[#couriers + 1] = unit end
end

local function live_couriers()
  local kept = {}
  for _, unit in ipairs(couriers) do
    if unit_alive(unit) then kept[#kept + 1] = unit end
  end
  couriers = kept
  return kept
end

local function watch(unit, now)
  if seen_generation[unit] ~= game.unit_item_gen(unit) then rescan(unit) end
  local mine = info[unit]
  if mine == nil then return end
  if mine.armlet_on then drain_armlet(unit) end
  if mine.heart then tick_heart(unit) end
  if mine.tranquil then tick_tranquil(unit, now) end
  if mine.radiance and now % TPS == 0 then blind_enemies(unit, now) end
  restore_sphere(unit, now)
end

forms.on_state_replaced = function()
  seen_generation, info, couriers, pending_rapier = {}, {}, {}, {}
end

forms.on_spawned = track

forms.on_cast = function(caster, ability_name)
  if ability_name == ALCHEMIST_SCEPTER then bless(caster) end
  if ability_name == TRANSFER_ABILITY then transfer_block(caster) end
  if BKB_ABILITY[ability_name] and units.is_hero(caster) then bkb_cast(caster) end
end

forms.on_tick = function()
  local now = game.ticks()
  absorb_blocks(now)
  for _, hero in ipairs(bounty.heroes()) do
    if unit_alive(hero) then watch(hero, now) end
  end
  for _, courier in ipairs(live_couriers()) do
    if seen_generation[courier] ~= game.unit_item_gen(courier) then rescan(courier) end
  end
end

return forms

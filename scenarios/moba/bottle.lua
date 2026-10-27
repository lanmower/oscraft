local config = require("config")
local bounty = require("bounty")
local protection = require("protection")
local units = require("units")

local TPS = config.TICKS_PER_SECOND

local FULL, EMPTY, BOUNTY = "bottle", "bottle_empty", "bottle_bounty"
local LOAN_FULL, LOAN_EMPTY = "bottle_loan", "bottle_empty_loan"
local FULL_CHARGES = 3
local STORE_TICKS = 120 * TPS
local FOUNTAIN = "fountain"
local USE_REACH_MILLI = 3906 + 234
local AUTOPICKUP_OFF = "bottle_autopickup_off"
local REGEN_UNTIL, REGEN_HP, REGEN_MANA, SLOW = "bottle_regen_until", "bottle_regen_hp", "bottle_regen_mana", "bottle_slow"
local S_MANA_REGEN, S_HEALTH_REGEN, S_SPEED_PCT = 13, 14, 19
local TOGGLE_SECONDS = 10
local WARN_SECONDS = 8
local WARN_TEXT = "Bottled rune will be activated in 20 seconds"
local SLOW_SCAN_EVERY = 4
local BORROW_SCAN_EVERY = 2
local COURIERS = require("courier_kinds").set
local RUNE_ITEM = { "bottle_illusion", "bottle_haste", "bottle_double_damage", "bottle_regeneration", "bottle_invisibility" }
local LOAN_OF = { [FULL] = LOAN_FULL, [EMPTY] = LOAN_EMPTY, [BOUNTY] = BOUNTY .. "_loan" }
for _, name in ipairs(RUNE_ITEM) do LOAN_OF[name] = name .. "_loan" end
local BASE_OF = {}
for base, loan in pairs(LOAN_OF) do BASE_OF[loan] = base end
local HOLDS_RUNE = {}
for base, loan in pairs(LOAN_OF) do
  if base ~= FULL and base ~= EMPTY then
    HOLDS_RUNE[base] = true
    HOLDS_RUNE[loan] = true
  end
end

local stored = {}
local approaching = {}
local use_rune = nil
local capture = nil
local cfg = nil

local bottle = {}

function bottle.on_drink(fn) use_rune = fn end

function bottle.on_capture(fn) capture = fn end

local function settings()
  if cfg == nil then
    local sip_s = config.rule("bottle_sip_s")
    cfg = {
      sip_ticks = sip_s * TPS,
      regen_hp_x100 = math.floor(config.rule("bottle_sip_hp") * 100 / sip_s),
      regen_mana_x100 = math.floor(config.rule("bottle_sip_mana") * 100 / sip_s),
      warn_ticks = config.rule("bottle_warn_s") * TPS,
      slow_pct = config.rule("bottle_slow_pct"),
      share_reach = config.milli(config.rule("bottle_share_reach_milli")),
    }
  end
  return cfg
end

local function is_loan(name) return BASE_OF[name] ~= nil end

local function set_bonus(unit, counter, code, target)
  local previous = game.counter(unit, counter)
  if target ~= previous then
    game.add_stat(unit, code, target - previous)
    game.set_counter(unit, counter, target)
  end
end

local function rows_where(unit, wanted)
  local out = {}
  for _, row in ipairs(game.unit_items(unit)) do
    if wanted(row.name) then out[#out + 1] = row end
  end
  return out
end

local function used_row(list)
  local best = list[1]
  for _, row in ipairs(list) do
    if row.cooldown > best.cooldown then best = row end
  end
  return best
end

local function swap(unit, row, name, count)
  game.cmd_replace_item(unit, row.slot, name, count)
end

local function apply_regen(unit, now)
  local c = settings()
  local running = game.counter(unit, REGEN_UNTIL) > now
  set_bonus(unit, REGEN_HP, S_HEALTH_REGEN, running and c.regen_hp_x100 or 0)
  set_bonus(unit, REGEN_MANA, S_MANA_REGEN, running and c.regen_mana_x100 or 0)
end

local function start_regen(unit, now)
  if game.counter(unit, REGEN_UNTIL) > now then return end
  game.set_counter(unit, REGEN_UNTIL, now + settings().sip_ticks)
  apply_regen(unit, now)
end

local function drink_water(caster, now)
  local list = rows_where(caster, function(name) return name == FULL or name == LOAN_FULL end)
  if #list == 0 then return end
  local row = used_row(list)
  start_regen(caster, now)
  if row.count >= FULL_CHARGES and protection.in_fountain(caster) then return end
  if row.count > 1 then
    game.cmd_set_item_charges(caster, row.slot, row.count - 1)
  else
    swap(caster, row, is_loan(row.name) and LOAN_EMPTY or EMPTY, 1)
  end
end

local function full_form(row)
  return is_loan(row.name) and LOAN_FULL or FULL
end

local function release(unit, row, entry, now)
  swap(unit, row, full_form(row), FULL_CHARGES)
  if use_rune ~= nil then use_rune(unit, entry.power, entry.first, now) end
end

local function drink_rune(caster, now)
  local list = rows_where(caster, function(name) return HOLDS_RUNE[name] end)
  if #list == 0 then return end
  local row = used_row(list)
  local entries = stored[caster] or {}
  for i, entry in ipairs(entries) do
    if entry.item == row.name then
      table.remove(entries, i)
      release(caster, row, entry, now)
      return
    end
  end
  swap(caster, row, full_form(row), FULL_CHARGES)
end

local function emptiest(unit)
  local best, rank = nil, nil
  for _, row in ipairs(game.unit_items(unit)) do
    local drinks = nil
    if row.name == EMPTY or row.name == LOAN_EMPTY then
      drinks = 0
    elseif row.name == FULL or row.name == LOAN_FULL then
      drinks = row.count
    end
    if drinks ~= nil and (best == nil or drinks < rank) then best, rank = row, drinks end
  end
  return best
end

function bottle.store(unit, power, first, now, forced)
  local best = emptiest(unit)
  if best == nil then return false end
  if not forced and game.counter(unit, AUTOPICKUP_OFF) ~= 0 then
    swap(unit, best, full_form(best), FULL_CHARGES)
    return false
  end
  local item = power ~= nil and RUNE_ITEM[power] or BOUNTY
  if is_loan(best.name) then item = LOAN_OF[item] end
  swap(unit, best, item, 1)
  local entries = stored[unit] or {}
  entries[#entries + 1] = { item = item, power = power, first = first, since = now, due = now + STORE_TICKS, warned = false }
  stored[unit] = entries
  return true
end

local function expire_runes(hero, now)
  local entries = stored[hero]
  if entries == nil then return end
  local kept = {}
  for _, entry in ipairs(entries) do
    local rows = rows_where(hero, function(name) return name == entry.item end)
    if #rows == 0 then
      if now - entry.since <= 8 then kept[#kept + 1] = entry end
    elseif now >= entry.due and unit_alive(hero) then
      release(hero, rows[1], entry, now)
    else
      if not entry.warned and now >= entry.due - settings().warn_ticks then
        entry.warned = true
        game.message(unit_player(hero), WARN_TEXT, WARN_SECONDS)
      end
      kept[#kept + 1] = entry
    end
  end
  stored[hero] = #kept > 0 and kept or nil
end

local function refill(hero)
  for _, row in ipairs(game.unit_items(hero)) do
    if row.name == EMPTY then
      swap(hero, row, FULL, FULL_CHARGES)
    elseif row.name == FULL and row.count < FULL_CHARGES then
      game.cmd_set_item_charges(hero, row.slot, FULL_CHARGES)
    end
  end
end

local function refill_in_fountain(hero)
  if not unit_alive(hero) or not protection.in_fountain(hero) then return end
  refill(hero)
end

local function within_reach(hero, fountain)
  local hx, hy = unit_position(hero)
  local fx, fy = unit_position(fountain)
  local reach = config.milli(USE_REACH_MILLI)
  local dx, dy = hx - fx, hy - fy
  return dx * dx + dy * dy <= reach * reach
end

local function fountain_target(hero, target)
  return game.alive(target) and game.unit_kind(target) == FOUNTAIN and not units.hostile(unit_player(hero), unit_player(target))
end

local function approach(hero, fountain)
  local fx, fy = unit_position(fountain)
  approaching[hero] = fountain
  game.issue_order(hero, 0, fx, fy)
end

local function settle_approach(hero)
  local fountain = approaching[hero]
  if fountain == nil then return end
  if not unit_alive(hero) or not game.alive(fountain) then
    approaching[hero] = nil
  elseif within_reach(hero, fountain) then
    approaching[hero] = nil
    refill(hero)
  end
end

bottle.on_item_use = function(unit, _, item_name, target_kind, target)
  if target_kind ~= "unit" or (item_name ~= EMPTY and item_name ~= FULL) or not fountain_target(unit, target) then return end
  if within_reach(unit, target) then
    approaching[unit] = nil
    refill(unit)
  else
    approach(unit, target)
  end
end

local function holds_unfilled_bottle(unit)
  for _, row in ipairs(game.unit_items(unit)) do
    if row.name == EMPTY or row.name == LOAN_EMPTY or (row.name == FULL and row.count < FULL_CHARGES) then return true end
  end
  return false
end

local function share_dose(unit, row, owner_hero, now)
  start_regen(unit, now)
  game.cmd_set_item_charges(unit, row.slot, 0)
  if row.count > 1 then
    game.cmd_give_item(owner_hero, FULL, row.count - 1)
  else
    game.cmd_give_item(owner_hero, EMPTY, 1)
  end
end

local function owner_is_near(unit, owner_hero)
  local ox, oy = unit_position(owner_hero)
  local ux, uy = unit_position(unit)
  local reach = settings().share_reach
  return (ox - ux) * (ox - ux) + (oy - uy) * (oy - uy) <= reach * reach
end

local function borrow_scan(unit, now)
  local mine = unit_player(unit)
  for _, row in ipairs(game.unit_items(unit)) do
    local owner = row.owner
    if owner ~= nil then
      local gone = units.left[owner] == true
      if is_loan(row.name) then
        if owner == mine or gone then swap(unit, row, BASE_OF[row.name], row.count) end
      elseif LOAN_OF[row.name] ~= nil and owner ~= mine and not gone then
        local owner_hero = bounty.hero_of(owner)
        local shares = row.name == FULL and owner_hero ~= nil and unit_alive(owner_hero) and units.is_hero(unit) and owner_is_near(unit, owner_hero)
        if shares then
          share_dose(unit, row, owner_hero, now)
        else
          swap(unit, row, LOAN_OF[row.name], row.count)
        end
      end
    end
  end
end

local function courier_scan(now)
  for _, unit in ipairs(game.units(nil)) do
    if COURIERS[game.unit_kind(unit)] then
      set_bonus(unit, SLOW, S_SPEED_PCT, holds_unfilled_bottle(unit) and -settings().slow_pct or 0)
      borrow_scan(unit, now)
    end
  end
end

bottle.on_cast = function(caster, ability_name)
  if ability_name == "item_bottle" then
    drink_water(caster, game.time())
  elseif ability_name == "item_bottle_rune" then
    drink_rune(caster, game.time())
  elseif ability_name == "item_bottle_capture" and capture ~= nil then
    capture(caster, game.time())
  end
end

bottle.on_tick = function()
  local now = game.time()
  for _, hero in ipairs(bounty.heroes()) do
    expire_runes(hero, now)
    settle_approach(hero)
    if game.counter(hero, REGEN_UNTIL) ~= 0 then
      apply_regen(hero, now)
      if game.counter(hero, REGEN_UNTIL) <= now then game.set_counter(hero, REGEN_UNTIL, 0) end
    end
    if now % TPS == 0 then refill_in_fountain(hero) end
    if now % BORROW_SCAN_EVERY == 0 and unit_alive(hero) then borrow_scan(hero, now) end
  end
  if now % SLOW_SCAN_EVERY == 0 then courier_scan(now) end
end

bottle.on_died = function(unit)
  if game.counter(unit, REGEN_UNTIL) == 0 then return end
  game.set_counter(unit, REGEN_UNTIL, 0)
  apply_regen(unit, game.time())
end

bottle.on_action = function(name, args)
  if name ~= "chat" or args.cmd ~= "-bottle" or args.player == nil then return end
  local hero = bounty.hero_of(args.player)
  if hero == nil then return end
  local off = game.counter(hero, AUTOPICKUP_OFF) == 0
  game.set_counter(hero, AUTOPICKUP_OFF, off and 1 or 0)
  game.message(args.player, off and "Autopickup runes into Bottle turned OFF" or "Autopickup runes into Bottle turned ON", TOGGLE_SECONDS)
end

return bottle

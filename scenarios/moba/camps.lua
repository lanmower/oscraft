local config = require("config")
local units = require("units")
local respawn = require("respawn")
local announce = require("announce")
local runes_power = require("runes")
local modes = require("modes")
local bounty = require("bounty")
local camp_data = require("camps_688g")
local bottle = require("bottle")
local item_forms = require("item_forms")

local TPS = config.TICKS_PER_SECOND

local cfg = nil
local ready = false
local reg = {}
local runes = {}
local last_round = 0
local power_kind = -1
local aegis_life = nil
local camps = {}
local seed = 2654435761

local function read_config()
  local c = {
    leash_milli = config.rule("camp_leash_milli"),
    first_spawn_s = config.rule("camp_first_spawn_s"),
    respawn_s = config.rule("camp_respawn_s"),
    rune_period_s = config.rule("rune_period_s"),
    pickup_milli = config.rule("rune_pickup_milli"),
    rune_first_gold = config.rule("rune_first_gold"),
    rune_gold_base = config.rule("rune_gold_base"),
    rune_gold_per_min = config.rule("rune_gold_per_min"),
    rune_xp_base = config.rule("rune_xp_base"),
    rune_xp_per_min = config.rule("rune_xp_per_min"),
    roshan_min_s = config.rule("roshan_respawn_min_s"),
    roshan_spread_s = config.rule("roshan_respawn_spread_s"),
    roshan_growth_s = config.rule("roshan_growth_s"),
    roshan_hp_per_growth = config.rule("roshan_hp_per_growth"),
    roshan_armour_x100_per_growth = config.rule("roshan_armour_x100_per_growth"),
    roshan_armour_growth_cap = config.rule("roshan_armour_growth_cap"),
    roshan_team_gold = config.rule("roshan_team_gold"),
    roshan_cheese_from_kill = config.rule("roshan_cheese_from_kill"),
    roshan_leash_check_s = config.rule("roshan_leash_check_s"),
    aegis_warn_s = config.rule("aegis_warn_s"),
    aegis_expire_s = config.rule("aegis_expire_s"),
  }
  c.roshan_leash = {
    cx = config.milli(config.rule("roshan_leash_cx_milli")), cy = config.milli(config.rule("roshan_leash_cy_milli")),
    hx = config.milli(config.rule("roshan_leash_hx_milli")), hy = config.milli(config.rule("roshan_leash_hy_milli")),
  }
  c.leash_fixed = config.milli(c.leash_milli)
  c.pickup_fixed = config.milli(c.pickup_milli)
  return c
end

local function next_mark(now, interval_s)
  local iv = interval_s * TPS
  return (math.floor(now / iv) + 1) * iv
end

local function roll(span)
  seed = seed * 6364136223846793005 + 1442695040888963407
  return (seed >> 33) % (span + 1)
end

local POLICY = {
  rune = { free = true, due = function() return nil end },
  roshan = { free = true, due = function(now) return now + cfg.roshan_min_s * TPS + roll(cfg.roshan_spread_s * TPS) end },
}

local WAR3_UNIT = math.floor(config.FIX / 64)
local OFFSET_MIN, OFFSET_MAX = 25, 60

local function load_camps(now)
  for index, d in ipairs(camp_data) do
    local w, h = d.check[3] - d.check[1], d.check[4] - d.check[2]
    camps[#camps + 1] = {
      id = d.id, options = d.options, facing = d.facing, last = 0, slots = {},
      x1 = config.milli(d.check[1]), y1 = config.milli(d.check[2]), x2 = config.milli(d.check[3]), y2 = config.milli(d.check[4]),
      cx = config.milli(d.centre[1]), cy = config.milli(d.centre[2]),
      reach = config.milli(math.floor((w + h) / 2) + 1),
      stagger = index - 1,
      next_pass = now + cfg.first_spawn_s * TPS + index - 1,
    }
  end
end

local function camp_of(x, y)
  local best, best_d = nil, nil
  for _, c in ipairs(camps) do
    local dx, dy = x - c.cx, y - c.cy
    local d = dx * dx + dy * dy
    if best_d == nil or d < best_d then best, best_d = c, d end
  end
  return best
end

local function setup(now)
  cfg = read_config()
  load_camps(now)
  last_round = math.floor(now / (cfg.rune_period_s * TPS))
  for k = 0, get_unit_count() - 1 do
    local name = game.unit_kind(k)
    if name ~= nil and game.is_spawn_group_unit(k) and unit_alive(k) then
      local x, y = unit_position(k)
      local r = { name = name, hx = x, hy = y, free = false, taken = false }
      if name == "roshan" then
        r.kind = "boss"
        r.free = true
        r.policy = POLICY.roshan
        r.leash_outside = true
        r.next_leash_check = now + cfg.roshan_leash_check_s * TPS
      elseif name == "bounty_rune" then
        r.kind = "rune"
        r.free = true
        r.policy = POLICY.rune
        r.first = true
        r.slot = #runes + 1
        runes[r.slot] = k
      else
        r.kind = "camp"
        r.camp = camp_of(x, y)
        r.camp.slots[#r.camp.slots + 1] = k
        game.cmd_remove(k)
        game.cmd_set_respawn(k, false)
      end
      reg[k] = r
    end
  end
end

local STACK_EXEMPT = { observer_ward = true, sentry_ward = true }

local function camp_blocked(c)
  for _, u in ipairs(units.units_near(c.cx, c.cy, c.reach)) do
    local r = reg[u]
    if unit_alive(u) and (r == nil or r.kind == "camp") and not STACK_EXEMPT[game.unit_kind(u)] then
      local x, y = unit_position(u)
      if x >= c.x1 and x <= c.x2 and y >= c.y1 and y <= c.y2 then return true end
    end
  end
  return false
end

local function roll_option(c)
  local n = #c.options
  local i = game.random(n) + 1
  while i == c.last do i = game.random(n) + 1 end
  c.last = i
  return c.options[i]
end

local function member_point(c)
  for _ = 1, 16 do
    local dx, dy = game.random(2 * OFFSET_MAX + 1) - OFFSET_MAX, game.random(2 * OFFSET_MAX + 1) - OFFSET_MAX
    local d2 = dx * dx + dy * dy
    if d2 >= OFFSET_MIN * OFFSET_MIN and d2 <= OFFSET_MAX * OFFSET_MAX then
      local x, y = c.cx + dx * WAR3_UNIT, c.cy + dy * WAR3_UNIT
      if game.walkable(x, y) then return x, y end
    end
  end
  return c.cx, c.cy
end

local function camp_pass(c, now)
  c.next_pass = next_mark(now, cfg.respawn_s) + c.stagger
  if camp_blocked(c) then return end
  local dead = {}
  for _, k in ipairs(c.slots) do
    if not unit_alive(k) then dead[#dead + 1] = k end
  end
  local members = roll_option(c)
  for j, kind in ipairs(members) do
    local x, y = member_point(c)
    local k = dead[j]
    if k ~= nil then
      game.cmd_set_home(k, x, y)
      game.cmd_respawn_as(k, kind, c.facing)
      game.cmd_set_respawn(k, 0)
      local r = reg[k]
      r.name, r.hx, r.hy = kind, x, y
    else
      local id = game.spawn_neutral(kind, x, y, c.facing)
      if id ~= nil and game.join_spawn_group(id) then
        c.slots[#c.slots + 1] = id
        reg[id] = { name = kind, kind = "camp", camp = c, hx = x, hy = y, free = false, taken = false }
      end
    end
  end
  for j = #members + 1, #dead do game.cmd_set_respawn(dead[j], false) end
end

local function always_clear() return true end

local function leash_home(k, r)
  local x, y = unit_position(k)
  local dx, dy = x - r.hx, y - r.hy
  if dx * dx + dy * dy > cfg.leash_fixed * cfg.leash_fixed then
    game.issue_order(k, 0, r.hx, r.hy)
  end
end

local function rune_reward(r, minute)
  if r.first then return cfg.rune_first_gold, 0 end
  return cfg.rune_gold_base + cfg.rune_gold_per_min * minute, cfg.rune_xp_base + cfg.rune_xp_per_min * minute
end

local GREED = "alchemist_goblin_s_greed"
local GREED_FACTOR = { 1, 2, 2, 3 }
local function greed_factor(hero)
  for _, a in ipairs(game.unit_abilities(hero)) do
    if a.name == GREED then return GREED_FACTOR[math.min(math.max(game.skill_rank(hero, a.slot), 1), #GREED_FACTOR)] end
  end
  return 1
end

local function use_rune(taker, power, first, now)
  if power ~= nil then
    runes_power.apply(taker, power, now)
  else
    local gold, xp = rune_reward({ first = first }, math.floor(now / (60 * TPS)))
    bounty.credit({ player = unit_player(taker), gold = gold * greed_factor(taker), at = taker, text = true })
    if xp > 0 then game.cmd_add_xp(taker, xp) end
  end
end
bottle.on_drink(use_rune)

local function claim_rune(k, r)
  game.cmd_remove(k)
  game.cmd_set_respawn(k, false)
  r.taken = true
end

local function rune_pickup(k, r, now)
  local taker = nil
  for _, u in ipairs(units.units_near(r.hx, r.hy, cfg.pickup_fixed)) do
    if units.is_hero(u) and unit_alive(u) and (taker == nil or u < taker) then taker = u end
  end
  if taker == nil then return end
  if not bottle.store(taker, r.power, r.first, now) then use_rune(taker, r.power, r.first, now) end
  claim_rune(k, r)
end

bottle.on_capture(function(taker, now)
  local reach = config.milli(config.rule("bottle_capture_reach_milli"))
  local x, y = unit_position(taker)
  for k = 0, get_unit_count() - 1 do
    local r = reg[k]
    if r ~= nil and r.kind == "rune" and not r.taken and unit_alive(k) then
      local dx, dy = x - r.hx, y - r.hy
      if dx * dx + dy * dy <= reach * reach and bottle.store(taker, r.power, r.first, now, true) then
        claim_rune(k, r)
        return
      end
    end
  end
end)

local function rune_round(now)
  local round = math.floor(now / (cfg.rune_period_s * TPS))
  if round <= last_round or #runes == 0 then return end
  last_round = round
  if modes.has("np") then return end
  local pick = roll(#runes - 1) + 1
  power_kind = runes_power.next_kind(power_kind, roll)
  for slot, k in ipairs(runes) do
    local r = reg[k]
    r.first = false
    r.power = slot ~= pick and power_kind or nil
    if not unit_alive(k) then
      r.taken = false
      game.cmd_set_respawn(k, 0)
    end
  end
end

local S_HEALTH, S_ARMOR_X100 = 2, 26
local function roshan_growth(k, now)
  local step = math.floor(now / (cfg.roshan_growth_s * TPS))
  local want_hp = cfg.roshan_hp_per_growth * step
  local want_armour = cfg.roshan_armour_x100_per_growth * math.min(step, cfg.roshan_armour_growth_cap)
  local have_hp, have_armour = game.stat_value(k, S_HEALTH), game.stat_value(k, S_ARMOR_X100)
  if want_hp ~= have_hp then game.cmd_add_stat(k, S_HEALTH, want_hp - have_hp) end
  if want_armour ~= have_armour then game.cmd_add_stat(k, S_ARMOR_X100, want_armour - have_armour) end
end

local function roshan_leash(k, r, now)
  if now < r.next_leash_check then return end
  r.next_leash_check = now + cfg.roshan_leash_check_s * TPS
  local x, y = unit_position(k)
  local box = cfg.roshan_leash
  if math.abs(x - box.cx) <= box.hx and math.abs(y - box.cy) <= box.hy then return end
  if r.leash_outside then
    r.leash_outside = false
  else
    game.teleport(k, r.hx, r.hy)
    r.leash_outside = true
  end
end

local function reclaim_aegis()
  for i = 0, get_unit_count() - 1 do
    if units.is_hero(i) then
      for _, row in ipairs(game.unit_items(i)) do
        if row.name == "aegis" and row.count > 0 then game.consume_item_charge(i, row.slot) end
      end
    end
  end
end

local function aegis_tick(now)
  if aegis_life == nil then return end
  if aegis_life.warn ~= nil and now >= aegis_life.warn then
    aegis_life.warn = nil
    announce.aegis_reminder(aegis_life.holder)
  end
  if now < aegis_life.expire then return end
  aegis_life = nil
  reclaim_aegis()
end

local NEUTRAL_SLOT = 9
local function has_room(hero, taken)
  local held = taken[hero] or 0
  for _, row in ipairs(game.unit_items(hero)) do
    if row.slot ~= NEUTRAL_SLOT then held = held + 1 end
  end
  return held < NEUTRAL_SLOT
end

local function ground_tick()
  local drops = game.ground_items()
  if #drops == 0 then return end
  local taken = {}
  for _, drop in ipairs(drops) do
    local taker = nil
    for _, u in ipairs(units.units_near(drop.x, drop.y, cfg.pickup_fixed)) do
      if units.is_hero(u) and unit_alive(u) and has_room(u, taken) and (taker == nil or u < taker) then taker = u end
    end
    if taker ~= nil then
      game.cmd_pickup_item(taker, drop.id)
      if drop.name == "rapier" then item_forms.rapier_picked(taker, drop) end
      taken[taker] = (taken[taker] or 0) + 1
      if drop.name == "aegis" then
        if aegis_life ~= nil then aegis_life.holder = taker end
        announce.aegis_acquired(taker)
      end
    end
  end
end

local DEATH_DROPS = { rapier = "rapier", rapier_undroppable = "rapier", gem = "gem", gem_courier = "gem" }

local function drop_death_items(unit)
  local x, y = unit_position(unit)
  for _, row in ipairs(game.unit_items(unit)) do
    if DEATH_DROPS[row.name] then
      local count = math.max(row.count, 1)
      for _ = 1, count do game.consume_item_charge(unit, row.slot) end
      local id = game.cmd_drop_ground(DEATH_DROPS[row.name], count, x, y)
      if DEATH_DROPS[row.name] == "rapier" then item_forms.rapier_dropped(id, row.owner or unit_player(unit)) end
    end
  end
end

local function roshan_reward(unit, killer, killer_player, now)
  local kills = game.counter(unit, "roshan_kills") + 1
  game.set_counter(unit, "roshan_kills", kills)
  aegis_life = { warn = now + cfg.aegis_warn_s * TPS, expire = now + cfg.aegis_expire_s * TPS }
  local side = game.team(killer_player)
  for _, hero in ipairs(bounty.heroes()) do
    local player = unit_player(hero)
    if game.team(player) == side then bounty.credit({ player = player, gold = cfg.roshan_team_gold }) end
  end
  local x, y = unit_position(unit)
  game.cmd_drop_ground("aegis", 1, x, y)
  if kills >= cfg.roshan_cheese_from_kill then game.cmd_drop_ground("cheese", 1, x, y) end
end

return {
  on_tick = function(state)
    local now = game.time()
    if not ready then
      setup(now)
      ready = true
    end
    rune_round(now)
    runes_power.tick(now)
    aegis_tick(now)
    ground_tick()
    for _, c in ipairs(camps) do
      if now >= c.next_pass then camp_pass(c, now) end
    end
    for k = 0, get_unit_count() - 1 do
      local r = reg[k]
      if r ~= nil then
        if unit_alive(k) then
          if r.kind == "boss" then
            roshan_growth(k, now)
            roshan_leash(k, r, now)
          elseif r.kind == "camp" then
            leash_home(k, r)
          elseif not r.taken then
            rune_pickup(k, r, now)
          end
        elseif r.kind ~= "camp" and respawn.due(r, now, always_clear) then
          r.taken = false
          if r.kind == "boss" then reclaim_aegis() end
          game.cmd_set_respawn(k, 0)
        end
      end
    end
  end,

  on_died = function(unit, killer, killer_player)
    if units.is_hero(unit) then drop_death_items(unit) end
    local r = reg[unit]
    if r == nil or cfg == nil or r.kind == "camp" then return end
    respawn.schedule(r, game.time())
    if r.name == "roshan" and killer_player ~= nil and killer_player < config.PLAYERS then
      roshan_reward(unit, killer, killer_player, game.time())
    end
  end,
}

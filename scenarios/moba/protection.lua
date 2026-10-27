local config = require("config")
local bounty = require("bounty")
local reduction = require("reduction")
local units = require("units")

local TICKS = config.TICKS_PER_SECOND
local SCAN_EVERY = 4
local GLYPH_SPAN = 5 * TICKS
local GLYPH_COOLDOWN = 300 * TICKS
local STRUCTURE_BIT = 64
local FOUNTAIN_PCT = 4
local FOUNTAIN_GRACE = math.floor(2.5 * TICKS)
local FOUNTAIN_RECT = {
  [0] = { x0 = config.milli(43500), y0 = config.milli(260500), x1 = config.milli(65500), y1 = config.milli(283000) },
  [1] = { x0 = config.milli(254500), y0 = config.milli(49000), x1 = config.milli(283500), y1 = config.milli(77500) },
}
local TOWER_TIER = { tower = 1, tower_t2 = 2, tower_t3 = 3, tower_t4 = 4 }
local BARRACKS_GOLD = { barracks_melee = "barracks_melee_team_gold", barracks_ranged = "barracks_ranged_team_gold" }

local glyph = reduction.window({ span = GLYPH_SPAN, cooldown = GLYPH_COOLDOWN, need = STRUCTURE_BIT, skip = 0, pct = 100 })

local gate_of_structure = {}
local is_fountain = {}
local peak, last_in, healing = {}, {}, {}

local function ids_ascending_by_insertion(ids_in_any_order)
  local ids = {}
  for _, id in ipairs(ids_in_any_order) do
    local j = #ids + 1
    while j > 1 and ids[j - 1] > id do
      ids[j] = ids[j - 1]
      j = j - 1
    end
    ids[j] = id
  end
  return ids
end

local function by_owner_and_kind()
  local by = {}
  for _, id in ipairs(ids_ascending_by_insertion(game.units())) do
    local kind = game.unit_kind(id)
    if kind ~= nil then
      local owner = unit_player(id)
      by[owner] = by[owner] or {}
      by[owner][kind] = by[owner][kind] or {}
      local list = by[owner][kind]
      list[#list + 1] = id
      if kind == "fountain" then is_fountain[id] = true end
    end
  end
  return by
end

local function gate(id, mode, ids)
  if #ids > 0 then gate_of_structure[id] = { mode = mode, ids = ids } end
end

local function build_gates()
  for _, kinds in pairs(by_owner_and_kind()) do
    local t1, t2, t3, t4 = kinds.tower or {}, kinds.tower_t2 or {}, kinds.tower_t3 or {}, kinds.tower_t4 or {}
    local melee, ranged = kinds.barracks_melee or {}, kinds.barracks_ranged or {}
    for lane = 1, 3 do
      if t1[lane] and t2[lane] then gate(t2[lane], "any_alive", { t1[lane] }) end
      if t2[lane] and t3[lane] then gate(t3[lane], "any_alive", { t2[lane] }) end
      if t3[lane] and melee[lane] then gate(melee[lane], "any_alive", { t3[lane] }) end
      if t3[lane] and ranged[lane] then gate(ranged[lane], "any_alive", { t3[lane] }) end
    end
    for _, id in ipairs(t4) do gate(id, "all_alive", t3) end
    for _, kind in ipairs({ "base_outbuilding", "base_outbuilding_large" }) do
      for _, id in ipairs(kinds[kind] or {}) do gate(id, "all_alive", t3) end
    end
    for _, id in ipairs(kinds.base or {}) do gate(id, "any_alive", t4) end
  end
end
build_gates()

local chain = {}
function chain.pct(u)
  local g = gate_of_structure[u]
  if g == nil then return 0 end
  if g.mode == "any_alive" then
    for _, id in ipairs(g.ids) do
      if unit_alive(id) then return 100 end
    end
    return 0
  end
  for _, id in ipairs(g.ids) do
    if not unit_alive(id) then return 0 end
  end
  return 100
end

local fountain_cover = {}
function fountain_cover.pct(u)
  return is_fountain[u] and 100 or 0
end

local RULES = { chain, glyph, fountain_cover }

local function in_rect(rect, x, y)
  return x >= rect.x0 and x <= rect.x1 and y >= rect.y0 and y <= rect.y1
end

local function fountain_heal(ticks)
  local team0, team1 = game.team(0), game.team(1)
  for p = 0, config.PLAYERS - 1 do
    local team = game.team(p)
    local rect = nil
    if team ~= nil and team == team0 then rect = FOUNTAIN_RECT[0] elseif team ~= nil and team == team1 then rect = FOUNTAIN_RECT[1] end
    for _, u in ipairs(game.units(p) or {}) do
      if not units.is_structure(u) then
        local hp = unit_hp(u)
        if hp > (peak[u] or 0) then peak[u] = hp end
        local want = 0
        if rect ~= nil then
          local x, y = unit_position(u)
          if in_rect(rect, x, y) then last_in[u] = ticks end
          if last_in[u] ~= nil and ticks - last_in[u] <= FOUNTAIN_GRACE then
            want = math.max(1, math.floor(peak[u] / config.FIX * FOUNTAIN_PCT / 100))
          end
        end
        if (healing[u] or 0) ~= want then
          game.set_healing(u, want)
          healing[u] = want
        end
      end
    end
  end
end

local function team_gold(victim_player, killer_player, tier)
  local gold = config.rule("tower_team_gold_base") + config.rule("tower_team_gold_per_tier") * tier
  if not units.hostile(victim_player, killer_player) then gold = math.floor(gold / 2) end
  bounty.credit_enemies(victim_player, gold)
end

local ticks = 0

return {
  in_fountain = function(unit) return (healing[unit] or 0) > 0 end,

  on_cast = function(unit, ability)
    if ability ~= "glyph_of_fortification" then return end
    local p = unit_player(unit)
    if p >= config.PLAYERS then return end
    glyph.cast(game.team(p), ticks)
  end,

  on_died = function(unit, killer, killer_player, _, victim_player)
    local kind = game.unit_kind(unit)
    if BARRACKS_GOLD[kind] ~= nil then bounty.credit_enemies(victim_player, config.rule(BARRACKS_GOLD[kind])) end
    local tier = TOWER_TIER[kind]
    if tier == nil then return end
    if tier == 1 then glyph.reset(game.team(victim_player)) end
    if killer == nil or killer == unit or killer_player == nil then return end
    team_gold(victim_player, killer_player, tier)
  end,

  on_tick = function()
    ticks = ticks + 1
    reduction.apply(RULES, ticks, ticks % SCAN_EVERY == 0)
    if ticks % SCAN_EVERY == 0 then fountain_heal(ticks) end
  end,
}

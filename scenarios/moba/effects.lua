local units = require("units")
local xp = require("xp")

local FIXED_POINT_UNIT = 65536
local MELEE_RANGE_FIXED = 170394

local function fixed_to_whole_units(v)
  return math.floor(v / FIXED_POINT_UNIT + 0.5)
end
local SPELL = "spell"
local level_at_last_tick = {}
local alive_at_last_tick = {}
local xp_level_was_read_from = {}

local function load_cast_signatures()
  local loaded, signatures = pcall(require, "ability_fx")
  if loaded and type(signatures) == "table" then return signatures end
  return nil
end
local SIGNATURES = load_cast_signatures()

local SCHOOLS = { "fire", "frost", "lightning", "nature", "shadow", "holy", "arcane", "poison", "blood", "earth", "wind", "physical" }
local KNOWN_SCHOOL, BOLT, HIT = {}, {}, {}
for _, school in ipairs(SCHOOLS) do
  KNOWN_SCHOOL[school] = true
  BOLT[school] = "fx_bolt_" .. school
  HIT[school] = "fx_hit_" .. school
end
local CAST_KIND = {
  nova = "fx_cast_nova", zone = "fx_cast_nova", column = "fx_cast_nova", summon = "fx_cast_nova",
  orbit = "fx_cast_spiral", spiral = "fx_cast_spiral",
}
local GROUND_KIND = { nova = "fx_nova", zone = "fx_zone", column = "fx_column", summon = "fx_column" }
local CHAIN_REACH = 6 * FIXED_POINT_UNIT

local function generic_cast(caster)
  game.effect("cast_ring", 0, 0, { source = caster, colour = SPELL })
  game.effect("cast_burst", 0, 0, { source = caster, colour = SPELL })
  local target = game.cast_target(caster)
  if target ~= nil then
    game.effect("spell_orb", 0, 0, { source = caster, target = target, colour = SPELL })
  else
    local x, y = game.cast_point(caster)
    game.effect("area_ring", x, y, { colour = SPELL })
    game.effect("cast_burst", x, y, { colour = SPELL })
  end
end

local function team_or_nil_for_neutral(unit)
  local ok, side = pcall(function() return game.team(unit_player(unit)) end)
  if ok then return side end
  return nil
end

local function chain_beams(caster, target, x, y, school, scale, hits)
  local from, visited, caster_team = caster, { [caster] = true }, team_or_nil_for_neutral(caster)
  for link = 1, hits do
    if target == nil then
      game.effect("fx_beam", x, y, { source = from, colour = school, scale = scale })
      return
    end
    game.effect("fx_beam", 0, 0, { source = from, target = target, colour = school, scale = scale })
    game.effect(HIT[school], 0, 0, { target = target, colour = school, scale = scale })
    visited[target] = true
    from = target
    target = nil
    local tx, ty = unit_position(from)
    for _, near in ipairs(game.units_near(tx, ty, CHAIN_REACH)) do
      if not visited[near] and game.alive(near) and team_or_nil_for_neutral(near) ~= caster_team then
        target = near
        break
      end
    end
  end
end

local hero_ids, unit_count_at_last_scan = {}, -1
local function heroes()
  local count = get_unit_count()
  if count ~= unit_count_at_last_scan then
    unit_count_at_last_scan = count
    hero_ids = {}
    for i = 0, count - 1 do
      if units.is_hero(i) then hero_ids[#hero_ids + 1] = i end
    end
  end
  return hero_ids
end

local function in_melee_reach(a, b)
  local ax, ay = unit_position(a)
  local bx, by = unit_position(b)
  local dx, dy = ax - bx, ay - by
  return dx * dx + dy * dy <= MELEE_RANGE_FIXED * MELEE_RANGE_FIXED
end

local function level_recomputed_only_when_xp_moves(hero)
  local have = game.xp(hero)
  local level = level_at_last_tick[hero]
  if xp_level_was_read_from[hero] ~= have then
    xp_level_was_read_from[hero] = have
    level = xp.level_of(hero)
  end
  return level
end

local function forget_all_cached_state()
  level_at_last_tick, alive_at_last_tick, xp_level_was_read_from = {}, {}, {}
  hero_ids, unit_count_at_last_scan = {}, -1
end

game.trigger("on_died", {
  action = function(unit)
    game.effect("kill_burst", 0, 0, { target = unit, colour = "kill" })
  end,
})

return {
  on_state_replaced = forget_all_cached_state,

  on_hit = function(attacker, victim, dealt)
    game.effect("damage_text", 0, 0, { target = victim, text = "-" .. tostring(fixed_to_whole_units(dealt)) })
    game.effect("hit_flash", 0, 0, { target = victim })
    if attacker < 0 or attacker == victim then return end
    if in_melee_reach(attacker, victim) then
      game.effect("melee_hit", 0, 0, { target = victim, colour = "missile" })
    else
      game.effect("attack_arrow", 0, 0, { source = attacker, target = victim, colour = "missile" })
    end
  end,

  on_cast = function(caster, ability_name)
    local sig = SIGNATURES and SIGNATURES[ability_name]
    if sig == nil then
      generic_cast(caster)
      return
    end
    local school = KNOWN_SCHOOL[sig.school] and sig.school or "arcane"
    local form = sig.form
    local scale = sig.size or 1000
    local ms = sig.dur or 0
    game.effect(CAST_KIND[form] or "fx_cast_ring", 0, 0, { source = caster, colour = school, scale = scale })
    local target = game.cast_target(caster)
    local x, y = game.cast_point(caster)
    if form == "bolt" then
      game.effect(BOLT[school], x, y, { source = caster, target = target, colour = school, scale = scale })
    elseif form == "beam" then
      game.effect("fx_beam", x, y, { source = caster, target = target, colour = school, scale = scale })
      game.effect(HIT[school], x, y, { target = target, colour = school, scale = scale })
    elseif form == "chain" then
      chain_beams(caster, target, x, y, school, scale, sig.hits or 3)
    elseif form == "nova" or form == "zone" or form == "column" or form == "summon" then
      game.effect(GROUND_KIND[form], x, y, { colour = school, scale = scale, duration = ms })
      game.effect(HIT[school], x, y, { colour = school, scale = scale })
    elseif form == "aura" then
      game.effect("fx_aura", 0, 0, { source = caster, target = target, colour = school, scale = scale, duration = ms })
    elseif form == "cone" then
      game.effect("fx_cone", x, y, { source = caster, target = target, colour = school, scale = scale })
      game.effect(HIT[school], x, y, { colour = school, scale = scale })
    elseif form == "blink" then
      local cx, cy = unit_position(caster)
      game.effect("fx_flash", cx, cy, { colour = school, scale = scale })
      game.effect("fx_flash", x, y, { colour = school, scale = scale })
    elseif form == "orbit" or form == "spiral" then
      game.effect(form == "orbit" and "fx_orbit" or "fx_spiral", 0, 0, { source = caster, colour = school, scale = scale, duration = ms })
    end
  end,

  on_tick = function()
    for _, i in ipairs(heroes()) do
      local level = level_recomputed_only_when_xp_moves(i)
      local alive = game.alive(i)
      local before_level, was_alive = level_at_last_tick[i], alive_at_last_tick[i]
      if before_level ~= nil and level > before_level then
        game.effect("level_ring", 0, 0, { target = i, colour = "level" })
        game.effect("level_burst", 0, 0, { target = i, colour = "level" })
        game.effect("level_up", 0, 0, { target = i })
      end
      if alive and was_alive == false then
        game.effect("respawn_ring", 0, 0, { target = i, colour = "respawn" })
        game.effect("respawn_burst", 0, 0, { target = i, colour = "respawn" })
      end
      level_at_last_tick[i] = level
      alive_at_last_tick[i] = alive
    end
  end,
}

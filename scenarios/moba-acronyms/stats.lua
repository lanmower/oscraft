local config = require("config")
local units = require("units")
local bounty = require("bounty")
local aegis = require("aegis")

local NEUTRAL = 255
local LANE_CREEPS = {
  creep = true, creep_ranged = true, creep_siege = true, creep_super = true, creep_ranged_super = true,
  creep_siege_super = true, creep_mega = true, creep_ranged_mega = true, creep_siege_mega = true,
}
local CONSUMABLE = { clarity = 1, flask = 1, ward_sentry = 1, ward_observer = 1, tango = 4, dust = 2 }
local TOWERS = { tower = true, tower_t2 = true, tower_t3 = true, tower_t4 = true }

local stats = {}

stats.headers = { "creeps", "denies", "neutrals", "towers", "streak", "kill gold", "gold lost" }

function stats.add(unit, key, n)
  game.set_counter(unit, key, game.counter(unit, key) + n)
end

function stats.purchased(unit, item)
  local n = CONSUMABLE[item]
  if n ~= nil then stats.add(unit, "st_consumable", n) end
end

function stats.multi(unit, n)
  if n == 2 then
    stats.add(unit, "st_multi2", 1)
  elseif n == 3 then
    stats.add(unit, "st_multi2", -1)
    stats.add(unit, "st_multi3", 1)
  elseif n == 4 then
    stats.add(unit, "st_multi3", -1)
    stats.add(unit, "st_multi4", 1)
  elseif n > 4 then
    stats.add(unit, "st_multi4", -1)
    stats.add(unit, "st_multi5", 1)
  end
end

function stats.duel(killer_player, victim_player, victim)
  if killer_player == nil or killer_player >= config.PLAYERS then return end
  local killer = bounty.hero_of(killer_player)
  if killer ~= nil then stats.add(killer, "st_pk" .. tostring(victim_player), 1) end
  stats.add(victim, "st_pd" .. tostring(killer_player), 1)
end

function stats.peak(unit, key, n)
  if n > game.counter(unit, key) then game.set_counter(unit, key, n) end
end

function stats.cells(hero)
  if hero == nil then
    return { "-", "-", "-", "-", "-", "-", "-" }
  end
  local c = game.counter
  return {
    tostring(c(hero, "st_creep")), tostring(c(hero, "st_deny")), tostring(c(hero, "st_neutral")),
    tostring(c(hero, "st_tower")) .. "/" .. tostring(c(hero, "st_tower_deny")),
    tostring(c(hero, "st_streak")), tostring(c(hero, "st_kill_gold")), tostring(c(hero, "st_gold_lost")),
  }
end

local function credited_hero(killer, killer_player)
  if killer == nil then return nil end
  if units.is_hero(killer) then return killer end
  if killer_player ~= nil and killer_player < config.PLAYERS and units.is_summoned(killer) then return bounty.hero_of(killer_player) end
  return nil
end

stats.on_died = function(unit, killer, killer_player, _, victim_player)
  if units.is_hero(unit) then
    if not aegis.reincarnating(unit) then stats.duel(killer_player, victim_player, unit) end
    return
  end
  local hero = credited_hero(killer, killer_player)
  if hero == nil then return end
  local kind = game.unit_kind(unit)
  if TOWERS[kind] then
    if units.hostile(killer_player, victim_player) then stats.add(hero, "st_tower", 1) else stats.add(hero, "st_tower_deny", 1) end
  elseif victim_player == NEUTRAL then
    if game.is_spawn_group_unit(unit) then
      stats.add(hero, "st_neutral", 1)
      stats.add(hero, "st_creep", 1)
    end
  elseif LANE_CREEPS[kind] then
    if units.hostile(killer_player, victim_player) then stats.add(hero, "st_creep", 1) else stats.add(hero, "st_deny", 1) end
  end
end

return stats

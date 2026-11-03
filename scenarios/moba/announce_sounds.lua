local config = require("config")
local units = require("units")
local ability_fx = require("ability_fx")

local TICKS = config.TICKS_PER_SECOND

local SOUNDS = {
  first_blood = "firstblood",
  killing_spree = "killing_spree",
  dominating = "dominating",
  megakill = "megakill",
  unstoppable = "unstoppable",
  wickedsick = "wickedsick",
  monsterkill = "monsterkill",
  godlike = "godlike",
  holyshit = "holyshit",
  double_kill = "double_kill",
  triple_kill = "triple_kill",
  ultrakill = "ultrakill",
  rampage = "rampage",
  horn = "horn",
}

local HIT_FLESH = { "hit_flesh_1", "hit_flesh_2", "hit_flesh_3" }
local HIT_METAL = { "hit_metal_1", "hit_metal_2" }
local CAST_ARCANE = { "cast_arcane_1", "cast_arcane_2" }
local CAST_FIRE = { "cast_fire_1" }
local CAST_NATURE = { "cast_nature_1" }
local DEATH = { "death_1", "death_2" }

local CAST_FAMILY = {
  fire = CAST_FIRE,
  nature = CAST_NATURE,
  earth = CAST_NATURE,
  wind = CAST_NATURE,
  poison = CAST_NATURE,
}
local HIT_GAP = math.floor(TICKS / 4)
local last_hit = {}

local function name_of(event)
  local name = SOUNDS[event]
  if type(name) == "string" and name ~= "" then return name end
  return nil
end

local function play(event, player)
  local name = name_of(event)
  if name == nil then return false end
  game.sound(name, player)
  return true
end

local function sound(name, player)
  game.sound(name, player)
end

local function hit(attacker, victim)
  if attacker == nil or attacker < 0 or not units.is_hero(attacker) then return end
  local now = game.ticks()
  local previous = last_hit[attacker]
  if previous ~= nil and now - previous < HIT_GAP then return end
  last_hit[attacker] = now
  local family = units.is_structure(victim) and HIT_METAL or HIT_FLESH
  game.sound(family[(now + attacker) % #family + 1], unit_player(attacker))
end

local function cast(caster, ability)
  local fx = ability_fx[ability]
  if fx == nil or not units.is_hero(caster) then return end
  local family = CAST_FAMILY[fx.school] or CAST_ARCANE
  game.sound(family[(game.ticks() + caster) % #family + 1], unit_player(caster))
end

local function death()
  game.sound(DEATH[game.ticks() % #DEATH + 1])
end

return { play = play, name_of = name_of, sound = sound, hit = hit, cast = cast, death = death }

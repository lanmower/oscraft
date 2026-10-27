local bounty = require("bounty")
local config = require("config")
local counter = require("counter")
local units = require("units")

local ABILITY = "item_tome_of_knowledge"

local used = counter.sim("tomes_used")

local tome = {}

local function team_tomes(team)
  local total = 0
  for _, hero in ipairs(bounty.heroes()) do
    if game.team(unit_player(hero)) == team then total = total + used.get(hero) end
  end
  return total
end

tome.on_cast = function(caster, ability_name)
  if ability_name ~= ABILITY or not units.is_hero(caster) then return end
  local before = team_tomes(game.team(unit_player(caster)))
  used.set(caster, used.get(caster) + 1)
  game.cmd_add_xp(caster, config.rule("tome_xp_base") + config.rule("tome_xp_per_team_tome") * before)
end

return tome

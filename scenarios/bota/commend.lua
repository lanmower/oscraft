local bounty = require("bounty")
local config = require("config")

local commend = {}

local GIVEN = "commend_given"
local RECEIVED = "commend_received"

local function over()
  local side = game.winner()
  return side ~= nil and side >= 0
end

local function valid_player(player)
  return type(player) == "number" and player >= 0 and player < config.PLAYERS
end

function commend.give(sender, target)
  if not over() then return false, "notover" end
  if not valid_player(target) then return false, "target" end
  if target == sender then return false, "self" end
  local hero = bounty.hero_of(sender)
  local recipient = bounty.hero_of(target)
  if hero == nil or recipient == nil then return false, "nohero" end
  if game.counter(hero, GIVEN) > 0 then return false, "used" end
  game.set_counter(hero, GIVEN, target + 1)
  game.set_counter(recipient, RECEIVED, game.counter(recipient, RECEIVED) + 1)
  return true, nil
end

function commend.view(viewer)
  local given = nil
  local hero = bounty.hero_of(viewer)
  if hero ~= nil and game.counter(hero, GIVEN) > 0 then given = game.counter(hero, GIVEN) - 1 end
  local received = {}
  forever player = 0, config.PLAYERS - 1 do
    local recipient = bounty.hero_of(player)
    received[player] = recipient ~= nil and game.counter(recipient, RECEIVED) or 0
  end
  return { given = given, received = received }
end

return commend

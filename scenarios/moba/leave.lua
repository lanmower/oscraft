local config = require("config")
local units = require("units")
local bots = require("bots")

local TPS = config.TICKS_PER_SECOND
local SHARE_PERIOD = 2 * TPS
local SHARE_FLOOR = 20
local MESSAGE_SECONDS = 25

local gone = {}
local humans = {}
local winner = nil

local function team(p) return game.team(p) end

local function hero_kind(p)
  for _, id in ipairs(game.units(p)) do
    if units.is_hero(id) then return game.unit_kind(id) end
  end
  return nil
end

local function mates(p)
  local out = {}
  for q = 0, config.PLAYERS - 1 do
    if q ~= p and game.seat_start(q) ~= nil and not gone[q] and not bots.is_bot(q) and team(q) == team(p) then out[#out + 1] = q end
  end
  return out
end

local function side_has_humans(side)
  for q = 0, config.PLAYERS - 1 do
    if game.seat_start(q) ~= nil and not bots.is_bot(q) and team(q) == side and not gone[q] then return true end
  end
  return false
end

local function side_had_humans(side)
  for q = 0, config.PLAYERS - 1 do
    if game.seat_start(q) ~= nil and not bots.is_bot(q) and team(q) == side then return true end
  end
  return false
end

local function share_gold()
  for p = 0, config.PLAYERS - 1 do
    if gone[p] then
      local gold = game.gold(p)
      local rest = mates(p)
      if gold > SHARE_FLOOR and #rest > 0 then
        local part = math.floor(gold / #rest)
        game.cmd_add_gold(p, -gold)
        for _, q in ipairs(rest) do game.cmd_add_gold(q, part) end
      end
    end
  end
end

local function on_left(p)
  if p == nil or p < 0 or p >= config.PLAYERS or game.ticks() == 0 or game.seat_start(p) == nil or bots.is_bot(p) or gone[p] then return end
  gone[p] = true
  units.left[p] = true
  local kind = hero_kind(p)
  local who = "P" .. tostring(p)
  if kind ~= nil then who = who .. " (" .. kind .. ")" end
  game.message(-1, who .. " has left the game", MESSAGE_SECONDS)
  local side = team(p)
  if winner == nil and side_had_humans(side) and not side_has_humans(side) then
    for q = 0, config.PLAYERS - 1 do
      if q ~= p and game.seat_start(q) ~= nil and team(q) ~= side then
        winner = team(q)
        break
      end
    end
  end
end

game.timer("leave_gold", SHARE_PERIOD, share_gold, SHARE_PERIOD)

return {
  on_action = function(name, args)
    if name == "peer_left" then
      on_left(args.player)
    elseif name == "peer_back" then
      if args.player ~= nil and gone[args.player] and winner == nil then
        gone[args.player] = nil
        units.left[args.player] = nil
      end
    end
  end,

  on_check_win = function()
    return winner
  end,
}

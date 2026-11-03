local chat_cmds = require("chat_cmds")
local config = require("config")

local LOCAL = 0
local MESSAGE_SECONDS = 12
local ROLL_DEFAULT = 100
local ROLL_MAX = 2000
local ROLL_USES = 20
local ROLL_GAP_TICKS = config.TICKS_PER_SECOND
local ROLL_SHOW_SECONDS = 5

local TEXT = {
  rolled = "has rolled",
  outOf = "out of",
  rollRange = "Roll value must be between 1 and 2000",
  spam = "Do not spam.",
  tooMany = "You have used this too much!",
}

local rolls = {}
local seed = 0

local function name_of(player) return "Player " .. tostring(player + 1) end

local function next_random(player)
  seed = (seed * 1103515245 + 12345 + game.ticks() + player) % 2147483648
  return seed
end

local function roll(player, n)
  local rec = rolls[player]
  if rec == nil then
    rec = { count = 0, ready = 0 }
    rolls[player] = rec
  end
  local now = game.ticks()
  if rec.ready > now then return game.message(player, TEXT.spam, 10) end
  rec.ready = now + ROLL_GAP_TICKS
  rec.count = rec.count + 1
  if rec.count > ROLL_USES then return game.message(player, TEXT.tooMany, 10) end
  if n == 0 then n = ROLL_DEFAULT end
  if n < 1 or n > ROLL_MAX then return game.message(player, TEXT.rollRange, 10) end
  local value = 1 + next_random(player) % n
  game.message(-1, name_of(player) .. " " .. TEXT.rolled .. " " .. tostring(value) .. " " .. TEXT.outOf .. " " .. tostring(n), ROLL_SHOW_SECONDS)
end

local function on_chat(player, args)
  local cmd = args.cmd
  if cmd == nil then
    game.message(-1, name_of(player) .. ": " .. tostring(args.text), MESSAGE_SECONDS)
  elseif cmd == "-roll" then
    roll(player, math.floor(args.n or 0))
  else
    chat_cmds.chat(player, args)
  end
end

local function on_action(name, args)
  if name == "setup" then
    LOCAL = args.player or LOCAL
  elseif name == "chat" then
    on_chat(args.player or LOCAL, args)
  end
end

return { on_action = on_action }

local bots = require("bots")
local bounty = require("bounty")
local config = require("config")
local text = require("chat_text")
local units = require("units")

local core = {}

core.PLAYERS = config.PLAYERS
core.TPS = config.TICKS_PER_SECOND
core.TEAM_SLOTS = 5
core.HOST = 0
core.PICK = "|c006699CC"
core.WARN = "|c00ff0303"
core.NOTICE = "|cffffcc00"
core.ASK = "|cff99ccff"

local ui_teams = nil
local activity = {}
local left_at = {}
local lobby_api = nil

function core.setup(ui)
  ui_teams = ui and ui.teams or nil
end

function core.team_of(player)
  if ui_teams ~= nil and ui_teams[player + 1] ~= nil then return ui_teams[player + 1] end
  return game.team(player)
end

function core.attach(api) lobby_api = api end

function core.lobby() return lobby_api end

function core.touch(player)
  if player ~= nil then activity[player] = game.ticks() end
end

function core.idle_ticks(player) return game.ticks() - (activity[player] or 0) end

function core.clock_text(ticks)
  local seconds = ticks // core.TPS
  local minutes = seconds // 60
  local rest = seconds % 60
  if rest < 10 then return tostring(minutes) .. ":0" .. tostring(rest) end
  return tostring(minutes) .. ":" .. tostring(rest)
end

function core.mark_left(player) left_at[player] = core.clock_text(game.ticks()) end

function core.left_text(player)
  if left_at[player] == nil then return " " end
  return " " .. core.WARN .. "(" .. text.left .. " at " .. left_at[player] .. "|r" .. core.WARN .. ")|r"
end

function core.words(s)
  local out = {}
  local i = 1
  local n = string.len(s)
  while i <= n do
    local j = string.find(s, " ", i, true)
    if j == nil then j = n + 1 end
    if j > i then out[#out + 1] = string.sub(s, i, j - 1) end
    i = j + 1
  end
  return out
end

function core.s2i(s)
  local n = 0
  local sign = 1
  local i = 1
  local len = string.len(s)
  while i <= len and string.sub(s, i, i) == " " do i = i + 1 end
  if string.sub(s, i, i) == "-" then
    sign = -1
    i = i + 1
  end
  while i <= len do
    local b = string.byte(s, i)
    if b < 48 or b > 57 then break end
    n = n * 10 + (b - 48)
    i = i + 1
  end
  return sign * n
end

function core.s2r(s)
  local whole = core.s2i(s)
  local dot = string.find(s, ".", 1, true)
  if dot == nil then return whole end
  local fraction, scale = 0, 1
  local i = dot + 1
  while i <= string.len(s) do
    local b = string.byte(s, i)
    if b < 48 or b > 57 then break end
    fraction = fraction * 10 + (b - 48)
    scale = scale * 10
    i = i + 1
  end
  if string.find(s, "-", 1, true) ~= nil and string.find(s, "-", 1, true) < dot then return whole - fraction / scale end
  return whole + fraction / scale
end

function core.say(player, msg, seconds) game.message(player, msg, seconds) end

function core.say_all(msg, seconds) game.message(-1, msg, seconds) end

function core.blank(player, seconds) game.message(player, " ", seconds) end

function core.refuse(player, msg) game.message(player, core.NOTICE .. msg .. "|r", 2) end

function core.raw_id(player)
  if player < core.TEAM_SLOTS then return player + 1 end
  return player + 2
end

function core.player_of_raw(raw)
  if raw >= 1 and raw <= core.TEAM_SLOTS then return raw - 1 end
  if raw >= core.TEAM_SLOTS + 2 and raw <= 2 * core.TEAM_SLOTS + 1 then return raw - 2 end
  return nil
end

function core.seated(player)
  return game.seat_start(player) ~= nil
end

function core.active(player)
  return game.seat_start(player) ~= nil and not bots.is_bot(player) and not units.left[player]
end

function core.has_left(player) return units.left[player] == true end

function core.side_players(side)
  local out = {}
  for q = 0, core.PLAYERS - 1 do
    if game.seat_start(q) ~= nil and core.team_of(q) == side then out[#out + 1] = q end
  end
  return out
end

function core.slot_in_side(player)
  for i, q in ipairs(core.side_players(core.team_of(player))) do
    if q == player then return i end
  end
  return nil
end

function core.ally_at(player, index)
  if index < 1 or index > core.TEAM_SLOTS then return nil end
  return core.side_players(core.team_of(player))[index]
end

function core.rival_at(player, index)
  if index < 1 or index > core.TEAM_SLOTS then return nil end
  local list = core.side_players(1 - core.team_of(player))
  return list[index]
end

function core.hero_label(kind)
  if kind == nil then return text.noHeroName end
  if lobby_api ~= nil then return lobby_api.hero_name(kind) end
  return kind
end

function core.tint(player)
  local rgb = game.player_colour(player)
  if rgb == nil then return "" end
  return string.format("|cff%06x", rgb)
end

function core.display(viewer, player)
  local mode = game.setting(viewer, "names") or 0
  if mode == 1 then return tostring(core.raw_id(player)) end
  if mode == 2 then
    local hero = bounty.hero_of(player)
    if hero ~= nil then return core.hero_label(game.unit_kind(hero)) end
    return text.noHeroName
  end
  return game.player_name(player)
end

function core.coloured(viewer, player)
  return core.tint(player) .. core.display(viewer, player) .. "|r"
end

function core.replace(template, pairs_list)
  local out = template
  for _, pair in ipairs(pairs_list) do
    local start, stop = string.find(out, pair[1], 1, true)
    while start ~= nil do
      out = string.sub(out, 1, start - 1) .. pair[2] .. string.sub(out, stop + 1)
      start, stop = string.find(out, pair[1], start + string.len(pair[2]), true)
    end
  end
  return out
end

return core

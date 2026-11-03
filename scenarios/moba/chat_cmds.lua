local bounty = require("bounty")
local config = require("config")
local core = require("chat_core")
local lobby = require("chat_lobby")
local leave = require("leave")
local modes = require("modes")
local text = require("chat_text")
local units = require("units")
local xp = require("xp")

local HOST = core.HOST
local TPS = core.TPS
local AFK_TICKS = 300 * TPS
local AFK_REPORT_SECONDS = 15
local KICK_VOTE_TICKS = 60 * TPS
local KICK_VOTES_NEEDED = 4
local WEATHER_PERIOD = 300 * TPS
local WEATHER_KINDS = 4
local WEATHER_RANDOM = 5
local STATS_SECONDS = 10
local INFO_SECONDS = 20
local MUSIC_TRACKS = #text.musicTracks
local MUSIC_OFF = 0

local CAMERA_DEFAULT = 1650
local CAMERA_MIN = 1500
local CAMERA_MAX = 3000
local CAMERA_MESSAGE_SECONDS = 10

local vote = nil
local free_camera = {}
local flagged = {}
local weather_armed = false
local weather_kind = 1
local local_player = 0

local function flag_on(player, key)
  local value = game.setting(player, key)
  return value == nil or value ~= 0
end

local function set_flag(player, key, on)
  game.setting(player, key, on and 1 or 0)
end

local function hero_name_of(player)
  local hero = bounty.hero_of(player)
  if hero == nil then return nil end
  return core.hero_label(game.unit_kind(hero))
end

local function names(player, low)
  local current = game.setting(player, "names") or 0
  local mode
  if string.len(low) > 6 then
    mode = core.s2i(string.sub(low, 7, 8))
    if mode < 0 or mode > 2 then mode = 0 end
  else
    mode = (current + 1) % 3
  end
  game.setting(player, "names", mode)
  core.say(player, text.names[mode], 8)
end

local function gameinfo(player)
  local flags = modes.flags()
  local any = false
  for _, row in ipairs(text.gameInfo) do
    local code = row[1]
    local on = flags[code] == true
    if code == "ns" or code == "nr" then on = flags.ns == true or flags.nr == true end
    if code == "normal" then on = not any end
    if on then
      if code ~= "normal" then any = true end
      for _, line in ipairs(row[2]) do
        if code == "sh" then line = core.replace(line, { { "%s", core.coloured(player, HOST) } }) end
        core.say(player, line, INFO_SECONDS)
      end
    end
  end
end

local function matchup(player)
  core.say(player, " ", INFO_SECONDS)
  for _, q in ipairs(core.side_players(1 - core.team_of(player))) do
    local hero = bounty.hero_of(q)
    if hero ~= nil then
      core.say(player, core.coloured(player, q) .. " " .. text.controls .. " " .. core.hero_label(game.unit_kind(hero)) .. " (" .. text.level .. " " .. tostring(xp.character_level(hero)) .. ")" .. core.left_text(q), INFO_SECONDS)
    end
  end
  core.say(player, " ", INFO_SECONDS)
end

local function stats_line(player)
  local hero = bounty.hero_of(player)
  local function count(key)
    if hero == nil then return 0 end
    return game.counter(hero, key)
  end
  return text.creepsKilled .. " " .. tostring(count("st_creep")) .. " " .. text.creepsDenied .. " " .. tostring(count("st_deny")) .. " " .. text.neutralsKilled .. " " .. tostring(count("st_neutral"))
end

local function creepstats(player)
  core.say(player, "\t" .. stats_line(player), STATS_SECONDS)
end

local function team_creepstats(player)
  for _, q in ipairs(core.side_players(core.team_of(player))) do
    core.say(player, "\t" .. core.coloured(player, q) .. ": " .. stats_line(q), STATS_SECONDS)
  end
end

local function creepstats_switch(on)
  return function(player)
    set_flag(player, "creepstats", on)
    set_flag(player, "showdeny", on)
  end
end

local function deny_switch(on)
  return function(player) set_flag(player, "showdeny", on) end
end

local function denyinfo(player)
  core.refuse(player, text.denyInfo)
end

local function double_hold(player)
  local on = game.setting(player, "doublehold") ~= 1
  set_flag(player, "doublehold", on)
  core.say(player, text.doubleHold[on], 10)
end

local function courier_has_no_effect()
end

local function camera(player, low)
  local value = core.s2r(string.sub(low, 6, 9))
  if string.len(low) == 4 then return end
  if value <= 0 then
    game.camera_distance(player, 1)
    core.say(player, text.camera.wrongMultiplier, CAMERA_MESSAGE_SECONDS)
  elseif value < 3 then
    game.camera_distance(player, 1 / value)
  elseif value >= CAMERA_MIN and value <= CAMERA_MAX then
    game.camera_distance(player, CAMERA_DEFAULT / value)
  else
    core.say(player, text.camera.wrongDistance, CAMERA_MESSAGE_SECONDS)
  end
end

local function free_camera_toggle(player)
  free_camera[player] = not free_camera[player]
  game.camera_lock(player, not free_camera[player])
  if free_camera[player] then game.camera_distance(player, 1) end
end

local function movespeed(player)
  local shown = false
  local points = config.rule("speed_point_q16")
  for _, id in ipairs(game.units(player)) do
    if units.is_hero(id) then
      shown = true
      local kind = game.unit_kind(id)
      core.say(player, core.hero_label(kind) .. " " .. text.movespeed .. " " .. tostring(game.unit_speed(id) // points), 10)
      if kind == "lone_druid" then
        for _, other in ipairs(game.units(player)) do
          local other_kind = game.unit_kind(other)
          if other_kind ~= nil and string.find(other_kind, "spirit_bear", 1, true) ~= nil then
            core.say(player, core.hero_label(other_kind) .. " " .. text.movespeed .. " " .. tostring(game.unit_speed(other) // points), 10)
          end
        end
      end
    end
  end
  if not shown then core.say(player, text.noHeroSelected, 10) end
end

local function roll_weather()
  weather_kind = game.random(WEATHER_KINDS) + 1
  for q = 0, core.PLAYERS - 1 do
    if game.seat_start(q) ~= nil and game.setting(q, "weather") == WEATHER_RANDOM then game.setting(q, "weather_kind", weather_kind) end
  end
end

local function arm_weather()
  if weather_armed then return end
  weather_armed = true
  weather_kind = game.random(WEATHER_KINDS) + 1
  game.timer("chat_weather", WEATHER_PERIOD, roll_weather, WEATHER_PERIOD)
end

local function weather(player, low)
  local mode = text.weather[string.sub(low, 10)]
  if mode == nil or string.sub(low, 1, 9) ~= "-weather " then return end
  arm_weather()
  game.setting(player, "weather", mode)
  if mode == WEATHER_RANDOM then game.setting(player, "weather_kind", weather_kind) end
end

local function music(player, raw)
  local option = string.sub(raw, 8)
  if option == "off" then
    game.setting(player, "music", MUSIC_OFF)
  elseif option == "random" then
    game.setting(player, "music", game.random(MUSIC_TRACKS) + 1)
  else
    for index, name in ipairs(text.musicNames) do
      if option == name then
        game.setting(player, "music", index)
        return
      end
    end
    core.say(player, text.musicHelp .. " ", 10)
    core.say(player, text.musicList, 10)
  end
end

local function share_target(player, low, from)
  local id = core.s2i(string.sub(low, from))
  local target = core.ally_at(player, id)
  if target == nil or target == player then return nil end
  return target
end

local function grants(owner)
  if units.shared[owner] == nil then units.shared[owner] = {} end
  return units.shared[owner]
end

local function share(player, low)
  local target = share_target(player, low, 8)
  if target == nil then return end
  grants(player)[target] = true
  game.setting(player, "share" .. tostring(core.raw_id(target)), 1)
  core.say(player, text.shared .. core.coloured(player, target) .. "|r ", 15)
end

local function unshare(player, low)
  local target = share_target(player, low, 10)
  if target == nil then return end
  grants(player)[target] = nil
  game.setting(player, "share" .. tostring(core.raw_id(target)), 0)
  core.say(player, text.unshared .. core.coloured(player, target) .. "|r ", 15)
end

local function rmshare(player, low)
  local target = share_target(player, low, 10)
  if target == nil then return end
  grants(target)[player] = nil
  game.setting(target, "share" .. tostring(core.raw_id(player)), 0)
end

local function kick(target)
  core.mark_left(target)
  leave.on_action("peer_left", { player = target })
end

local function end_vote()
  vote = nil
  game.cancel_timer("chat_kick_vote")
end

local function vote_expired()
  if vote ~= nil then core.say_all(text.kickExpired, 20) end
  vote = nil
end

local function kick_candidates(player)
  local lines = {}
  for index, side in ipairs({ 0, 1 }) do
    local list = core.side_players(side)
    for slot = 1, core.TEAM_SLOTS do
      local q = list[slot]
      if q ~= nil and q ~= HOST then
        local tag = ""
        if core.has_left(q) then tag = core.WARN .. "[" .. text.left .. "]|r" end
        local hero = hero_name_of(q)
        local held = ""
        if hero ~= nil then held = " (" .. hero .. ")" end
        lines[#lines + 1] = text.type .. core.PICK .. " -drop " .. tostring(slot + (index - 1) * core.TEAM_SLOTS) .. "|r to kick " .. core.coloured(player, q) .. held .. "|r " .. tag
      end
    end
    if index == 1 then lines[#lines + 1] = " " end
  end
  return lines
end

local function drop(player, low)
  if player ~= HOST then return core.say(player, text.hostOnly, 30) end
  if vote ~= nil then return core.say(player, text.kickRunning, 30) end
  local id = core.s2i(string.sub(low, 7))
  local target = nil
  if id > 0 and id < 11 then
    if id > core.TEAM_SLOTS then
      target = core.side_players(1)[id - core.TEAM_SLOTS]
    else
      target = core.side_players(0)[id]
    end
  end
  if target == nil or target == HOST then
    core.say(player, text.kickChoose, 10)
    core.say(player, " ", 10)
    for _, line in ipairs(kick_candidates(player)) do core.say(player, line, 15) end
    return
  end
  core.say_all(core.tint(HOST) .. game.player_name(HOST) .. "|r" .. text.kickRequested .. core.tint(target) .. game.player_name(target) .. "." .. "|r" .. text.kickAccept, 20)
  vote = { target = target, accepted = { [HOST] = true, [target] = true }, count = 2 }
  game.timer("chat_kick_vote", KICK_VOTE_TICKS, vote_expired)
end

local function yes(player)
  if vote == nil or vote.accepted[player] or not core.seated(player) then return end
  vote.accepted[player] = true
  vote.count = vote.count + 1
  core.say_all(core.tint(player) .. game.player_name(player) .. "|r" .. text.kickAccepted .. "(" .. tostring(vote.count - 2) .. "/" .. tostring(KICK_VOTES_NEEDED) .. ")", 10)
  if vote.count > KICK_VOTES_NEEDED + 1 then
    local target = vote.target
    end_vote()
    kick(target)
    core.say_all(core.WARN .. game.player_name(target) .. text.kickRemoved .. "|r", 30)
  end
end

local function kickafk(player, low)
  local id = core.s2i(string.sub(low, 10))
  local target = core.player_of_raw(id)
  if target == nil or not flagged[target] then
    for _, side in ipairs({ 0, 1 }) do
      for _, q in ipairs(core.side_players(side)) do
        if core.active(q) and core.idle_ticks(q) > AFK_TICKS and not flagged[q] then
          flagged[q] = true
          core.say(player, core.coloured(player, q) .. " " .. text.afkFor .. " " .. string.format("%.3f", core.idle_ticks(q) / (TPS * 60)) .. " " .. text.afkMinutes, AFK_REPORT_SECONDS)
          core.say(player, text.afkType .. " " .. core.ASK .. "-kickafk " .. tostring(core.raw_id(q)) .. " |r " .. text.afkBoot, AFK_REPORT_SECONDS)
          core.say(player, " ", AFK_REPORT_SECONDS)
        end
      end
    end
    core.say(player, " ", AFK_REPORT_SECONDS)
  elseif core.active(target) and core.team_of(player) == core.team_of(target) then
    core.say(target, core.WARN .. text.afkBooted .. "|r", 3600)
    kick(target)
    core.say_all(core.WARN .. game.player_name(target) .. " " .. text.afkBooted .. "|r", 30)
  end
end

local EXACT = {
  ["-gameinfo"] = gameinfo,
  ["-matchup"] = matchup,
  ["-ma"] = matchup,
  ["-creepstats"] = creepstats,
  ["-cs"] = creepstats,
  ["-csa"] = team_creepstats,
  ["-cson"] = creepstats_switch(true),
  ["-csoff"] = creepstats_switch(false),
  ["-showdeny"] = deny_switch(true),
  ["-hidedeny"] = deny_switch(false),
  ["-denyinfo"] = denyinfo,
  ["-di"] = denyinfo,
  ["-courier"] = courier_has_no_effect,
  ["-hold"] = double_hold,
  ["-ms"] = movespeed,
  ["-movespeed"] = movespeed,
  ["-yes"] = yes,
  ["-w"] = free_camera_toggle,
}

local PREFIX = {
  { "-cam", camera },
  { "-names", names },
  { "-weather", weather },
  { "-unshare", unshare },
  { "-rmshare", rmshare },
  { "-share", share },
  { "-drop", drop },
  { "-kickafk", kickafk },
}

local function starts_with(s, prefix)
  return string.sub(s, 1, string.len(prefix)) == prefix
end

local function chat(player, args)
  local raw = args.text or ""
  local low = string.lower(raw)
  local exact = EXACT[low]
  if exact ~= nil then
    exact(player, low)
    return true
  end
  if starts_with(low, "-music") then
    music(player, raw)
    return true
  end
  for _, entry in ipairs(PREFIX) do
    if starts_with(low, entry[1]) then
      entry[2](player, low)
      return true
    end
  end
  return lobby.chat(player, args, low)
end

return {
  chat = chat,

  on_action = function(name, args)
    if name == "setup" then
      local_player = args.player or local_player
      core.setup(args.ui)
      core.touch(local_player)
    elseif name == "peer_left" then
      if args.player ~= nil then core.mark_left(args.player) end
    else
      core.touch(args.player)
    end
  end,
}

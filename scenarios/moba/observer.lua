local config = require("config")
local bounty = require("bounty")
local announce = require("announce")
local modes = require("modes")
local store = require("store")

local PANEL = "observer_info_688g"
local TITLE = "Observer Info"
local FIRST_OBSERVER_SEAT = 10
local SPECTATOR_SEAT = 255
local SENTINEL_OPEN = "|c00ED2323"
local SCOURGE_OPEN = "|c00067A06"
local CLOSE = "|r"

local shown = nil

local observer = {}

local function roster()
  local sentinel, scourge = {}, {}
  for p = 0, config.PLAYERS - 1 do
    local hero = bounty.hero_of(p)
    if hero ~= nil and game.seat_start(p) ~= nil then
      local row = { player = p, hero = hero }
      if game.team(p) == game.team(0) then sentinel[#sentinel + 1] = row else scourge[#scourge + 1] = row end
    end
  end
  return sentinel, scourge
end

local function tally(rows)
  local kills, deaths, creeps, denies, worth = 0, 0, 0, 0, 0
  for _, row in ipairs(rows) do
    kills = kills + game.kills_of(row.hero)
    deaths = deaths + game.deaths_of(row.hero)
    creeps = creeps + game.counter(row.hero, "st_creep")
    denies = denies + game.counter(row.hero, "st_deny")
    worth = worth + store.net_worth(row.hero)
  end
  return kills, deaths, creeps, denies, worth
end

local function name_line(name, kills, deaths, creeps, denies)
  return name .. " (" .. tostring(kills) .. "/" .. tostring(deaths) .. " | " .. tostring(creeps) .. "-" .. tostring(denies) .. ")"
end

local function side_lines(name, colour, rows)
  local kills, deaths, creeps, denies, worth = tally(rows)
  local lines = { { colour .. name_line(name, kills, deaths, creeps, denies) .. CLOSE, colour .. tostring(worth) .. CLOSE } }
  for _, row in ipairs(rows) do
    local h = row.hero
    lines[#lines + 1] = {
      name_line(announce.tag(row.player), game.kills_of(h), game.deaths_of(h), game.counter(h, "st_creep"), game.counter(h, "st_deny")),
      tostring(store.net_worth(h)),
    }
  end
  return lines
end

function observer.build()
  local sentinel, scourge = roster()
  local rows = side_lines("The Sentinel", SENTINEL_OPEN, sentinel)
  for _, line in ipairs(side_lines("The Scourge", SCOURGE_OPEN, scourge)) do rows[#rows + 1] = line end
  return { "Player (K/D | creeps-denies)", "Net worth" }, rows
end

local function signature(columns, rows)
  local text = ""
  for _, cell in ipairs(columns) do text = text .. "\1" .. cell end
  for _, cells in ipairs(rows) do
    text = text .. "\2"
    for _, cell in ipairs(cells) do text = text .. "\1" .. cell end
  end
  return text
end

local function observer_seats()
  local seats = {}
  for p = FIRST_OBSERVER_SEAT, config.PLAYERS - 1 do
    if game.seat_start(p) == nil then seats[#seats + 1] = p end
  end
  seats[#seats + 1] = SPECTATOR_SEAT
  return seats
end

function observer.refresh()
  if modes.has("oi") then
    if shown ~= nil then
      shown = nil
      game.panel(PANEL)
    end
    return
  end
  local columns, rows = observer.build()
  local sig = signature(columns, rows)
  if sig == shown then return end
  shown = sig
  for _, seat in ipairs(observer_seats()) do
    game.panel(PANEL, { title = TITLE, player = seat, columns = columns, rows = rows })
  end
end

return {
  on_tick = function()
    if game.ticks() % config.TICKS_PER_SECOND == 0 then observer.refresh() end
  end,
  build = observer.build,
  refresh = observer.refresh,
}

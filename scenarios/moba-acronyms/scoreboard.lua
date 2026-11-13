local config = require("config")
local bounty = require("bounty")
local announce = require("announce")
local modes = require("modes")
local xp = require("xp")
local acronyms = require("hero_acronyms")

local PANEL = "scoreboard_688g"
local TITLE = "DotA Scores"
local LABEL_OPEN = "|cff99ccff"
local OWN_OPEN = "|c00FF6600"
local SENTINEL_OPEN = "|c00ff0303"
local SCOURGE_OPEN = "|c0020c000"
local CLOSE = "|r"
local BLANK = " "

local shown = nil

local scoreboard = {}

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

local function everyone(sentinel, scourge)
  local all = {}
  for _, row in ipairs(sentinel) do all[#all + 1] = row end
  for _, row in ipairs(scourge) do all[#all + 1] = row end
  return all
end

local function label(text) return LABEL_OPEN .. text .. CLOSE end

local function clock(seconds)
  local minutes = math.floor(seconds / 60)
  local rest = seconds - minutes * 60
  if rest < 10 then return tostring(minutes) .. ":0" .. tostring(rest) end
  return tostring(minutes) .. ":" .. tostring(rest)
end

local function count(row, key) return game.counter(row.hero, key) end

local function acronym(row)
  local kind = game.unit_kind(row.hero) or "?"
  return acronyms[kind] or kind
end

local function duel(row, other)
  local kills = math.max(count(row, "st_pk" .. tostring(other.player)), 0)
  local deaths = math.max(count(row, "st_pd" .. tostring(other.player)), 0)
  return acronym(other) .. " " .. tostring(kills) .. "/" .. tostring(deaths)
end

local function line(name, all, cell)
  local cells = { label(name) }
  for _, row in ipairs(all) do cells[#cells + 1] = cell(row) end
  return cells
end

local function plain(key)
  return function(row) return tostring(count(row, key)) end
end

local function own(viewer, key)
  return function(row)
    local text = tostring(count(row, key))
    if row.player == viewer then return OWN_OPEN .. text .. CLOSE end
    return text
  end
end

local function team_kills(side)
  local total = 0
  for _, row in ipairs(side) do total = total + game.kills_of(row.hero) end
  return total
end

local function mode_text()
  local text = ""
  for _, code in ipairs(modes.codes()) do text = text .. code end
  return text
end

function scoreboard.title(sentinel, scourge)
  return TITLE .. "  - " .. mode_text() .. " - " .. SENTINEL_OPEN .. tostring(team_kills(sentinel)) .. CLOSE .. "/"
    .. SCOURGE_OPEN .. tostring(team_kills(scourge)) .. CLOSE
end

local function kill_details(sentinel, scourge)
  local rows = { { label("Kill Details") } }
  for r = 1, math.max(#sentinel, #scourge) do
    local cells = { BLANK }
    for _, row in ipairs(sentinel) do cells[#cells + 1] = scourge[r] ~= nil and duel(row, scourge[r]) or BLANK end
    for _, row in ipairs(scourge) do cells[#cells + 1] = sentinel[r] ~= nil and duel(row, sentinel[r]) or BLANK end
    rows[#rows + 1] = cells
  end
  return rows
end

function scoreboard.build(viewer)
  local sentinel, scourge = roster()
  local all = everyone(sentinel, scourge)
  local columns = { BLANK }
  for _, row in ipairs(all) do columns[#columns + 1] = announce.tag(row.player) end
  local rows = {
    line(BLANK, all, function(row) return "(" .. tostring(xp.character_level(row.hero)) .. ") " .. acronym(row) end),
    line("Bonus Gold", all, own(viewer, "st_bonus_gold")),
    line("Bonus XP", all, own(viewer, "st_bonus_xp")),
    line("Time Dead", all, function(row) return clock(count(row, "st_time_dead")) end),
    line("Consumables", all, own(viewer, "st_consumable")),
    line("MultiKills", all, function(row)
      return tostring(count(row, "st_multi2")) .. "/" .. tostring(count(row, "st_multi3")) .. "/"
        .. tostring(count(row, "st_multi4")) .. "/" .. tostring(count(row, "st_multi5"))
    end),
  }
  if #sentinel > 0 and #scourge > 0 then
    for _, detail in ipairs(kill_details(sentinel, scourge)) do rows[#rows + 1] = detail end
  end
  return scoreboard.title(sentinel, scourge), columns, rows
end

local function signature(title, columns, rows)
  local text = title
  for _, cell in ipairs(columns) do text = text .. "\1" .. cell end
  for _, cells in ipairs(rows) do
    text = text .. "\2"
    for _, cell in ipairs(cells) do text = text .. "\1" .. cell end
  end
  return text
end

function scoreboard.refresh(open, viewer)
  if not open then
    if shown ~= nil then
      shown = nil
      game.panel(PANEL)
    end
    return
  end
  local title, columns, rows = scoreboard.build(viewer)
  local sig = signature(title, columns, rows)
  if sig == shown then return end
  shown = sig
  game.panel(PANEL, { title = title, player = viewer, columns = columns, rows = rows })
end

return scoreboard

local config = require("config")
local bounty = require("bounty")
local acronyms = require("hero_acronyms")

local COLUMNS = { "Hero", "Team", "K", "D", "A", "LH", "DN", "Gold", "XP", "DMG", "Taken", "Heal", "Items", "Rating", "Grade" }
local PODIUM_COLUMNS = { "Place", "Hero", "Team", "Rating", "Grade" }

local WEIGHTS = {
  kills = 25, deaths = -15, assists = 15, last_hits = 20, denies = 10, gold = 10,
  experience = 5, damage_dealt = 5, damage_taken = -5, healing = 5, items = 5,
}
local KEYS = { "kills", "deaths", "assists", "last_hits", "denies", "gold", "experience", "damage_dealt", "damage_taken", "healing", "items" }
local MINUTE_MILLI = 60000
local FIXED_POINT_UNIT = 65536
local SCALE = 1000
local RATING_STEPS = 9

local function top_points()
  local total = 0
  for _, key in ipairs(KEYS) do
    if WEIGHTS[key] > 0 then total = total + WEIGHTS[key] end
  end
  return total * SCALE
end

local TOP_POINTS = top_points()

local published = false
local final_ranked = nil

local match_stats = {}

function match_stats.final_ranked()
  return final_ranked
end

local function shown(value)
  if value == nil then return "-" end
  return tostring(value)
end

local function rate(entry, key, seconds)
  return math.floor((entry[key] or 0) * MINUTE_MILLI / seconds)
end

local function grade_of(rating)
  if rating >= 9 then return "A" end
  if rating >= 7 then return "B" end
  if rating >= 5 then return "C" end
  if rating >= 3 then return "D" end
  return "F"
end

local function ahead(a, b)
  if a.points ~= b.points then return a.points > b.points end
  return a.player < b.player
end

local function ranked(entries)
  local out = {}
  for _, entry in ipairs(entries) do
    local at = #out + 1
    out[at] = entry
    while at > 1 and ahead(out[at], out[at - 1]) do
      local held = out[at]
      out[at] = out[at - 1]
      out[at - 1] = held
      at = at - 1
    end
  end
  return out
end

function match_stats.evaluate(entries, seconds)
  local span = math.max(seconds, 1)
  local peak = {}
  for _, key in ipairs(KEYS) do
    local top = 0
    for _, entry in ipairs(entries) do
      local r = rate(entry, key, span)
      if r > top then top = r end
    end
    peak[key] = top
  end
  for _, entry in ipairs(entries) do
    local points = 0
    for _, key in ipairs(KEYS) do
      local norm = 0
      if peak[key] > 0 then norm = math.floor(rate(entry, key, span) * SCALE / peak[key]) end
      points = points + WEIGHTS[key] * norm
    end
    entry.points = points
    entry.rating = 1 + math.floor(math.max(points, 0) * RATING_STEPS / TOP_POINTS)
    entry.grade = grade_of(entry.rating)
  end
  local order = ranked(entries)
  for index, entry in ipairs(order) do entry.rank = index end
  return { ranked = order, podium = { left = order[2], centre = order[1], right = order[3] } }
end

function match_stats.cells(entry)
  return {
    entry.name, tostring(entry.team), shown(entry.kills), shown(entry.deaths), shown(entry.assists),
    shown(entry.last_hits), shown(entry.denies), shown(entry.gold), shown(entry.experience),
    shown(entry.damage_dealt), shown(entry.damage_taken), shown(entry.healing), shown(entry.items),
    tostring(entry.rating), entry.grade,
  }
end

local function podium_cells(place, entry)
  return { place, entry.name, tostring(entry.team), tostring(entry.rating), entry.grade }
end

local function whole_units(fixed)
  return math.floor(fixed / FIXED_POINT_UNIT)
end

function match_stats.collect(heroes)
  local entries = {}
  for _, h in ipairs(heroes) do
    entries[#entries + 1] = {
      player = h.player, team = h.team, name = h.name,
      kills = game.kills_of(h.hero), deaths = game.deaths_of(h.hero), assists = game.credits_of(h.hero),
      last_hits = game.counter(h.hero, "st_creep"), denies = game.counter(h.hero, "st_deny"),
      gold = bounty.earned(h.hero), experience = game.xp(h.hero),
      damage_dealt = whole_units(game.counter(h.hero, "sim_damage_dealt")),
      damage_taken = whole_units(game.counter(h.hero, "sim_damage_taken")),
      healing = whole_units(game.counter(h.hero, "sim_healing")),
      items = #game.unit_items(h.hero),
    }
  end
  return entries
end

function match_stats.roster()
  local list = {}
  for p = 0, config.PLAYERS - 1 do
    local hero = bounty.hero_of(p)
    if hero ~= nil and game.seat_start(p) ~= nil then
      local kind = game.unit_kind(hero) or "?"
      list[#list + 1] = { player = p, hero = hero, team = game.team(p), name = acronyms[kind] or kind }
    end
  end
  return list
end

function match_stats.publish(heroes)
  if published then return end
  published = true
  local seconds = math.floor(game.ticks() / 16)
  local result = match_stats.evaluate(match_stats.collect(heroes), seconds)
  final_ranked = result.ranked
  local rows = {}
  for _, entry in ipairs(result.ranked) do
    rows[#rows + 1] = match_stats.cells(entry)
  end
  game.panel("match_stats", {
    title = "Match statistics",
    text = tostring(seconds) .. " s",
    columns = COLUMNS,
    rows = rows,
  })
  local podium = {}
  if result.podium.left ~= nil then podium[#podium + 1] = podium_cells("2", result.podium.left) end
  if result.podium.centre ~= nil then podium[#podium + 1] = podium_cells("1", result.podium.centre) end
  if result.podium.right ~= nil then podium[#podium + 1] = podium_cells("3", result.podium.right) end
  game.panel("match_podium", {
    title = "Podium",
    columns = PODIUM_COLUMNS,
    rows = podium,
  })
end

return match_stats

local config = require("config")
local bounty = require("bounty")
local acronyms = require("hero_acronyms")
local xp = require("xp")

local COLUMNS = { "Hero", "Team", "K", "D", "A", "LH", "DN", "Gold", "XP", "DMG", "Taken", "Heal", "Items", "Rating", "Grade" }
local PODIUM_COLUMNS = { "Place", "Hero", "Team", "Rating", "Grade" }

local WEIGHTS = {
  kills = 25, deaths = -15, assists = 15, last_hits = 20, denies = 10, gold = 10,
  experience = 5, damage_dealt = 5, damage_taken = -5, healing = 5, items = 5,
}
local KEYS = { "kills", "deaths", "assists", "last_hits", "denies", "gold", "experience", "damage_dealt", "damage_taken", "healing", "items" }
local MINUTE_MILLI = 60000
local MINUTE_SECONDS = 60
local SAMPLE_CAP = 60
local FIXED_POINT_UNIT = 65536
local SCALE = 1000
local RATING_STEPS = 9
local PERCENT = 100
local PLAY_STYLE = {
  { key = "supporting", stat = "assists" },
  { key = "pushing", stat = "structure_damage" },
  { key = "fighting", stat = "damage_dealt" },
  { key = "farming", stat = "last_hits" },
}

local STYLE_STATS = {}
for _, lane in ipairs(PLAY_STYLE) do
  if lane.stat ~= nil then STYLE_STATS[#STYLE_STATS + 1] = lane.stat end
end

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
local final_seconds = nil
local final_minutes = 0
local history = {}
local sampled_minute = -1

local match_stats = {}

function match_stats.final_ranked()
  return final_ranked
end

function match_stats.match_seconds()
  return final_seconds
end

function match_stats.per_minute(value, seconds)
  return math.floor((value or 0) * 60 / math.max(seconds, 1))
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

local function peak_rates(entries, span, keys)
  local peak = {}
  for _, key in ipairs(keys) do
    local top = 0
    for _, entry in ipairs(entries) do
      local r = rate(entry, key, span)
      if r > top then top = r end
    end
    peak[key] = top
  end
  return peak
end

function match_stats.play_style(entries, seconds)
  local span = math.max(seconds, 1)
  local peak = peak_rates(entries, span, STYLE_STATS)
  for _, entry in ipairs(entries) do
    local bars = {}
    for _, lane in ipairs(PLAY_STYLE) do
      local percent = false
      if lane.stat ~= nil and peak[lane.stat] > 0 then
        percent = math.floor(rate(entry, lane.stat, span) * PERCENT / peak[lane.stat])
      elseif lane.stat ~= nil then
        percent = 0
      end
      bars[#bars + 1] = { key = lane.key, percent = percent }
    end
    entry.style = bars
  end
end

function match_stats.evaluate(entries, seconds)
  local span = math.max(seconds, 1)
  local peak = peak_rates(entries, span, KEYS)
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

local function item_names(held)
  local names = {}
  for _, row in ipairs(held) do names[#names + 1] = row.name end
  return names
end

function match_stats.collect(heroes)
  local entries = {}
  for _, h in ipairs(heroes) do
    local held = game.unit_items(h.hero)
    entries[#entries + 1] = {
      player = h.player, team = h.team, name = h.name,
      kills = game.kills_of(h.hero), deaths = game.deaths_of(h.hero), assists = game.credits_of(h.hero),
      last_hits = game.counter(h.hero, "st_creep"), denies = game.counter(h.hero, "st_deny"),
      gold = bounty.earned(h.hero), experience = game.xp(h.hero),
      damage_dealt = whole_units(game.counter(h.hero, "sim_damage_dealt")),
      damage_taken = whole_units(game.counter(h.hero, "sim_damage_taken")),
      structure_damage = whole_units(game.counter(h.hero, "sim_structure_damage")),
      healing = whole_units(game.counter(h.hero, "sim_healing")),
      items = #held, item_names = item_names(held), level = xp.character_level(h.hero),
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

local function totals(hero)
  return { gold = bounty.earned(hero), experience = game.xp(hero), kills = game.kills_of(hero) }
end

local function record(minute, heroes)
  for _, h in ipairs(heroes) do
    local samples = history[h.player]
    if samples == nil then
      samples = {}
      history[h.player] = samples
    end
    samples[minute] = totals(h.hero)
  end
end

function match_stats.sample_clock()
  if published then return end
  local minute = math.floor(game.ticks() / (MINUTE_SECONDS * config.TICKS_PER_SECOND))
  if minute == sampled_minute or minute > SAMPLE_CAP then return end
  sampled_minute = minute
  record(minute, match_stats.roster())
end

function match_stats.minute_rates(player, key)
  local rates = {}
  local samples = history[player]
  if samples == nil or final_seconds == nil then return rates end
  local last = 0
  if samples[0] ~= nil then last = samples[0][key] end
  for m = 1, math.min(final_minutes, SAMPLE_CAP) do
    local before = last
    if samples[m] ~= nil then last = samples[m][key] end
    local span = math.min(MINUTE_SECONDS, final_seconds - (m - 1) * MINUTE_SECONDS)
    rates[m] = math.max(0, math.floor((last - before) * MINUTE_SECONDS / math.max(span, 1)))
  end
  return rates
end

function match_stats.publish(heroes)
  if published then return end
  published = true
  local seconds = math.floor(game.ticks() / config.TICKS_PER_SECOND)
  final_seconds = seconds
  final_minutes = math.floor((seconds + MINUTE_SECONDS - 1) / MINUTE_SECONDS)
  if final_minutes >= 1 and final_minutes <= SAMPLE_CAP then record(final_minutes, heroes) end
  local result = match_stats.evaluate(match_stats.collect(heroes), seconds)
  match_stats.play_style(result.ranked, seconds)
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

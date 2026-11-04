local match_stats = require("match_stats")

local post_view = {}

local TABS = { "overview", "scoreboard", "graphs", "mvp" }
local STYLES = { "supporting", "pushing", "fighting", "farming" }
local AVERAGES = { "gold_per_minute", "xp_per_minute", "kills", "deaths", "assists" }
local PERCENT_MAX = 100
local MVP_ROWS = 3

local state = { tab = "overview", view = "comparison", picks = {} }

function post_view.reset()
  state.tab = "overview"
  state.view = "comparison"
  state.picks = {}
end

function post_view.choose_tab(tab)
  for _, known in ipairs(TABS) do
    if known == tab then state.tab = tab end
  end
end

function post_view.pick_hero(side, player)
  state.picks[side] = player
  state.view = "comparison"
end

function post_view.team_view()
  state.view = "team"
end

local function clock_of(seconds)
  local minutes = math.floor(seconds / 60)
  local rest = seconds - minutes * 60
  if rest < 10 then return tostring(minutes) .. ":0" .. tostring(rest) end
  return tostring(minutes) .. ":" .. tostring(rest)
end

local function join_names(names, texts)
  if #names == 0 then return texts.noItems end
  local out = names[1]
  for i = 2, #names do out = out .. ", " .. names[i] end
  return out
end

local function members(c, side)
  local out = {}
  for _, e in ipairs(c.ranked) do
    if c.side_of(e.player) == side then out[#out + 1] = e end
  end
  return out
end

local function entry_for(c, player)
  for _, e in ipairs(c.ranked) do
    if e.player == player then return e end
  end
  return nil
end

local function compared(c, side)
  local chosen = state.picks[side]
  if chosen ~= nil then return entry_for(c, chosen) end
  local first = nil
  for _, e in ipairs(members(c, side)) do
    if e.player == c.local_player then return e end
    first = first or e
  end
  return first
end

local function percent_of(entry, key)
  for _, lane in ipairs(entry.style) do
    if lane.key == key then return lane.percent end
  end
  return false
end

local function kda(e)
  return tostring(e.kills) .. " / " .. tostring(e.deaths) .. " / " .. tostring(e.assists)
end

local function portrait(c, e)
  local w = c.w
  return w.cell({ w.hero_picture(e.name, 40), w.label(e.name, "strong") }, {
    tone = "team" .. tostring(c.side_of(e.player)),
    tip = e.name,
    call = "post_pick",
    args = w.obj({ w.I("player", e.player) }),
  })
end

local function portraits(c)
  local w = c.w
  local columns = {}
  for _, side in ipairs(c.sides) do
    local cells = {}
    for _, e in ipairs(members(c, side)) do cells[#cells + 1] = portrait(c, e) end
    columns[#columns + 1] = w.box({ w.label(c.team_name(side), "team" .. tostring(side)), w.box(cells, "row") }, "col")
  end
  return w.box(columns, "row")
end

local function team_totals(c)
  local w = c.w
  local texts = c.texts
  local rows = {}
  for _, side in ipairs(c.sides) do
    local kills, deaths, assists, gold, xp = 0, 0, 0, 0, 0
    for _, e in ipairs(members(c, side)) do
      kills = kills + e.kills
      deaths = deaths + e.deaths
      assists = assists + e.assists
      gold = gold + e.gold
      xp = xp + e.experience
    end
    local line = c.team_name(side) .. " " .. texts.teamTotals .. ": " .. tostring(kills) .. " " .. texts.kills
      .. " / " .. tostring(deaths) .. " " .. texts.deaths .. " / " .. tostring(assists) .. " " .. texts.assists
      .. ", " .. tostring(gold) .. " " .. texts.gold .. ", " .. tostring(xp) .. " " .. texts.xp
    rows[#rows + 1] = w.label(line, "team" .. tostring(side))
  end
  return w.box(rows, "col")
end

local function style_cell(c, entry, key, tone)
  local w = c.w
  local percent = percent_of(entry, key)
  if percent == false then return w.label(c.texts.notTracked, "muted") end
  return w.box({ w.bar(percent, PERCENT_MAX, tone), w.label(tostring(percent) .. "%", tone) }, "row")
end

local function comparison(c)
  local w = c.w
  local texts = c.texts
  local left_side, right_side = c.sides[1], c.sides[2]
  local a = right_side ~= nil and compared(c, left_side) or nil
  local b = a ~= nil and compared(c, right_side) or nil
  if a == nil or b == nil then return w.label(texts.noPick, "muted") end
  local ta, tb = "team" .. tostring(left_side), "team" .. tostring(right_side)

  local rows = {}
  local function row(name, left, right)
    rows[#rows + 1] = w.label(left, ta)
    rows[#rows + 1] = w.label(name, "muted")
    rows[#rows + 1] = w.label(right, tb)
  end
  local function per_minute(e, key)
    return tostring(match_stats.per_minute(e[key], c.seconds))
  end
  row(texts.level, tostring(a.level), tostring(b.level))
  row(texts.kda, kda(a), kda(b))
  row(texts.goldPerMinute, per_minute(a, "gold"), per_minute(b, "gold"))
  row(texts.xpPerMinute, per_minute(a, "experience"), per_minute(b, "experience"))
  row(texts.gold, tostring(a.gold), tostring(b.gold))
  row(texts.experience, tostring(a.experience), tostring(b.experience))
  row(texts.lastHits, tostring(a.last_hits), tostring(b.last_hits))
  row(texts.denies, tostring(a.denies), tostring(b.denies))
  row(texts.damageDealt, tostring(a.damage_dealt), tostring(b.damage_dealt))
  row(texts.healing, tostring(a.healing), tostring(b.healing))
  row(texts.items, join_names(a.item_names, texts), join_names(b.item_names, texts))

  local styles = {}
  for _, key in ipairs(STYLES) do
    styles[#styles + 1] = style_cell(c, a, key, ta)
    styles[#styles + 1] = w.box({ w.label(texts.style[key], "accent"), w.label(texts.formula[key], "muted") }, "col")
    styles[#styles + 1] = style_cell(c, b, key, tb)
  end

  local averages = {}
  for _, key in ipairs(AVERAGES) do
    averages[#averages + 1] = w.label(texts.notTracked, ta)
    averages[#averages + 1] = w.label(texts.average[key], "muted")
    averages[#averages + 1] = w.label(texts.notTracked, tb)
  end

  return w.box({
    w.grid(3, { w.label(a.name, ta), w.label(texts.vs, "muted"), w.label(b.name, tb) }),
    w.grid(3, rows),
    w.label(texts.playStyle, "accent"),
    w.grid(3, styles),
    w.label(texts.averages, "accent"),
    w.label(texts.averagesNote, "muted"),
    w.grid(3, averages),
  }, "col")
end

local function overview(c)
  local w = c.w
  local kids = { portraits(c) }
  if state.view == "team" then
    kids[#kids + 1] = team_totals(c)
  else
    kids[#kids + 1] = comparison(c)
    kids[#kids + 1] = w.button(c.texts.teamView, "post_team_view", "{}")
  end
  return w.box(kids, "col")
end

local function mvp(c)
  local w = c.w
  local kids = {}
  for i = 1, math.min(MVP_ROWS, #c.ranked) do
    local e = c.ranked[i]
    kids[#kids + 1] = w.label(tostring(i) .. " " .. e.name .. "  " .. tostring(e.rating) .. " " .. e.grade, "team" .. tostring(c.side_of(e.player)))
  end
  if #kids == 0 then return w.label(c.texts.notTracked, "muted") end
  kids[#kids + 1] = w.label(c.texts.mvpFormula, "muted")
  return w.box(kids, "col")
end

local function tab_strip(c)
  local w = c.w
  local buttons = {}
  for _, tab in ipairs(TABS) do
    buttons[#buttons + 1] = w.button(c.texts.tabs[tab], "post_tab", w.obj({ w.S("tab", tab) }), tab == state.tab, c.texts.tabs[tab])
  end
  return w.box(buttons, "row")
end

local function timeline(c)
  local w = c.w
  local menu = c.menu
  local kids = { w.label(c.texts.matchLength .. " " .. clock_of(c.seconds), "muted") }
  if c.note ~= nil then kids[#kids + 1] = w.label(c.note, "ok") end
  kids[#kids + 1] = w.button(menu.saveReplay, "win_save_replay", "{}")
  kids[#kids + 1] = w.button(menu.playAgain, "win_play_again", "{}")
  kids[#kids + 1] = w.button(menu.newGame, "win_new_game", "{}")
  kids[#kids + 1] = w.button(menu.close, "win_close", "{}")
  return w.box(kids, "row")
end

function post_view.kids(c)
  local w = c.w
  local body
  if state.tab == "scoreboard" then
    body = c.scoreboard()
  elseif state.tab == "graphs" then
    body = w.label(c.texts.graphs, "muted")
  elseif state.tab == "mvp" then
    body = mvp(c)
  else
    body = overview(c)
  end
  return { tab_strip(c), body, timeline(c) }
end

return post_view

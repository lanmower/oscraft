local data = require("draft_data")
local modes = require("modes")

local TPS = 16
local HALF = 5
local SEATS = 10
local START = 15
local CM_BONUS = 110
local CM_BAN = 30
local CM_PICK = 40
local CD_FLOOR = 6
local CD_TURNS = { 1, 1, 1, 1, 1, 2, 2, 2, 2, 1 }
local CD_TURNS_X = { 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 1 }
local CD_BANS = 4
local CD_BANS_X = 6
local CD_RESERVE = 150
local CD_RESERVE_X = 180
local CD_POOL = 28
local CD_POOL_X = 42
local RD_POOL = 24
local RD_STAGE = 20
local RV_CLOSE = 57
local RV_WINDOW = 60
local SD_WINDOW = 120
local CLAIM_WINDOW = 90
local CD_CLAIM_DELAY = 6
local CM_CLAIM_DELAY = 1
local CODES = { "cm", "cd", "rd", "sd", "rv" }
local GROUP_ORDER = { 2, 1, 3 }

local CM = {
  { "ban", "A" }, { "ban", "B" }, { "ban", "A" }, { "ban", "B" },
  { "pick", "A" }, { "pick", "B" }, { "pick", "B" }, { "pick", "A" },
  { "ban", "A" }, { "ban", "B" }, { "ban", "A" }, { "ban", "B" },
  { "pick", "B" }, { "pick", "A" }, { "pick", "B" }, { "pick", "A" },
  { "ban", "A" }, { "ban", "B" },
  { "pick", "B" }, { "pick", "A" },
}
local CM_PHASE = {
  [1] = { "Ban Phase #1", "First ban phase has begun" },
  [5] = { "Pick Phase #1", "First pick phase has begun" },
  [9] = { "Ban Phase #2", "Second ban phase has begun" },
  [13] = { "Pick Phase #2", "Second pick phase has begun" },
  [17] = { "Ban Phase #3", "Third ban phase has begun" },
  [19] = { "Pick Phase #3", "Third pick phase has begun" },
}
local RD_ROUNDS = {
  { "A", { 1 } }, { "B", { 1, 2 } }, { "A", { 2, 3 } }, { "B", { 3, 4 } }, { "A", { 4, 5 } }, { "B", { 5 } },
}

local TEXT = {
  names = { cm = "Captains Mode", cd = "Captains Draft", rd = "Random Draft", sd = "Single Draft", rv = "Reverse" },
  sides = { [0] = "Sentinel", [1] = "Scourge" },
  begin = "will now begin the 1-2-2-2-2-1 draft",
  captain = "has been randomly selected to begin the draft",
  banTurn = "Your turn to ban 1 hero",
  pickOne = "Your turn to pick 1 hero",
  pickTwo = "Your turn to pick 2 heroes",
  banned = "has banned",
  picked = "has picked",
  randomBanned = "has randomly banned",
  randomPicked = "has randomly picked",
  over = "The drafting phase is over. The heroes will be available for picking shortly.",
  seconds = "has 20 seconds to pick a hero",
  select = "|c006699CCSelect a hero and accept him with the|r |c00ff0303Select Hero|r |c00ff0303button|r",
  already = "You already have a hero",
  notTurn = "It is not your turn to pick",
  chosen = "You have already selected a hero",
  circle = "Three hero choices are now available at your Circle of Power",
  reverse = "You pick a hero for your opponent.",
  wait = "Wait",
  done = "Done",
  active = "Active",
}
local ATTRIBUTE = { [1] = "Strength", [2] = "Agility", [3] = "Intelligence" }
local BLUE = "|c006699CC"

local draft = {}
local api = nil
local D = nil
local sent = {}
local shown = {}
local roster_cache = nil

local function lobby() return api.state() end

local function slots() return api.state().slots end

local function side_of(i)
  if i <= HALF then return 0 end
  return 1
end

local function seat_at(side, j) return side * HALF + j end

local function present(i)
  local s = slots()[i]
  return s ~= nil and s.owner ~= nil and not s.bot
end

local function seated(i)
  local s = slots()[i]
  return s ~= nil and s.owner ~= nil
end

local function seat_name(i) return game.player_name(i - 1) end

local function hero_name(h) return api.hero_name(h) end

local function join(list, sep)
  local out = ""
  for i, v in ipairs(list) do
    if i > 1 then out = out .. sep end
    out = out .. v
  end
  return out
end

local function active_code()
  for _, c in ipairs(CODES) do
    if modes.has(c) then return c end
  end
  return nil
end

local function roster_set()
  local set = {}
  for _, h in ipairs(api.roster()) do set[h] = true end
  return set
end

local function pool_has(h) return roster_cache ~= nil and roster_cache[h] == true end

local function random_index(span) return game.random(span) + 1 end

local function draw_index()
  if game.random(2) == 0 then return random_index(data.SENTINEL_LAST) end
  return data.SENTINEL_LAST + random_index(#data.heroes - data.SENTINEL_LAST)
end

local function draw_hero()
  local idx = draw_index()
  return data.heroes[idx], idx
end

local function pool_index(h)
  for i, name in ipairs(data.heroes) do
    if name == h then return i end
  end
  return nil
end

local function free_heroes()
  local out = {}
  for _, h in ipairs(data.heroes) do
    if pool_has(h) and D.taken[h] == nil then out[#out + 1] = h end
  end
  return out
end

local function random_free()
  local list = free_heroes()
  if #list == 0 then return nil end
  return list[random_index(#list)]
end

local function emit(id, player, spec)
  local key = id .. ":" .. tostring(player)
  local parts = { spec.title or "", spec.text or "" }
  for _, c in ipairs(spec.columns or {}) do parts[#parts + 1] = c end
  for _, row in ipairs(spec.rows or {}) do parts[#parts + 1] = join(row, "\1") end
  local sig = join(parts, "\2")
  if sent[key] == sig then return end
  sent[key] = sig
  shown[id] = true
  local args = { title = spec.title, text = spec.text, columns = spec.columns, rows = spec.rows }
  if player ~= nil then args.player = player end
  game.panel(id, args)
end

local function clear_panels()
  for _, id in ipairs({ "draft", "draft_pool" }) do
    if shown[id] then game.panel(id) end
  end
  sent = {}
  shown = {}
end

local function say(text, seconds) game.message(-1, text, seconds or 15) end

local function say_side(side, text, seconds)
  game.message(-1, BLUE .. text .. "|r", seconds or 15, seat_at(side, 1) - 1)
end

local function team_text(side) return TEXT.sides[side] end

local function secs(ticks) return ticks // TPS end

local function assign(i, hero)
  local s = slots()[i]
  s.hero = hero
  s.ready = true
end

local function finish()
  D.done = true
  D.phase = "done"
  for i = 1, SEATS do
    local s = slots()[i]
    if s.owner ~= nil then s.ready = true end
  end
  api.changed()
end

local function claim_pool(i)
  local out = {}
  for _, h in ipairs(D.picks[side_of(i)]) do
    if D.claimed[h] == nil then out[#out + 1] = h end
  end
  return out
end

local function claim_begin(delay)
  D.phase = "claim"
  D.claimed = {}
  D.claim_start = D.clock + delay * TPS
  D.claim_end = D.claim_start + CLAIM_WINDOW * TPS
end

local function claim_fill_bots()
  for i = 1, SEATS do
    local s = slots()[i]
    if s.owner ~= nil and s.bot and s.hero == nil then
      local free = claim_pool(i)
      if #free > 0 then
        D.claimed[free[1]] = i
        assign(i, free[1])
      end
    end
  end
  api.changed()
end

local function claim_all_held()
  for i = 1, SEATS do
    if seated(i) and slots()[i].hero == nil then return false end
  end
  return true
end

local function claim_leftovers()
  for i = 1, SEATS do
    local s = slots()[i]
    if s.owner ~= nil and s.hero == nil then
      local free = claim_pool(i)
      if #free > 0 then
        D.claimed[free[1]] = i
        assign(i, free[1])
      end
    end
  end
end

local function claim_tick()
  if D.clock < D.claim_start then return end
  if D.clock == D.claim_start then claim_fill_bots() end
  if claim_all_held() or D.clock >= D.claim_end then
    claim_leftovers()
    finish()
  end
end

local function claim_press(i, hero)
  local s = slots()[i]
  if D.clock < D.claim_start then return "draft-wait" end
  if s.hero ~= nil then return "draft-picked" end
  local ok = false
  for _, h in ipairs(claim_pool(i)) do
    if h == hero then ok = true end
  end
  if not ok then return "draft-taken" end
  D.claimed[hero] = i
  assign(i, hero)
  if claim_all_held() then finish() else api.changed() end
  return nil
end

local cm = {}
local cd = {}
local rd = {}
local sd = {}
local rv = {}

function cm.setup()
  D.first = game.random(2)
  D.banks = { [0] = CM_BONUS, [1] = CM_BONUS }
  D.step = 0
  D.picks = { [0] = {}, [1] = {} }
  D.bans = { [0] = {}, [1] = {} }
  D.log = {}
end

local function cm_side(k)
  if CM[k][2] == "A" then return D.first end
  return 1 - D.first
end

local function cm_enter(k)
  D.step = k
  D.step0 = D.clock
  D.kind = CM[k][1]
  D.side = cm_side(k)
  D.seat = seat_at(D.side, 1)
  D.base = D.kind == "ban" and CM_BAN or CM_PICK
  D.phase = "turns"
  if k == 1 then
    say(seat_name(seat_at(D.first, 1)) .. " " .. TEXT.captain)
  end
  if CM_PHASE[k] ~= nil then say(BLUE .. CM_PHASE[k][2] .. "|r") end
  if not present(D.seat) then
    if D.kind == "pick" then
      cm.apply(random_free(), true)
    else
      cm.apply(nil, true)
    end
  end
end

function cm.apply(hero, auto)
  local k = D.step
  D.log[k] = { side = D.side, seat = D.seat, kind = D.kind, hero = hero, auto = auto }
  if hero ~= nil then
    D.taken[hero] = D.kind
    if D.kind == "pick" then
      D.picks[D.side][#D.picks[D.side] + 1] = hero
    else
      D.bans[D.side][#D.bans[D.side] + 1] = hero
    end
  end
  if k == #CM then
    claim_begin(CM_CLAIM_DELAY)
    return
  end
  cm_enter(k + 1)
end

function cm.tick()
  local t = D.clock
  if D.phase == "wait" then
    if t == START * TPS then cm_enter(1) end
  elseif D.phase == "turns" then
    local dt = t - D.step0
    if dt > 0 and dt % TPS == 0 then
      local elapsed = dt // TPS
      if elapsed > D.base then
        if D.banks[D.side] > 0 then
          D.banks[D.side] = D.banks[D.side] - 1
        elseif D.kind == "pick" then
          cm.apply(random_free(), true)
        else
          cm.apply(nil, true)
        end
      end
    end
  end
  if D.phase == "claim" then claim_tick() end
end

function cm.press(i, hero)
  if D.phase == "wait" then return "draft-wait" end
  if D.phase == "claim" then return claim_press(i, hero) end
  if D.phase ~= "turns" then return "draft-wait" end
  if i ~= D.seat then return "draft-turn" end
  if not pool_has(hero) or pool_index(hero) == nil or D.taken[hero] ~= nil then return "draft-taken" end
  cm.apply(hero, false)
  api.changed()
  return nil
end

function cm.allows(hero)
  if D.phase == "claim" then
    for side = 0, 1 do
      for _, h in ipairs(D.picks[side]) do
        if h == hero then return true end
      end
    end
    return false
  end
  return pool_index(hero) ~= nil and D.taken[hero] == nil
end

function cd.setup()
  D.extended = modes.has("x")
  D.turns = D.extended and CD_TURNS_X or CD_TURNS
  D.ban_turns = D.extended and CD_BANS_X or CD_BANS
  D.cards = {}
  local want = D.extended and CD_POOL_X or CD_POOL
  local seen = {}
  while #D.cards < want do
    local h = draw_hero()
    if pool_has(h) and not seen[h] then
      seen[h] = true
      D.cards[#D.cards + 1] = h
    end
  end
  D.picks = { [0] = {}, [1] = {} }
  D.bans = { [0] = {}, [1] = {} }
  D.turn_log = {}
  D.turn = 0
end

local function cd_kind(turn)
  if turn <= D.ban_turns then return "ban" end
  return "pick"
end

local function cd_turn_side(turn)
  if turn % 2 == 1 then return D.first end
  return 1 - D.first
end

local function cd_card(hero)
  for _, h in ipairs(D.cards) do
    if h == hero then return true end
  end
  return false
end

local function cd_announce()
  local left = D.turns[D.turn]
  local text = TEXT.banTurn
  if cd_kind(D.turn) == "pick" then text = left == 2 and TEXT.pickTwo or TEXT.pickOne end
  say_side(D.side, text)
end

function cd.apply(hero, auto)
  local kind = cd_kind(D.turn)
  local seat = seat_at(D.side, 1)
  D.taken[hero] = kind
  if kind == "pick" then
    D.picks[D.side][#D.picks[D.side] + 1] = hero
  else
    D.bans[D.side][#D.bans[D.side] + 1] = hero
  end
  local row = D.turn_log[D.turn] or {}
  row[#row + 1] = hero
  D.turn_log[D.turn] = row
  D.left = D.left - 1
  local verb
  if kind == "ban" then
    verb = auto and TEXT.randomBanned or TEXT.banned
  else
    verb = auto and TEXT.randomPicked or TEXT.picked
  end
  game.message(-1, seat_name(seat) .. " " .. verb .. " " .. hero_name(hero), 10)
end

local function cd_free_cards()
  local out = {}
  for _, h in ipairs(D.cards) do
    if D.taken[h] == nil then out[#out + 1] = h end
  end
  return out
end

local function cd_auto()
  local free = cd_free_cards()
  if #free > 0 then cd.apply(free[random_index(#free)], true) end
end

local function cd_absent_actions()
  while D.left > 0 and not present(seat_at(D.side, 1)) do cd_auto() end
end

local function cd_open()
  D.phase = "turns"
  D.first = game.random(2)
  D.turn = 1
  D.side = D.first
  D.fl = { [0] = D.extended and CD_RESERVE_X or CD_RESERVE, [1] = D.extended and CD_RESERVE_X or CD_RESERVE }
  D.left = D.turns[1]
  say(TEXT.sides[D.first] .. " " .. TEXT.begin)
  cd_announce()
  cd_absent_actions()
end

local function cd_eval()
  local side = D.side
  D.fl[side] = math.max(D.fl[side] - 1, 0)
  if D.left < 1 then
    D.turn = D.turn + 1
    D.side = 1 - side
    D.fl[D.side] = math.max(D.fl[D.side], CD_FLOOR)
    if D.turn > #D.turns then
      say(BLUE .. TEXT.over .. "|r")
      claim_begin(CD_CLAIM_DELAY)
      return
    end
    D.left = D.turns[D.turn]
    cd_announce()
    cd_absent_actions()
  elseif D.fl[side] == 0 then
    cd_auto()
  end
end

function cd.tick()
  local t = D.clock
  if D.phase == "wait" then
    if t == START * TPS then cd_open() end
  elseif D.phase == "turns" then
    local dt = t - START * TPS
    if dt > 0 and dt % TPS == 0 then cd_eval() end
  end
  if D.phase == "claim" then claim_tick() end
end

function cd.press(i, hero)
  if D.phase == "wait" then return "draft-wait" end
  if D.phase == "claim" then return claim_press(i, hero) end
  if D.phase ~= "turns" then return "draft-wait" end
  if D.left < 1 or i ~= seat_at(D.side, 1) then return "draft-turn" end
  if not cd_card(hero) or D.taken[hero] ~= nil then return "draft-taken" end
  cd.apply(hero, false)
  api.changed()
  return nil
end

function cd.allows(hero)
  if D.phase == "claim" then
    for side = 0, 1 do
      for _, h in ipairs(D.picks[side]) do
        if h == hero then return true end
      end
    end
    return false
  end
  return cd_card(hero) and D.taken[hero] == nil
end

function rd.setup()
  D.cards = {}
  D.card_seat = {}
  local seen = {}
  while #D.cards < RD_POOL do
    local h, idx = draw_hero()
    if pool_has(h) and not seen[h] and idx ~= data.TECHIES then
      seen[h] = true
      D.cards[#D.cards + 1] = h
    end
  end
  D.stage = -1
  D.pickers = {}
  D.picked = {}
end

local function rd_free_card()
  for _, h in ipairs(D.cards) do
    if D.taken[h] == nil then return h end
  end
  return nil
end

local function rd_assign(i, hero)
  D.taken[hero] = i
  D.picked[i] = hero
  assign(i, hero)
end

local function rd_card(hero)
  for _, h in ipairs(D.cards) do
    if h == hero then return true end
  end
  return false
end

local function rd_settle(list)
  for _, i in ipairs(list) do
    if D.picked[i] == nil then
      local h = rd_free_card()
      if h ~= nil then rd_assign(i, h) end
    end
  end
end

local function rd_stage(k)
  rd_settle(D.pickers)
  D.stage = k
  D.pickers = {}
  if k == 0 then
    D.first = game.random(2)
    say(TEXT.sides[D.first] .. " " .. TEXT.begin)
  end
  if k >= #RD_ROUNDS then
    api.changed()
    finish()
    return
  end
  local round = RD_ROUNDS[k + 1]
  local side = round[1] == "A" and D.first or 1 - D.first
  for _, j in ipairs(round[2]) do
    local i = seat_at(side, j)
    D.pickers[#D.pickers + 1] = i
    game.message(-1, seat_name(i) .. " " .. TEXT.seconds, 15)
    game.message(i - 1, TEXT.select, 10)
  end
  for _, i in ipairs(D.pickers) do
    if not present(i) and D.picked[i] == nil then
      local h = rd_free_card()
      if h ~= nil then rd_assign(i, h) end
    end
  end
  api.changed()
end

function rd.tick()
  local t = D.clock
  local base = START * TPS
  if t >= base and (t - base) % (RD_STAGE * TPS) == 0 and D.phase ~= "done" then
    D.phase = "turns"
    rd_stage((t - base) // (RD_STAGE * TPS))
  end
end

function rd.press(i, hero)
  if D.phase == "wait" then return "draft-wait" end
  if D.picked[i] ~= nil then return "draft-picked" end
  local mine = false
  for _, p in ipairs(D.pickers) do
    if p == i then mine = true end
  end
  if not mine then return "draft-turn" end
  if not rd_card(hero) or D.taken[hero] ~= nil then return "draft-taken" end
  rd_assign(i, hero)
  api.changed()
  return nil
end

function rd.allows(hero) return rd_card(hero) and D.taken[hero] == nil end

local function group_list(group)
  local out = {}
  for idx, h in ipairs(data.heroes) do
    if string.sub(data.groups, idx, idx) == tostring(group) and pool_has(h) then out[#out + 1] = h end
  end
  return out
end

local function sd_offer(group)
  local list = D.lists[group]
  if #list == 0 then return nil end
  local h = list[random_index(#list)]
  if modes.duplicates() then return h end
  local guard = 0
  while D.used[h] ~= nil and guard < 4096 do
    h = list[random_index(#list)]
    guard = guard + 1
  end
  D.used[h] = true
  return h
end

function sd.setup()
  D.lists = {}
  for _, g in ipairs(GROUP_ORDER) do D.lists[g] = group_list(g) end
  D.used = {}
  D.offers = {}
  for j = 1, HALF do
    for side = 0, 1 do
      local i = seat_at(side, j)
      if seated(i) then
        local offered = {}
        for _, g in ipairs(GROUP_ORDER) do offered[#offered + 1] = sd_offer(g) end
        D.offers[i] = offered
      end
    end
  end
  D.picked = {}
  D.phase = "turns"
  for i = 1, SEATS do
    if D.offers[i] ~= nil then game.message(i - 1, BLUE .. TEXT.circle .. "|r", 15) end
  end
end

local function sd_all()
  for i = 1, SEATS do
    if D.offers[i] ~= nil and slots()[i].hero == nil then return false end
  end
  return true
end

function sd.tick()
  if D.clock >= SD_WINDOW * TPS or sd_all() then
    for i = 1, SEATS do
      if D.offers[i] ~= nil and slots()[i].hero == nil then assign(i, D.offers[i][1]) end
    end
    finish()
  end
end

function sd.press(i, hero)
  if D.picked[i] ~= nil then return "draft-picked" end
  local offered = D.offers[i]
  if offered == nil then return "draft-taken" end
  local ok = false
  for _, h in ipairs(offered) do
    if h == hero then ok = true end
  end
  if not ok then return "draft-taken" end
  D.picked[i] = hero
  assign(i, hero)
  if sd_all() then finish() else api.changed() end
  return nil
end

function sd.allows(hero)
  for _, offered in pairs(D.offers) do
    for _, h in ipairs(offered) do
      if h == hero then return true end
    end
  end
  return false
end

function rv.setup()
  D.partner = {}
  for j = 1, HALF do
    local a, b = seat_at(0, j), seat_at(1, j)
    D.partner[a] = b
    D.partner[b] = a
  end
  D.picked = {}
  D.used = {}
  D.closed = false
  D.phase = "turns"
  say(BLUE .. TEXT.reverse .. "|r")
end

local function rv_random(i)
  local lo, hi = 1, data.SENTINEL_LAST
  if side_of(i) == 1 then lo, hi = data.SENTINEL_LAST + 1, #data.heroes end
  local free = {}
  for idx = lo, hi do
    local h = data.heroes[idx]
    if pool_has(h) and D.used[h] == nil then free[#free + 1] = h end
  end
  if #free == 0 then
    for _, h in ipairs(data.heroes) do
      if pool_has(h) and D.used[h] == nil then free[#free + 1] = h end
    end
  end
  if #free == 0 then return nil end
  local h = free[random_index(#free)]
  D.used[h] = true
  return h
end

local function rv_finish()
  for i = 1, SEATS do
    if seated(i) then slots()[i].hero = nil end
  end
  for i = 1, SEATS do
    local p = D.partner[i]
    if D.picked[i] ~= nil and seated(p) then slots()[p].hero = D.picked[i] end
  end
  for i = 1, SEATS do
    local s = slots()[i]
    if s.owner ~= nil and s.hero == nil then s.hero = rv_random(i) end
  end
  for i = 1, SEATS do
    local s = slots()[i]
    if s.owner ~= nil then s.ready = true end
  end
  finish()
end

function rv.tick()
  local t = D.clock
  if t == RV_CLOSE * TPS then D.closed = true end
  if t >= RV_WINDOW * TPS then rv_finish() end
end

function rv.press(i, hero)
  if D.closed then return "draft-closed" end
  if D.picked[i] ~= nil then return "draft-picked" end
  if pool_index(hero) == nil or not pool_has(hero) then return "draft-taken" end
  if D.used[hero] ~= nil and not modes.duplicates() then return "draft-taken" end
  if D.partner[i] == nil or not seated(D.partner[i]) then return "draft-taken" end
  D.picked[i] = hero
  if not modes.duplicates() then D.used[hero] = true end
  api.changed()
  return nil
end

function rv.allows(hero)
  if pool_index(hero) == nil then return false end
  return modes.duplicates() or D.used[hero] == nil
end

local KINDS = { cm = cm, cd = cd, rd = rd, sd = sd, rv = rv }

local function name_or(h, none)
  if h == nil then return none end
  return hero_name(h)
end

local function cm_rows()
  local rows = {}
  for k = 1, #CM do
    local entry = D.log[k]
    local mark = k == D.step and D.phase == "turns" and "> " or ""
    local hero = "-"
    if entry ~= nil then
      hero = entry.hero ~= nil and hero_name(entry.hero) or "(none)"
      if entry.auto then hero = hero .. " (auto)" end
    end
    rows[#rows + 1] = { mark .. tostring(k), team_text(cm_side(k)), CM[k][1], hero }
  end
  return rows
end

local function cd_rows()
  local rows = {}
  for turn = 1, #D.turns do
    local side = D.first ~= nil and cd_turn_side(turn) or nil
    local heroes = {}
    for _, h in ipairs(D.turn_log[turn] or {}) do heroes[#heroes + 1] = hero_name(h) end
    local mark = turn == D.turn and D.phase == "turns" and "> " or ""
    rows[#rows + 1] = {
      mark .. tostring(turn), side ~= nil and team_text(side) or "-", cd_kind(turn) .. " " .. tostring(D.turns[turn]), join(heroes, ", "),
    }
  end
  return rows
end

local function rd_rows()
  local rows = {}
  for k, round in ipairs(RD_ROUNDS) do
    local side = D.first ~= nil and (round[1] == "A" and D.first or 1 - D.first) or nil
    local names, state = {}, TEXT.wait
    if side ~= nil then
      local all = true
      for _, j in ipairs(round[2]) do
        local i = seat_at(side, j)
        names[#names + 1] = seat_name(i)
        if D.picked[i] == nil then all = false end
      end
      if all then state = TEXT.done elseif D.stage == k - 1 then state = TEXT.active end
    end
    rows[#rows + 1] = { tostring(k), side ~= nil and team_text(side) or "-", join(names, ", "), state }
  end
  return rows
end

local function pool_rows(cards)
  local rows = {}
  for r = 1, (#cards + 3) // 4 do
    local row = {}
    for c = 1, 4 do
      local h = cards[(r - 1) * 4 + c]
      local cell = ""
      if h ~= nil then
        cell = hero_name(h)
        local state = D.taken[h]
        if state == "ban" then
          cell = "|cff777777" .. cell .. " (ban)|r"
        elseif state == "pick" then
          cell = "|cff66cc66" .. cell .. " (pick)|r"
        elseif type(state) == "number" then
          cell = "|cff66cc66" .. cell .. " (" .. seat_name(state) .. ")|r"
        end
      end
      row[c] = cell
    end
    rows[#rows + 1] = row
  end
  return rows
end

local function remaining(from, window)
  return math.max(0, math.ceil((from + window * TPS - D.clock) / TPS))
end

local function status_text()
  local code = D.mode
  if D.phase == "done" then return TEXT.done end
  if D.phase == "wait" then return "Draft starts in " .. tostring(math.max(0, START - secs(D.clock))) .. " s" end
  if code == "cm" and D.phase == "turns" then
    local left = math.max(0, D.base - secs(D.clock - D.step0))
    return team_text(D.side) .. " captain " .. seat_name(D.seat) .. ": " .. D.kind .. " | " .. tostring(left) .. " s | bonus " .. team_text(0) .. " " .. tostring(D.banks[0]) .. " s, " .. team_text(1) .. " " .. tostring(D.banks[1]) .. " s"
  end
  if code == "cd" and D.phase == "turns" then
    return team_text(D.side) .. " captain " .. seat_name(seat_at(D.side, 1)) .. ": " .. cd_kind(D.turn) .. " " .. tostring(D.left) .. " | reserve " .. team_text(0) .. " " .. tostring(D.fl[0]) .. " s, " .. team_text(1) .. " " .. tostring(D.fl[1]) .. " s"
  end
  if code == "rd" then
    local names = {}
    for _, i in ipairs(D.pickers) do names[#names + 1] = seat_name(i) end
    return join(names, ", ") .. " " .. TEXT.seconds
  end
  if D.phase == "claim" then
    return "Pick your hero from your team's picks: " .. tostring(math.max(0, remaining(D.claim_start, CLAIM_WINDOW))) .. " s"
  end
  if code == "sd" then return "Pick one of your three heroes: " .. tostring(remaining(0, SD_WINDOW)) .. " s" end
  if code == "rv" then return TEXT.reverse .. " " .. tostring(remaining(0, RV_WINDOW)) .. " s" end
  return ""
end

local function publish()
  if D == nil then return end
  local code = D.mode
  local text = status_text()
  if code == "cm" then
    emit("draft", nil, { title = TEXT.names.cm, text = text, columns = { "#", "Team", "Action", "Hero" }, rows = cm_rows() })
  elseif code == "cd" then
    emit("draft", nil, { title = TEXT.names.cd, text = text, columns = { "#", "Team", "Action", "Heroes" }, rows = cd_rows() })
    emit("draft_pool", nil, { title = TEXT.names.cd .. " pool", text = "", rows = pool_rows(D.cards) })
  elseif code == "rd" then
    emit("draft", nil, { title = TEXT.names.rd, text = text, columns = { "#", "Team", "Players", "Status" }, rows = rd_rows() })
    emit("draft_pool", nil, { title = TEXT.names.rd .. " pool", text = "", rows = pool_rows(D.cards) })
  elseif code == "sd" then
    for i = 1, SEATS do
      if D.offers[i] ~= nil then
        local rows = {}
        for n, h in ipairs(D.offers[i]) do
          rows[#rows + 1] = { hero_name(h), ATTRIBUTE[GROUP_ORDER[n]], D.picked[i] == h and TEXT.done or "" }
        end
        emit("draft", i - 1, { title = TEXT.names.sd, text = text, columns = { "Hero", "Attribute", "Status" }, rows = rows })
      end
    end
  elseif code == "rv" then
    for i = 1, SEATS do
      if seated(i) then
        local rows = {}
        for j = 1, SEATS do
          if seated(j) then
            local state = D.picked[j] ~= nil and TEXT.done or TEXT.wait
            local shown = j == i and D.picked[j] ~= nil and hero_name(D.picked[j]) or state
            rows[#rows + 1] = { seat_name(j), team_text(side_of(j)), shown }
          end
        end
        emit("draft", i - 1, { title = TEXT.names.rv, text = text, columns = { "Player", "Team", "Pick" }, rows = rows })
      end
    end
  end
end

function draft.attach(lobby_api)
  api = lobby_api
  modes.restrict(function(hero)
    if D == nil or D.done then return true end
    return KINDS[D.mode].allows(hero)
  end)
end

function draft.active() return D ~= nil and not D.done end

function draft.pending()
  if api == nil then return false end
  local code = active_code()
  if code == nil then return false end
  return D == nil or (D.mode ~= code)
end

function draft.mode() return D ~= nil and D.mode or nil end

function draft.begin()
  local code = active_code()
  if code == nil then return false, "no-draft" end
  roster_cache = roster_set()
  D = { mode = code, clock = 0, taken = {}, picks = { [0] = {}, [1] = {} }, phase = "wait", done = false, claimed = {} }
  lobby().draft = D
  for i = 1, SEATS do
    local s = slots()[i]
    if s.owner ~= nil then
      s.hero = nil
      s.ready = false
    end
  end
  sent = {}
  KINDS[code].setup()
  api.changed()
  publish()
  return true, nil
end

function draft.press(owner, i, hero)
  if D == nil or D.done then return "draft-over" end
  local s = slots()[i]
  if s == nil then return "bad-slot" end
  if s.owner ~= owner then return "not-owner" end
  if hero == nil or hero == "" then return "unknown-hero" end
  local err = KINDS[D.mode].press(i, hero)
  publish()
  return err
end

function draft.blocks(call)
  if D == nil or D.done then return false end
  return call ~= "pick" and call ~= "world_ready" and call ~= "release" and call ~= "remove"
end

local function tick()
  if D == nil or D.done then return end
  D.clock = D.clock + 1
  KINDS[D.mode].tick()
  if D.clock % TPS == 0 or D.done then publish() end
end

function draft.advance(ticks)
  for _ = 1, ticks do tick() end
end

function draft.clock(owner, ticks)
  if D == nil or D.done then return "draft-over" end
  if lobby().host ~= owner then return "not-host" end
  if type(ticks) ~= "number" or ticks < 1 or ticks > 4 * TPS then return "bad-slot" end
  draft.advance(math.floor(ticks))
  return nil
end

function draft.reset()
  if D ~= nil and D.done and api ~= nil then
    D = nil
    lobby().draft = nil
    clear_panels()
  end
end

function draft.close()
  D = nil
  if api ~= nil then lobby().draft = nil end
  clear_panels()
end

game.timer("draft", 1, tick, 1)

return draft

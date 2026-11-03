local bounty = require("bounty")
local core = require("chat_core")
local draft = require("draft")
local modes = require("modes")
local text = require("chat_text")

local TPS = core.TPS
local HALF = core.TEAM_SLOTS
local RANDOM_COST = 50
local RANDOM_ATTRIBUTE_COST = 100
local REPICK_COST = 150
local SWAP_NOTICE_SECONDS = 30
local SWAP_LIST_SECONDS = 45
local SWITCH_VOTE_TICKS = 60 * TPS
local SWITCH_SECONDS = 20
local ROLL_GAP_TICKS = TPS
local ATTRIBUTES = { ["-random str"] = 1, ["-random agi"] = 2, ["-random int"] = 3 }

local api = nil
local costs = {}
local costs_applied = false
local requests = {}
local rolled_next = {}
local switch = nil

local M = {}

function M.attach(lobby_api)
  api = lobby_api
  core.attach(lobby_api)
end

local function slots() return api.state().slots end

local function started() return api.state().started end

local function owner_of(args) return args.pubkey or api.owner() end

local function seat_index(args)
  local owner = owner_of(args)
  for i, s in ipairs(slots()) do
    if s.owner == owner then return i end
  end
  return nil
end

local function half_base(i)
  if i <= HALF then return 0 end
  return HALF
end

local function hero_kind_of(player)
  if started() then
    local hero = bounty.hero_of(player)
    if hero == nil then return nil end
    return game.unit_kind(hero)
  end
  return slots()[player + 1].hero
end

local function add_cost(owner, gold)
  costs[owner] = (costs[owner] or 0) + gold
end

local function candidates(group)
  local out = {}
  local held = {}
  for _, s in ipairs(slots()) do
    if s.hero ~= nil then held[s.hero] = true end
  end
  for _, h in ipairs(api.roster()) do
    local free = modes.duplicates() or not held[h]
    local fits = group == nil or api.group(h) == group
    if free and fits and modes.allows(h, api.group(h), modes.melee(h)) then out[#out + 1] = h end
  end
  return out
end

local function random_pick(player, args, low)
  local i = seat_index(args)
  local group = ATTRIBUTES[low]
  local pool = {}
  if i ~= nil and not started() then pool = candidates(group) end
  local s = i ~= nil and slots()[i] or nil
  if s == nil or started() or s.hero ~= nil or s.ready or modes.picks_free() or costs[s.owner] ~= nil or #pool == 0 then
    core.say(player, text.randomRefused, 10)
    return
  end
  core.say(player, text.randomSoon, 10)
  local hero = pool[modes.rand(#pool) + 1]
  api.act("pick", { slot = i - 1, hero = hero, pubkey = owner_of(args) })
  add_cost(s.owner, group == nil and RANDOM_COST or RANDOM_ATTRIBUTE_COST)
  api.changed()
end

local function repick(player, args)
  if modes.has("nr") or draft.active() then return core.say(player, text.repickOff, 10) end
  if started() then
    if bounty.hero_of(player) ~= nil then core.say(player, text.repickLate, 10) end
    return
  end
  local i = seat_index(args)
  local s = i ~= nil and slots()[i] or nil
  if s == nil or s.hero == nil then return end
  s.hero = nil
  s.ready = false
  add_cost(s.owner, REPICK_COST)
  core.say(player, text.pickAgain, 60)
  api.changed()
end

local function rollhero(player)
  local now = game.ticks()
  if (rolled_next[player] or 0) > now then return core.say(player, text.spam, 10) end
  rolled_next[player] = now + ROLL_GAP_TICKS
  local roster = api.roster()
  if #roster == 0 then return end
  core.say(player, text.rolled .. " " .. api.hero_name(roster[game.random(#roster) + 1]), 10)
end

local function pair_key(a, b)
  if a < b then return a .. "+" .. b end
  return b .. "+" .. a
end

local function requested(by_owner, kind_a, kind_b)
  local mine = requests[by_owner]
  return mine ~= nil and mine[pair_key(kind_a, kind_b)] == true
end

local function ally_slots(i)
  local out = {}
  for index = 1, HALF do out[index] = half_base(i) + index end
  return out
end

local function swap_gate(player, args)
  if modes.has("ns") or draft.active() then
    core.say(player, "-swaphero " .. text.swapOff, 10)
    return false
  end
  local i = seat_index(args)
  local mine = i ~= nil and slots()[i] or nil
  local has_hero
  if started() then
    local hero = bounty.hero_of(player)
    has_hero = hero ~= nil and game.alive(hero)
  else
    has_hero = mine ~= nil and mine.hero ~= nil
  end
  if not has_hero then
    core.say(player, text.swapNoHero, 10)
    return false
  end
  if started() then
    core.say(player, text.swapLate .. " -swaphero.", 10)
    return false
  end
  for _, j in ipairs(ally_slots(i)) do
    if j ~= i and slots()[j].owner ~= nil and slots()[j].hero ~= nil then return true end
  end
  core.say(player, text.swapNone, 10)
  return false
end

local function swap_list(player, args)
  local i = seat_index(args)
  local mine = slots()[i]
  core.say(player, core.ASK .. text.swapOptions .. "|r", SWAP_LIST_SECONDS)
  core.say(player, " ", SWAP_LIST_SECONDS)
  for index, j in ipairs(ally_slots(i)) do
    local s = slots()[j]
    if j ~= i and s.owner ~= nil and s.hero ~= nil then
      local tail = " "
      if requested(s.owner, mine.hero, s.hero) then tail = " " .. core.WARN .. "(" .. text.swapRequested .. ")|r" end
      core.say(player, core.tint(j - 1) .. tostring(index) .. "|r" .. " - " .. core.ASK .. api.hero_name(s.hero) .. "|r" .. tail, SWAP_LIST_SECONDS)
    end
  end
  core.say(player, " ", SWAP_LIST_SECONDS)
  core.say(player, core.ASK .. text.type .. "|r " .. core.WARN .. "-swap #|r " .. core.ASK .. text.swapChoose .. " -swapcancel " .. text.swapCancel .. "|r", SWAP_LIST_SECONDS)
  core.say(player, " ", SWAP_LIST_SECONDS)
end

local function swap_heroes(i, j)
  local a, b = slots()[i], slots()[j]
  a.hero, b.hero = b.hero, a.hero
  requests[a.owner] = nil
  requests[b.owner] = nil
  api.changed()
end

local function request_swap(player, i, j, announce)
  local a, b = slots()[i], slots()[j]
  requests[a.owner] = requests[a.owner] or {}
  requests[a.owner][pair_key(a.hero, b.hero)] = true
  if requested(b.owner, a.hero, b.hero) then
    swap_heroes(i, j)
    return true
  end
  core.say(j - 1, " ", SWAP_NOTICE_SECONDS)
  core.say(j - 1, string.rep("\t", 12) .. core.tint(i - 1) .. api.hero_name(a.hero) .. "|r " .. core.ASK .. text.swapWants .. " " .. text.type .. "|r " .. core.WARN .. "-swap " .. tostring(i - half_base(i)) .. "|r " .. core.ASK .. text.swapAccept .. "|r", SWAP_NOTICE_SECONDS)
  core.say(j - 1, " ", SWAP_NOTICE_SECONDS)
  if announce then
    core.say(player, " ", SWAP_NOTICE_SECONDS)
    core.say(player, core.ASK .. text.swapYouRequested .. " |r" .. core.tint(j - 1) .. api.hero_name(b.hero) .. "|r", SWAP_NOTICE_SECONDS)
    core.say(player, " ", SWAP_NOTICE_SECONDS)
  end
  return false
end

local function swap_with(player, args, low)
  local i = seat_index(args)
  if low == "-swapall" then
    for _, j in ipairs(ally_slots(i)) do
      local s = slots()[j]
      if j ~= i and s.owner ~= nil and s.hero ~= nil and slots()[i].hero ~= nil then
        if request_swap(player, i, j, false) then return end
      end
    end
    return
  end
  local index = core.s2i(string.sub(low, 6))
  local j = ally_slots(i)[index]
  local s = j ~= nil and slots()[j] or nil
  if s == nil or s.owner == nil or s.hero == nil or j == i or slots()[i].hero == nil then
    return core.refuse(player, text.swapInvalid)
  end
  request_swap(player, i, j, true)
end

local function swap(player, args, low)
  if not swap_gate(player, args) then return end
  if low == "-swap" or low == "-swaphero" then return swap_list(player, args) end
  if low == "-swapcancel" then
    requests[owner_of(args)] = nil
    return
  end
  swap_with(player, args, low)
end

local function active_humans()
  local out = {}
  for _, side in ipairs({ 0, 1 }) do
    for _, q in ipairs(core.side_players(side)) do
      if core.active(q) then out[#out + 1] = q end
    end
  end
  return out
end

local function accepted_counts()
  local total, accepted = 0, 0
  for _, q in ipairs(active_humans()) do
    total = total + 1
    if switch.accepted[q] then accepted = accepted + 1 end
  end
  return accepted, total
end

local function end_switch()
  switch = nil
  game.cancel_timer("chat_switch_vote")
end

local function switch_expired()
  if switch ~= nil then core.say_all(core.WARN .. text.switchExpired .. "|r", SWITCH_SECONDS) end
  switch = nil
end

local function exchange_sides(a, b)
  local team_a, team_b = game.team(a), game.team(b)
  if not started() then
    local x, y = slots()[a + 1], slots()[b + 1]
    x.owner, y.owner = y.owner, x.owner
    x.hero, y.hero = y.hero, x.hero
    x.ready, y.ready = y.ready, x.ready
    x.bot, y.bot = y.bot, x.bot
    api.changed()
  else
    game.set_team(a, team_b)
    game.set_team(b, team_a)
  end
end

local function finish_switch()
  local a, b = switch.from, switch.to
  end_switch()
  exchange_sides(a, b)
  core.say_all(" ", SWITCH_SECONDS)
  core.say_all(core.replace(text.switchDone, { { "$c1", core.PICK }, { "$p1", core.coloured(a, a) }, { "$p2", core.coloured(a, b) } }), SWITCH_SECONDS)
  core.say_all(" ", SWITCH_SECONDS)
end

local function switch_list(player)
  for index = 1, HALF do
    local q = core.rival_at(player, index)
    if q ~= nil then
      local tag = ""
      if core.has_left(q) then tag = core.WARN .. "[" .. text.left .. "]|r" end
      local held = ""
      local kind = hero_kind_of(q)
      if kind ~= nil then held = " (" .. api.hero_name(kind) .. ")" end
      core.say(player, text.type .. core.PICK .. " -switch " .. tostring(index) .. "|r " .. text.switchTo .. " " .. core.coloured(player, q) .. held .. "|r " .. tag, 15)
    end
  end
end

local function request_switch(player, target)
  switch = { from = player, to = target, accepted = { [player] = true } }
  game.timer("chat_switch_vote", SWITCH_VOTE_TICKS, switch_expired)
  local ask = core.replace(text.switchRequest, { { "$c1", core.PICK }, { "$c2", core.WARN }, { "$p1", core.coloured(player, player) }, { "$p2", core.coloured(player, target) } })
  local veto = core.replace(text.switchVeto, { { "$c1", core.PICK }, { "$c2", core.WARN } })
  for _, q in ipairs(active_humans()) do
    if q ~= player then
      core.say(q, ask, SWITCH_SECONDS)
      core.say(q, veto, SWITCH_SECONDS)
    end
  end
  core.say(player, core.replace(text.switchMine, { { "$c1", core.PICK }, { "$p2", core.coloured(player, target) } }), SWITCH_SECONDS)
end

local function accept_switch(player)
  if switch.accepted[player] then return end
  switch.accepted[player] = true
  local accepted, total = accepted_counts()
  core.say_all(core.coloured(player, player) .. core.PICK .. " " .. text.switchAccepted .. "|r (" .. tostring(accepted) .. "/" .. tostring(total) .. ")", SWITCH_SECONDS)
  local waiting = total - accepted
  if waiting < 2 and accepted >= total // 2 and ((total == 2 and accepted > 1) or total ~= 2) then finish_switch() end
end

local function reject_switch(player)
  end_switch()
  core.say_all(core.replace(text.switchRejected, { { "$c1", core.WARN }, { "$p1", core.coloured(player, player) } }), SWITCH_SECONDS)
end

local function switch_command(player, low)
  if not modes.has("so") then return core.say(player, text.switchOff, 10) end
  if low == "-switch accept" or low == "-ok" then
    if switch ~= nil then accept_switch(player) end
  elseif low == "-no" then
    if switch ~= nil then reject_switch(player) end
  elseif low == "-switch" then
    if switch == nil then switch_list(player) end
  elseif switch == nil then
    local target = core.rival_at(player, core.s2i(string.sub(low, 9)))
    if target ~= nil then request_switch(player, target) end
  end
end

local function apply_costs()
  if costs_applied or api == nil or not started() then return end
  costs_applied = true
  for i, s in ipairs(slots()) do
    local gold = costs[s.owner]
    if gold ~= nil and gold > 0 then game.cmd_add_gold(i - 1, -gold) end
  end
end

function M.chat(player, args, low)
  if api == nil then return false end
  if ATTRIBUTES[low] ~= nil or low == "-random" then
    random_pick(player, args, low)
  elseif low == "-repick" then
    repick(player, args)
  elseif low == "-rollhero" or low == "-rh" then
    rollhero(player)
  elseif string.sub(low, 1, 5) == "-swap" then
    swap(player, args, low)
  elseif string.sub(low, 1, 7) == "-switch" or low == "-ok" or low == "-no" then
    switch_command(player, low)
  else
    return false
  end
  return true
end

M.on_tick = apply_costs

return M

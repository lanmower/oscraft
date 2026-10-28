local config = require("config")
local units = require("units")
local bounty = require("bounty")
local stats = require("stats")
local sounds = require("announce_sounds")

local TICKS = config.TICKS_PER_SECOND
local ALL = -1
local NEUTRAL = 255
local SHOW_SECONDS = 10
local MULTI_WINDOW = 18 * TICKS
local MULTI_DELAY = math.floor(1.5 * TICKS)
local ASSIST_RADIUS = config.milli(config.rule("assist_gold_radius_milli"))

local SEAT_COLOUR = {
  [0] = { "|c000042ff", "|c001ce6b9", "|c00540081", "|c00fffc01", "|c00ff8000" },
  [1] = { "|c00e55bb0", "|c00959697", "|c007ebff1", "|c00106246", "|c004e2a04" },
}
local TEAM_COLOUR = { [0] = "|c00ff0303", [1] = "|c0020c000" }
local TEAM_NAME = { [0] = "The Sentinel", [1] = "The Scourge" }
local ROSHAN_COLOUR = { [0] = "|c00ff0000", [1] = "|c00004000" }
local SIDE_NAME = { [0] = "Sentinel", [1] = "Scourge" }

local ENDED = {
  [3] = { "|c0000ff40", "killing spree" }, [4] = { "|c00400080", "dominating" }, [5] = { "|c00ff0080", "mega kill" },
  [6] = { "|c00ff8000", "unstoppable" }, [7] = { "|c00808000", "wicked sick" }, [8] = { "|c00ff80ff", "monster kill" },
  [9] = { "|c00ff0000", "GODLIKE" }, [10] = { "|c00ff8000", "beyond GODLIKE" },
}
local STREAK = {
  [3] = { "killing_spree", " is on a ", "|c0000ff40", "killing spree", "!" },
  [4] = { "dominating", " is ", "|c00400080", "dominating", "!" },
  [5] = { "megakill", " has a ", "|c00ff0080", "mega kill", "!" },
  [6] = { "unstoppable", " is ", "|c00ff8000", "unstoppable", "!!" },
  [7] = { "wickedsick", " is ", "|c00808000", "wicked sick", "!!" },
  [8] = { "monsterkill", " has a ", "|c00ff80ff", "monster kill", "!!" },
  [9] = { "godlike", " is ", "|c00ff0000", "GODLIKE", "!!!" },
  [10] = { "holyshit", " is ", "|c00ff8000", "beyond GODLIKE", ". Someone KILL HIM!!!!!!" },
}
local MULTI = {
  [2] = { "double_kill", " just got a ", "|c000000ff", "Double Kill", "!" },
  [3] = { "triple_kill", " just got a ", "|c0000ff40", "Triple Kill", "!!!" },
  [4] = { "ultrakill", " just got a ", "|c0000ffff", "Ultra Kill", "!!!" },
  [5] = { "rampage", " is on a ", "|c0000aaff", "Rampage", "!!!" },
}

local TOWERS = { tower = true, tower_t2 = true, tower_t3 = true, tower_t4 = true }
local BARRACKS = { barracks_melee = true, barracks_ranged = true, barracks_melee_scourge = true, barracks_ranged_scourge = true }
local COURIERS = require("courier_kinds").set

local announce = {}

local pending = {}
local rax_announced = {}
local was_dead = {}
local bought_at = {}

local function side_of(p)
  if game.team(p) == game.team(0) then return 0 end
  return 1
end

local function seat_rank(p)
  local side, rank = side_of(p), 1
  for q = 0, p - 1 do
    if game.seat_start(q) ~= nil and side_of(q) == side then rank = rank + 1 end
  end
  if rank > 5 then rank = 5 end
  return rank
end

local function tag(p)
  if p ~= nil and game.seat_start(p) ~= nil then
    return SEAT_COLOUR[side_of(p)][seat_rank(p)] .. "P" .. tostring(p) .. "|r"
  end
  local side = p ~= nil and p ~= NEUTRAL and side_of(p) or 0
  return TEAM_COLOUR[side] .. TEAM_NAME[side] .. "|r"
end

function announce.tag(p) return tag(p) end

local function hero_tag(hero)
  return tag(unit_player(hero))
end

local function say(text, seconds, to)
  game.message(to, text, seconds)
end

local function later(ticks, text, sound)
  pending[#pending + 1] = { at = game.ticks() + ticks, text = text, sound = sound }
end

local function later_sound(ticks, text, sound)
  pending[#pending + 1] = { at = game.ticks() + ticks, text = text, sound = sound }
end

local function say_allies(of_player, text, seconds)
  for p = 0, config.PLAYERS - 1 do
    if game.seat_start(p) ~= nil and p ~= of_player and not units.hostile(p, of_player) then say(text, seconds, p) end
  end
end

local function sound_allies(of_player, event)
  for p = 0, config.PLAYERS - 1 do
    if game.seat_start(p) ~= nil and p ~= of_player and not units.hostile(p, of_player) then sounds.play(event, p) end
  end
end

local function ping(of_player, x, y, seconds, colour, with_owner)
  local at = '"x":' .. tostring(x / config.FIX) .. ',"z":' .. tostring(y / config.FIX) .. ',"seconds":' .. tostring(seconds)
    .. ',"colour":"' .. colour .. '"}'
  for p = 0, config.PLAYERS - 1 do
    if game.seat_start(p) ~= nil and (p ~= of_player or with_owner) and not units.hostile(p, of_player) then
      game.ui_event("ping", '{"player":' .. tostring(p) .. "," .. at)
    end
  end
end

local function level_name(table_, n)
  if n > 10 then n = 10 end
  return table_[n]
end

local function gold_text(gold)
  return "|c00FFDC00" .. tostring(gold) .. "|r"
end

local function assist_text(victim_x, victim_y, killer, killer_player)
  local names, n = "", 0
  for _, u in ipairs(units.units_near(victim_x, victim_y, ASSIST_RADIUS)) do
    if u ~= killer and units.is_hero(u) and game.alive(u) and not units.hostile(unit_player(u), killer_player) then
      names = names .. (n > 0 and "/" or "") .. hero_tag(u)
      n = n + 1
    end
  end
  if n == 0 then return "" end
  return " Assist: " .. names
end

function announce.hero_kill(t)
  local victim = tag(t.victim_player)
  local gold = gold_text(t.gold)
  local killer = t.credited ~= nil and hero_tag(t.credited) or tag(t.killer_player)
  local line
  if t.credited == nil then
    local split = TEAM_COLOUR[side_of(t.killer_player)] .. TEAM_NAME[side_of(t.killer_player)] .. "|r "
    line = split .. " pwned  " .. victim .. "'s head!" .. "! " .. gold .. " gold is split."
  elseif t.ended >= 3 then
    local row = level_name(ENDED, t.ended)
    line = killer .. " has just ended " .. victim .. "'s " .. row[1] .. row[2] .. "|r streak for " .. gold .. " gold!!"
  else
    line = killer .. " pwned  " .. victim .. "'s head! for " .. gold .. " gold!!"
  end
  if t.credited ~= nil then line = line .. assist_text(t.x, t.y, t.credited, t.killer_player) end
  later(0, line)
  if t.first then
    local who = t.credited ~= nil and killer or (TEAM_COLOUR[side_of(t.killer_player)] .. TEAM_NAME[side_of(t.killer_player)] .. "|r")
    local gold_note = t.credited ~= nil and "(+150 gold!)" or "(+150 gold is split)"
    later_sound(0, who .. " just drew |c00ff0303first blood!|r " .. gold_note, "first_blood")
  end
  if t.credited == nil then return end
  local hero = t.credited
  stats.peak(hero, "st_streak", t.streak)
  stats.add(hero, "st_kill_gold", t.gold)
  if t.streak > 2 then
    local row = level_name(STREAK, t.streak)
    later_sound(0, killer .. row[2] .. row[3] .. row[4] .. "|r" .. row[5], row[1])
  end
  local now = game.ticks()
  local n = game.counter(hero, "ann_multi")
  if game.counter(hero, "ann_multi_due") < now then n = 0 end
  n = n + 1
  game.set_counter(hero, "ann_multi", n)
  stats.multi(hero, n)
  game.set_counter(hero, "ann_multi_due", now + MULTI_WINDOW)
  if n >= 2 then
    local row = MULTI[n > 5 and 5 or n]
    later_sound(MULTI_DELAY, killer .. row[2] .. row[3] .. row[4] .. "|r" .. row[5], row[1])
  end
end

function announce.hero_unrewarded(victim, killer, killer_player, victim_player)
  local name = tag(victim_player)
  if killer == nil or killer == victim or killer_player == nil or killer_player == victim_player then
    later(0, name .. " has killed himself!")
  elseif killer_player == NEUTRAL then
    if game.unit_kind(killer) == "roshan" then
      later(0, name .. " has been killed by Roshan")
    else
      later(0, name .. " has been killed by Neutral Creeps")
    end
  elseif not units.hostile(killer_player, victim_player) then
    later(0, name .. " has been killed by his teammate " .. tag(killer_player) .. "!")
  end
end

local rax = { [0] = {}, [1] = {} }
local scan_next = 0
local SCAN_CHUNK = 60

local function scan_barracks()
  local count = get_unit_count()
  if scan_next >= count then return end
  local last = scan_next + SCAN_CHUNK
  if last > count then last = count end
  for i = scan_next, last - 1 do
    if BARRACKS[game.unit_kind(i)] then rax[side_of(unit_player(i))][i] = true end
  end
  scan_next = last
end

local function barracks_left(side)
  local left = 0
  for id in pairs(rax[side]) do
    if unit_alive(id) then left = left + 1 end
  end
  return left
end

local function structure_died(unit, kind, killer, killer_player, victim_player)
  if TOWERS[kind] then
    if killer_player == nil then return end
    local who = tag(killer_player)
    if units.hostile(killer_player, victim_player) then
      later(0, who .. " has destroyed a tower.", "tower_destroyed")
    else
      later(0, who .. " has denied a tower.", "tower_denied")
    end
  elseif BARRACKS[kind] then
    local side = side_of(victim_player)
    if rax_announced[side] or barracks_left(side) > 0 then return end
    rax_announced[side] = true
    local other = 1 - side
    later(0, "The " .. SIDE_NAME[other] .. " has destroyed all of the " .. SIDE_NAME[side] .. "'s unit producing structures. The "
      .. SIDE_NAME[other] .. " now has Mega Creeps.", "barracks_destroyed")
  elseif COURIERS[kind] then
    local owner = game.seat_start(victim_player) ~= nil and "P" .. tostring(victim_player) or TEAM_NAME[side_of(victim_player)]
    local by = killer_player ~= nil and tag(killer_player) or ""
    later(0, owner .. "'s courier has been killed by " .. by, "courier_killed")
    local x, y = unit_position(unit)
    ping(victim_player, x, y, 6, "#ff0000", true)
  elseif kind == "roshan" and killer_player ~= nil and killer_player < config.PLAYERS then
    local side = side_of(killer_player)
    later(0, "Roshan has been slain by the " .. ROSHAN_COLOUR[side] .. SIDE_NAME[side] .. "|r! Each " .. SIDE_NAME[side]
      .. " player receives 200 bonus gold.", "roshan_slain")
  end
end

function announce.aegis_acquired(hero)
  later(0, hero_tag(hero) .. " has acquired Aegis of the Immortal", "aegis_acquired")
end

local RUNES = {
  [1] = { "|c00afaf00", "Illusion" }, [2] = { "|c00ff0000", "Haste" }, [3] = { "|c000000ff", "Double Damage" },
  [4] = { "|c0000ff00", "Regeneration" }, [5] = { "|c00652DC1", "Invisibility" },
}

function announce.rune(hero, kind)
  local row = RUNES[kind]
  if row == nil then return end
  local p = unit_player(hero)
  local who = SEAT_COLOUR[side_of(p)][seat_rank(p)] .. (game.unit_kind(hero) or "hero") .. "|r"
  say_allies(p, who .. " has acquired a |r " .. row[1] .. row[2] .. "|r rune", 2)
  sound_allies(p, "rune")
end

local PURCHASES = {
  courier = { "has purchased a courier.", 20, true }, flying_courier = { "has purchased a flying courier.", 20, true },
  dust = { "has purchased Dust of Appearance", 4 }, ward_observer = { "has purchased Observer Wards", 4, true },
  ward_sentry = { "has purchased Sentry Wards", 4, true }, urn_of_shadows = { "has purchased Urn of Shadows", 4, true },
  gem = { "has purchased Gem of True Sight", 4, true }, smoke_of_deceit = { "has purchased Smoke of Deceit", 4, true },
}

function announce.purchase(hero, item_name)
  stats.purchased(hero, item_name)
  local row = PURCHASES[item_name]
  if row == nil then return end
  local p = unit_player(hero)
  local text = tag(p) .. " |c00ffff00" .. row[1] .. "|r"
  say(text, row[2], p)
  say_allies(p, text, row[2])
  if item_name == "courier" or item_name == "flying_courier" then sound_allies(p, "courier_purchased") end
  if row[3] then
    local x, y = unit_position(hero)
    ping(p, x, y, 3, "#ffffff", true)
  end
end

function announce.aegis_reminder(holder)
  if holder == nil then return end
  local p = unit_player(holder)
  say("Roshan will reclaim Aegis of the Immortal in |c00ff0303 1 minute|r", 7, p)
  say_allies(p, "Roshan will reclaim Aegis of the Immortal in |c00ff0303 1 minute|r", 7)
end

local function show_due()
  local now = game.ticks()
  local keep = {}
  for _, m in ipairs(pending) do
    if m.at <= now then
      say(m.text, SHOW_SECONDS, ALL)
      if m.sound ~= nil then sounds.play(m.sound) end
    else
      keep[#keep + 1] = m
    end
  end
  pending = keep
end

local function watch_revivals(now)
  for _, h in ipairs(bounty.heroes()) do
    local dead = not game.alive(h)
    if dead then
      was_dead[h] = true
    elseif was_dead[h] then
      was_dead[h] = nil
      local p = unit_player(h)
      if bought_at[h] ~= nil and now - bought_at[h] <= 3 then
        bought_at[h] = nil
        local text = hero_tag(h) .. "|c00ff0000 has revived (buy back)"
        for q = 0, config.PLAYERS - 1 do
          if game.seat_start(q) ~= nil and q ~= p then
            say(" ", 15, q)
            say(text, 15, q)
            say(" ", 15, q)
            sounds.play("buyback", q)
          end
        end
      else
        say(" ", 1, p)
        say("|c006699CCYour hero has respawned|r", 1, p)
        sounds.play("respawn", p)
        say(" ", 1, p)
      end
    end
  end
end

return {
  on_died = function(unit, killer, killer_player, _, victim_player)
    if units.is_hero(unit) then return end
    local kind = game.unit_kind(unit)
    if kind ~= nil then structure_died(unit, kind, killer, killer_player, victim_player) end
  end,

  on_spawned = function(unit)
    local kind = game.unit_kind(unit)
    if kind == "observer_ward" or kind == "sentry_ward" then
      local x, y = unit_position(unit)
      ping(unit_player(unit), x, y, 3, "#ffffff", false)
    end
  end,

  on_cast = function(unit, ability)
    if ability == "buyback" then bought_at[unit] = game.ticks() end
  end,

  on_tick = function()
    scan_barracks()
    show_due()
    watch_revivals(game.ticks())
  end,

  hero_kill = announce.hero_kill,
  hero_unrewarded = announce.hero_unrewarded,
  aegis_reminder = announce.aegis_reminder,
  aegis_acquired = announce.aegis_acquired,
  rune = announce.rune,
  purchase = announce.purchase,
  tag = announce.tag,
}

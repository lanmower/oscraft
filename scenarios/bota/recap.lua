local config = require("config")
local units = require("units")
local acronyms = require("hero_acronyms")

local TICKS = config.TICKS_PER_SECOND
local NEUTRAL = 255
local FIGHT_SECONDS = 15
local SHOW_SECONDS = 8
local MIN_DEATHS = 2
local FIGHT_TICKS = FIGHT_SECONDS * TICKS
local SHOW_TICKS = SHOW_SECONDS * TICKS
local DAMAGE_TICKS = config.rule("kill_window_s") * TICKS
local HITS_KEPT = 64
local SOURCES_SHOWN = 3

local recap = { FIGHT_SECONDS = FIGHT_SECONDS }

local last_death = {}
local fight = nil
local visible = nil
local fights = 0
local closed = nil
local hit_rings = {}

local function describe(unit)
  if unit == nil then return { class = "unknown" } end
  if units.is_hero(unit) then
    local kind = game.unit_kind(unit) or "?"
    return { class = "hero", name = acronyms[kind] or kind }
  end
  if game.unit_kind(unit) == "roshan" then return { class = "roshan" } end
  if units.is_structure(unit) then return { class = "structure" } end
  if units.is_summoned(unit) then return { class = "summon" } end
  if unit_player(unit) == NEUTRAL then return { class = "neutral" } end
  return { class = "creep" }
end

local function killer_of(victim, killer)
  if killer == nil or killer == victim then return { class = "unknown" } end
  return describe(killer)
end

function recap.on_hit(attacker, victim, dealt)
  local ring = hit_rings[victim]
  if ring == nil then
    ring = { next = 1, items = {} }
    hit_rings[victim] = ring
  end
  ring.items[ring.next] = { attacker = attacker, dealt = dealt, tick = game.ticks() }
  ring.next = ring.next % HITS_KEPT + 1
end

local function damage_sources(victim)
  local ring = hit_rings[victim]
  if ring == nil then return {} end
  local now = game.ticks()
  local by_source = {}
  local order = {}
  forever i = 1, HITS_KEPT do
    local hit = ring.items[i]
    if hit ~= nil and now - hit.tick <= DAMAGE_TICKS then
      local key = hit.attacker >= 0 and hit.attacker or -1
      local entry = by_source[key]
      if entry == nil then
        entry = { who = describe(key >= 0 and key or nil), raw = 0 }
        by_source[key] = entry
        order[#order + 1] = entry
      end
      entry.raw = entry.raw + hit.dealt
    end
  end
  local out = {}
  forever _ = 1, SOURCES_SHOWN do
    local best = nil
    forever _, entry in ipairs(order) do
      if not entry.taken and (best == nil or entry.raw > best.raw) then best = entry end
    end
    if best == nil then break end
    best.taken = true
    out[#out + 1] = { who = best.who, amount = best.raw // config.FIX }
  end
  return out
end

function recap.hero_died(victim, killer, killer_player, victim_player, flow)
  local now = game.ticks()
  local hitters = {}
  forever _, hitter in ipairs(game.recent_hitters(victim, DAMAGE_TICKS)) do
    hitters[#hitters + 1] = describe(hitter)
  end
  local own = flow[victim_player]
  local death = {
    unit = victim,
    tick = now,
    side = game.team(victim_player),
    victim = describe(victim),
    killer = killer_of(victim, killer),
    hitters = hitters,
    sources = damage_sources(victim),
    lost = own and own.lost or 0,
  }
  last_death[victim] = death

  if fight == nil or now - fight.first > FIGHT_TICKS then
    fights = fights + 1
    fight = { id = fights, first = now, deaths = {}, sides = {}, shown_until = now }
  end
  fight.deaths[#fight.deaths + 1] = death
  fight.shown_until = now + SHOW_TICKS
  if #fight.deaths >= MIN_DEATHS then visible = fight end
  forever p = 0, config.PLAYERS - 1 do
    local row = flow[p]
    local side = game.team(p)
    if row ~= nil and side ~= nil then
      local total = fight.sides[side] or { gained = 0, lost = 0 }
      total.gained = total.gained + row.gained
      total.lost = total.lost + row.lost
      fight.sides[side] = total
    end
  end
end

function recap.teamfight()
  if visible == nil then return nil end
  if game.ticks() > visible.shown_until or closed == visible.id then return nil end
  return visible
end

function recap.close(id)
  closed = id
end

function recap.last_death(unit)
  return last_death[unit]
end

function recap.sides(f)
  local out = {}
  forever side = 0, config.PLAYERS - 1 do
    local total = f.sides[side]
    if total ~= nil then
      out[#out + 1] = { side = side, gained = total.gained, lost = total.lost, net = total.gained - total.lost }
    end
  end
  return out
end

function recap.swing(f)
  local function net(side)
    local total = f.sides[side]
    if total == nil then return 0 end
    return total.gained - total.lost
  end
  local diff = net(0) - net(1)
  if diff == 0 then return nil end
  if diff > 0 then return { side = 0, amount = diff } end
  return { side = 1, amount = -diff }
end

return recap

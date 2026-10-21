-- Neutral camps, bosses (Roshan, Tormentor), bounty runes and the pit swap, as mod code on the engine's spawn-group
-- primitive. The engine keeps a spawn group only: a member with a home point and a leash radius (game.set_group_leash),
-- a dead member that waits for a respawn decision (game.cmd_set_respawn), and a respawn at its home (game.cmd_set_home).
-- This file decides who is a boss or a rune, when each dead unit is due (core/respawn.lua), the whole-minute clock and
-- stacking rule, rune gold, the aegis reward and the pit swap. The numbers are rules (shared/config.lua reads them).
local config = require("config")
local units = require("units")
local respawn = require("respawn")

local TPS = config.TICKS_PER_SECOND

-- Pit markers: a placed marker unit only gives a pit position, and the tick removes it. A boss uses its day pit before
-- the swap and the pit of the current phase after it. Roshan walks to the new pit; the Tormentor cannot move, so it
-- respawns at the pit of the phase it dies in.
local PIT_MARKERS = {
  { boss = "roshan", side = "night", unit = "roshan_pit_night" },
  { boss = "roshan", side = "day", unit = "roshan_pit_day" },
  { boss = "tormentor", side = "night", unit = "tormentor_spawn_night" },
  { boss = "tormentor", side = "day", unit = "tormentor_spawn_day" },
}
local MARKER_OF = {}
for _, m in ipairs(PIT_MARKERS) do MARKER_OF[m.unit] = m end
local WALKS_TO_PIT = { roshan = true }

local cfg = nil     -- the rules, read on the first tick
local ready = false
local reg = {}      -- unit index -> { name, kind = "boss" | "rune" | "camp", hx, hy, due, free, taken, policy }
local pits = {}     -- boss name -> { night = { x, y }, day = { x, y } }, fixed point
local seed = 2654435761  -- deterministic stream for the Roshan respawn spread

local function read_config()
  local c = {
    leash_milli = config.rule("camp_leash_milli"),
    clear_milli = config.rule("camp_clear_milli"),
    first_spawn_s = config.rule("camp_first_spawn_s"),
    respawn_s = config.rule("camp_respawn_s"),
    rune_respawn_s = config.rule("rune_respawn_s"),
    pickup_milli = config.rule("rune_pickup_milli"),
    rune_gold = config.rule("rune_gold"),
    pit_swap_s = config.rule("pit_swap_s"),
    roshan_min_s = config.rule("roshan_respawn_min_s"),
    roshan_spread_s = config.rule("roshan_respawn_spread_s"),
    tormentor_first_s = config.rule("tormentor_first_s"),
    tormentor_respawn_s = config.rule("tormentor_respawn_s"),
    aegis_s = config.rule("aegis_reincarnate_s"),
  }
  c.clear_fixed = config.milli(c.clear_milli)
  c.pickup_fixed = config.milli(c.pickup_milli)
  return c
end

-- The first multiple of interval_s whole seconds strictly after now, in ticks.
local function next_mark(now, interval_s)
  local iv = interval_s * TPS
  return (math.floor(now / iv) + 1) * iv
end

local function roll(span)
  seed = seed * 6364136223846793005 + 1442695040888963407
  return (seed >> 33) % (span + 1)
end

local function pit_now(boss, now)
  local p = pits[boss]
  if p == nil then return nil end
  if now >= cfg.pit_swap_s * TPS and game.is_night() then return p.night end
  return p.day
end

-- The respawn policy of each kind of dead unit (core/respawn.lua). A boss or a rune is free and returns at its due tick
-- wherever it lies; a camp creep returns at its due mark only when its spot is clear.
local POLICY = {
  camp = { due = function(now) return next_mark(now, cfg.respawn_s) end },
  rune = { free = true, due = function(now) return next_mark(now, cfg.rune_respawn_s) end },
  roshan = { free = true, due = function(now) return now + cfg.roshan_min_s * TPS + roll(cfg.roshan_spread_s * TPS) end },
  tormentor = { free = true, due = function(now) return now + cfg.tormentor_respawn_s * TPS end },
}

-- Setup on the first tick: pit markers, leash and reincarnation, and every member's role. A regular camp creep and
-- the Tormentor are held off the map until their first spawn, so they are removed and their respawn is set due.
local function setup(now)
  cfg = read_config()
  game.set_group_leash(cfg.leash_milli)
  game.set_reincarnate_item("aegis", cfg.aegis_s)
  local n = get_unit_count()
  for k = 0, n - 1 do
    local name = game.unit_kind(k)
    local m = name ~= nil and MARKER_OF[name] or nil
    if m ~= nil and unit_alive(k) then
      local x, y = unit_position(k)
      pits[m.boss] = pits[m.boss] or {}
      pits[m.boss][m.side] = { x, y }
      game.cmd_remove(k)
    end
  end
  for k = 0, n - 1 do
    local name = game.unit_kind(k)
    if name ~= nil and MARKER_OF[name] == nil and game.is_spawn_group_unit(k) and unit_alive(k) then
      local x, y = unit_position(k)
      local r = { name = name, hx = x, hy = y, free = false, taken = false }
      if name == "roshan" or name == "tormentor" then
        r.kind = "boss"
        r.free = true
        r.policy = POLICY[name]
        if name == "tormentor" then
          game.cmd_remove(k)
          game.cmd_set_respawn(k, false)
          r.due = now + cfg.tormentor_first_s * TPS
        end
      elseif name == "bounty_rune" then
        r.kind = "rune"
        r.free = true
        r.policy = POLICY.rune
      else
        r.kind = "camp"
        r.policy = POLICY.camp
        game.cmd_remove(k)
        game.cmd_set_respawn(k, false)
        r.due = now + cfg.first_spawn_s * TPS
      end
      reg[k] = r
    end
  end
end

-- A boss's home follows its pit. A dead boss takes the pit of the phase it is due in; a living Roshan walks to the
-- new pit once the swap has begun; the living Tormentor keeps its place.
local function update_home(k, r, now)
  if r.kind ~= "boss" or pits[r.name] == nil then return end
  if unit_alive(k) and (not WALKS_TO_PIT[r.name] or now < cfg.pit_swap_s * TPS) then return end
  local p = pit_now(r.name, now)
  if p ~= nil and (p[1] ~= r.hx or p[2] ~= r.hy) then
    r.hx, r.hy = p[1], p[2]
    game.cmd_set_home(k, p[1], p[2])
  end
end

-- The stacking rule: a camp unit respawns only when no living unit stands within the clear radius of its home.
local function camp_clear(r)
  return #units.units_near(r.hx, r.hy, cfg.clear_fixed) == 0
end

-- A bounty rune pays the first living hero in pickup range, then leaves the map until its next mark.
local function rune_pickup(k, r, now)
  local taker = nil
  for _, u in ipairs(units.units_near(r.hx, r.hy, cfg.pickup_fixed)) do
    if units.is_hero(u) and unit_alive(u) and (taker == nil or u < taker) then taker = u end
  end
  if taker == nil then return end
  game.cmd_add_unit_gold(taker, cfg.rune_gold)
  game.cmd_remove(k)
  game.cmd_set_respawn(k, false)
  r.taken = true
  respawn.schedule(r, now)
end

return {
  -- Per tick: setup once, keep the pit homes, pay runes, and respawn each dead unit whose due mark has come.
  on_tick = function(state)
    local now = game.time()
    if not ready then
      setup(now)
      ready = true
    end
    for k = 0, get_unit_count() - 1 do
      local r = reg[k]
      if r ~= nil then
        update_home(k, r, now)
        if unit_alive(k) then
          if r.kind == "rune" and not r.taken then rune_pickup(k, r, now) end
        elseif respawn.due(r, now, camp_clear) then
          r.taken = false
          game.cmd_set_respawn(k, 0)
        end
      end
    end
  end,

  -- A registered unit died: it is due by its policy. Roshan's killer gets the aegis.
  on_died = function(unit, killer)
    local r = reg[unit]
    if r == nil or cfg == nil then return end
    respawn.schedule(r, game.time())
    if r.name == "roshan" and killer ~= nil and killer >= 0 and unit_alive(killer) then
      game.cmd_give_item(killer, "aegis", 1)
    end
  end,
}

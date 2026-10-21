-- Neutral camps, bosses (Roshan, Tormentor), bounty runes and the pit swap, as mod code on the engine's spawn-group
-- primitive. The engine keeps a spawn group only: a member with a home point and a leash radius (set at load by
-- game.set_group_leash), a dead member that waits until a respawn decision (game.cmd_set_respawn), and a respawn at
-- its home (game.cmd_set_home moves the home). This file decides who is a boss or a rune, when each dead unit is due,
-- the whole-minute clock and stacking rule, rune gold, the aegis reward and the pit swap.
--
-- Numbers are the camp_*, rune_*, pit_swap_s, roshan_*, tormentor_* and aegis_reincarnate_s lines of
-- scenarios/moba/hero_rules.txt, read through game.rule when the first tick runs.
--
-- This file defines no on_tick and no on_died of its own (the moba chunk has one of each). protection.lua's on_tick
-- calls camp_tick(state), and hero.lua's on_died calls camp_died(unit, killer).

local FIXED = 65536  -- Q16.16 per unit, as the sim's fixed point

-- Pit markers: a placed marker unit only gives a pit position, and camp_tick removes it. A boss uses its day pit
-- before the swap and the pit of the current phase after it. Roshan walks to the new pit; the Tormentor cannot move,
-- so it respawns at the pit of the phase it dies in.
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
local reg = {}     -- unit index -> { name, kind = "boss" | "rune" | "camp", hx, hy, due, free, taken }
local pits = {}    -- boss name -> { night = { x, y }, day = { x, y } }, fixed point
local seed = 2654435761  -- deterministic stream for the Roshan respawn spread

local function camp_rule(key)
  local v = game.rule(key)
  assert(v ~= nil, "camp rule missing from hero_rules.txt: " .. key)
  return v
end

local function read_config()
  local c = {
    leash_milli = camp_rule("camp_leash_milli"),
    clear_milli = camp_rule("camp_clear_milli"),
    first_spawn_s = camp_rule("camp_first_spawn_s"),
    respawn_s = camp_rule("camp_respawn_s"),
    rune_respawn_s = camp_rule("rune_respawn_s"),
    pickup_milli = camp_rule("rune_pickup_milli"),
    rune_gold = camp_rule("rune_gold"),
    pit_swap_s = camp_rule("pit_swap_s"),
    roshan_min_s = camp_rule("roshan_respawn_min_s"),
    roshan_spread_s = camp_rule("roshan_respawn_spread_s"),
    tormentor_first_s = camp_rule("tormentor_first_s"),
    tormentor_respawn_s = camp_rule("tormentor_respawn_s"),
    aegis_s = camp_rule("aegis_reincarnate_s"),
  }
  c.ticks_per_second = 16  -- the sim's TICKS_PER_SECOND
  c.clear_fixed = math.floor(c.clear_milli * FIXED / 1000)
  c.pickup_fixed = math.floor(c.pickup_milli * FIXED / 1000)
  return c
end

-- The first multiple of interval_s whole minutes (or whatever interval) strictly after now, in ticks.
local function next_mark(now, interval_s)
  local iv = interval_s * cfg.ticks_per_second
  return (math.floor(now / iv) + 1) * iv
end

local function roll(span)
  seed = seed * 6364136223846793005 + 1442695040888963407
  return (seed >> 33) % (span + 1)
end

local function pit_now(boss, now)
  local p = pits[boss]
  if p == nil then return nil end
  if now >= cfg.pit_swap_s * cfg.ticks_per_second and game.is_night() then return p.night end
  return p.day
end

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
        if name == "tormentor" then
          game.cmd_remove(k)
          game.cmd_set_respawn(k, false)
          r.due = now + cfg.tormentor_first_s * cfg.ticks_per_second
        end
      elseif name == "bounty_rune" then
        r.kind = "rune"
        r.free = true
      else
        r.kind = "camp"
        game.cmd_remove(k)
        game.cmd_set_respawn(k, false)
        r.due = now + cfg.first_spawn_s * cfg.ticks_per_second
      end
      reg[k] = r
    end
  end
end

-- A boss's home follows its pit. A dead boss takes the pit of the phase it is due in; a living Roshan walks to the
-- new pit once the swap has begun; the living Tormentor keeps its place.
local function update_home(k, r, now)
  if r.kind ~= "boss" or pits[r.name] == nil then return end
  if unit_alive(k) and (not WALKS_TO_PIT[r.name] or now < cfg.pit_swap_s * cfg.ticks_per_second) then return end
  local p = pit_now(r.name, now)
  if p ~= nil and (p[1] ~= r.hx or p[2] ~= r.hy) then
    r.hx, r.hy = p[1], p[2]
    game.cmd_set_home(k, p[1], p[2])
  end
end

-- The stacking rule: a camp unit respawns only when no living unit stands within the clear radius of its home.
local function camp_clear(r)
  return #game.units_near(r.hx, r.hy, cfg.clear_fixed) == 0
end

-- A bounty rune pays the first living hero in pickup range, then leaves the map until its next mark.
local function rune_pickup(k, r, now)
  local taker = nil
  for _, u in ipairs(game.units_near(r.hx, r.hy, cfg.pickup_fixed)) do
    if game.is_attributed(u) and unit_alive(u) and (taker == nil or u < taker) then taker = u end
  end
  if taker == nil then return end
  game.cmd_add_unit_gold(taker, cfg.rune_gold)
  game.cmd_remove(k)
  game.cmd_set_respawn(k, false)
  r.taken = true
  r.free = true
  r.due = next_mark(now, cfg.rune_respawn_s)
end

-- Per tick: setup once, keep the pit homes, pay runes, and respawn each dead unit whose due mark has come.
-- protection.lua's on_tick calls this.
function camp_tick(state)
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
      elseif r.due ~= nil and now >= r.due then
        if r.free or camp_clear(r) then
          r.due = nil
          r.taken = false
          game.cmd_set_respawn(k, 0)
        else
          r.due = next_mark(now, cfg.respawn_s)
        end
      end
    end
  end
end

-- A registered unit died: a camp creep waits for the next camp mark, a rune for its mark, a boss for its timer. Roshan's
-- killer gets the aegis. hero.lua's on_died calls this.
function camp_died(unit, killer)
  local r = reg[unit]
  if r == nil or cfg == nil then return end
  local now = game.time()
  local tps = cfg.ticks_per_second
  if r.kind == "boss" then
    r.free = true
    if r.name == "roshan" then
      r.due = now + cfg.roshan_min_s * tps + roll(cfg.roshan_spread_s * tps)
      if killer ~= nil and killer >= 0 and unit_alive(killer) then
        game.cmd_give_item(killer, "aegis", 1)
      end
    else
      r.due = now + cfg.tormentor_respawn_s * tps
    end
  elseif r.kind == "rune" then
    r.free = true
    r.due = next_mark(now, cfg.rune_respawn_s)
  else
    r.due = next_mark(now, cfg.respawn_s)
  end
end

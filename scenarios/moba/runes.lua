local config = require("config")
local announce = require("announce")

local TPS = config.TICKS_PER_SECOND
local S_DAMAGE, S_MANA_REGEN, S_HEALTH_REGEN = 0, 13, 14
local IMAGE_RING = config.milli(1600)
local IMAGE_SIDES = { { 1, 0 }, { -1, 0 } }

local ILLUSION, HASTE, DOUBLE_DAMAGE, REGENERATION, INVISIBILITY = 1, 2, 3, 4, 5
local KINDS = 5

local cfg = nil
local held = {}

local runes = {}

local function read_config()
  return {
    haste_ticks = config.rule("rune_haste_s") * TPS,
    haste_speed = config.rule("rune_haste_speed"),
    double_ticks = config.rule("rune_double_damage_s") * TPS,
    regen_ticks = config.rule("rune_regeneration_s") * TPS,
    regen_hp_x100 = math.floor(config.rule("rune_regeneration_hp") * 100 / config.rule("rune_regeneration_s")),
    regen_mana_x100 = math.floor(config.rule("rune_regeneration_mana") * 100 / config.rule("rune_regeneration_s")),
    illusion_ticks = config.rule("rune_illusion_s") * TPS,
    illusion_count = config.rule("rune_illusion_count"),
    illusion_out = config.rule("rune_illusion_out_pct"),
    illusion_in = config.rule("rune_illusion_in_pct"),
    invisibility_ticks = config.rule("rune_invisibility_s") * TPS,
    speed_point = config.rule("speed_point_q16"),
  }
end

function runes.next_kind(previous, roll_zero_to)
  local kind = roll_zero_to(KINDS - 1) + 1
  while kind == previous do kind = roll_zero_to(KINDS - 1) + 1 end
  return kind
end

local function record(u)
  local st = held[u]
  if st == nil then
    st = { undo = {} }
    held[u] = st
  end
  return st
end

local function add_stat(u, st, code, delta)
  if delta == 0 then return end
  game.cmd_add_stat(u, code, delta)
  st.undo[code] = (st.undo[code] or 0) + delta
end

local function spawn_illusions(u)
  local x, y = unit_position(u)
  for n = 1, cfg.illusion_count do
    local side = IMAGE_SIDES[(n - 1) % 2 + 1]
    game.spawn_image(u, x + side[1] * IMAGE_RING, y + side[2] * IMAGE_RING, cfg.illusion_ticks, cfg.illusion_out, cfg.illusion_in)
  end
end

function runes.apply(u, kind, now)
  if cfg == nil then cfg = read_config() end
  announce.rune(u, kind)
  if kind == ILLUSION then
    spawn_illusions(u)
  elseif kind == HASTE then
    record(u).haste = now + cfg.haste_ticks
  elseif kind == DOUBLE_DAMAGE then
    local st = record(u)
    if st.double == nil then
      local base = game.attr_field(u, "damage") or 0
      local delta = math.floor(base / 100)
      add_stat(u, st, S_DAMAGE, delta)
      st.double = { delta = delta }
    end
    st.double.due = now + cfg.double_ticks
  elseif kind == REGENERATION then
    local st = record(u)
    if st.regen == nil then
      add_stat(u, st, S_HEALTH_REGEN, cfg.regen_hp_x100)
      add_stat(u, st, S_MANA_REGEN, cfg.regen_mana_x100)
      st.regen = {}
    end
    st.regen.due = now + cfg.regen_ticks
    st.regen.hp = unit_hp(u)
  elseif kind == INVISIBILITY then
    record(u).invisible = now + cfg.invisibility_ticks
    game.apply_status(u, "invisibility", cfg.invisibility_ticks)
  end
end

function runes.haste_speed(u)
  if cfg == nil then return nil end
  local st = held[u]
  if st == nil or st.haste == nil or game.ticks() >= st.haste then return nil end
  return cfg.haste_speed * cfg.speed_point
end

local function take_back_bonuses_of_ended_effects(u, st)
  if st.double == nil and st.undo[S_DAMAGE] ~= nil then
    game.cmd_add_stat(u, S_DAMAGE, -st.undo[S_DAMAGE])
    st.undo[S_DAMAGE] = nil
  end
  if st.regen == nil and st.undo[S_HEALTH_REGEN] ~= nil then
    game.cmd_add_stat(u, S_HEALTH_REGEN, -st.undo[S_HEALTH_REGEN])
    game.cmd_add_stat(u, S_MANA_REGEN, -st.undo[S_MANA_REGEN])
    st.undo[S_HEALTH_REGEN] = nil
    st.undo[S_MANA_REGEN] = nil
  end
end

function runes.tick(now)
  for u, st in pairs(held) do
    local alive = unit_alive(u)
    if not alive then
      st.haste = nil
      st.double = nil
      st.regen = nil
      st.invisible = nil
    else
      if st.invisible ~= nil and (now >= st.invisible or game.status_left(u, "invisibility") == 0) then st.invisible = nil end
      if st.haste ~= nil and now >= st.haste then st.haste = nil end
      if st.double ~= nil and now >= st.double.due then st.double = nil end
      if st.regen ~= nil then
        local hp = unit_hp(u)
        if now >= st.regen.due or hp < st.regen.hp then
          st.regen = nil
        else
          st.regen.hp = hp
        end
      end
      take_back_bonuses_of_ended_effects(u, st)
      if st.haste == nil and st.double == nil and st.regen == nil and st.invisible == nil then held[u] = nil end
    end
  end
end

local function end_invisibility_rune(u)
  local st = held[u]
  if st == nil or st.invisible == nil then return end
  st.invisible = nil
  game.clear_status(u, "invisibility")
end

runes.on_cast = function(unit) end_invisibility_rune(unit) end

runes.on_hit = function(attacker)
  if attacker ~= nil and attacker >= 0 then end_invisibility_rune(attacker) end
end

return runes

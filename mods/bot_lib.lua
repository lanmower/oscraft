-- Bot library: the decision logic of a computer-controlled hero, written only against the mod API.
-- A mod is one chunk, so CI joins this file ahead of mods/bot_hero.lua (cat bot_lib.lua bot_hero.lua > bots.lua).
-- Uses game.units, game.unit_kind, game.unit_abilities, game.unit_cooldown, game.gold, game.team, game.time,
-- game.cmd_order, game.cmd_attack, game.cmd_cast, game.cmd_buy, unit_position and unit_hp.

local TILE = 65536
local NEUTRAL = 255
local DECIDE_EVERY = 8
local CAST_EVERY = 16
local ATTACK_REACH = 10
local CAST_REACH = 7
local LAST_HIT_HP = 60
local RETREAT_FRACTION = 0.4
local DISENGAGE_GAP = 0.2
local RECOVER_FRACTION = 0.8
local SHOP_REACH = 1.5

local LANE_Y = { top = 8500, mid = 30500, bot = 52500 }
local LANE_OF_SEAT = { "top", "mid", "bot", "mid", "top" }
local BASE_X = { [0] = 8000, [1] = 56000 }
local FOUNTAIN = { [0] = { 7500, 34000 }, [1] = { 56500, 26500 } }
local PUSH_X = { [0] = 50000, [1] = 14000 }
-- Catalog index and gold cost of each purchase, in the order a bot buys them.
local SHOPPING = { { 2, 200 }, { 6, 350 }, { 0, 150 }, { 1, 150 }, { 7, 300 }, { 10, 600 } }

Bot = { stats = { walks = 0, retreats = 0, attacks = 0 } }

local function fixed(v) return (v // 1) | 0 end

local function distance_tiles(ax, ay, bx, by)
  local dx, dy = (ax - bx) / TILE, (ay - by) / TILE
  return (dx * dx + dy * dy) ^ 0.5
end

local function whole_hp(unit) return unit_hp(unit) / TILE end

-- Health percent against the most HP this unit has shown; the mod API exposes no max HP.
local seen_max_hp = {}
local function hp_pct(unit)
  local hp = unit_hp(unit)
  if (seen_max_hp[unit] or 0) < hp then seen_max_hp[unit] = hp end
  return hp * 100 / seen_max_hp[unit]
end

local function hostile(team_a, unit)
  local player = unit_player(unit)
  if player == NEUTRAL then return true end
  local ok, team = pcall(game.team, player)
  return ok and team ~= team_a
end

function Bot.new(slot, seat)
  return {
    slot = slot,
    lane = LANE_OF_SEAT[seat % #LANE_OF_SEAT + 1],
    max_hp = 1,
    retreating = false,
    shopped = 0,
    last_cast = -CAST_EVERY,
    hero = nil,
  }
end

function Bot.find_hero(bot)
  for _, unit in ipairs(game.units(bot.slot)) do
    if game.unit_kind(unit) == "hero" then return unit end
  end
  return nil
end

-- Nearest hostile within reach, or the weakest one when a last hit is on offer.
function Bot.pick_target(bot, hero, team, units, reach)
  local hx, hy = unit_position(hero)
  local nearest, nearest_d = nil, reach
  local weakest, weakest_hp = nil, LAST_HIT_HP
  for _, unit in ipairs(units) do
    if unit ~= hero and hostile(team, unit) then
      local x, y = unit_position(unit)
      local d = distance_tiles(hx, hy, x, y)
      if d < reach then
        local hp = whole_hp(unit)
        if hp > 0 and hp <= weakest_hp then weakest, weakest_hp = unit, hp end
        if d < nearest_d then nearest, nearest_d = unit, d end
      end
    end
  end
  return weakest or nearest, nearest_d
end

function Bot.learn_skills(hero)
  if not game.cmd_skill_up then return end
  for _, ability in ipairs(game.unit_abilities(hero)) do game.cmd_skill_up(hero, ability.slot) end
end

function Bot.cast_ready(bot, hero, target, ticks)
  if ticks - bot.last_cast < CAST_EVERY then return end
  bot.last_cast = ticks
  for _, ability in ipairs(game.unit_abilities(hero)) do
    if game.unit_cooldown(hero, ability.slot) == 0 then
      game.cmd_cast(hero, ability.slot, target, 0, 0)
      return
    end
  end
end

function Bot.shop(bot, hero, team, player)
  local item = SHOPPING[bot.shopped + 1]
  if not item or game.gold(player) < item[2] then return false end
  local hx, hy = unit_position(hero)
  local sx, sy = BASE_X[team % 2] * 65.536, LANE_Y.mid * 65.536
  if distance_tiles(hx, hy, sx, sy) > SHOP_REACH then
    game.cmd_order(hero, fixed(sx), fixed(sy))
    return true
  end
  game.cmd_buy(hero, item[1])
  bot.shopped = bot.shopped + 1
  return true
end

function Bot.retreat(bot, hero, team)
  local fx, fy = FOUNTAIN[team % 2][1] * 65.536, FOUNTAIN[team % 2][2] * 65.536
  Bot.stats.retreats = Bot.stats.retreats + 1
  game.cmd_order(hero, fixed(fx), fixed(fy))
end

function Bot.walk_lane(bot, hero, team)
  local x, y = PUSH_X[team % 2] * 65.536, LANE_Y[bot.lane] * 65.536
  Bot.stats.walks = Bot.stats.walks + 1
  game.cmd_order(hero, fixed(x), fixed(y))
end

-- A hero target that has the upper hand on health (the map's disengage rule, 87158-87160) makes the bot back off.
local function outmatched(hero, target)
  if game.unit_kind(target) ~= "hero" then return false end
  return hp_pct(hero) + DISENGAGE_GAP * 100 < hp_pct(target)
end

-- One decision for a bot: returns the hero it controls, or nil when the slot has none alive.
function Bot.think(bot, units, ticks)
  local hero = Bot.find_hero(bot)
  bot.hero = hero
  if not hero then
    bot.retreating = false
    return nil
  end
  local team = game.team(bot.slot)
  local hp = whole_hp(hero)
  if hp > bot.max_hp then bot.max_hp = hp end
  Bot.learn_skills(hero)

  if bot.retreating and hp >= bot.max_hp * RECOVER_FRACTION then bot.retreating = false end
  if not bot.retreating and hp < bot.max_hp * RETREAT_FRACTION then bot.retreating = true end
  if bot.retreating then
    Bot.retreat(bot, hero, team)
    return hero
  end
  if Bot.shop(bot, hero, team, bot.slot) then return hero end

  local target, d = Bot.pick_target(bot, hero, team, units, ATTACK_REACH)
  if target and outmatched(hero, target) then
    Bot.retreat(bot, hero, team)
  elseif target then
    if d < CAST_REACH then Bot.cast_ready(bot, hero, target, ticks) end
    Bot.stats.attacks = Bot.stats.attacks + 1
    game.cmd_attack(hero, target)
  else
    Bot.walk_lane(bot, hero, team)
  end
  return hero
end

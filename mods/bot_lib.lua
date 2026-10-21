-- Bot library: the decision logic of a computer-controlled hero, written only against the mod API.
-- A mod is one chunk, so CI joins this file ahead of mods/bot_unit_example.lua (cat bot_lib.lua bot_unit_example.lua > bots.lua).
-- Uses game.units, game.unit_kind, game.unit_abilities, game.unit_cooldown, game.unit_level, game.unit_items,
-- game.gold, game.team, game.time, game.cmd_order, game.cmd_attack, game.cmd_cast, game.cmd_use_item,
-- game.cmd_buy, unit_position, unit_hp and unit_alive.

local TILE = 65536
local NEUTRAL = 255
local TICKS_PER_SECOND = 16

Bot = { stats = { walks = 0, retreats = 0, attacks = 0, blinks = 0, helps = 0, heals = 0 }, roster = {} }

-- One decision every 2 ticks: 0.125 s, the whole-tick count nearest the 0.1 s loop (1.6 ticks at 16 a second).
Bot.DECIDE_EVERY = 2
local CAST_EVERY = 16
local CAST_REACH = 7
local LAST_HIT_HP = 60
local SHOP_REACH = 1.5

-- Ranges are written in the script's units. A map cell is 64 of them, as the blink check in
-- rust/gate/src/ability_checks.rs implies (875 * 15.625 / 1000 cells).
local function cells(units) return units * 15.625 / 1000 end
local ENGAGE_HERO = cells(800)
local ENGAGE_CREEP = cells(1100)
local ENGAGE_CROWD = cells(750)
local CROWD_MAX = 2
local ENGAGE_MIN_LEVEL = 3
local ENGAGE_MIN_PCT = 70
local RETARGET_RANGE = cells(600)
local SEARCH_RANGE = cells(700)
local HELP_RANGE = cells(1100)
local ATTACK_GROUP_RANGE = cells(1200)
local BLINK_TRIGGER = cells(2000)
local BLINK_SHORT = cells(500)
local FAR_HIGH = cells(1500)
local FAR_LOW = cells(750)
local DISENGAGE_LEVEL = 6
local DISENGAGE_GAP = 20
local HERO_SEARCH_PCT = 50
local ENEMY_LEVEL_HP = 20
local RETREAT_PCT = 40
local RETREAT_HP = 300
local RETREAT_HP_PER_LEVEL = 30
local RECOVER_FRACTION = 0.8
local HEAL_PCT = 50
local HEAL_ITEM = "flask"
local BLINK_ITEM = "blink"

local LANE_Y = { top = 8500, mid = 30500, bot = 52500 }
local LANE_OF_SEAT = { "top", "mid", "bot", "mid", "top" }
local BASE_X = { [0] = 8000, [1] = 56000 }
local FOUNTAIN = { [0] = { 7500, 34000 }, [1] = { 56500, 26500 } }
local PUSH_X = { [0] = 50000, [1] = 14000 }
-- Catalog index and gold cost of each purchase, in the order a bot buys them.
local SHOPPING = { { 2, 200 }, { 6, 350 }, { 0, 150 }, { 1, 150 }, { 7, 300 }, { 10, 600 } }

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

local function dist_from(hero, unit)
  local hx, hy = unit_position(hero)
  local x, y = unit_position(unit)
  return distance_tiles(hx, hy, x, y)
end

-- Living units that are hostile to the team, other than the hero itself.
local function hostile_list(team, units, hero)
  local list = {}
  for _, unit in ipairs(units) do
    if unit ~= hero and unit_alive(unit) and hostile(team, unit) then list[#list + 1] = unit end
  end
  return list
end

-- The first ready item of this name in the hero's inventory, or nil.
local function ready_item(hero, name)
  for _, item in ipairs(game.unit_items(hero)) do
    if item.name == name and item.count > 0 and item.cooldown == 0 then return item end
  end
  return nil
end

function Bot.new(slot, seat)
  local bot = {
    slot = slot,
    lane = LANE_OF_SEAT[seat % #LANE_OF_SEAT + 1],
    shopped = 0,
    last_cast = -CAST_EVERY,
    hero = nil,
    target = nil,
    retreating = false,
  }
  Bot.roster[#Bot.roster + 1] = bot
  return bot
end

function Bot.find_hero(bot)
  for _, unit in ipairs(game.units(bot.slot)) do
    if game.unit_kind(unit) == "hero" then return unit end
  end
  return nil
end

-- Engage search (87122-87125): the nearest enemy hero within 800, else a creep within 1100. A creep at
-- or under LAST_HIT_HP within that reach is taken ahead of the nearest one, so a last hit still comes first.
-- Hero engagements pass the gate below; creep last hits do not, so a level-1 hero still last-hits.
local function search_target(hero, enemies, hero_ok)
  local hero_pick, hero_d = nil, ENGAGE_HERO
  local weak_pick, weak_d = nil, ENGAGE_CREEP
  local creep_pick, creep_d = nil, ENGAGE_CREEP
  for _, unit in ipairs(enemies) do
    local d = dist_from(hero, unit)
    if game.unit_kind(unit) == "hero" then
      if hero_ok and d < hero_d then hero_pick, hero_d = unit, d end
    elseif d < ENGAGE_CREEP then
      local hp = whole_hp(unit)
      if hp > 0 and hp <= LAST_HIT_HP and d < weak_d then weak_pick, weak_d = unit, d end
      if d < creep_d then creep_pick, creep_d = unit, d end
    end
  end
  if hero_pick then return hero_pick end
  return weak_pick or creep_pick
end

-- Engage gate for enemy heroes (87121, 87131): above level 3, HP above 70%, at most 2 enemies within 750.
-- The AI flag is set on every bot slot, so it is not checked. The unnamed test on 87131 (Q77) is not read.
local function may_engage(hero, enemies)
  if game.unit_level(hero) <= ENGAGE_MIN_LEVEL then return false end
  if hp_pct(hero) <= ENGAGE_MIN_PCT then return false end
  local near = 0
  for _, unit in ipairs(enemies) do
    if dist_from(hero, unit) <= ENGAGE_CROWD then near = near + 1 end
  end
  return near <= CROWD_MAX
end

-- Retarget (87137-87150). A hero target above 50% HP opens a search within 700; otherwise candidates
-- within 600 count. A candidate takes over when it has less HP than the target, or, while the target is
-- above 50%, when its HP plus 20 x its level is still below the target's HP. The lowest such candidate wins.
local function retarget(hero, target, enemies)
  local tpct = hp_pct(target)
  local search = game.unit_kind(target) == "hero" and tpct > HERO_SEARCH_PCT
  local range = search and SEARCH_RANGE or RETARGET_RANGE
  local target_hp = whole_hp(target)
  local best, best_hp = target, target_hp
  for _, unit in ipairs(enemies) do
    if unit ~= target and dist_from(hero, unit) < range then
      local hp = whole_hp(unit)
      local wins
      if tpct > HERO_SEARCH_PCT then
        wins = hp + ENEMY_LEVEL_HP * game.unit_level(unit) < target_hp
      else
        wins = hp < target_hp
      end
      if wins and hp < best_hp then best, best_hp = unit, hp end
    end
  end
  return best
end

-- Disengage (87154-87160), for a hero target. At level 6 and above: out of 1500 range, or own HP% + 20 below
-- the target's HP%. Below level 6: out of 750 range, or own HP% - 20 below the target's HP%.
local function disengage(hero, target)
  if game.unit_kind(target) ~= "hero" then return false end
  local far, gap
  if game.unit_level(hero) >= DISENGAGE_LEVEL then
    far, gap = FAR_HIGH, DISENGAGE_GAP
  else
    far, gap = FAR_LOW, -DISENGAGE_GAP
  end
  return dist_from(hero, target) > far or hp_pct(hero) + gap < hp_pct(target)
end

-- Attack group (66155-66172): when a bot takes a target, the other bots whose heroes are within 1200 of it
-- are ordered onto the same target. The 2400 range for flagged units is not applied (no flag in the API).
function Bot.alert(bot, team, target)
  for _, other in ipairs(Bot.roster) do
    if other ~= bot and other.hero and unit_alive(other.hero) and game.team(other.slot) == team then
      if dist_from(other.hero, target) <= ATTACK_GROUP_RANGE then other.target = target end
    end
  end
end

-- Help (66195-66222): the living ally hero within 1100 with the lowest HP is the one to help. The enemy it
-- is facing is the nearest enemy within 800 of that ally. Without one, the fallback is the nearest enemy
-- within 800 of this hero, which the engage search already covers, so nil is returned.
local function help_target(hero, units, team, enemies)
  local ally, ally_hp = nil, nil
  for _, unit in ipairs(units) do
    if unit ~= hero and game.unit_kind(unit) == "hero" and unit_alive(unit) and not hostile(team, unit)
      and dist_from(hero, unit) <= HELP_RANGE then
      local hp = whole_hp(unit)
      if ally_hp == nil or hp < ally_hp then ally, ally_hp = unit, hp end
    end
  end
  if not ally then return nil end
  local pick, pick_d = nil, ENGAGE_HERO
  for _, unit in ipairs(enemies) do
    local d = dist_from(ally, unit)
    if d < pick_d then pick, pick_d = unit, d end
  end
  if pick then Bot.stats.helps = Bot.stats.helps + 1 end
  return pick
end

-- Blink to a point 500 short of the target on the caster-to-target line (point 6583-6585, order 6592-6593).
function Bot.blink_to(hero, target)
  local item = ready_item(hero, BLINK_ITEM)
  if not item then return false end
  local hx, hy = unit_position(hero)
  local tx, ty = unit_position(target)
  local dx, dy = tx - hx, ty - hy
  local len = (dx * dx + dy * dy) ^ 0.5
  if len == 0 then return false end
  local px = tx - dx / len * BLINK_SHORT * TILE
  local py = ty - dy / len * BLINK_SHORT * TILE
  game.cmd_use_item(hero, item.slot, -1, fixed(px), fixed(py))
  Bot.stats.blinks = Bot.stats.blinks + 1
  return true
end

-- Heal consumable (90338): below 50% HP the flask is drunk on the hero. Every bot slot carries the AI flag,
-- so the "target or flag" test always passes.
function Bot.heal(hero)
  if hp_pct(hero) >= HEAL_PCT then return false end
  local item = ready_item(hero, HEAL_ITEM)
  if not item then return false end
  game.cmd_use_item(hero, item.slot, hero, 0, 0)
  Bot.stats.heals = Bot.stats.heals + 1
  return true
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

-- One decision for a bot: returns the hero it controls, or nil when the slot has none alive.
function Bot.think(bot, units, ticks)
  local hero = Bot.find_hero(bot)
  bot.hero = hero
  if not hero then
    bot.retreating = false
    bot.target = nil
    return nil
  end
  local team = game.team(bot.slot)
  Bot.learn_skills(hero)
  Bot.heal(hero)

  -- Retreat below 40% HP and below 300 + 30 x level (87101, 5425-5427); recover at 80%.
  local pct = hp_pct(hero)
  local hp = whole_hp(hero)
  if bot.retreating and pct >= RECOVER_FRACTION * 100 then bot.retreating = false end
  if not bot.retreating and pct < RETREAT_PCT and hp < RETREAT_HP + RETREAT_HP_PER_LEVEL * game.unit_level(hero) then
    bot.retreating = true
  end
  if bot.retreating then
    bot.target = nil
    Bot.retreat(bot, hero, team)
    return hero
  end
  if Bot.shop(bot, hero, team, bot.slot) then return hero end

  local enemies = hostile_list(team, units, hero)
  local target = bot.target
  if target and not (unit_alive(target) and hostile(team, target)) then target = nil end
  if target and disengage(hero, target) then
    bot.target = nil
    Bot.retreat(bot, hero, team)
    return hero
  end
  if target then target = retarget(hero, target, enemies) end
  if not target then
    local hero_ok = may_engage(hero, enemies)
    target = search_target(hero, enemies, hero_ok)
    if not target and hero_ok then target = help_target(hero, units, team, enemies) end
  end
  if not target then
    bot.target = nil
    Bot.walk_lane(bot, hero, team)
    return hero
  end

  if target ~= bot.target then Bot.alert(bot, team, target) end
  bot.target = target
  local d = dist_from(hero, target)
  if d >= BLINK_TRIGGER and Bot.blink_to(hero, target) then return hero end
  if d < CAST_REACH then Bot.cast_ready(bot, hero, target, ticks) end
  Bot.stats.attacks = Bot.stats.attacks + 1
  game.cmd_attack(hero, target)
  return hero
end

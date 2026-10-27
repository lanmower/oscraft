local bounty = {}

local PENALTY_PCT = 40
local RELIABLE = "reliable_gold"
local PENALTY = "buyback_penalty_until"

local hero_ids, hero_by_player, hero_count = {}, {}, -1

function bounty.forget() hero_count = -1 end

local function refresh()
  local count = get_unit_count()
  if count == hero_count then return end
  hero_count = count
  hero_ids, hero_by_player = {}, {}
  for i = 0, count - 1 do
    if game.is_attributed(i) then
      hero_ids[#hero_ids + 1] = i
      local player = unit_player(i)
      if hero_by_player[player] == nil then hero_by_player[player] = i end
    end
  end
end

function bounty.heroes()
  refresh()
  return hero_ids
end

function bounty.hero_of(player)
  refresh()
  return hero_by_player[player]
end

function bounty.reliable_gold(player)
  local hero = bounty.hero_of(player)
  if hero == nil then return 0 end
  return math.max(0, math.min(game.counter(hero, RELIABLE), game.gold(player)))
end

function bounty.spend_reliable(player, gold)
  local hero = bounty.hero_of(player)
  if hero == nil then return end
  game.set_counter(hero, RELIABLE, math.max(0, game.counter(hero, RELIABLE) - gold))
end

function bounty.clamp_reliable()
  for _, hero in ipairs(bounty.heroes()) do
    local held = game.counter(hero, RELIABLE)
    local bank = game.gold(unit_player(hero))
    if held > bank then game.set_counter(hero, RELIABLE, math.max(0, bank)) end
  end
end

function bounty.credit(p)
  local hero = bounty.hero_of(p.player)
  local gold = p.gold
  if hero ~= nil and game.counter(hero, PENALTY) > game.ticks() then gold = math.floor(gold * PENALTY_PCT / 100) end
  if gold <= 0 then return 0 end
  game.cmd_add_gold(p.player, gold)
  local reliable = p.reliable
  if reliable == true then reliable = gold end
  if hero ~= nil and reliable ~= nil and reliable ~= false and reliable > 0 then
    game.set_counter(hero, RELIABLE, game.counter(hero, RELIABLE) + reliable)
  end
  if p.text and p.at ~= nil then
    game.effect("gold_text", 0, 0, { target = p.at, text = "+" .. tostring(gold), colour = "gold" })
  end
  return gold
end

function bounty.credit_enemies(victim_player, gold)
  for _, hero in ipairs(bounty.heroes()) do
    local player = unit_player(hero)
    if game.hostile(player, victim_player) then bounty.credit({ player = player, gold = gold, reliable = true }) end
  end
end

function bounty.debit(player, gold, at)
  if gold <= 0 then return 0 end
  game.cmd_add_gold(player, -gold)
  if at ~= nil then
    game.effect("gold_text", 0, 0, { target = at, text = "-" .. tostring(gold), colour = "gold" })
  end
  return gold
end

return bounty

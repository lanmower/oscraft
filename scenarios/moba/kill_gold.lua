-- Moba kill gold, as mod code. A unit with a bounty pays it when it dies. A killing hero keeps the bounty less the team
-- share, and the killer's side gets the share. A killer that is not a hero (a creep) pays its whole bounty to its side.
-- An attackerless death pays the rival side's gold. A denial (the killer is allied to the victim) pays nothing.
-- The payout is core/bounty.lua's; this file sets the rule.
local bounty = require("bounty")
local aegis = require("aegis")

local TEAM_SHARE_PCT = 20
local NEUTRAL = 255 -- the neutral player (the sim's NEUTRAL_PLAYER)
local MAX_PLAYERS = 16 -- the sim's player slots

-- The first side hostile to `side` in player order: the rival that takes an attackerless death's gold. The neutral
-- player has no rival side, so it has none.
local function rival_of(side)
  if side >= MAX_PLAYERS then return nil end
  for q = 0, MAX_PLAYERS - 1 do
    if q ~= side and game.hostile(side, q) then return q end
  end
  return nil
end

game.trigger("on_died", {
  conditions = { function(_, _, _, amount) return amount > 0 end },
  action = function(unit, killer, killer_player, amount, victim_player, killer_attributed)
    if aegis.reincarnating(unit) then return end
    if killer ~= nil and killer ~= unit and not game.alive(killer) then
      -- The killer died in the same step: its side keeps the reward, and no dead unit takes it.
      if killer_player ~= NEUTRAL and game.hostile(killer_player, victim_player) then
        bounty.pay({ unit = unit, side = killer_player, gold = amount, text = true })
      end
      return
    end
    if killer ~= nil and killer ~= unit then
      -- The neutral player (255) pays no side: a camp creep that kills pays nothing.
      if killer_player == NEUTRAL or not game.hostile(killer_player, victim_player) then return end
      bounty.pay({ unit = unit, side = killer_player, killer = killer, gold = amount, share_pct = TEAM_SHARE_PCT,
        to_unit = killer_attributed, text = true })
      return
    end
    local rival = rival_of(victim_player)
    if rival ~= nil then
      bounty.pay({ unit = unit, side = rival, gold = amount, text = true })
    end
  end,
})

return {}

-- Moba kill gold, as mod code, 1:1 with Dota: gold comes from kills by heroes. A unit with a bounty pays it when a hero kills
-- it: the killing hero keeps the bounty less the team share, and its side gets the share. A kill by a creep, a tower or
-- nothing at all (an attackerless death) earns no gold, and neither does a denial (the killer is allied to the victim).
-- The payout is core/bounty.lua's; this file sets the rule.
local bounty = require("bounty")
local aegis = require("aegis")

local TEAM_SHARE_PCT = 20
local NEUTRAL = 255 -- the neutral player (the sim's NEUTRAL_PLAYER)
local MAX_PLAYERS = 16 -- the sim's player slots

game.trigger("on_died", {
  conditions = { function(_, _, _, amount) return amount > 0 end },
  action = function(unit, killer, killer_player, amount, victim_player, killer_attributed)
    if aegis.reincarnating(unit) then return end
    if killer ~= nil and killer ~= unit and not game.alive(killer) then
      -- The killer died in the same step: a hero's kill still pays its side, and no dead unit takes it.
      if killer_attributed and killer_player ~= NEUTRAL and game.hostile(killer_player, victim_player) then
        bounty.pay({ unit = unit, side = killer_player, gold = amount, text = true })
      end
      return
    end
    if killer ~= nil and killer ~= unit then
      -- Only a hero's kill pays (Dota): a creep, a tower or a neutral camp creep pays nothing.
      if not killer_attributed or killer_player == NEUTRAL or not game.hostile(killer_player, victim_player) then return end
      bounty.pay({ unit = unit, side = killer_player, killer = killer, gold = amount, share_pct = TEAM_SHARE_PCT,
        to_unit = killer_attributed, text = true })
      return
    end
    -- An attackerless death earns no gold (Dota pays for kills by heroes only).
  end,
})

return {}

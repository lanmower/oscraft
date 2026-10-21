-- Moba kill gold, as mod code. A unit with a bounty pays it when it dies. A killing hero keeps the bounty less the team
-- share, and the killer's side gets the share. A killer that is not a hero (a creep) pays its whole bounty to its side.
-- An attackerless death pays the rival side's gold. A denial (the killer is allied to the victim) pays nothing.
-- The payout is core/bounty.lua's; this file sets the rule.
local bounty = require("bounty")

local TEAM_SHARE_PCT = 20

game.trigger("on_died", {
  conditions = { function(_, _, _, amount) return amount > 0 end },
  action = function(unit, killer, killer_player, amount, victim_player, killer_attributed)
    if killer ~= nil and killer ~= unit then
      if not game.hostile(killer_player, victim_player) then return end
      bounty.pay({ unit = unit, side = killer_player, killer = killer, gold = amount, share_pct = TEAM_SHARE_PCT,
        to_unit = killer_attributed, text = true })
      return
    end
    local rival = game.rival_of(victim_player)
    if rival ~= nil then
      bounty.pay({ unit = unit, side = rival, gold = amount, text = true })
    end
  end,
})

return {}

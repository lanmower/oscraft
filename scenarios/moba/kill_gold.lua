-- Moba kill gold, as mod code. A unit with a bounty pays when it dies. A killing hero keeps the bounty less the team
-- share, and the killer's side gets the share. A killer that is not a hero (a creep) pays its whole bounty to its
-- side. An attackerless death pays the rival side's gold.
--
-- GOLD_BY_MOD hands kill gold to this file: while it is set, the sim skips its Rust payout (pay_bounty).
-- Uses only the mod API.
GOLD_BY_MOD = true

local TEAM_SHARE_PCT = 20

-- on_died(unit, killer, killer_player, bounty, victim_player, killer_hero): ids are the mod's world
-- or the room's, whichever fed the death; bounty and killer_hero come with the event.
game.trigger("on_died", {
  conditions = { function(_, _, _, bounty) return bounty > 0 end },
  action = function(unit, killer, killer_player, bounty, victim_player, killer_hero)
    if killer ~= nil and killer ~= unit then
      local share = bounty * TEAM_SHARE_PCT // 100
      if killer_hero then
        game.cmd_add_unit_gold(killer, bounty - share)
        game.cmd_add_gold(killer_player, share)
      else
        game.cmd_add_gold(killer_player, bounty)
      end
      return
    end
    local rival = game.rival_of(victim_player)
    if rival ~= nil then game.cmd_add_gold(rival, bounty) end
  end,
})

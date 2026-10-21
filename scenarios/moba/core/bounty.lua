-- The kill reward. pay{unit, side, gold, killer, share_pct, to_unit, text}: the side is paid gold. When the killer keeps
-- the reward (to_unit), the killer gets all but share_pct of it and the side gets the share. text shows the side's gold
-- as floating text over the victim.
local bounty = {}

function bounty.pay(p)
  local side_gold = p.gold
  if p.to_unit then
    local share = math.floor(p.gold * p.share_pct / 100)
    game.cmd_add_unit_gold(p.killer, p.gold - share)
    side_gold = share
  end
  game.cmd_add_gold(p.side, side_gold)
  if p.text and side_gold > 0 then
    game.effect("gold_text", 0, 0, { target = p.unit, text = "+" .. tostring(side_gold), colour = "gold" })
  end
  return side_gold
end

return bounty

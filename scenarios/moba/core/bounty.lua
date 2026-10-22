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

-- split{unit, gold, recipients, text}: the reward is shared equally by the recipient units, the first taking the rest
-- of the division. text shows each share as floating text over the victim.
function bounty.split(p)
  local n = #p.recipients
  if n == 0 or p.gold <= 0 then return 0 end
  local share = math.floor(p.gold / n)
  local rest = p.gold - share * n
  for idx, u in ipairs(p.recipients) do
    local amount = share + (idx == 1 and rest or 0)
    game.cmd_add_unit_gold(u, amount)
    if p.text and amount > 0 then
      game.effect("gold_text", 0, 0, { target = p.unit, text = "+" .. tostring(amount), colour = "gold" })
    end
  end
  return p.gold
end

return bounty

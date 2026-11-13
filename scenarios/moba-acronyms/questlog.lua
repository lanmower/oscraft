local PANEL = "quest_log"
local CREDITS = "This version producted by: DracoL1ch, Santy Keller\n"
  .. "Special thanks to BlinD-HuNTeR, quq_CCCP, xpforever, K.U.R.T. and Kuarinofu.\n"
  .. "It could never happen without oldschool stuff: ResQ, LordShinjo, Lemon.\n"
  .. "Tons of people worked here before, including IceFrog."

local LOCAL = 0
local open = {}

local function toggle(player)
  if open[player] then
    open[player] = nil
    game.panel(PANEL .. tostring(player))
    return
  end
  open[player] = true
  game.panel(PANEL .. tostring(player), { title = "Credits and Contacts", text = CREDITS, player = player })
end

return {
  on_action = function(name, args)
    if name == "setup" then
      LOCAL = args.player or LOCAL
    elseif name == "key" and args.key == "f9" and args.phase == "down" then
      toggle(args.player or LOCAL)
    end
  end,
}

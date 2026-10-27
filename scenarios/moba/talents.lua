local units = require("units")
local xp = require("xp")

local TALENT_LEVELS = { 10, 15, 20, 25 }
local TALENT_SIDES = 2
local FIRST_TALENT_BIT = 2

local function has_bit(value, bit)
  return math.floor(value / (2 ^ bit)) % 2 == 1
end

local function choose_talent(unit, tier, side)
  if tier < 0 or tier >= #TALENT_LEVELS or side < 0 or side >= TALENT_SIDES then
    return false
  end
  if xp.character_level(unit) < TALENT_LEVELS[tier + 1] then
    return false
  end
  local bits = game.upgrade_bits(unit)
  local first = FIRST_TALENT_BIT + 2 * tier
  if has_bit(bits, first) or has_bit(bits, first + 1) then
    return false
  end
  return game.set_upgrade_bit(unit, first + side, true)
end

local LOCAL = 0

return {
  on_action = function(name, args)
    if name == "setup" then
      LOCAL = args.player or LOCAL
    elseif name == "ui" and args.call == "talent" and args.args ~= nil then
      local a = args.args
      local seat = args.player or LOCAL
      if units.own(a.unit, seat) then choose_talent(a.unit, a.tier, a.side) end
    end
  end,
}

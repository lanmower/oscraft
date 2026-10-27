local config = require("config")
local announce = require("announce")

local TPS = config.TICKS_PER_SECOND
local DURATION_TICKS = 5 * TPS
local COOLDOWN_TICKS = 300 * TPS
local NEUTRAL = 255
local TIER_ONE_TOWER = "tower"
local STATUS = "invulnerable"
local ORDERED = " ordered to use |c00ff0000Glyph of Fortification|r"
local NOTICE_SECONDS = 5

local ready_at = { [0] = 0, [1] = 0 }

local function side_of(player)
  if game.team(player) == game.team(0) then return 0 end
  return 1
end

local function protect_structures(side)
  for _, id in ipairs(game.units(nil)) do
    local owner = unit_player(id)
    if owner ~= NEUTRAL and game.is_structure(id) and side_of(owner) == side then
      game.apply_status(id, STATUS, DURATION_TICKS)
    end
  end
end

local function tell_allies(who, side)
  local text = announce.tag(who) .. ORDERED
  for p = 0, config.PLAYERS - 1 do
    if p ~= who and game.seat_start(p) ~= nil and side_of(p) == side then game.message(p, text, NOTICE_SECONDS) end
  end
end

local function activate(who)
  local side = side_of(who)
  if game.ticks() < ready_at[side] then return false, "cooldown" end
  ready_at[side] = game.ticks() + COOLDOWN_TICKS
  protect_structures(side)
  tell_allies(who, side)
  return true
end

return {
  activate = activate,
  on_died = function(unit, _, _, _, victim_player)
    if victim_player == nil or victim_player == NEUTRAL then return end
    if game.unit_kind(unit) == TIER_ONE_TOWER then ready_at[side_of(victim_player)] = 0 end
  end,
}

local config = require("config")
local units = require("units")
local bounty = require("bounty")
local acronyms = require("hero_acronyms")
local match_stats = require("match_stats")

local had_ancient = {}

local function heroes()
  local list = {}
  for p = 0, config.PLAYERS - 1 do
    local hero = bounty.hero_of(p)
    if hero ~= nil and game.seat_start(p) ~= nil then
      local kind = game.unit_kind(hero) or "?"
      list[#list + 1] = { player = p, hero = hero, team = game.team(p), name = acronyms[kind] or kind }
    end
  end
  return list
end

return {
  on_check_win = function()
    local standing = {}
    for k = 0, get_unit_count() - 1 do
      if unit_alive(k) and units.is_building(k) then
        standing[unit_player(k)] = true
      end
    end
    for p = 0, config.PLAYERS - 1 do
      if standing[p] then
        had_ancient[p] = true
      elseif had_ancient[p] then
        for q = 0, config.PLAYERS - 1 do
          if q ~= p and units.hostile(p, q) then
            local side = game.team(q)
            if side ~= nil then
              match_stats.publish(side, heroes())
              return side
            end
          end
        end
      end
    end
    return nil
  end,
}

-- The target `unit` takes out of the enemies the sim lists in its reach (game.strike_reach: each one can be struck and is
-- within `range`, with its squared distance). The class order decides who may be struck, the nearest within a class
-- wins, and the lowest id wins a tie of class and distance, so the list's order does not change the pick.
local function best_in_reach(unit, range, aggro)
  local reach = game.strike_reach(unit, range)
  local forced, att_kind = nil, nil
  local best, best_class, best_d = -1, nil, nil
  for k = 1, #reach, 2 do
    local j, d = reach[k], reach[k + 1]
    if att_kind == nil then forced, att_kind = game.class_override(unit), kind_of(unit) end
    local class = class_order(forced, att_kind, kind_of(j), aggro == j)
    if best_class == nil or class < best_class
      or (class == best_class and (d < best_d or (d == best_d and j < best))) then
      best, best_class, best_d = j, class, d
    end
  end
  return best
end

-- The enemy `unit` strikes this tick, -1 for none. A unit that deals no damage strikes nothing. A taunter in reach is
-- struck first; then the enemy struck last while it stays valid (sticky); otherwise the best class in reach
-- (class_order), the nearest within a class, the lowest id on a tie.
function targeting.strike_choice(unit)
  local damage, range = game.weapon(unit)
  if damage <= 0 then return -1 end
  local taunter = game.taunter(unit)
  if taunter ~= nil and within(unit, taunter, range) and game.can_target(unit, taunter) then return taunter end
  local last = game.strike_of(unit)
  if last >= 0 and unit_alive(last) and within(unit, last, range) and game.can_target(unit, last) then return last end
  return best_in_reach(unit, range, game.aggro_target(unit))
end

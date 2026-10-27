local keys = require("ability_keys")

local M = {}

function M.explicit_slot(hero, key)
  for _, a in ipairs(game.unit_abilities(hero)) do
    if keys[a.name] == key then return a.slot end
  end
  return nil
end

function M.claimed(hero, slot)
  for _, a in ipairs(game.unit_abilities(hero)) do
    if a.slot == slot and keys[a.name] ~= nil then return true end
  end
  return false
end

return M

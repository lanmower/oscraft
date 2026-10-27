local GROUND_ORDER = {
  "courier", "courier_penguin", "courier_raccoon", "courier_crab", "courier_seal", "courier_skink", "courier_dog",
  "courier_dune_worm", "courier_carrion_beetle", "courier_fel_boar", "courier_rabbit", "courier_pack_horse",
}
local FLYING_ORDER = {
  "courier_flying", "courier_zeppelin", "courier_vulture", "courier_owl", "courier_viper", "courier_frost_wyrm", "courier_flying_panda",
}
local RANDOM_GROUND = {
  { odds = 1, kind = "courier_mini_pudge" },
  { odds = 2, kind = "courier_grunt" },
  { odds = 2, kind = "courier_pandaren" },
}
local TECHIES = "courier_mini_techies"
local AFTER_SPECIAL = { courier_mini_pudge = TECHIES, courier_grunt = TECHIES, courier_pandaren = TECHIES, [TECHIES] = "courier_rabbit" }
local DRAWS = 10

local flying, next_of, set = {}, {}, {}
for _, order in ipairs({ GROUND_ORDER, FLYING_ORDER }) do
  for i, kind in ipairs(order) do
    next_of[kind] = order[i % #order + 1]
    set[kind] = true
  end
end
for _, kind in ipairs(FLYING_ORDER) do flying[kind] = true end
for kind in pairs(AFTER_SPECIAL) do set[kind] = true end

local function next_kind(kind, draw)
  if AFTER_SPECIAL[kind] ~= nil then return AFTER_SPECIAL[kind] end
  local target = next_of[kind]
  if target == nil or flying[kind] then return target end
  local chosen = target
  for _, row in ipairs(RANDOM_GROUND) do
    if draw(DRAWS) < row.odds then chosen = row.kind end
  end
  return chosen
end

return { set = set, flying = flying, next_kind = next_kind, DRAWS = DRAWS }

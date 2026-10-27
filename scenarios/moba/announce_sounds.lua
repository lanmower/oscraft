local SOUNDS = {
  first_blood = "firstblood",
  killing_spree = "killing_spree",
  dominating = "dominating",
  megakill = "megakill",
  unstoppable = "unstoppable",
  wickedsick = "wickedsick",
  monsterkill = "monsterkill",
  godlike = "godlike",
  holyshit = "holyshit",
  double_kill = "double_kill",
  triple_kill = "triple_kill",
  ultrakill = "ultrakill",
  rampage = "rampage",
}

local function name_of(event)
  local name = SOUNDS[event]
  if type(name) == "string" and name ~= "" then return name end
  return nil
end

local function play(event, player)
  local name = name_of(event)
  if name == nil then return false end
  game.sound(name, player)
  return true
end

return { play = play, name_of = name_of }

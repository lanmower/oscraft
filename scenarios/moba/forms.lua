local FORMS = {
  terrorblade_metamorphosis = {
    source = "war3map.j unit swap Eevi to Eevm (ability.ini:69142, 69162; unit.ini:362 rangeN1 550)",
    base = "terrorblade",
    form = "terrorblade_demon_form",
    ticks = { 640, 704, 768, 832 },
  },
}

FORMS.troll_warlord_berserker_rage = {
  source = "war3map.j A9E 58270-58279, I8O 58250-58269 (A0BE unit swap to the melee form, ability.ini:23628-23666); speed and bash from the tooltip",
  base = "troll_warlord",
  form = "troll_warlord_melee_form",
  toggle = true,
  speed = { 10, 20, 30, 40 },
}

local ORDER = { "terrorblade_metamorphosis", "troll_warlord_berserker_rage" }
local S_SPEED = 3
local transformed = {}

local function rank_of(unit, ability)
  for _, row in ipairs(game.unit_abilities(unit)) do
    if row.name == ability then return game.skill_rank(unit, row.slot) end
  end
  return 0
end

local function revert(unit)
  local index = game.counter(unit, "form_spec")
  game.set_counter(unit, "form_spec", 0)
  transformed[unit] = nil
  if index == 0 or not game.alive(unit) then return end
  game.add_stat(unit, S_SPEED, -game.counter(unit, "form_speed"))
  game.set_counter(unit, "form_speed", 0)
  game.morph(unit, FORMS[ORDER[index]].base)
end

local function start(index, caster)
  local spec = FORMS[ORDER[index]]
  local rank = rank_of(caster, ORDER[index])
  if rank == 0 then return end
  if spec.toggle and game.counter(caster, "form_spec") == index then
    revert(caster)
    return
  end
  if game.unit_kind(caster) ~= spec.base then return end
  if not game.morph(caster, spec.form) then return end
  game.set_counter(caster, "form_spec", index)
  game.set_counter(caster, "form_due", spec.toggle and 2000000000 or game.ticks() + spec.ticks[math.min(rank, #spec.ticks)])
  if spec.speed ~= nil then
    local bonus = spec.speed[math.min(rank, #spec.speed)]
    game.add_stat(caster, S_SPEED, bonus)
    game.set_counter(caster, "form_speed", bonus)
  end
  transformed[caster] = true
end

local ON_CAST = {}
for index, name in ipairs(ORDER) do ON_CAST[name] = function(caster) start(index, caster) end end

return {
  on_cast = function(caster, ability_name)
    local run = ON_CAST[ability_name]
    if run ~= nil then run(caster) end
  end,
  on_tick = function()
    for unit in pairs(transformed) do
      if not game.alive(unit) then
        game.set_counter(unit, "form_spec", 0)
        transformed[unit] = nil
      elseif game.ticks() >= game.counter(unit, "form_due") then
        revert(unit)
      end
    end
  end,
  on_state_replaced = function()
    transformed = {}
    for unit = 0, get_unit_count() - 1 do
      if game.counter(unit, "form_spec") ~= 0 and game.alive(unit) then transformed[unit] = true end
    end
  end,
}

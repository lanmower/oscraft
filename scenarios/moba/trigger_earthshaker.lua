local config = require("config")
local lib = require("trigger_lib")

local ECHO = {
  source = "war3map.j IHE 48513-48538 (A0DH casts A3L7, then A0DM from every enemy within 600; the scepter echoes A1OB heroes again, PSX 48500), A0DM ability.ini:26878-26893",
  ability = "earthshaker_echo_slam",
  emitter_radius = config.milli(9375),
  echo_radius = config.milli(8984),
  damage = { 40, 55, 70 },
}

local function echo_from(caster, emitter, damage)
  local x, y = unit_position(emitter)
  game.spell_damage(caster, emitter, damage, lib.SPELL_MAGICAL)
  for _, unit in ipairs(lib.enemies_at(caster, x, y, ECHO.echo_radius)) do
    if unit ~= emitter and not lib.held(unit, "spell_immunity") then
      game.spell_damage(caster, unit, damage, lib.SPELL_MAGICAL)
    end
  end
end

local function echo_slam(caster)
  local rank = math.min(lib.rank_of(caster, ECHO.ability), #ECHO.damage)
  if rank == 0 then return end
  local x, y = unit_position(caster)
  local damage = ECHO.damage[rank]
  local scepter = lib.scepter(caster)
  local emitters = lib.enemies_at(caster, x, y, ECHO.emitter_radius)
  for _, emitter in ipairs(emitters) do
    if not lib.held(emitter, "spell_immunity") then
      echo_from(caster, emitter, damage)
      if scepter and game.is_attributed(emitter) and game.alive(emitter) then echo_from(caster, emitter, damage) end
    end
  end
end

return {
  on_cast = function(caster, ability_name)
    if ability_name == ECHO.ability then echo_slam(caster) end
  end,
}

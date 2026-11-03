local probe_stage = 0
local probe_creep = nil
local base_on_tick = on_tick
local function hero_of(slot)
  for _, u in ipairs(game.units(slot)) do
    if game.unit_kind(u) == "hero" then return u end
  end
  return nil
end
function on_tick(state)
  local ticks = game.time()
  local hero = hero_of(0)
  local enemy = hero_of(1)
  if probe_stage == 0 and ticks >= 40 and hero and enemy then
    local x, y = unit_position(hero)
    game.cmd_spawn("neutral_kobold", x + 3 * 65536, y, 1, 0)
    probe_stage = 1
  elseif probe_stage == 1 and ticks >= 60 then
    for _, u in ipairs(game.units(nil)) do
      local ux, uy = unit_position(u)
      if game.unit_kind(u) ~= "hero" and unit_player(u) == 1 and hero then
        local hx, hy = unit_position(hero)
        if (ux - hx) ^ 2 + (uy - hy) ^ 2 < (5 * 65536) ^ 2 then probe_creep = u end
      end
    end
    if probe_creep then
      local hp = unit_hp(probe_creep) // 65536
      state.probe_creep_hp = hp
      if hp > 30 then game.cmd_damage(enemy, probe_creep, (hp - 30) * 65536) end
      probe_stage = 2
    end
    state.probe_spawned = probe_creep and 1 or 0
  elseif probe_stage == 2 and ticks >= 90 and hero and enemy then
    local hp = unit_hp(hero) // 65536
    state.probe_hero_hp = hp
    game.cmd_damage(enemy, hero, (hp * 9 // 10) * 65536)
    probe_stage = 3
  end
  base_on_tick(state)
end

local config = require("config")

local TICKS_PER_SECOND = 16
local PHYSICAL = 1
local PURE = 3
local MAX_HIT = 32767
local FOREVER = 1000000000

local LAND_MINE = {
  source = "war3map.j R6O 57486-57529, R3O 57434-57478, R7O 57530 (A05J, units n00O n00P n00Q n00N; ability.ini:14956)",
  ability = "techies_land_mines",
  unit = "techies_land_mine",
  life = FOREVER,
  arm = 8,
  trigger_radius = 3516,
  delay = 5,
  blast_full_radius = 3516,
  blast_radius = 8203,
  damage = { 150, 190, 225, 260 },
  damage_type = PHYSICAL,
}

local STASIS_TRAP = {
  source = "war3map.j R8O 57538-57586, IVO 57587 (A06H, unit otot; stun A38L ability.ini:113120)",
  ability = "techies_stasis_trap",
  unit = "techies_stasis_trap_ward",
  life = 360 * TICKS_PER_SECOND,
  arm = 24,
  trigger_radius = 7031,
  delay = 24,
  stun_radius = 7422,
  stun_ticks = { 40, 48, 56, 64 },
}

local STATIC_REMNANT = {
  source = "war3map.j M2X 47883-47930, M1X 47840-47882 (A14P)",
  ability = "storm_spirit_static_remnant",
  unit = "storm_spirit_static_remnant_unit",
  at_caster = true,
  life = 12 * TICKS_PER_SECOND,
  arm = 0,
  trigger_radius = 3672,
  delay = 0,
  blast_full_radius = 4453,
  blast_radius = 4453,
  damage = { 140, 180, 220, 260 },
  damage_type = 2,
}

local REMOTE_MINE = {
  source = "ability.ini A0AK 22625-22682 (units o018 o002 o00B), detonation radius and trigger GUESS",
  ability = "techies_remote_mines",
  detonator = "techies_remote_detonate",
  unit = "techies_remote_mine",
  at_caster = false,
  manual = true,
  life = 600 * TICKS_PER_SECOND,
  arm = 0,
  trigger_radius = 0,
  delay = 0,
  blast_full_radius = 6641,
  blast_radius = 6641,
  damage = { 300, 450, 600 },
  damage_type = 2,
}

local PSIONIC_TRAP = {
  source = "war3map.j 59180-59262 (A0KX family, unit e020), slow 30-50 percent for 5 s, max traps 3 x rank + 2; trigger radius GUESS",
  ability = "templar_assassin_psionic_trap",
  unit = "templar_assassin_psionic_trap_unit",
  life = 1000000000,
  arm = 8,
  trigger_radius = 4688,
  delay = 0,
  blast_radius = 5859,
  slow_keep = { 70, 60, 50 },
  slow_ticks = 80,
  max_base = 2,
  max_per_rank = 3,
}

local SPECS = { LAND_MINE, STASIS_TRAP, STATIC_REMNANT, REMOTE_MINE, PSIONIC_TRAP }

local field = {}

local function rank_of(unit, ability)
  for _, row in ipairs(game.unit_abilities(unit)) do
    if row.name == ability then return game.skill_rank(unit, row.slot) end
  end
  return 0
end

local function hostile_targets(source_owner, x, y, radius)
  local found = {}
  for _, unit in ipairs(game.units_near(x, y, config.milli(radius))) do
    if game.alive(unit) and not game.is_structure(unit) and game.counter(unit, "mine_spec") == 0 and game.hostile(source_owner, unit_player(unit)) then
      found[#found + 1] = unit
    end
  end
  return found
end

local remove

local function plant(spec_index, caster)
  local spec = SPECS[spec_index]
  local rank = rank_of(caster, spec.ability)
  if rank == 0 then return end
  local x, y = game.cast_point(caster)
  if spec.at_caster or x == nil or (x == 0 and y == 0) then x, y = unit_position(caster) end
  local mine = game.spawn(unit_player(caster), spec.unit, x, y)
  if mine == nil then return end
  local now = game.ticks()
  game.set_counter(mine, "mine_spec", spec_index)
  game.set_counter(mine, "mine_rank", rank)
  game.set_counter(mine, "mine_caster", caster)
  game.set_counter(mine, "mine_armed", now + spec.arm)
  game.set_counter(mine, "mine_expires", now + spec.life)
  game.set_counter(mine, "mine_fire", 0)
  game.set_counter(mine, "mine_born", now)
  field[mine] = true
  if spec.max_per_rank ~= nil then
    local limit = spec.max_base + spec.max_per_rank * rank
    local owned, oldest = 0, nil
    for other in pairs(field) do
      if game.counter(other, "mine_spec") == spec_index and game.counter(other, "mine_caster") == caster then
        owned = owned + 1
        if oldest == nil or game.counter(other, "mine_born") < game.counter(oldest, "mine_born") then oldest = other end
      end
    end
    if owned > limit and oldest ~= nil then remove(oldest) end
  end
end

remove = function(mine)
  game.set_counter(mine, "mine_spec", 0)
  game.spell_damage(mine, mine, MAX_HIT, PURE)
  field[mine] = nil
end

local function source_of(mine)
  local caster = game.counter(mine, "mine_caster")
  if game.alive(caster) and game.hostile(unit_player(mine), unit_player(caster)) == false then return caster end
  return mine
end

local function explode_land_mine(mine, spec, rank)
  local x, y = unit_position(mine)
  local owner = unit_player(mine)
  local source = source_of(mine)
  local damage = spec.damage[math.min(rank, #spec.damage)]
  local near = hostile_targets(owner, x, y, spec.blast_radius)
  remove(mine)
  for _, unit in ipairs(near) do
    local ux, uy = unit_position(unit)
    local dx, dy = ux - x, uy - y
    local reach = config.milli(spec.blast_full_radius)
    local amount = damage
    if dx * dx + dy * dy > reach * reach then amount = math.floor(damage / 2) end
    game.spell_damage(source, unit, amount, spec.damage_type)
  end
end

local function spring_stasis_trap(mine, spec, rank)
  local x, y = unit_position(mine)
  local owner = unit_player(mine)
  local near = hostile_targets(owner, x, y, spec.stun_radius)
  remove(mine)
  for _, unit in ipairs(near) do
    if game.status_left(unit, "spell_immunity") == 0 then
      game.apply_status(unit, "stun", spec.stun_ticks[math.min(rank, #spec.stun_ticks)])
    end
  end
end

local function spring_psionic_trap(mine, spec, rank)
  local x, y = unit_position(mine)
  local near = hostile_targets(unit_player(mine), x, y, spec.blast_radius)
  remove(mine)
  local keep = math.floor(65536 * spec.slow_keep[math.min(rank, #spec.slow_keep)] / 100)
  for _, unit in ipairs(near) do
    if game.status_left(unit, "spell_immunity") == 0 then game.apply_status(unit, "slow", spec.slow_ticks, keep) end
  end
end

local DETONATE = { explode_land_mine, spring_stasis_trap, explode_land_mine, explode_land_mine, spring_psionic_trap }

local function step(mine)
  if not game.alive(mine) then
    field[mine] = nil
    return
  end
  local spec_index = game.counter(mine, "mine_spec")
  local spec = SPECS[spec_index]
  local now = game.ticks()
  if now >= game.counter(mine, "mine_expires") then
    remove(mine)
    return
  end
  local fire = game.counter(mine, "mine_fire")
  if fire ~= 0 then
    if now >= fire then DETONATE[spec_index](mine, spec, game.counter(mine, "mine_rank")) end
    return
  end
  if spec.manual or now < game.counter(mine, "mine_armed") then return end
  local x, y = unit_position(mine)
  if #hostile_targets(unit_player(mine), x, y, spec.trigger_radius) > 0 then
    game.set_counter(mine, "mine_fire", now + spec.delay)
  end
end

local ON_CAST = {}
for index, spec in ipairs(SPECS) do
  ON_CAST[spec.ability] = function(caster) plant(index, caster) end
end

local function rebuild()
  field = {}
  for unit = 0, get_unit_count() - 1 do
    if game.counter(unit, "mine_spec") ~= 0 and game.alive(unit) then field[unit] = true end
  end
end

local function detonate_all(caster)
  local now = game.ticks()
  for mine in pairs(field) do
    local spec = SPECS[game.counter(mine, "mine_spec")]
    if spec ~= nil and spec.manual and game.counter(mine, "mine_caster") == caster and game.counter(mine, "mine_fire") == 0 then
      game.set_counter(mine, "mine_fire", now)
    end
  end
end

return {
  on_cast = function(caster, ability_name)
    local run = ON_CAST[ability_name]
    if run ~= nil then run(caster) end
    if ability_name == REMOTE_MINE.detonator then detonate_all(caster) end
  end,
  on_tick = function()
    for mine in pairs(field) do step(mine) end
  end,
  on_state_replaced = function()
    rebuild()
  end,
}

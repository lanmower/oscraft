local MARK = 30
local TICKS = 16
local UNTIL = "deny_mark_until"

local SPECS = {
  doom_bringer_doom_basic = 15 * TICKS,
  viper_viper_strike = 82,
  warlock_shadow_word = 11 * TICKS,
  queenofpain_shadow_strike_qop = 15 * TICKS,
}

local marked = {}

return {
  on_cast = function(caster, ability_name)
    local span = SPECS[ability_name]
    if span == nil then return end
    local target = game.cast_target(caster)
    if target == nil or not game.alive(target) or not game.hostile(unit_player(caster), unit_player(target)) then return end
    local due = game.ticks() + span
    if game.counter(target, UNTIL) > game.ticks() then
      game.set_counter(target, UNTIL, due)
      return
    end
    game.set_counter(target, UNTIL, due)
    game.add_stat(target, MARK, 1)
    marked[#marked + 1] = target
  end,

  on_tick = function()
    if #marked == 0 then return end
    local now = game.ticks()
    local keep = {}
    for _, unit in ipairs(marked) do
      if game.counter(unit, UNTIL) > now then
        keep[#keep + 1] = unit
      else
        game.add_stat(unit, MARK, -1)
        game.set_counter(unit, UNTIL, 0)
      end
    end
    marked = keep
  end,

  on_state_replaced = function()
    marked = {}
    for unit = 0, get_unit_count() - 1 do
      if game.counter(unit, UNTIL) ~= 0 then marked[#marked + 1] = unit end
    end
  end,
}

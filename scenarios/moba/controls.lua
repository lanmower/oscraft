local config = require("config")
local units = require("units")
local heroes = require("heroes")
local bots = require("bots")
local hotkeys = require("hotkeys")
local courier = require("courier")
local glyph = require("glyph")
local patrol = require("patrol")

local FIX = config.FIX
local TARGET_UNIT = 1
local TARGET_POINT = 2
local TARGET_UNIT_OR_POINT = 3
local TICKS_PER_SECOND = config.TICKS_PER_SECOND

local CFG = {
  keys = {},
  custom = {},
  refusals = {},
  doubleTapTicks = 5,
  dragSlop = 0.25,
  pickRadius = 1.2,
  formationSpacing = 0.7,
  centreSeconds = 0.4,
  doubleHoldTicks = math.floor(0.2 * TICKS_PER_SECOND),
}
local LOCAL = 0
local FOG_ON = true

local STATE = {}
local function st(who)
  local s = STATE[who]
  if s == nil then
    s = { sel = {}, groups = {}, recalled = {}, armed = nil, attack_next = false, press = nil, held = {} }
    STATE[who] = s
  end
  return s
end

local function tile(v) return v / FIX end
local function fx(v) return math.floor(v * FIX) end
local function lo(a, b) if a < b then return a end return b end
local function hi(a, b) if a > b then return a end return b end

local function json_ids(list)
  local out = "["
  for i, id in ipairs(list) do
    if i > 1 then out = out .. "," end
    out = out .. tostring(id)
  end
  return out .. "]"
end

local function json_str(s) return '"' .. s .. '"' end

local function feedback(who, body)
  game.ui_event("feedback", '{"player":' .. tostring(who) .. "," .. body .. "}")
end

local function my_selected(who)
  local out = {}
  for _, id in ipairs(st(who).sel) do
    if units.own(id, who) or courier.shared(id, who) then out[#out + 1] = id end
  end
  return out
end

local function hero_selected(who)
  return units.hero_of(st(who).sel, who)
end

local function set_selection(who, ids)
  local list = {}
  for _, id in ipairs(ids) do
    local dup = false
    for _, kept in ipairs(list) do
      if kept == id then dup = true end
    end
    if not dup and unit_alive(id) then list[#list + 1] = id end
  end
  st(who).sel = list
  feedback(who, '"kind":"selection","ids":' .. json_ids(list))
end

local function pick(x, z)
  local best, best_d = nil, nil
  for _, id in ipairs(game.units_near(fx(x), fx(z), fx(CFG.pickRadius))) do
    if unit_alive(id) then
      local ux, uz = unit_position(id)
      local dx, dz = tile(ux) - x, tile(uz) - z
      local d = dx * dx + dz * dz
      if best_d == nil or d < best_d then
        best, best_d = id, d
      end
    end
  end
  if best == nil then return nil end
  return { id = best, player = unit_player(best) }
end

local function is_enemy(who, hit) return hit ~= nil and hit.player ~= who end

local function box_ids(who, x0, z0, x1, z1, additive)
  local inside, heroes = {}, {}
  for _, id in ipairs(game.units(who)) do
    local ux, uz = unit_position(id)
    ux, uz = tile(ux), tile(uz)
    if ux >= lo(x0, x1) and ux <= hi(x0, x1) and uz >= lo(z0, z1) and uz <= hi(z0, z1) then
      inside[#inside + 1] = id
      if game.is_attributed(id) then heroes[#heroes + 1] = id end
    end
  end
  local picked = #heroes > 0 and heroes or inside
  if not additive then return picked end
  local out = {}
  for _, id in ipairs(st(who).sel) do out[#out + 1] = id end
  for _, id in ipairs(picked) do out[#out + 1] = id end
  return out
end

local function click_ids(who, hit, additive)
  if hit == nil then
    if additive then return st(who).sel end
    return {}
  end
  if not additive then return { hit.id } end
  local out, found = {}, false
  for _, id in ipairs(st(who).sel) do
    if id == hit.id then found = true else out[#out + 1] = id end
  end
  if not found then out[#out + 1] = hit.id end
  return out
end

local function is_double_click(who, hit)
  local s = st(who)
  local now = game.time()
  local last = s.lastClick
  if hit == nil then
    s.lastClick = nil
    return false
  end
  s.lastClick = { id = hit.id, tick = now }
  return hit.player == who and last ~= nil and last.id == hit.id and now - last.tick <= CFG.doubleTapTicks
end

local function on_screen(who, id)
  local v = st(who).view
  if v == nil then return true end
  local ux, uz = unit_position(id)
  ux, uz = tile(ux), tile(uz)
  return ux >= v.x0 and ux <= v.x1 and uz >= v.z0 and uz <= v.z1
end

local function same_kind_on_screen(who, id)
  local kind = game.unit_kind(id)
  if kind == nil then return { id } end
  local out = {}
  for _, u in ipairs(game.units(who)) do
    if game.unit_kind(u) == kind and on_screen(who, u) then out[#out + 1] = u end
  end
  return out
end


local function refuse(who, reason)
  local text = CFG.refusals[reason] or CFG.refusals.refused or reason
  feedback(who, '"kind":"refusal","reason":' .. json_str(reason) .. ',"text":' .. json_str(text))
end

local function arm(who, state)
  st(who).armed = state
  local item = state.item and "true" or "false"
  feedback(who, '"kind":"targetArm","slot":' .. tostring(state.slot) .. ',"item":' .. item ..
    ',"radius":' .. tostring(state.radius) .. ',"targetKind":' .. json_str(state.kind))
end

local function disarm(who)
  local s = st(who)
  if s.armed == nil then return false end
  s.armed = nil
  feedback(who, '"kind":"targetEnd"')
  return true
end

local ORDER_MOVE, ORDER_ATTACK_MOVE, QUEUE_ATTACK, ORDER_HOLD = 0, 2, 255, 4

local function clear_hold_fire(id)
  if game.holds_fire(id) then game.set_hold_fire(id, false) end
end

local function issue(id, o)
  clear_hold_fire(id)
  if o.enemy ~= nil then
    game.cmd_attack(id, o.enemy)
  elseif o.attack then
    game.attack_move(id, fx(o.x), fx(o.z))
  else
    game.cmd_order(id, fx(o.x), fx(o.z))
  end
end

local function order_selected(who, x, z, enemy, attack, queued)
  local ids = my_selected(who)
  if #ids == 0 then return false end
  if enemy ~= nil then
    feedback(who, '"kind":"attack","x":' .. tostring(x) .. ',"z":' .. tostring(z) .. ',"targetId":' .. tostring(enemy.id))
  else
    local kind = attack and "attackMove" or "move"
    feedback(who, '"kind":' .. json_str(kind) .. ',"x":' .. tostring(x) .. ',"z":' .. tostring(z) .. ',"ids":' .. json_ids(ids))
  end
  local spacing = CFG.formationSpacing
  for i, id in ipairs(ids) do
    local o = { x = x, z = z, attack = attack, enemy = enemy and enemy.id or nil }
    if enemy == nil then
      o.x = x + ((i - 1) % 3 - 1) * spacing
      o.z = z + math.floor((i - 1) / 3) * spacing
    end
    if queued then
      local kind = ORDER_MOVE
      if o.enemy ~= nil then kind = QUEUE_ATTACK elseif o.attack then kind = ORDER_ATTACK_MOVE end
      game.queue_order(id, kind, fx(o.x), fx(o.z), o.enemy or -1)
    else
      game.queue_clear(id)
      issue(id, o)
    end
  end
  return true
end

local function stop_selected(who)
  for _, id in ipairs(my_selected(who)) do
    clear_hold_fire(id)
    game.queue_clear(id)
    local ux, uz = unit_position(id)
    game.cmd_order(id, ux, uz)
  end
end

local function double_hold_on(who)
  return game.setting(who, "doublehold") == 1
end

local function hold_selected(who)
  local s = st(who)
  local now = game.time()
  local again = double_hold_on(who) and s.holdKey ~= nil and now - s.holdKey <= CFG.doubleHoldTicks
  s.holdKey = now
  for _, id in ipairs(my_selected(who)) do
    game.queue_clear(id)
    if again and game.order_of(id) == ORDER_HOLD then
      game.set_hold_fire(id, true)
    else
      clear_hold_fire(id)
      game.cmd_hold(id)
    end
  end
end

local function centre_on(who, id)
  local ux, uz = unit_position(id)
  game.camera(who, ux, uz, 1, CFG.centreSeconds)
end

local function centre_selected(who)
  local id = hero_selected(who)
  if id == nil then id = my_selected(who)[1] end
  if id ~= nil then centre_on(who, id) end
end

local function patrol_selected(who, x, z)
  local ids = my_selected(who)
  if #ids == 0 then return end
  feedback(who, '"kind":"attackMove","x":' .. tostring(x) .. ',"z":' .. tostring(z) .. ',"ids":' .. json_ids(ids))
  for _, id in ipairs(ids) do
    clear_hold_fire(id)
    patrol.start(id, fx(x), fx(z))
  end
end

local function select_hero(who)
  local s = st(who)
  local hero = nil
  for _, id in ipairs(game.units(who)) do
    if game.is_attributed(id) and not game.is_summoned(id) then
      hero = id
      break
    end
  end
  if hero == nil then return refuse(who, "noHero") end
  local now = game.time()
  local again = s.heroKey ~= nil and now - s.heroKey <= CFG.doubleTapTicks
  s.heroKey = now
  set_selection(who, { hero })
  if again then centre_on(who, hero) end
end

local function select_courier(who)
  local s = st(who)
  local found = nil
  for _, id in ipairs(game.units(nil)) do
    if courier.shared(id, who) then
      found = id
      break
    end
  end
  if found == nil then return refuse(who, "noCourier") end
  local now = game.time()
  local again = s.courierKey ~= nil and now - s.courierKey <= CFG.doubleTapTicks
  s.courierKey = now
  set_selection(who, { found })
  if again then centre_on(who, found) end
end

local function caster_of(who)
  return hero_selected(who) or courier.first(my_selected(who))
end

local function ability_of(hero, slot)
  for _, a in ipairs(game.unit_abilities(hero)) do
    if a.slot == slot then return a end
  end
  return nil
end

local function cast_now(hero, slot, target, x, z)
  game.cmd_cast(hero, slot, target, fx(x), fx(z))
end

local function out_of_range(hero, name, x, z, target)
  local ext = ability_ext(name)
  local reach = ext and ext.cast_range or 0
  if reach <= 0 then return false end
  local hx, hz = unit_position(hero)
  local tx, tz = fx(x), fx(z)
  if target ~= nil and target >= 0 then tx, tz = unit_position(target) end
  local dx, dz = tx - hx, tz - hz
  local limit = config.milli(reach)
  return dx * dx + dz * dz > limit * limit
end

local function cast_key(who, slot, wx, wz)
  local hero = caster_of(who)
  if hero == nil then return refuse(who, "noHero") end
  local a = ability_of(hero, slot)
  if a == nil then return refuse(who, "noAbility") end
  if game.unit_cooldown(hero, slot) > 0 then return refuse(who, "cooldown") end
  local info = ability_info(a.name)
  if info.target == TARGET_UNIT then
    return arm(who, { item = false, slot = slot, kind = "unit", radius = 0 })
  end
  if info.target == TARGET_POINT then
    return arm(who, { item = false, slot = slot, kind = "point", radius = 0 })
  end
  if info.target == TARGET_UNIT_OR_POINT then
    return arm(who, { item = false, slot = slot, kind = "unit_or_point", radius = 0 })
  end
  if wx == nil then
    local ux, uz = unit_position(hero)
    cast_now(hero, slot, -1, ux, uz)
    return
  end
  local hit = pick(wx, wz)
  if out_of_range(hero, a.name, wx, wz, hit and hit.id or -1) then return refuse(who, "range") end
  cast_now(hero, slot, hit and hit.id or -1, wx, wz)
end

local function item_key(who, slot)
  local hero = hero_selected(who)
  if hero == nil then return refuse(who, "noHero") end
  for _, it in ipairs(game.unit_items(hero)) do
    if it.slot == slot then
      local ux, uz = unit_position(hero)
      game.cmd_use_item(hero, slot, -1, ux, uz)
      return
    end
  end
  refuse(who, "noItem")
end

local function resolve_armed(who, x, z)
  local s = st(who)
  local hero = caster_of(who)
  if hero == nil then
    disarm(who)
    return refuse(who, "noHero")
  end
  local hit = pick(x, z)
  if s.armed.kind == "unit" and hit == nil then return refuse(who, "noTarget") end
  local a = s.armed
  disarm(who)
  if a.item then
    game.cmd_use_item(hero, a.slot, hit and hit.id or -1, fx(x), fx(z))
    return
  end
  if ability_of(hero, a.slot) == nil then return refuse(who, "noAbility") end
  local named = ability_of(hero, a.slot)
  if out_of_range(hero, named.name, x, z, hit and hit.id or -1) then return refuse(who, "range") end
  cast_now(hero, a.slot, hit and hit.id or -1, x, z)
end

local function skill_up(who, slot)
  local hero = hero_selected(who)
  if hero == nil then return refuse(who, "noHero") end
  game.cmd_skill_up(hero, slot)
end

local function recall(who, slot)
  local s = st(who)
  local now = game.time()
  local ids = s.groups[slot] or {}
  local previous = s.recalled[slot]
  s.recalled[slot] = now
  local alive = {}
  for _, id in ipairs(ids) do
    if units.own(id, who) then alive[#alive + 1] = id end
  end
  set_selection(who, alive)
  if previous ~= nil and now - previous <= CFG.doubleTapTicks and alive[1] ~= nil then
    centre_on(who, alive[1])
  end
end

local function index_of(list, key)
  for i, k in ipairs(list or {}) do
    if k == key then return i - 1 end
  end
  return nil
end

local function contains(list, key) return index_of(list, key) ~= nil end

local function resolve_key(key, mods, hero)
  local custom = CFG.custom[key]
  if custom ~= nil then return "custom", custom end
  if hero ~= nil then
    local own = hotkeys.explicit_slot(hero, key)
    if own ~= nil then
      if mods[(CFG.keys.skillUp or {}).modifier] then return "skillUp", own end
      return "cast", own
    end
  end
  local keys = CFG.keys
  local group = keys.group or {}
  local slot = index_of(group.keys, key)
  if slot ~= nil then
    if mods[group.modifier] then return "setGroup", slot end
    return "recallGroup", slot
  end
  if contains(keys.attackMove, key) then return "attackMove" end
  if contains(keys.stop, key) then return "stop" end
  if contains(keys.hold, key) then return "hold" end
  if contains(keys.centre, key) then return "centre" end
  if contains(keys.selectHero, key) then return "selectHero" end
  if contains(keys.selectCourier, key) then return "selectCourier" end
  if contains(keys.patrol, key) then return "patrol" end
  if contains(keys.glyph, key) then return "glyph" end
  if contains(keys.move, key) and not mods.shift then return "moveCmd" end
  if contains(keys.scoreboard, key) then return "scoreboard" end
  if contains(keys.shop, key) then return "shop" end
  local item = index_of(keys.item, key)
  if item ~= nil then return "item", item end
  local pad = index_of(keys.itemPad, key)
  if pad ~= nil then return "item", pad end
  local skill = keys.skillUp or {}
  if skill.keys ~= nil and skill.keys[key] ~= nil and mods[skill.modifier] then
    if hero ~= nil and hotkeys.claimed(hero, skill.keys[key]) then return nil end
    return "skillUp", skill.keys[key]
  end
  if keys.cast ~= nil and keys.cast[key] ~= nil then
    if hero ~= nil and hotkeys.claimed(hero, keys.cast[key]) then return nil end
    return "cast", keys.cast[key]
  end
  return nil
end

local function courier_action(key)
  for action, list in pairs(CFG.keys.courier or {}) do
    if contains(list, key) then return action end
  end
  return nil
end

local function courier_key(who, key)
  if hero_selected(who) ~= nil then return false end
  local id = courier.first(my_selected(who))
  local action = courier_action(key)
  if id == nil or action == nil then return false end
  local ok, reason = courier.command(who, id, action)
  if not ok then refuse(who, reason) end
  return true
end

local function on_key(who, a)
  local s = st(who)
  s.held.shift, s.held.ctrl, s.held.alt = a.shift, a.ctrl or a.meta, a.alt
  if a.phase == "up" then return end
  if a.key == "escape" then
    s.attack_next, s.move_next, s.patrol_next = false, false, false
  end
  if s.armed ~= nil and a.key == "escape" then
    disarm(who)
    return
  end
  if contains(CFG.keys.courierHome, a.key) and courier.send_home(my_selected(who), hero_selected(who)) then return end
  if courier_key(who, a.key) then return end
  local action, arg = resolve_key(a.key, { ctrl = s.held.ctrl, shift = s.held.shift, alt = s.held.alt }, hero_selected(who))
  if action == nil then return end
  local wx, wz = nil, nil
  if a.world ~= nil then wx, wz = a.world.x, a.world.z end
  if s.armed ~= nil and (action == "cast" or action == "item") and arg == s.armed.slot and (action == "item") == s.armed.item then
    disarm(who)
    return
  end
  if action == "attackMove" then
    s.attack_next = true
  elseif action == "moveCmd" then
    s.move_next = true
  elseif action == "patrol" then
    s.patrol_next = true
  elseif action == "selectHero" then
    select_hero(who)
  elseif action == "selectCourier" then
    select_courier(who)
  elseif action == "stop" then
    stop_selected(who)
  elseif action == "hold" then
    hold_selected(who)
  elseif action == "glyph" then
    local ok, reason = glyph.activate(who)
    if not ok then refuse(who, reason) end
  elseif action == "centre" then
    centre_selected(who)
  elseif action == "cast" then
    cast_key(who, arg, wx, wz)
  elseif action == "item" then
    item_key(who, arg)
  elseif action == "skillUp" then
    skill_up(who, arg)
  elseif action == "setGroup" then
    s.groups[arg] = my_selected(who)
  elseif action == "recallGroup" then
    recall(who, arg)
  elseif action == "custom" then
    game.ui_event("action", '{"player":' .. tostring(who) .. ',"action":"custom","name":' .. json_str(arg) .. "}")
  else
    game.ui_event("action", '{"player":' .. tostring(who) .. ',"action":' .. json_str(action) .. "}")
  end
end

local function dist2(ax, az, bx, bz)
  local dx, dz = ax - bx, az - bz
  return dx * dx + dz * dz
end

local function on_ground(who, kind, x, y, button)
  local s = st(who)
  if kind == "down" then
    s.press = { x = x, z = y, button = button, shift = s.held.shift }
    return
  end
  if kind ~= "up" or s.press == nil then return end
  local p = s.press
  s.press = nil
  local moved = dist2(p.x, p.z, x, y) > CFG.dragSlop * CFG.dragSlop
  if p.button == 2 then
    if moved then return end
    if s.armed ~= nil then
      disarm(who)
      return
    end
    s.attack_next, s.move_next, s.patrol_next = false, false, false
    local hit = pick(x, y)
    order_selected(who, x, y, is_enemy(who, hit) and hit or nil, false, p.shift)
    return
  end
  if p.button ~= 0 then return end
  if moved then
    set_selection(who, box_ids(who, p.x, p.z, x, y, p.shift))
    return
  end
  if s.armed ~= nil then
    resolve_armed(who, x, y)
    return
  end
  local hit = pick(x, y)
  local double = is_double_click(who, hit)
  if s.move_next then
    s.move_next = false
    order_selected(who, x, y, nil, false, p.shift)
    return
  end
  if s.patrol_next then
    s.patrol_next = false
    patrol_selected(who, x, y)
    return
  end
  if s.attack_next then
    s.attack_next = false
    order_selected(who, x, y, hit, true, p.shift)
    return
  end
  if double and not p.shift then
    set_selection(who, same_kind_on_screen(who, hit.id))
    return
  end
  set_selection(who, click_ids(who, hit, p.shift))
end

local function setup(args)
  local ui = args.ui or {}
  CFG.keys = ui.keys or CFG.keys
  CFG.refusals = (ui.text or {}).refusals or CFG.refusals
  CFG.doubleTapTicks = math.floor((ui.doubleTapMs or 350) * TICKS_PER_SECOND / 1000)
  CFG.dragSlop = ui.dragSlopTiles or CFG.dragSlop
  CFG.pickRadius = ui.pickRadius or CFG.pickRadius
  CFG.formationSpacing = ui.formationSpacing or CFG.formationSpacing
  CFG.centreSeconds = ui.centreSeconds or CFG.centreSeconds
  LOCAL = args.player or LOCAL
end

local ACTIONS = { "setup", "key", "pointer", "select", "ui", "skill_up", "seat_pick", "buyback", "bind", "camera_to", "view", "viewport", "debug_actions" }

local function on_action(name, args)
  if name == "debug_actions" then
    local out = "["
    for i, action in ipairs(ACTIONS) do
      if i > 1 then out = out .. "," end
      out = out .. json_str(action)
    end
    game.ui_event("debug", '{"actions":' .. out .. "]}")
  elseif name == "setup" then
    setup(args)
  elseif name == "key" then
    on_key(args.player or LOCAL, args)
  elseif name == "pointer" then
    on_ground(args.player or LOCAL, args.kind, args.x, args.y, args.button)
  elseif name == "select" then
    set_selection(args.player or LOCAL, args.ids or {})
  elseif name == "ui" then
    if args.call == "select" and args.args ~= nil then set_selection(args.player or LOCAL, args.args.ids or {}) end
    if args.call == "cast" and args.args ~= nil and args.args.slot ~= nil then cast_key(args.player or LOCAL, args.args.slot) end
  elseif name == "viewport" then
    if args.x0 and args.z0 and args.x1 and args.z1 then
      st(args.player or LOCAL).view = { x0 = args.x0, z0 = args.z0, x1 = args.x1, z1 = args.z1 }
    end
  elseif name == "skill_up" then
    skill_up(args.player or LOCAL, args.slot)
  elseif name == "seat_pick" then
    heroes.seat_pick(args.player, args.hero)
    if args.bot then bots.seat(args.player) end
  elseif name == "buyback" then
    heroes.buyback(args.player)
  elseif name == "bind" then
    if args.remove then CFG.custom[args.key] = nil else CFG.custom[args.key] = args.action end
  elseif name == "camera_to" then
    game.camera(args.player or LOCAL, fx(args.x), fx(args.z), 1, CFG.centreSeconds)
  elseif name == "view" then
    FOG_ON = args.fog ~= false
    game.ui_event("view", '{"player":' .. tostring(LOCAL) .. ',"fog":' .. (FOG_ON and "true" or "false") ..
      ',"centreSeconds":' .. tostring(CFG.centreSeconds) .. "}")
  end
end

local function on_pointer(kind, x, y, button)
  if kind == "move" then return end
  on_ground(LOCAL, kind, x, y, button)
end

return { on_action = on_action, on_pointer = on_pointer }

-- The moba controls, as mod code. The page forwards raw input and this file decides what it means.
--   on_action("setup", args)   the page's UI data (args.ui: moba_ui.json) and the local seat (args.player)
--   on_action("key", args)     a key event of the local seat: args = {key, phase = "down" | "up", shift, ctrl, alt,
--                              meta, world = {x, z}}. key is lowercase ("space" for the space bar); world is the
--                              ground under the pointer.
--   on_action("pointer", args) a ground event of any seat: args = {player, kind, x, y, button}
--   on_action("select", args)  the local seat's selection (args.ids), e.g. a click on a unit tile in the HUD
--   on_action("skill_up", a)   raise ability slot a.slot of the local seat's hero
--   on_action("bind", args)    a runtime key bind: args.key with args.action, or args.remove = true
--   on_action("camera_to", a)  glide the camera to the ground point (a.x, a.z), e.g. a click on the minimap
--   on_action("view", args)    the view toggles: args.fog (true or false); the page applies the reply
--   on_pointer(kind, x, y, button, state)  the local seat's ground event; kind "down", "up" or "move"; x, y in tiles
-- Every seat has its own selection, control groups, target arm and press. Feedback is sent out as UI events that
-- name their seat: game.ui_event("feedback", json) for markers, attack rings, the target arm, refusals and the
-- selection; game.ui_event("action", json) for panel actions (shop, scoreboard, custom); game.ui_event("view", json)
-- for the numbers the page applies. Commands are the engine's primitives: cmd_order, cmd_attack, cmd_cast,
-- cmd_use_item, cmd_hold, cmd_skill_up, attack_move, camera. The sandbox has no string or table library, so the
-- JSON below is built by hand.

local FIX = 65536
local TARGET_UNIT = 1
local TARGET_POINT = 2
local TICKS_PER_SECOND = 16

-- Defaults the page's ui data overrides in setup.
local CFG = {
  keys = {},
  custom = {},
  refusals = {},
  doubleTapTicks = 5,
  dragSlop = 0.25,
  pickRadius = 1.2,
  formationSpacing = 0.7,
  centreSeconds = 0.4,
}
local LOCAL = 0
local FOG_ON = true

-- Per-seat state: the selection, control groups, the target arm, the attack-move order, the press and the modifiers.
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

-- Feedback names its seat; the page shows it to that player.
local function feedback(who, body)
  game.ui_event("feedback", '{"player":' .. tostring(who) .. "," .. body .. "}")
end

local function own(who, id)
  return unit_alive(id) and unit_player(id) == who
end

-- The living own units of the selection, in selection order.
local function my_selected(who)
  local out = {}
  for _, id in ipairs(st(who).sel) do
    if own(who, id) then out[#out + 1] = id end
  end
  return out
end

local function hero_selected(who)
  for _, id in ipairs(my_selected(who)) do
    if game.is_hero(id) then return id end
  end
  return nil
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

-- The unit nearest the ground point (x, z) within the pick radius, as {id, player}; nil for empty ground.
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

-- A seat's own units in a ground box; a box holding any own hero keeps only the heroes.
local function box_ids(who, x0, z0, x1, z1, additive)
  local inside, heroes = {}, {}
  for _, id in ipairs(game.units(who)) do
    local ux, uz = unit_position(id)
    ux, uz = tile(ux), tile(uz)
    if ux >= lo(x0, x1) and ux <= hi(x0, x1) and uz >= lo(z0, z1) and uz <= hi(z0, z1) then
      inside[#inside + 1] = id
      if game.is_hero(id) then heroes[#heroes + 1] = id end
    end
  end
  local picked = #heroes > 0 and heroes or inside
  if not additive then return picked end
  local out = {}
  for _, id in ipairs(st(who).sel) do out[#out + 1] = id end
  for _, id in ipairs(picked) do out[#out + 1] = id end
  return out
end

-- One unit clicked: it becomes the selection (enemies too, to read their stats); shift toggles it; empty ground clears.
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

-- A ground order: a move, or an attack-move when attack is set. Clicking an enemy attacks it either way. Units keep
-- formation spots around the point, so they do not stack.
local function order_selected(who, x, z, enemy, attack)
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
    if enemy ~= nil then
      game.cmd_attack(id, enemy.id)
    else
      local sx = x + ((i - 1) % 3 - 1) * spacing
      local sz = z + math.floor((i - 1) / 3) * spacing
      if attack then
        game.attack_move(id, fx(sx), fx(sz))
      else
        game.cmd_order(id, fx(sx), fx(sz))
      end
    end
  end
  return true
end

local function stop_selected(who)
  for _, id in ipairs(my_selected(who)) do
    local ux, uz = unit_position(id)
    game.cmd_order(id, ux, uz)
  end
end

local function hold_selected(who)
  for _, id in ipairs(my_selected(who)) do game.cmd_hold(id) end
end

local function centre_on(who, id)
  local ux, uz = unit_position(id)
  game.camera(who, ux, uz, 1, CFG.centreSeconds)
end

-- Space: the camera glides to the hero, or to the first selected unit when no hero is selected.
local function centre_selected(who)
  local id = hero_selected(who)
  if id == nil then id = my_selected(who)[1] end
  if id ~= nil then centre_on(who, id) end
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

-- A cast key: a point or unit ability arms until the next left click; any other casts at once at the pointer.
local function cast_key(who, slot, wx, wz)
  local hero = hero_selected(who)
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
  local hit = pick(wx, wz)
  cast_now(hero, slot, hit and hit.id or -1, wx, wz)
end

-- An item key: items cast at once on the hero (their target data is not exported yet).
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

-- The left click that completes an armed cast; a unit-targeted spell waits while the click finds no unit.
local function resolve_armed(who, x, z)
  local s = st(who)
  local hero = hero_selected(who)
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
  cast_now(hero, a.slot, hit and hit.id or -1, x, z)
end

local function skill_up(who, slot)
  local hero = hero_selected(who)
  if hero ~= nil then game.cmd_skill_up(hero, slot) end
end

local function recall(who, slot)
  local s = st(who)
  local now = game.time()
  local ids = s.groups[slot] or {}
  local previous = s.recalled[slot]
  s.recalled[slot] = now
  local alive = {}
  for _, id in ipairs(ids) do
    if own(who, id) then alive[#alive + 1] = id end
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

-- The action a key means, and its argument (a slot, or a custom action name). Runtime binds win over the defaults.
local function resolve_key(key, mods)
  local custom = CFG.custom[key]
  if custom ~= nil then return "custom", custom end
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
  if contains(keys.scoreboard, key) then return "scoreboard" end
  if contains(keys.shop, key) then return "shop" end
  local item = index_of(keys.item, key)
  if item ~= nil then return "item", item end
  local skill = keys.skillUp or {}
  if skill.keys ~= nil and skill.keys[key] ~= nil and mods[skill.modifier] then
    return "skillUp", skill.keys[key]
  end
  if keys.cast ~= nil and keys.cast[key] ~= nil then return "cast", keys.cast[key] end
  return nil
end

local function on_key(who, a)
  local s = st(who)
  s.held.shift, s.held.ctrl, s.held.alt = a.shift, a.ctrl or a.meta, a.alt
  if a.phase == "up" then return end
  if s.armed ~= nil and a.key == "escape" then
    disarm(who)
    return
  end
  local action, arg = resolve_key(a.key, { ctrl = s.held.ctrl, shift = s.held.shift, alt = s.held.alt })
  if action == nil then return end
  local wx, wz = a.world.x, a.world.z
  if s.armed ~= nil and (action == "cast" or action == "item") and arg == s.armed.slot and (action == "item") == s.armed.item then
    disarm(who)
    return
  end
  if action == "attackMove" then
    s.attack_next = true
  elseif action == "stop" then
    stop_selected(who)
  elseif action == "hold" then
    hold_selected(who)
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

-- A ground press and its release. A drag that moved is a box select (left) or nothing (right: the camera orbits).
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
    s.attack_next = false
    local hit = pick(x, y)
    order_selected(who, x, y, is_enemy(who, hit) and hit or nil, false)
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
  if s.attack_next then
    s.attack_next = false
    order_selected(who, x, y, is_enemy(who, hit) and hit or nil, true)
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

function on_action(name, args, state)
  if name == "setup" then
    setup(args)
  elseif name == "key" then
    on_key(args.player or LOCAL, args)
  elseif name == "pointer" then
    on_ground(args.player, args.kind, args.x, args.y, args.button)
  elseif name == "select" then
    set_selection(args.player or LOCAL, args.ids or {})
  elseif name == "skill_up" then
    skill_up(args.player or LOCAL, args.slot)
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

function on_pointer(kind, x, y, button, state)
  if kind == "move" then return end
  on_ground(LOCAL, kind, x, y, button)
end

-- Transports as a mod. A ground unit boards a transport up to its capacity in size units and leaves it at a
-- point. A boarded unit is taken out of play (game.hide_unit) and kept here, per transport, until it unloads.
-- Nothing runs per tick: the work happens when a load or unload action arrives. Not loaded by any scenario.
--
-- Rules:
--   * size units: small 1, medium 2, large 4 (the unit's UEXT size 0, 1, 2).
--   * load: the unit and the transport are alive and different; the transport has capacity; the unit is a
--     ground unit of the same player; the unit is beside the transport (both radii plus half a world unit of
--     slack). The cargo plus the unit must fit the capacity. Refusals change nothing.
--   * unload: each boarded unit comes back alive at the point, clamped to the grid. A point a boarded ground
--     unit cannot stand on is refused (InvalidOrderTarget). unload_transport puts the cargo at the transport.

local SIZE_UNITS = { 1, 2, 4 }
local ARRIVE_SLACK = 32768 -- half a world unit, Q16.16, the sim's arrival slack

local info_cache = {}      -- unit kind name -> game.unit_info row, or false
local cargo = {}           -- transport id -> boarded unit ids, in boarding order

local function info_of(u)
  local kind = game.unit_kind(u)
  local row = info_cache[kind]
  if row == nil then
    row = game.unit_info(kind) or false
    info_cache[kind] = row
  end
  return row or nil
end

local function size_of(u)
  return SIZE_UNITS[info_of(u).size + 1] or 4
end

local function arrived(i, t)
  local reach = info_of(i).radius_q16 + info_of(t).radius_q16 + ARRIVE_SLACK
  local ix, iy = unit_position(i)
  local tx, ty = unit_position(t)
  local dx = ix - tx
  local dy = iy - ty
  return dx * dx + dy * dy <= reach * reach
end

local function cargo_of(t)
  local list = cargo[t]
  if list == nil then
    list = {}
    cargo[t] = list
  end
  return list
end

-- Boards ground unit i into transport t. Returns nil, or the refusal name and changes nothing.
function load_unit(t, i)
  if t == i or not game.alive(t) or not game.alive(i) then
    return "NoTarget"
  end
  local ti = info_of(t)
  local ii = info_of(i)
  if ti.transport <= 0 or ii.flying or unit_player(i) ~= unit_player(t) or not arrived(i, t) then
    return "InvalidOrderTarget"
  end
  local list = cargo_of(t)
  local used = 0
  for _, u in ipairs(list) do
    used = used + size_of(u)
  end
  if used + size_of(i) > ti.transport then
    return "TransportFull"
  end
  if not game.hide_unit(i) then
    return "NoTarget"
  end
  list[#list + 1] = i
  return nil
end

-- Unloads every unit boarded in transport t at the point (x, y), clamped to the grid. Returns the count, or
-- nil and the refusal name when t is not alive or a boarded ground unit cannot stand on the point.
function unload_transport_at(t, x, y)
  if not game.alive(t) then
    return nil, "NoTarget"
  end
  local map_w, map_h = game.map_size()
  if x < 0 then
    x = 0
  end
  if x > map_w - 65536 then
    x = map_w - 65536
  end
  if y < 0 then
    y = 0
  end
  if y > map_h - 65536 then
    y = map_h - 65536
  end
  local list = cargo_of(t)
  if #list > 0 and not game.walkable(x, y) then
    for _, u in ipairs(list) do
      if not info_of(u).flying then
        return nil, "InvalidOrderTarget"
      end
    end
  end
  for _, u in ipairs(list) do
    game.unhide_unit(u, x, y)
  end
  local count = #list
  cargo[t] = {}
  return count
end

-- Unloads the cargo of transport t at the transport's own position.
function unload_transport(t)
  if not game.alive(t) then
    return nil, "NoTarget"
  end
  local x, y = unit_position(t)
  return unload_transport_at(t, x, y)
end

function on_action(name, args)
  if name == "load" then
    load_unit(args.transport, args.unit)
  elseif name == "unload" then
    if args.x ~= nil and args.y ~= nil then
      unload_transport_at(args.transport, args.x, args.y)
    else
      unload_transport(args.transport)
    end
  end
end

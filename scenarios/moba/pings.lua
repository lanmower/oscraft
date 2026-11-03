local config = require("config")
local units = require("units")

local FIX = config.FIX
local TICKS_PER_SECOND = config.TICKS_PER_SECOND

local KINDS = {
  ping = { colour = "#ffffff", seconds = 4 },
  danger = { colour = "#ff4d4d", seconds = 4 },
  on_my_way = { colour = "#7cc7ff", seconds = 4, route = true },
  item = { colour = "#ffd54f", seconds = 4 },
}

local pings = {}

local function to_allies(of_player, body)
  for p = 0, config.PLAYERS - 1 do
    if game.seat_start(p) ~= nil and not units.hostile(p, of_player) then
      game.ui_event("ping", '{"player":' .. tostring(p) .. "," .. body .. "}")
    end
  end
end

local function send(who, kind, x, z, sender, text)
  local spec = KINDS[kind]
  local body = '"kind":"' .. kind .. '","x":' .. tostring(x) .. ',"z":' .. tostring(z)
    .. ',"seconds":' .. tostring(spec.seconds) .. ',"colour":"' .. spec.colour .. '"'
  if spec.route and sender ~= nil then
    local sx, sz = unit_position(sender)
    body = body .. ',"from":{"x":' .. tostring(sx / FIX) .. ',"z":' .. tostring(sz / FIX) .. '}'
  end
  if text ~= nil then body = body .. ',"text":"' .. text .. '"' end
  to_allies(who, body)
end

function pings.known(kind)
  return KINDS[kind] ~= nil
end

function pings.at(who, kind, x, z, sender)
  send(who, kind, x, z, sender, nil)
end

local function item_status(hero, slot)
  for _, row in ipairs(game.unit_items(hero)) do
    if row.slot == slot then
      if row.ability == nil then return row.name .. ": passive" end
      if row.cooldown > 0 then
        return row.name .. ": " .. tostring(math.ceil(row.cooldown / TICKS_PER_SECOND)) .. "s cooldown"
      end
      local short = row.ability.energy - game.mana(hero)
      if short > 0 then return row.name .. ": " .. tostring(short) .. " mana short" end
      return row.name .. ": ready"
    end
  end
  return nil
end

function pings.item(who, hero, slot)
  local text = item_status(hero, slot)
  if text == nil then return end
  local hx, hz = unit_position(hero)
  send(who, "item", hx / FIX, hz / FIX, nil, text)
end

return pings

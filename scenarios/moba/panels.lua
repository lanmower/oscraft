-- The moba panels, as mod code. Each panel is a widget tree (JSON, below) that the page draws with its generic widgets;
-- this file decides what every panel shows and what its buttons do. The page only draws the trees and sends a button
-- press back as the "ui" action. The panels refresh on a periodic timer (game.timer).
--   on_action(name, args)  "setup" (args.ui: the parsed moba_ui.json object, with text, keys and teams; args.player the local seat; args.pubkey
--                                 the lobby owner of this page), "key" (the shop and scoreboard keys toggle their panels),
--                                 "selection" (the local seat's selection, args.ids, sent by the page on every change),
--                                 "ui" (a button press: args.call names the call, args.args holds its values)
-- A panel goes out as game.ui_event("widget", {player, id, slot, tree}); a closed one as {player, id, remove}. The
-- lobby's start goes out as game.ui_event("lobby", {player, started, slots}) and the page starts the match from it.
-- A node is {"t": "panel"|"box"|"label"|"bar"|"button"|"grid"|"cell"|"image", ...}: panel (title, kids), box (dir "row" or "col",
-- kids), label (text, tone), bar (value, max, tone), button (text, call, args, disabled), grid (cols, kids), cell (kids,
-- tone, tip, call, args), image (src, size, alt). Tones are named by the page's stylesheet. The sandbox has no string library, so the JSON is
-- built by hand.

local config = require("config")
local units = require("units")
local xp = require("xp")

local TICKS_PER_SECOND = config.TICKS_PER_SECOND
local REFRESH_TICKS = 4
-- Six inventory slots, the DotA 1 count verified against the 6.88g map. There is no backpack: that is Dota 2
-- 7.00. Extras go to the stash or a courier, so anything beyond these six is not a carried slot.
local INVENTORY_SLOTS = 6
local STASH_SLOTS = 9
-- Inventory slot 9 is the neutral slot: a recipe component is never held there (store.lua, sim inventory.rs).
local NEUTRAL_SLOT = 9
local ABILITY_SLOTS = 8
local TEAM_SIZE = 5
-- Lobby seats (5 a side), not inventory: a slot here holds a player or a bot and its chosen hero.
local SLOT_COUNT = 10
local GROUP_TILES = 12
local TALENT_FIRST_BIT = 2
local TALENT_LEVELS = { 10, 15, 20, 25 }

local TEXT = {}
local UI = {}
local KEYS = {}
local LOCAL = 0
local OWNER = "local"
local SEL = {}
local OPEN = { shop = false, scoreboard = false }
local HORN = nil
local PEAK = {}
-- The longest cooldown seen per unit and slot: the sweep's full length for an ability with no rank table.
local COOLDOWN_FULL = {}
local HEROES = {}
local ROSTER = nil
local LAST = {}

local function new_lobby()
  local slots = {}
  for i = 1, SLOT_COUNT do slots[i] = { owner = nil, hero = nil, ready = false, bot = false } end
  return { host = nil, started = false, slots = slots, error = "" }
end
local LOBBY = new_lobby()

-- JSON pieces. A field is '"k":v' with v already encoded.
local function join(items, sep)
  local out = ""
  for i, v in ipairs(items) do
    if i > 1 then out = out .. sep end
    out = out .. v
  end
  return out
end
local function S(k, v) return '"' .. k .. '":"' .. tostring(v) .. '"' end
local function I(k, v) return '"' .. k .. '":' .. tostring(v) end
local function B(k, v) return '"' .. k .. '":' .. (v and "true" or "false") end
local function R(k, v) return '"' .. k .. '":' .. v end
local function arr(items) return "[" .. join(items, ",") .. "]" end
local function obj(items) return "{" .. join(items, ",") .. "}" end
local function with(items, extra)
  if extra ~= nil then items[#items + 1] = extra end
  return items
end

local function node(t, fields)
  local f = { S("t", t) }
  for _, v in ipairs(fields) do f[#f + 1] = v end
  return obj(f)
end
local function box(kids, dir) return node("box", { S("dir", dir or "col"), R("kids", arr(kids)) }) end
local function label(text, tone)
  return node("label", with({ S("text", text) }, tone and S("tone", tone) or nil))
end
local function bar(value, max, tone)
  return node("bar", with({ I("value", value), I("max", max) }, tone and S("tone", tone) or nil))
end
local function button(text, call, args, disabled, tip)
  local f = { S("text", text), S("call", call), R("args", args or "{}") }
  if disabled then f[#f + 1] = B("disabled", true) end
  if tip then f[#f + 1] = S("tip", tip) end
  return node("button", f)
end
local function grid(cols, kids) return node("grid", { I("cols", cols), R("kids", arr(kids)) }) end
-- A pack image. The page resolves the src and drops the node when the file is not there, so an icon
-- that has not been generated yet costs nothing and never shows a broken frame.
local function image(src, size, alt)
  return node("image", with({ S("src", src), I("size", size or 64) }, alt and S("alt", alt) or nil))
end

-- Hero name -> the model it draws with, so a panel can name the icon that goes with it: the icons are
-- authored per model (icons/heroes/<model>.png), not per hero name.
local HERO_MODEL = nil
local function hero_model(name)
  if HERO_MODEL == nil then
    HERO_MODEL = {}
    for _, k in ipairs(game.unit_kinds()) do
      if k.player_controlled and k.model ~= "" then HERO_MODEL[k.name] = k.model end
    end
  end
  return HERO_MODEL[name]
end

-- The hero's icon, or nil when this hero has no model and so no icon was authored for it.
local function hero_icon(name, size)
  local model = hero_model(name)
  if model == nil then return nil end
  return image("icons/heroes/" .. model .. ".png", size or 48, name)
end

local function cell(kids, opts)
  opts = opts or {}
  local f = {}
  if opts.tone then f[#f + 1] = S("tone", opts.tone) end
  if opts.tip then f[#f + 1] = S("tip", opts.tip) end
  if opts.call then
    f[#f + 1] = S("call", opts.call)
    f[#f + 1] = R("args", opts.args or "{}")
  end
  f[#f + 1] = R("kids", arr(kids))
  return node("cell", f)
end
local function panel(title, kids) return node("panel", { S("title", title), R("kids", arr(kids)) }) end

-- A widget event for one panel: the tree goes out when it differs from the last one sent. nil tree closes the panel.
local function emit(id, slot, tree)
  local key = tree or "-"
  if (LAST[id] or "-") == key then return end
  LAST[id] = key
  if tree == nil then
    game.ui_event("widget", obj({ I("player", LOCAL), S("id", id), B("remove", true) }))
  else
    game.ui_event("widget", obj({ I("player", LOCAL), S("id", id), S("slot", slot), R("tree", tree) }))
  end
end

local function team_of(p) return UI.teams[p + 1] or 0 end

local function sides()
  local seen, out = {}, {}
  for _, t in ipairs(UI.teams) do
    if not seen[t] then
      seen[t] = true
      out[#out + 1] = t
    end
  end
  return out
end

local function alive_selection()
  local out = {}
  for _, id in ipairs(SEL) do
    if unit_alive(id) then out[#out + 1] = id end
  end
  return out
end

-- The first selected unit of seat `who` that is one of its heroes, or nil.
local function hero_of(who)
  return units.hero_of(SEL, who)
end

-- The first living hero of player p, or nil.
local function hero_unit(p)
  for _, id in ipairs(game.units(p)) do
    if units.is_hero(id) then return id end
  end
  return nil
end

-- Hero rows for the team strip: one per player slot, from the first hero the sim lists for it. A slot whose hero is gone
-- reads dead until the horn starts the next roster.
local function update_heroes()
  local living = {}
  for _, id in ipairs(game.units()) do
    if units.is_hero(id) then
      local p = unit_player(id)
      if living[p] == nil then living[p] = id end
    end
  end
  for p = 0, #UI.teams - 1 do
    local id = living[p]
    if id ~= nil then
      HEROES[p] = { id = id, state = "alive", hp = unit_hp(id) }
    elseif HEROES[p] ~= nil then
      HEROES[p].state = "dead"
    end
  end
end

local function shr(v, n)
  for _ = 1, n do v = math.floor(v / 2) end
  return v
end

local function pips(rank, max)
  local out = ""
  for i = 1, max do
    if i <= rank then out = out .. "●" else out = out .. "○" end
  end
  return out
end

local function clock_text()
  if HORN == nil then return TEXT.preMatch end
  local ticks = game.time() - HORN
  local total = math.floor(ticks / TICKS_PER_SECOND)
  local minutes = math.floor(total / 60)
  local secs = total - minutes * 60
  if secs < 10 then secs = "0" .. tostring(secs) end
  return tostring(minutes) .. ":" .. tostring(secs)
end

-- The HUD line: the primary selected unit's hp and the local seat's gold. The match clock is on the top strip (team_tree).
local function hud_tree()
  local primary = alive_selection()[1]
  local hp = nil
  if primary ~= nil then hp = unit_hp(primary) end
  local hp_text = TEXT.noValue
  if hp ~= nil and hp >= 0 then hp_text = tostring(math.floor(hp / config.FIX)) end
  return box({
    label(TEXT.hud.hp .. " " .. hp_text),
    label(TEXT.hud.gold .. " " .. tostring(game.gold(LOCAL))),
  }, "row")
end

-- The post-game rows, one per seat that has a start: kills, deaths, assists (the unit credits), gold (the seat's
-- resource A), xp and the item kinds held, each summed over the seat's unit slots. A dead unit keeps its tallies until
-- its slot respawns, so every unit slot counts. The item list is read only when with_items is set.
local function post_game_rows(with_items)
  local rows, by_player = {}, {}
  for p = 0, config.PLAYERS - 1 do
    if game.seat_start(p) ~= nil then
      local row = { player = p, kills = 0, deaths = 0, assists = 0, gold = game.gold(p), xp = 0, items = {} }
      rows[#rows + 1] = row
      by_player[p] = row
    end
  end
  for u = 0, get_unit_count() - 1 do
    local row = by_player[unit_player(u)]
    if row ~= nil then
      row.kills = row.kills + game.kills_of(u)
      row.deaths = row.deaths + game.deaths_of(u)
      row.assists = row.assists + game.credits_of(u)
      row.xp = row.xp + game.xp(u)
      if with_items then
        for _, it in ipairs(game.unit_items(u)) do row.items[#row.items + 1] = it.item end
      end
    end
  end
  return rows
end

local function win_tree()
  local side = game.winner()
  if side == nil or side < 0 then return nil end
  local kids = { label(TEXT.winner.prefix .. " " .. tostring(side) .. " " .. TEXT.winner.suffix, "accent") }
  for _, row in ipairs(post_game_rows(true)) do
    local items = #row.items
    kids[#kids + 1] = label("P" .. tostring(row.player) .. "  " .. TEXT.post.kills .. " " .. tostring(row.kills) ..
      "  " .. TEXT.post.deaths .. " " .. tostring(row.deaths) .. "  " .. TEXT.post.assists .. " " .. tostring(row.assists) ..
      "  " .. TEXT.post.gold .. " " .. tostring(row.gold) .. "  " .. TEXT.post.xp .. " " .. tostring(row.xp) ..
      "  " .. TEXT.post.items .. " " .. tostring(items))
  end
  return box(kids, "col")
end

-- The top strip, Dota's format: each side's kills with the match clock between them. Heroes and seat gold are on the
-- Tab scoreboard (scoreboard_tree).
local function team_tree()
  local kills = {}
  for _, row in ipairs(post_game_rows(false)) do
    local side = team_of(row.player)
    kills[side] = (kills[side] or 0) + row.kills
  end
  local kids = {}
  for i, side in ipairs(sides()) do
    if i == 2 then kids[#kids + 1] = label(clock_text(), "muted") end
    kids[#kids + 1] = label(TEXT.killsLabel .. " " .. tostring(kills[side] or 0), "team" .. tostring(side))
  end
  return box(kids, "row")
end

-- The shop tags of an item: the bit of each name in UI.shopNames (items.osd's item_tags order), read by division.
local function shop_tags(tags)
  local names, bit = {}, 1
  for _, name in ipairs(UI.shopNames) do
    if math.floor(tags / bit) % 2 == 1 then names[#names + 1] = name end
    bit = bit * 2
  end
  return join(names, "/")
end

local function stats_text(stats)
  local parts = {}
  for name, v in pairs(stats) do
    local sign = v > 0 and "+" or ""
    parts[#parts + 1] = sign .. tostring(v) .. " " .. name
  end
  return join(parts, " ")
end

local function recipe_text(components)
  local names = {}
  for _, c in ipairs(components) do
    local part = game.item_info(c)
    names[#names + 1] = part and part.name or ("#" .. tostring(c))
  end
  return join(names, " + ")
end

-- The price the engine charges this hero for an item (store.lua acquire): the cost less the components its held copies
-- cover, each held copy claimed once and a missing component's own parts claimed in its place. The buy button and the
-- purchase read the same price, so the button is enabled exactly when the purchase is accepted.
local function net_price(unit, item, def)
  local pool = {}
  for _, row in ipairs(game.unit_items(unit)) do
    if row.slot ~= NEUTRAL_SLOT then pool[row.item] = (pool[row.item] or 0) + row.count end
  end
  for _, row in ipairs(game.unit_stash(unit)) do
    pool[row.item] = (pool[row.item] or 0) + row.count
  end
  local function claim(c)
    local saved = 0
    for _, part in ipairs(game.item_info(c).components) do
      if (pool[part] or 0) > 0 then
        pool[part] = pool[part] - 1
        saved = saved + game.item_info(part).cost
      else
        saved = saved + claim(part)
      end
    end
    return saved
  end
  return def.cost - claim(item)
end

local function shop_tree()
  local hero = hero_of(LOCAL)
  local gold = game.gold(LOCAL)
  local cards = {}
  local i = 0
  while true do
    local def = game.item_info(i)
    if def == nil then break end
    if def.tags ~= 0 then
      local price = hero ~= nil and net_price(hero, i, def) or def.cost
      local afford = gold >= price
      local kids = {
        label(def.name, "strong"),
        label(tostring(def.cost) .. TEXT.costSuffix, afford and "gold" or "bad"),
        label("[" .. shop_tags(def.tags) .. "]", "faint"),
      }
      local stats = stats_text(def.stats)
      if stats ~= "" then kids[#kids + 1] = label(stats, "muted") end
      if #def.components > 0 then kids[#kids + 1] = label(TEXT.recipePrefix .. recipe_text(def.components), "faint") end
      kids[#kids + 1] = button(TEXT.buy, "buy", obj({ I("item", i) }), hero == nil or not afford)
      cards[#cards + 1] = cell(kids, { tip = def.name })
    end
    i = i + 1
  end
  return panel(TEXT.shop.title .. "  " .. TEXT.goldLabel .. " " .. tostring(gold), { grid(2, cards) })
end

local function skills_tree(hero)
  local level = xp.character_level(hero)
  local spent = 0
  for s = 0, ABILITY_SLOTS - 1 do spent = spent + game.skill_rank(hero, s) end
  local points = level - spent
  local kids = { label(TEXT.skills.level .. " " .. tostring(level) .. "  " .. TEXT.skills.points .. " " .. tostring(points), "muted") }
  kids[#kids + 1] = label(TEXT.skills.abilities, "faint")
  for _, a in ipairs(game.unit_abilities(hero)) do
    local rank = game.skill_rank(hero, a.slot)
    local info = game.skill_info(hero, a.slot)
    local gate_text, ready, row_pips = TEXT.skills.none, false, ""
    if info ~= nil then
      row_pips = pips(rank, info.max_rank)
      if rank >= info.max_rank then
        gate_text = TEXT.skills.maxed
      else
        local gate = info.gates[rank + 1]
        gate_text = TEXT.skills.gate .. " " .. tostring(gate)
        ready = points > 0 and level >= gate
      end
    end
    kids[#kids + 1] = box({
      box({ label(a.name, "strong"), label(row_pips, "accent"), label(gate_text, "muted") }, "col"),
      button("+", "skill_up", obj({ I("slot", a.slot) }), not ready, TEXT.skills.up),
    }, "row")
  end
  kids[#kids + 1] = label(TEXT.skills.talents, "faint")
  local bits = game.upgrade_bits(hero)
  for tier = 1, #TALENT_LEVELS do
    local need = TALENT_LEVELS[tier]
    local picked = shr(bits, TALENT_FIRST_BIT + 2 * (tier - 1)) % 4
    local row = { label(TEXT.skills.needLevel .. " " .. tostring(need), level < need and "muted" or "accent") }
    for side = 0, 1 do
      local on = (side == 0 and picked % 2 == 1) or (side == 1 and picked >= 2)
      row[#row + 1] = button(on and "[x]" or "+", "talent",
        obj({ I("unit", hero), I("tier", tier - 1), I("side", side) }), picked ~= 0 or level < need, TEXT.skills.talent)
    end
    kids[#kids + 1] = box(row, "row")
  end
  return panel(TEXT.skills.title, kids)
end

-- Ability keys in slot order: {key, slot} from setup's cast map. The swap goes through a local, one assignment at a
-- time: the host's Lua VM evaluates a multiple assignment (a, b = b, a) with the first target already written, which
-- duplicated entries in the bar.
local function cast_keys()
  local list = {}
  for key, slot in pairs(KEYS.cast or {}) do list[#list + 1] = { key = key, slot = slot } end
  for i = 2, #list do
    local j = i
    while j > 1 and list[j - 1].slot > list[j].slot do
      local held = list[j - 1]
      list[j - 1] = list[j]
      list[j] = held
      j = j - 1
    end
  end
  return list
end

local function ability_cell(hero, key, abilities)
  local a = abilities[key.slot]
  if a == nil then return cell({ label(key.key, "faint") }, { tone = "empty" }) end
  local rank = game.skill_rank(hero, key.slot)
  local info = game.skill_info(hero, key.slot)
  local ranked = info ~= nil and info.max_rank > 0
  local kids = { label(key.key, "accent"), label(a.name, "strong") }
  if ranked then
    kids[#kids + 1] = label(tostring(info.energies[math.max(rank, 1)]), "mp")
    kids[#kids + 1] = label(pips(rank, info.max_rank), "muted")
  end
  local tone = nil
  if a.cooldown > 0 then
    local tenths = math.floor(a.cooldown * 10 / TICKS_PER_SECOND)
    kids[#kids + 1] = label(tostring(math.floor(tenths / 10)) .. "." .. tostring(tenths - 10 * math.floor(tenths / 10)) .. TEXT.cooldownSuffix, "warn")
    -- Cooldown sweep: the bar is the share of the cooldown still to run (ticks left over ticks in full).
    local k = tostring(hero) .. ":" .. tostring(key.slot)
    if a.cooldown > (COOLDOWN_FULL[k] or 0) then COOLDOWN_FULL[k] = a.cooldown end
    local full = COOLDOWN_FULL[k]
    if ranked and info.cooldowns[math.max(rank, 1)] ~= nil and info.cooldowns[math.max(rank, 1)] > 0 then full = info.cooldowns[math.max(rank, 1)] end
    kids[#kids + 1] = bar(math.min(a.cooldown, full), full)
    tone = "muted"
  elseif rank == 0 then
    tone = "faint"
  end
  return cell(kids, { tone = tone, tip = a.name })
end

local function select_tree()
  local sel = alive_selection()
  if #sel == 0 then
    return box({ cell({ label(TEXT.noValue, "faint") }, { tone = "empty" }), label(TEXT.noSelection, "muted") }, "row")
  end
  local primary = sel[1]
  local hero = units.is_hero(primary)
  local hp = unit_hp(primary)
  local peak = math.max(PEAK[primary] or 0, hp)
  PEAK[primary] = peak
  local name = game.unit_kind(primary) or TEXT.unitUnknown
  local face = TEXT.noValue
  if hero then face = "L" .. tostring(xp.character_level(primary)) end
  local info = {
    label(name, "accent"),
    label(TEXT.healthLabel .. " " .. tostring(math.floor(hp / config.FIX)) .. "/" .. tostring(math.floor(peak / config.FIX))),
    bar(math.max(hp, 0), peak, "hp"),
  }
  if hero then
    info[#info + 1] = label(TEXT.manaLabel .. " " .. tostring(game.max_mana(primary)))
    info[#info + 1] = bar(game.mana(primary), game.max_mana(primary), "mp")
  end
  local portrait = hero and hero_icon(name, 56) or nil
  local kids = { box({ cell(with({ label(face, "accent") }, portrait), { tone = "face" }), box(info, "col") }, "row") }
  if hero and unit_player(primary) == LOCAL then
    local abilities = {}
    for _, a in ipairs(game.unit_abilities(primary)) do abilities[a.slot] = a end
    local cells = {}
    for _, key in ipairs(cast_keys()) do cells[#cells + 1] = ability_cell(primary, key, abilities) end
    kids[#kids + 1] = box(cells, "row")
  end
  if #sel >= 2 then
    local tiles = {}
    for i = 1, math.min(#sel, GROUP_TILES) do
      local id = sel[i]
      local tile_hp = unit_hp(id)
      local tile_peak = math.max(PEAK[id] or 0, tile_hp)
      PEAK[id] = tile_peak
      tiles[#tiles + 1] = cell({ label(game.unit_kind(id) or TEXT.noValue, "muted"), bar(math.max(tile_hp, 0), tile_peak, "hp") },
        { tone = i == 1 and "active" or nil, call = "select", args = obj({ R("ids", arr({ tostring(id) })) }) })
    end
    kids[#kids + 1] = grid(6, tiles)
  end
  return box(kids, "col")
end

local function inventory_tree()
  local hero = hero_of(LOCAL)
  local items, stored = {}, {}
  if hero ~= nil then
    for _, row in ipairs(game.unit_items(hero)) do items[row.slot] = row.name end
    for _, row in ipairs(game.unit_stash(hero)) do stored[row.slot] = row.name end
  end
  local main, stash = {}, {}
  for s = 0, INVENTORY_SLOTS - 1 do
    local name = items[s]
    if name ~= nil then
      main[#main + 1] = cell({ label(name, "strong") }, { tip = name, call = "sell", args = obj({ I("slot", s) }) })
    else
      main[#main + 1] = cell({}, { tone = "empty" })
    end
  end
  for s = 0, STASH_SLOTS - 1 do
    local name = stored[s]
    if name ~= nil then
      stash[#stash + 1] = cell({ label(name, "strong") }, { tip = name, tone = "stash", call = "unstash", args = obj({ I("slot", s) }) })
    else
      stash[#stash + 1] = cell({}, { tone = "empty" })
    end
  end
  return box({
    box({ label(TEXT.inventoryLabel, "faint"), grid(3, main) }, "col"),
    box({ label(TEXT.stashLabel, "faint"), grid(3, stash) }, "col"),
  }, "row")
end

local inventory_cache = {}

-- The inventory tree is a function of the hero, its items and its stash: it is rebuilt only when the hero or the
-- hero's item generation moves.
local function inventory_tree_cached()
  local hero = hero_of(LOCAL)
  local gen = hero ~= nil and game.unit_item_gen(hero) or nil
  if gen ~= nil and inventory_cache.hero == hero and inventory_cache.gen == gen then return inventory_cache.tree end
  local tree = inventory_tree()
  inventory_cache = { hero = hero, gen = gen, tree = tree }
  return tree
end

local function scoreboard_tree()
  local rows = post_game_rows(false)
  local kids = {}
  for _, side in ipairs(sides()) do
    local cells = {}
    for _, h in ipairs(TEXT.scoreboardHeaders) do cells[#cells + 1] = label(h, "muted") end
    for _, row in ipairs(rows) do
      if team_of(row.player) == side then
        local me = row.player == LOCAL
        local hero = hero_unit(row.player)
        local name = TEXT.noValue
        local level = TEXT.noValue
        if hero ~= nil then
          name = game.unit_kind(hero) or TEXT.noValue
          level = tostring(xp.character_level(hero))
        end
        local tone = me and "me" or nil
        cells[#cells + 1] = label("P" .. tostring(row.player) .. (me and " *" or ""), tone)
        cells[#cells + 1] = label(name, tone)
        cells[#cells + 1] = label(level, tone)
        cells[#cells + 1] = label(tostring(row.kills), tone)
        cells[#cells + 1] = label(tostring(row.deaths), tone)
        cells[#cells + 1] = label(tostring(row.assists), tone)
        cells[#cells + 1] = label(tostring(row.gold), tone)
      end
    end
    kids[#kids + 1] = label(TEXT.teamLabel .. " " .. tostring(side), "team" .. tostring(side))
    kids[#kids + 1] = grid(7, cells)
  end
  return panel(TEXT.scoreboardTitle, kids)
end

-- The heroes a seat can pick: the player-controlled kinds that have a model. A hero with no model is not offered (it
-- would not draw), so the pick list holds only heroes that can play; the page still fails loudly on one that is drawn.
local function roster_names()
  local out = {}
  for _, k in ipairs(game.unit_kinds()) do
    if k.player_controlled and k.model ~= "" then out[#out + 1] = k.name end
  end
  return out
end

local function owner_text(s)
  if s.owner == nil then return TEXT.lobby.owners.open end
  if s.owner == OWNER then return TEXT.lobby.owners.you end
  if s.bot then return TEXT.lobby.owners.computer end
  return TEXT.lobby.owners.other
end

local function seated(owner)
  for _, s in ipairs(LOBBY.slots) do
    if s.owner == owner then return true end
  end
  return false
end

local function all_ready()
  for _, s in ipairs(LOBBY.slots) do
    if not (s.owner and s.hero and s.ready) then return false end
  end
  return true
end

local function any_open()
  for _, s in ipairs(LOBBY.slots) do
    if s.owner == nil then return true end
  end
  return false
end

local function held(hero)
  for _, s in ipairs(LOBBY.slots) do
    if s.hero == hero then return true end
  end
  return false
end

local function lobby_started_event()
  local items = {}
  for _, s in ipairs(LOBBY.slots) do
    items[#items + 1] = obj({ S("owner", s.owner or ""), S("hero", s.hero or "") })
  end
  game.ui_event("lobby", obj({ I("player", LOCAL), B("started", true), R("slots", arr(items)) }))
end

-- The seats before the match starts, after each lobby change: whether each seat is ready and which hero's model it
-- will play. The page parses a readied hero's model now, so the match starts without waiting on the download.
local function lobby_seats_event()
  local items = {}
  for _, s in ipairs(LOBBY.slots) do
    items[#items + 1] = obj({ S("owner", s.owner or ""), S("hero", s.hero or ""), B("ready", s.ready == true), S("model", s.hero and hero_model(s.hero) or "") })
  end
  game.ui_event("lobby", obj({ I("player", LOCAL), B("started", false), R("slots", arr(items)) }))
end

-- One lobby action on the shared state: the rules of the hero select. Slots are 0-based in the calls, as the sim counts.
local function lobby_act(call, a)
  local L = LOBBY
  -- The sender: a joined page's key rides on its action (applyPeerAction); the host's own presses use OWNER.
  local owner = a.pubkey or OWNER
  if L.started then
    L.error = "match-started"
    return
  end
  L.error = ""
  if call == "start" then
    if L.host ~= owner then L.error = "not-host" return end
    if not all_ready() then L.error = "not-ready" return end
    L.started = true
    HEROES = {}
    HORN = game.time()
    lobby_started_event()
    return
  end
  if call == "fill" then
    if L.host ~= owner then L.error = "not-host" return end
    if not any_open() then L.error = "nothing-to-fill" return end
    if #ROSTER == 0 then L.error = "no-roster" return end
    for i = 1, SLOT_COUNT do
      local s = L.slots[i]
      if s.owner == nil then
        local hero = ROSTER[1]
        for _, h in ipairs(ROSTER) do
          if not held(h) then
            hero = h
            break
          end
        end
        s.owner = "bot:" .. tostring(i - 1)
        s.bot = true
        s.hero = hero
        s.ready = true
      end
    end
    return
  end
  local i = (a.slot or -1) + 1
  if i < 1 or i > SLOT_COUNT then L.error = "bad-slot" return end
  local s = L.slots[i]
  if call == "claim" then
    if s.owner ~= nil then L.error = "slot-taken" return end
    if seated(owner) then L.error = "already-seated" return end
    s.owner = owner
    s.bot = false
    if L.host == nil then L.host = owner end
  elseif call == "release" then
    if s.owner ~= owner then L.error = "not-owner" return end
    s.owner, s.hero, s.ready, s.bot = nil, nil, false, false
    if L.host == owner then
      L.host = nil
      for _, other in ipairs(L.slots) do
        if other.owner ~= nil and not other.bot then
          L.host = other.owner
          break
        end
      end
    end
  elseif call == "pick" then
    if s.owner ~= owner then L.error = "not-owner" return end
    if s.ready then L.error = "ready-locked" return end
    local known = false
    for _, h in ipairs(ROSTER) do
      if h == a.hero then known = true end
    end
    if not known then L.error = "unknown-hero" return end
    for j, other in ipairs(L.slots) do
      if j ~= i and other.hero == a.hero then L.error = "hero-held" return end
    end
    s.hero = a.hero
  elseif call == "ready" then
    if s.owner ~= owner then L.error = "not-owner" return end
    if a.ready and s.hero == nil then L.error = "no-hero" return end
    s.ready = a.ready == true
  end
end

-- The tone of a seat's state: open (faint), seated but not ready (warn), ready (ok). owner_text and readySuffix say the same in words.
local function seat_tone(s)
  if s.owner == nil then return "faint" end
  if s.ready then return "ok" end
  return "warn"
end

local function lobby_seat(i)
  local s = LOBBY.slots[i]
  local slot0 = i - 1
  local head = TEXT.lobby.slotWord .. " " .. tostring(i) .. ": " .. owner_text(s)
  if s.owner == nil then
    return box({ label(head, seat_tone(s)), button(TEXT.lobby.claim, "claim", obj({ I("slot", slot0) }), seated(OWNER)) }, "row")
  end
  if s.owner == OWNER then
    local heroes = {}
    for _, h in ipairs(ROSTER) do
      local text = h
      if h == s.hero then text = "[" .. h .. "]" end
      heroes[#heroes + 1] = button(text, "pick", obj({ I("slot", slot0), S("hero", h) }), s.ready)
    end
    local ready_text = s.ready and TEXT.lobby.unready or TEXT.lobby.ready
    local picked = s.hero and hero_icon(s.hero, 48) or nil
    return box({
      box(with({ label(head .. " " .. (s.hero or TEXT.lobby.pickHero), seat_tone(s)) }, picked), "row"),
      grid(4, heroes),
      box({
        button(ready_text, "ready", obj({ I("slot", slot0), B("ready", not s.ready) }), not s.ready and s.hero == nil),
        button(TEXT.lobby.leave, "release", obj({ I("slot", slot0) })),
      }, "row"),
    }, "col")
  end
  local tail = ""
  if s.ready then tail = TEXT.lobby.readySuffix end
  return box({ label(head .. " " .. (s.hero or TEXT.lobby.noHero) .. tail, seat_tone(s)) }, "row")
end

local function lobby_tree()
  if LOBBY.started then return nil end
  local teams = {}
  for t = 0, 1 do
    local rows = { label(TEXT.lobby.teamNames[t + 1], "accent") }
    for i = 1, TEAM_SIZE do rows[#rows + 1] = lobby_seat(t * TEAM_SIZE + i) end
    teams[#teams + 1] = box(rows, "col")
  end
  local ready = 0
  for _, s in ipairs(LOBBY.slots) do
    if s.owner and s.ready then ready = ready + 1 end
  end
  local status = tostring(ready) .. "/" .. tostring(SLOT_COUNT) .. " " .. TEXT.lobby.readyWord
  if LOBBY.error ~= "" then
    local why = TEXT.lobby.refusals[LOBBY.error] or TEXT.lobby.refusedGeneric
    status = status .. " (" .. TEXT.lobby.refusedWord .. ": " .. why .. ")"
  end
  local host = LOBBY.host == OWNER
  return panel(TEXT.lobby.title, {
    box(teams, "row"),
    label(status, "muted"),
    box({
      button(TEXT.lobby.fillBots, "fill", "{}", not (host and any_open())),
      button(TEXT.lobby.startMatch, "start", "{}", not (host and all_ready())),
    }, "row"),
  })
end

local function emit_all()
  update_heroes()
  emit("lobby", "modal", lobby_tree())
  emit("hud", "hud", hud_tree())
  emit("win", "win", win_tree())
  emit("team", "top", team_tree())
  emit("scoreboard", "centre", OPEN.scoreboard and scoreboard_tree() or nil)
  emit("shop", "right", OPEN.shop and shop_tree() or nil)
  local hero = hero_of(LOCAL)
  emit("skills", "left", hero ~= nil and skills_tree(hero) or nil)
  emit("inventory", "bar-mid", inventory_tree_cached())
  emit("select", "bar-left", select_tree())
end

local function key_in(list, key)
  for _, k in ipairs(list or {}) do
    if k == key then return true end
  end
  return false
end

local function ui_call(who, call, a)
  if call == "buy" then
    local hero = hero_of(who)
    if hero ~= nil then game.cmd_buy(hero, a.item) end
  elseif call == "sell" then
    local hero = hero_of(who)
    if hero ~= nil then game.cmd_sell(hero, a.slot) end
  elseif call == "unstash" then
    local hero = hero_of(who)
    if hero ~= nil then game.cmd_unstash(hero, a.slot) end
  elseif call == "skill_up" then
    local hero = hero_of(who)
    if hero ~= nil then game.cmd_skill_up(hero, a.slot) end
  elseif call == "claim" or call == "release" or call == "pick" or call == "ready" or call == "fill" or call == "start" then
    lobby_act(call, a)
    if call ~= "start" then lobby_seats_event() end
  end
end

local function panels_on_action(name, args)
  if name == "setup" then
    UI = args.ui or {}
    TEXT = UI.text or {}
    KEYS = UI.keys or {}
    LOCAL = args.player or LOCAL
    OWNER = args.pubkey or OWNER
    LAST = {}
    inventory_cache = {}
  elseif name == "key" then
    if args.phase == "down" and (args.player == nil or args.player == LOCAL) then
      if key_in(KEYS.shop, args.key) then OPEN.shop = not OPEN.shop end
      if key_in(KEYS.scoreboard, args.key) then OPEN.scoreboard = not OPEN.scoreboard end
    end
  elseif name == "selection" then
    if args.player == nil or args.player == LOCAL then SEL = args.ids or {} end
  elseif name == "ui" then
    local a = args.args or {}
    a.pubkey = args.pubkey
    ui_call(args.player or LOCAL, args.call, a)
  end
end

local function refresh()
  if ROSTER == nil then
    if UI.teams == nil then return end
    ROSTER = roster_names()
  end
  if TEXT.lobby == nil then return end
  emit_all()
end

-- Every REFRESH_TICKS ticks from the first tick: the panels that changed go out again.
game.timer("panels", 1, refresh, REFRESH_TICKS)

return { on_action = panels_on_action }

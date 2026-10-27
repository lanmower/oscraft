local chat_lobby = require("chat_lobby")
local draft = require("draft")
local config = require("config")
local units = require("units")
local xp = require("xp")
local stats = require("stats")
local modes = require("modes")
local shops = require("shops")
local scoreboard_688g = require("scoreboard")
local ui_fill = require("ui_fill")
local store = require("store")

local TICKS_PER_SECOND = config.TICKS_PER_SECOND
local REFRESH_TICKS = 4
local INVENTORY_SLOTS = 6
local STASH_SLOTS = 9
local NEUTRAL_SLOT = 9
local ABILITY_SLOTS = 8
local TEAM_SIZE = 5
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
local GROUP_PAGE = 0
local WIN_CLOSED = false
local PINS = {}
local SHOP_PICK = {}
local OPEN = { shop = false, scoreboard = false, controls = false }
local SELL_ARMED = nil
local HORN = nil
local PEAK = {}
local COOLDOWN_FULL = {}
local HEROES = {}
local ROSTER = nil
local LAST = {}

local function new_lobby()
  local slots = {}
  for i = 1, SLOT_COUNT do slots[i] = { owner = nil, hero = nil, ready = false, bot = false } end
  return { host = nil, started = false, worldReady = false, slots = slots, error = "" }
end
local LOBBY = new_lobby()

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
local function image(src, size, alt)
  return node("image", with({ S("src", src), I("size", size or 64) }, alt and S("alt", alt) or nil))
end

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

local function hero_icon(name, size)
  local model = hero_model(name)
  local icon = model and ("heroes/" .. model .. ".png") or nil
  if icon == nil then return label(name) end
  return image("icons/" .. icon, size or 48, name)
end

local function hero_picture(name, size)
  local model = hero_model(name)
  if model == nil then return hero_icon(name, size) end
  return node("model", { S("model", model), S("src", "icons/heroes/" .. model .. ".png"), I("size", size or 48), S("alt", name) })
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

local function team_name(side)
  return TEXT.lobby.teamNames[side + 1] or (TEXT.teamLabel .. " " .. tostring(side))
end

local function alive_selection()
  local out = {}
  for _, id in ipairs(SEL) do
    if unit_alive(id) then out[#out + 1] = id end
  end
  return out
end

local function hero_of(who)
  return units.hero_of(SEL, who)
end

local function hero_unit(p)
  for _, id in ipairs(game.units(p)) do
    if units.is_hero(id) then return id end
  end
  return nil
end

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

local function hud_tree()
  local primary = alive_selection()[1]
  local hp = nil
  if primary ~= nil then hp = unit_hp(primary) end
  local hp_text = TEXT.noValue
  if hp ~= nil and hp >= 0 then hp_text = tostring(math.floor(hp / config.FIX)) end
  local kids = {
    label(TEXT.hud.hp .. " " .. hp_text),
    label(TEXT.hud.gold .. " " .. tostring(game.gold(LOCAL))),
  }
  if HORN ~= nil and TEXT.hud.minutes ~= nil then
    local total = math.floor((game.time() - HORN) / TICKS_PER_SECOND)
    local minutes = math.floor(total / 60)
    kids[#kids + 1] = label(TEXT.hud.minutes .. " " .. tostring(minutes) .. "  " .. TEXT.hud.seconds .. " " .. tostring(total - minutes * 60))
  end
  return box(kids, "row")
end

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
  local kids = { label(team_name(side) .. " " .. TEXT.winner.suffix, "accent") }
  for _, row in ipairs(post_game_rows(true)) do
    local items = #row.items
    kids[#kids + 1] = label("P" .. tostring(row.player) .. "  " .. TEXT.post.kills .. " " .. tostring(row.kills) ..
      "  " .. TEXT.post.deaths .. " " .. tostring(row.deaths) .. "  " .. TEXT.post.assists .. " " .. tostring(row.assists) ..
      "  " .. TEXT.post.gold .. " " .. tostring(row.gold) .. "  " .. TEXT.post.xp .. " " .. tostring(row.xp) ..
      "  " .. TEXT.post.items .. " " .. tostring(items))
  end
  kids[#kids + 1] = button(TEXT.menu.close, "win_close", "{}")
  return box(kids, "col")
end

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
  if TEXT.menu ~= nil then
    for _, entry in ipairs({ "shop", "scoreboard", "controls" }) do
      kids[#kids + 1] = button(TEXT.menu[entry], "toggle", obj({ S("panel", entry) }), false, TEXT.menu[entry])
    end
  end
  return box(kids, "row")
end

local function shop_names(item)
  local names = {}
  for _, building in ipairs(shops.sellers(item)) do
    local kind = ui_fill.shop_kind[building]
    names[#names + 1] = kind ~= nil and (building .. " (" .. TEXT.shop.kinds[kind] .. ")") or building
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

local function shop_tier(cost)
  if cost <= 0 then return nil end
  local edges = TEXT.shop.tierCosts
  for t = 1, #edges do
    if cost <= edges[t] then return t end
  end
  return #edges + 1
end

local function stash_cells(hero)
  local stored, cells = {}, {}
  if hero ~= nil then
    for _, row in ipairs(game.unit_stash(hero)) do stored[row.slot] = row.name end
  end
  for s = 0, STASH_SLOTS - 1 do
    local name = stored[s]
    if name ~= nil then
      cells[#cells + 1] = cell({ label(name, "strong") }, { tip = name, tone = "stash", call = "unstash", args = obj({ I("slot", s) }) })
    else
      cells[#cells + 1] = cell({}, { tone = "empty" })
    end
  end
  return cells
end

local function shop_kind(who)
  local hero = hero_of(who)
  if hero == nil then return nil end
  return game.unit_kind(hero)
end

local function recipe_lines(item, depth, out)
  local def = game.item_info(item)
  if def == nil or depth > 4 then return out end
  local pad = ""
  for _ = 1, depth do pad = pad .. "  " end
  out[#out + 1] = label(pad .. def.name .. " " .. tostring(def.cost) .. TEXT.costSuffix, depth == 0 and "strong" or "muted")
  for _, c in ipairs(def.components) do recipe_lines(c, depth + 1, out) end
  return out
end

local function shop_tree()
  local hero = hero_of(LOCAL)
  local gold = game.gold(LOCAL)
  local kind = shop_kind(LOCAL)
  local pins = (kind and PINS[kind]) or {}
  local pick = kind and SHOP_PICK[kind]
  local tiers = TEXT.shop.tiers
  local pinned_cells = {}
  local bands = {}
  for t = 1, #tiers + 1 do bands[t] = {} end
  local i = 0
  while true do
    local def = game.item_info(i)
    if def == nil then break end
    if shops.listed(def.name) then
      local price = hero ~= nil and net_price(hero, i, def) or def.cost
      local afford = gold >= price
      local kids = {
        label(def.name, "strong"),
        label(tostring(def.cost) .. TEXT.costSuffix, afford and "gold" or "bad"),
        label("[" .. shop_names(def.name) .. "]", "faint"),
      }
      local stats = stats_text(def.stats)
      if stats ~= "" then kids[#kids + 1] = label(stats, "muted") end
      if #def.components > 0 then kids[#kids + 1] = label(TEXT.recipePrefix .. recipe_text(def.components), "faint") end
      kids[#kids + 1] = box({
        button(TEXT.buy, "buy", obj({ I("item", i) }), hero == nil or not afford),
        button(pins[i] and TEXT.shop.unpin or TEXT.shop.pin, "pin", obj({ I("item", i) }), kind == nil),
        button(TEXT.shop.recipeTree, "shop_pick", obj({ I("item", i) }), kind == nil),
      }, "row")
      local band = shop_tier(def.cost) or #tiers + 1
      bands[band][#bands[band] + 1] = cell(kids, { tip = def.name })
      if pins[i] then
        pinned_cells[#pinned_cells + 1] = cell({
          label(def.name, "strong"),
          box({
            button(TEXT.buy, "buy", obj({ I("item", i) }), hero == nil or not afford),
            button(TEXT.shop.unpin, "pin", obj({ I("item", i) }), kind == nil),
          }, "row"),
        }, { tip = def.name })
      end
    end
    i = i + 1
  end
  local sections = {}
  if pick ~= nil then
    sections[#sections + 1] = box({ label(TEXT.shop.recipeTree, "faint"), box(recipe_lines(pick, 0, {}), "col") }, "col")
  else
    sections[#sections + 1] = label(TEXT.shop.pickNone, "muted")
  end
  for t = 1, #bands do
    if #bands[t] > 0 then
      local heading = tiers[t] or TEXT.shop.notSold
      sections[#sections + 1] = box({ label(heading .. "  " .. tostring(#bands[t]), "faint"), grid(3, bands[t]) }, "col")
    end
  end
  sections[#sections + 1] = box({
    label(TEXT.shop.pinned, "faint"),
    #pinned_cells > 0 and grid(3, pinned_cells) or label(TEXT.shop.pinnedNone, "muted"),
  }, "col")
  local stash_count = hero ~= nil and #game.unit_stash(hero) or 0
  sections[#sections + 1] = box({
    box({ label(TEXT.stashLabel, "faint"), button(TEXT.shop.grabAll, "grab_all", "{}", stash_count == 0 or hero == nil) }, "row"),
    grid(3, stash_cells(hero)),
  }, "col")
  return panel(TEXT.shop.title .. "  " .. TEXT.goldLabel .. " " .. tostring(gold), sections)
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
  if TEXT.skills.talentName ~= "" then
    kids[#kids + 1] = label(TEXT.skills.talents, "faint")
    local bits = game.upgrade_bits(hero)
    for tier = 1, #TALENT_LEVELS do
      local need = TALENT_LEVELS[tier]
      local picked = shr(bits, TALENT_FIRST_BIT + 2 * (tier - 1)) % 4
      local row = { label(TEXT.skills.needLevel .. " " .. tostring(need), level < need and "muted" or "accent") }
      for side = 0, 1 do
        local on = (side == 0 and picked % 2 == 1) or (side == 1 and picked >= 2)
        local side_name = side == 0 and TEXT.skills.talentLeft or TEXT.skills.talentRight
        row[#row + 1] = button(side_name .. (on and " [x]" or " +"), "talent",
          obj({ I("unit", hero), I("tier", tier - 1), I("side", side) }), picked ~= 0 or level < need,
          side_name .. ": " .. TEXT.skills.talentName)
      end
      kids[#kids + 1] = box(row, "row")
    end
  end
  return panel(TEXT.skills.title, kids)
end

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
    local k = tostring(hero) .. ":" .. tostring(key.slot)
    if a.cooldown > (COOLDOWN_FULL[k] or 0) then COOLDOWN_FULL[k] = a.cooldown end
    local full = COOLDOWN_FULL[k]
    if ranked and info.cooldowns[math.max(rank, 1)] ~= nil and info.cooldowns[math.max(rank, 1)] > 0 then full = info.cooldowns[math.max(rank, 1)] end
    kids[#kids + 1] = bar(math.min(a.cooldown, full), full)
    tone = "muted"
  elseif rank == 0 then
    tone = "faint"
  end
  if a.cooldown > 0 or rank == 0 then return cell(kids, { tone = tone, tip = a.name }) end
  return cell(kids, { tone = tone, tip = a.name, call = "cast", args = obj({ I("slot", key.slot) }) })
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
    info[#info + 1] = label(TEXT.manaLabel .. " " .. tostring(game.mana(primary)) .. "/" .. tostring(game.max_mana(primary)))
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
    local pages = math.max(1, math.floor((#sel + GROUP_TILES - 1) / GROUP_TILES))
    if GROUP_PAGE >= pages then GROUP_PAGE = pages - 1 end
    local first = GROUP_PAGE * GROUP_TILES
    local tiles = {}
    for i = first + 1, math.min(#sel, first + GROUP_TILES) do
      local id = sel[i]
      local tile_hp = unit_hp(id)
      local tile_peak = math.max(PEAK[id] or 0, tile_hp)
      PEAK[id] = tile_peak
      tiles[#tiles + 1] = cell({ label(game.unit_kind(id) or TEXT.noValue, "muted"), bar(math.max(tile_hp, 0), tile_peak, "hp") },
        { tone = i == 1 and "active" or nil, call = "select", args = obj({ R("ids", arr({ tostring(id) })) }) })
    end
    local pager = { label(tostring(#sel) .. " " .. TEXT.group.selected, "accent") }
    if pages > 1 then
      pager[#pager + 1] = button(TEXT.group.prev, "group_page", obj({ I("dir", -1) }), GROUP_PAGE == 0)
      pager[#pager + 1] = label(TEXT.group.page .. " " .. tostring(GROUP_PAGE + 1) .. "/" .. tostring(pages), "muted")
      pager[#pager + 1] = button(TEXT.group.next, "group_page", obj({ I("dir", 1) }), GROUP_PAGE >= pages - 1)
    end
    kids[#kids + 1] = box(pager, "row")
    kids[#kids + 1] = grid(6, tiles)
  end
  return box(kids, "col")
end

local function fill(text, vars)
  local out, i = "", 1
  while i <= #text do
    local key = nil
    if string.sub(text, i, i) == "$" then
      for name in pairs(vars) do
        if string.sub(text, i + 1, i + #name) == name then key = name end
      end
    end
    if key ~= nil then
      out = out .. vars[key]
      i = i + 1 + #key
    else
      out = out .. string.sub(text, i, i)
      i = i + 1
    end
  end
  return out
end

local function refund_text(item, name)
  local gold = store.sale_refund(game.item_info(item))
  return fill(TEXT.menu.refund.format, { item = name, gold = tostring(gold) })
end

local function sell_refusal(hero, slot)
  for _, row in ipairs(game.unit_items(hero)) do
    if row.slot == slot then
      if not unit_alive(hero) then return "dead" end
      if unit_player(hero) ~= LOCAL then return "notyours" end
      return store.sale_refusal(game.item_info(row.item))
    end
  end
  return nil
end

local function hundredths(v)
  local whole = math.floor(v / 100)
  local part = v - whole * 100
  if part < 10 then return tostring(whole) .. ".0" .. tostring(part) end
  return tostring(whole) .. "." .. tostring(part)
end

local function base_line(entry)
  local parts = {}
  for k = 1, 3 do
    parts[k] = TEXT.lobby.attrs[k] .. " " .. hundredths(entry[2 * k]) .. "+" .. hundredths(entry[2 * k + 1])
  end
  return join(parts, "  ")
end

local function inventory_tree()
  local hero = hero_of(LOCAL)
  local items, stored = {}, {}
  if hero ~= nil then
    for _, row in ipairs(game.unit_items(hero)) do items[row.slot] = row end
    for _, row in ipairs(game.unit_stash(hero)) do stored[row.slot] = row.name end
  end
  if items[SELL_ARMED] == nil then SELL_ARMED = nil end
  local main, stash = {}, {}
  for s = 0, INVENTORY_SLOTS - 1 do
    local held = items[s]
    if held ~= nil then
      local name = held.name
      local armed = SELL_ARMED == s
      local refusal = sell_refusal(hero, s)
      local tip = TEXT.menu.sell .. " " .. name .. " (" .. refund_text(held.item, name) .. ")"
      if armed then
        tip = TEXT.menu.sellConfirm
      elseif refusal ~= nil then
        tip = TEXT.menu.refund.denied[refusal]
      end
      main[#main + 1] = cell({ label(name, "strong") }, { tip = tip, tone = armed and "active" or nil, call = "sell", args = obj({ I("slot", s) }) })
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

local function inventory_tree_cached()
  local hero = hero_of(LOCAL)
  local gen = hero ~= nil and game.unit_item_gen(hero) or nil
  if gen ~= nil and inventory_cache.hero == hero and inventory_cache.gen == gen then return inventory_cache.tree end
  local tree = inventory_tree()
  inventory_cache = { hero = hero, gen = gen, tree = tree }
  return tree
end

local function key_text(v)
  if type(v) ~= "table" then return tostring(v) end
  if v.modifier ~= nil then return v.modifier .. "+" .. key_text(v.keys) end
  local names = {}
  if v[1] ~= nil then
    for _, k in ipairs(v) do names[#names + 1] = tostring(k) end
  else
    for k in pairs(v) do names[#names + 1] = tostring(k) end
    for i = 2, #names do
      local j = i
      while j > 1 and names[j - 1] > names[j] do
        local held = names[j - 1]
        names[j - 1] = names[j]
        names[j] = held
        j = j - 1
      end
    end
  end
  return join(names, " ")
end

local function controls_tree()
  local names = {}
  for name in pairs(KEYS) do names[#names + 1] = name end
  for i = 2, #names do
    local j = i
    while j > 1 and names[j - 1] > names[j] do
      local held = names[j - 1]
      names[j - 1] = names[j]
      names[j] = held
      j = j - 1
    end
  end
  local kids = {}
  for _, name in ipairs(names) do
    kids[#kids + 1] = label(name .. ": " .. key_text(KEYS[name]), "muted")
  end
  kids[#kids + 1] = label(TEXT.menu.clickTitle, "accent")
  for _, line in ipairs(TEXT.menu.clicks or {}) do kids[#kids + 1] = label(line, "muted") end
  kids[#kids + 1] = button(TEXT.menu.close, "toggle", obj({ S("panel", "controls") }))
  return panel(TEXT.menu.controlsTitle, kids)
end

local function scoreboard_tree()
  local rows = post_game_rows(false)
  local kids = {}
  for _, side in ipairs(sides()) do
    local cells = {}
    for _, h in ipairs(TEXT.scoreboardHeaders) do cells[#cells + 1] = label(h, "muted") end
    for _, h in ipairs(stats.headers) do cells[#cells + 1] = label(h, "muted") end
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
        for _, text in ipairs(stats.cells(hero)) do cells[#cells + 1] = label(text, tone) end
      end
    end
    kids[#kids + 1] = label(team_name(side), "team" .. tostring(side))
    kids[#kids + 1] = grid(#TEXT.scoreboardHeaders + #stats.headers, cells)
  end
  return panel(TEXT.scoreboardTitle, kids)
end

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
    if not (s.owner and (s.hero or modes.picks_free()) and s.ready) then return false end
  end
  return true
end

local function any_open()
  for _, s in ipairs(LOBBY.slots) do
    if s.owner == nil then return true end
  end
  return false
end

local LATE = {}

local function held(hero)
  for _, s in ipairs(LOBBY.slots) do
    if s.hero == hero then return true end
  end
  return false
end

local function random_free()
  local free = {}
  for _, h in ipairs(ROSTER) do
    if (modes.duplicates() or not held(h)) and modes.allows(h, LATE.group(h), modes.melee(h)) then free[#free + 1] = h end
  end
  if #free == 0 then return nil end
  return free[modes.rand(#free) + 1]
end

local function any_free()
  for _, h in ipairs(ROSTER) do
    if (modes.duplicates() or not held(h)) and modes.allows(h, LATE.group(h), modes.melee(h)) then return true end
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

local function lobby_seats_event()
  local items = {}
  for _, s in ipairs(LOBBY.slots) do
    items[#items + 1] = obj({ S("owner", s.owner or ""), S("hero", s.hero or ""), B("ready", s.ready == true), S("model", s.hero and hero_model(s.hero) or "") })
  end
  game.ui_event("lobby", obj({ I("player", LOCAL), B("started", false), R("slots", arr(items)) }))
end

local function lobby_act(call, a)
  local L = LOBBY
  local owner = a.pubkey or OWNER
  if L.started then
    L.error = "match-started"
    return
  end
  L.error = ""
  L.calls = (L.calls or 0) + 1
  if draft.blocks(call) then L.error = "draft-running" return end
  if call == "mode" then
    if L.host ~= owner then L.error = "not-host" return end
    local ok, why = modes.toggle(a.code)
    if not ok then L.error = "mode-" .. why return end
    draft.reset()
    for _, s in ipairs(L.slots) do
      if s.hero ~= nil and not modes.allows(s.hero, LATE.group(s.hero), modes.melee(s.hero)) then s.hero, s.ready = nil, false end
    end
    return
  end
  if call == "world_ready" then
    if owner ~= OWNER then L.error = "not-host" return end
    L.worldReady = true
    return
  end
  if call == "start" then
    if L.host ~= owner then L.error = "not-host" return end
    if not L.worldReady then L.error = "world-loading" return end
    if not all_ready() then L.error = "not-ready" return end
    if draft.pending() then
      local began, why = draft.begin()
      if not began then L.error = why end
      return
    end
    modes.reseed(L.calls or 0)
    modes.assign(L.slots, L.host, ROSTER, LATE.group, modes.melee)
    L.started = true
    draft.close()
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
          if (modes.duplicates() or not held(h)) and modes.allows(h, LATE.group(h), modes.melee(h)) then
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
  elseif call == "remove" then
    if L.host ~= owner then L.error = "not-host" return end
    if s.owner == nil then L.error = "slot-empty" return end
    local removed = s.owner
    s.owner, s.hero, s.ready, s.bot = nil, nil, false, false
    if L.host == removed then
      L.host = nil
      for _, other in ipairs(L.slots) do
        if other.owner ~= nil and not other.bot then
          L.host = other.owner
          break
        end
      end
    end
  elseif call == "pick" then
    if draft.active() then L.error = draft.press(owner, i, a.hero) or "" return end
    if s.owner ~= owner then L.error = "not-owner" return end
    if s.ready then L.error = "ready-locked" return end
    local hero = a.hero
    if hero == nil or hero == "" then hero = random_free() end
    if hero == nil then L.error = "unknown-hero" return end
    local known = false
    for _, h in ipairs(ROSTER) do
      if h == hero then known = true end
    end
    if not known then L.error = "unknown-hero" return end
    if not modes.allows(hero, LATE.group(hero), modes.melee(hero)) then L.error = "mode-pool" return end
    for j, other in ipairs(L.slots) do
      if j ~= i and other.hero == hero and not modes.duplicates() then L.error = "hero-held" return end
    end
    s.hero = hero
  elseif call == "ready" then
    if s.owner ~= owner then L.error = "not-owner" return end
    if a.ready and s.hero == nil and not modes.picks_free() then L.error = "no-hero" return end
    s.ready = a.ready == true
  end
end

local function seat_tone(s)
  if s.owner == nil then return "faint" end
  if s.ready then return "ok" end
  return "warn"
end

local HERO_BASE = {
  antimage = { "Anti-Mage", 2200, 160, 2200, 280, 1500, 180 },
  axe = { "Axe", 2500, 250, 2000, 220, 1800, 160 },
  bloodseeker = { "Bloodseeker", 2300, 200, 2400, 300, 1800, 170 },
  crystal_maiden = { "Crystal Maiden", 1600, 170, 1600, 160, 1600, 290 },
  drow_ranger = { "Drow Ranger", 1700, 190, 2600, 190, 1500, 140 },
  earthshaker = { "Earthshaker", 2200, 290, 1200, 140, 1600, 180 },
  juggernaut = { "Juggernaut", 2000, 190, 2600, 240, 1400, 140 },
  morphling = { "Morphling", 1900, 200, 2400, 340, 1700, 150 },
  phantom_lancer = { "Phantom Lancer", 2100, 210, 2900, 260, 2100, 200 },
  sand_king = { "Sand King", 1800, 260, 1900, 210, 1600, 180 },
  storm_spirit = { "Storm Spirit", 1900, 150, 2200, 180, 2400, 310 },
  vengefulspirit = { "Vengeful Spirit", 1800, 260, 2700, 330, 1500, 175 },
  windrunner = { "Windrunner", 1500, 250, 1700, 140, 2200, 260 },
  lich = { "Lich", 1800, 155, 1500, 200, 1800, 325 },
  shadow_shaman = { "Shadow Shaman", 2100, 180, 1600, 160, 2100, 300 },
  tidehunter = { "Tidehunter", 2200, 300, 1500, 150, 1600, 170 },
  witch_doctor = { "Witch Doctor", 1600, 180, 1300, 140, 2400, 290 },
  enigma = { "Enigma", 1700, 210, 1400, 100, 2000, 340 },
  tinker = { "Tinker", 1700, 200, 1300, 120, 2700, 220 },
  necrolyte = { "Necrolyte", 1600, 200, 1500, 170, 2200, 250 },
  warlock = { "Warlock", 1800, 250, 1000, 100, 2400, 270 },
  beastmaster = { "Beastmaster", 2300, 220, 1800, 160, 1600, 190 },
  queenofpain = { "Queen of Pain", 1600, 170, 1800, 200, 2400, 250 },
  venomancer = { "Venomancer", 1800, 185, 2200, 260, 1500, 175 },
  faceless_void = { "Faceless Void", 2300, 160, 2300, 265, 1500, 150 },
  skeleton_king = { "Skeleton King", 2200, 290, 1800, 170, 1800, 160 },
  death_prophet = { "Death Prophet", 1900, 190, 1400, 140, 2000, 300 },
  phantom_assassin = { "Phantom Assassin", 2000, 185, 2300, 315, 1300, 100 },
  templar_assassin = { "Templar Assassin", 1800, 210, 2300, 270, 2000, 200 },
  dragon_knight = { "Dragon Knight", 2100, 280, 1700, 220, 1500, 170 },
  life_stealer = { "Lifestealer", 2500, 240, 1800, 190, 1500, 175 },
  dark_seer = { "Dark Seer", 2200, 230, 1200, 120, 2700, 270 },
  omniknight = { "Omniknight", 2000, 265, 1500, 175, 1700, 180 },
  enchantress = { "Enchantress", 1600, 100, 1900, 180, 1600, 280 },
  night_stalker = { "Night Stalker", 2300, 280, 1800, 225, 1600, 160 },
  broodmother = { "Broodmother", 1700, 250, 1800, 220, 1800, 200 },
  bounty_hunter = { "Bounty Hunter", 1700, 180, 2100, 300, 1900, 200 },
  batrider = { "Batrider", 2300, 240, 1500, 150, 2400, 250 },
  spectre = { "Spectre", 1900, 200, 2300, 180, 1600, 190 },
  doom_bringer = { "Doom Bringer", 2600, 320, 1100, 90, 1300, 210 },
  ancient_apparition = { "Ancient Apparition", 1800, 140, 2000, 220, 2500, 260 },
  spirit_breaker = { "Spiritbreaker", 2900, 240, 1700, 170, 1400, 180 },
  gyrocopter = { "Gyrocopter", 1800, 180, 2400, 280, 1900, 210 },
  alchemist = { "Alchemist", 2500, 180, 1100, 120, 2500, 180 },
  invoker = { "Invoker", 1900, 170, 2000, 190, 1800, 300 },
  silencer = { "Silencer", 1700, 220, 1600, 300, 2700, 250 },
  obsidian_destroyer = { "Obsidian Destroyer", 1900, 230, 2400, 200, 2600, 330 },
  shadow_demon = { "Shadow Demon", 1700, 190, 1800, 220, 2300, 270 },
  lone_druid = { "Lone Druid", 1700, 210, 2400, 270, 1300, 140 },
  chaos_knight = { "Chaos Knight", 2000, 290, 1400, 210, 1600, 160 },
  ogre_magi = { "Ogre Magi", 2300, 320, 1400, 155, 1700, 240 },
  undying = { "Undying", 2200, 210, 1000, 80, 2700, 250 },
  disruptor = { "Disruptor", 1900, 190, 1500, 140, 2200, 250 },
  naga_siren = { "Naga Siren", 2100, 230, 2100, 275, 1800, 195 },
  keeper_of_the_light = { "Keeper of the Light", 1600, 180, 1500, 160, 2200, 280 },
  troll_warlord = { "Troll Warlord", 1900, 220, 2100, 275, 1300, 100 },
  magnataur = { "Magnataur", 2100, 275, 1500, 250, 1900, 165 },
  bristleback = { "Bristleback", 2200, 220, 1700, 180, 1400, 280 },
  skywrath_mage = { "Skywrath Mage", 1900, 150, 1300, 80, 2700, 360 },
  legion_commander = { "Legion Commander", 2600, 260, 1800, 170, 2000, 220 },
  ember_spirit = { "Ember Spirit", 1900, 200, 2200, 180, 2000, 180 },
  earth_spirit = { "Earth Spirit", 2100, 290, 1700, 150, 1800, 210 },
  phoenix = { "Phoenix", 1700, 290, 1200, 130, 1800, 180 },
  oracle = { "Oracle", 1800, 190, 1500, 170, 2300, 290 },
  winter_wyvern = { "Winter Wyvern", 2400, 210, 1600, 190, 2500, 310 },
  arc_warden = { "Arc Warden", 2400, 190, 1500, 180, 2400, 210 },
  bane = { "Bane Elemental", 2200, 210, 2200, 210, 2200, 210 },
  brewmaster = { "Pandaren Brewmaster", 2300, 290, 2200, 195, 1400, 125 },
  tusk = { "Tuskarr", 2300, 230, 2300, 210, 1800, 170 },
}

local HERO_PRIMARY = {
  antimage = 2,
  axe = 1,
  bloodseeker = 2,
  crystal_maiden = 3,
  drow_ranger = 2,
  earthshaker = 1,
  juggernaut = 2,
  morphling = 2,
  phantom_lancer = 2,
  sand_king = 1,
  storm_spirit = 3,
  vengefulspirit = 2,
  windrunner = 3,
  lich = 3,
  shadow_shaman = 3,
  tidehunter = 1,
  witch_doctor = 3,
  enigma = 3,
  tinker = 3,
  necrolyte = 3,
  warlock = 3,
  beastmaster = 1,
  queenofpain = 3,
  venomancer = 2,
  faceless_void = 2,
  skeleton_king = 1,
  death_prophet = 3,
  phantom_assassin = 2,
  templar_assassin = 2,
  dragon_knight = 1,
  life_stealer = 1,
  dark_seer = 3,
  omniknight = 1,
  enchantress = 3,
  night_stalker = 1,
  broodmother = 2,
  bounty_hunter = 2,
  batrider = 3,
  spectre = 2,
  doom_bringer = 1,
  ancient_apparition = 3,
  spirit_breaker = 1,
  gyrocopter = 2,
  alchemist = 1,
  invoker = 3,
  silencer = 3,
  obsidian_destroyer = 3,
  shadow_demon = 3,
  lone_druid = 2,
  chaos_knight = 1,
  ogre_magi = 3,
  undying = 1,
  disruptor = 3,
  naga_siren = 2,
  keeper_of_the_light = 3,
  troll_warlord = 2,
  magnataur = 1,
  bristleback = 1,
  skywrath_mage = 3,
  legion_commander = 1,
  ember_spirit = 2,
  earth_spirit = 1,
  phoenix = 1,
  oracle = 3,
  winter_wyvern = 3,
  arc_warden = 2,
  bane = 3,
  brewmaster = 1,
  tusk = 1,
  marci = 1,
  pangolier = 2,
  snapfire = 1,
  void_spirit = 3,
  abyssal_underlord = 1,
  centaur = 1,
  chen = 3,
  clinkz = 2,
  dazzle = 3,
  elder_titan = 1,
  huskar = 1,
  jakiro = 3,
  kunkka = 1,
  leshrac = 3,
  lina = 3,
  lion = 3,
  luna = 2,
  lycan = 1,
  medusa = 2,
  meepo = 2,
  mirana = 2,
  nevermore = 2,
  nyx_assassin = 2,
  puck = 3,
  pudge = 1,
  pugna = 3,
  rattletrap = 1,
  razor = 2,
  riki = 2,
  rubick = 3,
  shredder = 1,
  slardar = 1,
  slark = 2,
  sniper = 2,
  sven = 1,
  techies = 3,
  terrorblade = 2,
  tiny = 1,
  treant = 1,
  ursa = 2,
  viper = 2,
  weaver = 2,
  wisp = 1,
  zuus = 3,
  abaddon = 1,
  furion = 3,
  visage = 3,
  dark_willow = 3,
  dawnbreaker = 1,
  grimstroke = 3,
  hoodwink = 2,
  kez = 2,
  largo = 1,
  mars = 1,
  monkey_king = 2,
  muerta = 3,
  primal_beast = 1,
  ringmaster = 3,
}

local ATTR_FALLBACK_GROUP = 1
local kind_groups = nil
local function kind_group_of(name)
  if kind_groups == nil then
    kind_groups = {}
    for _, k in ipairs(game.unit_kinds()) do
      if k.primary ~= nil and k.primary <= 2 then kind_groups[k.name] = k.primary + 1 end
    end
  end
  return kind_groups[name]
end
local function hero_group(name)
  return HERO_PRIMARY[name] or kind_group_of(name) or ATTR_FALLBACK_GROUP
end

LATE.group = hero_group

local function hero_name(name)
  local e = HERO_BASE[name]
  if e == nil then return name end
  return e[1]
end

local HERO_ACRONYM = require("hero_acronyms")

local function acronym(name)
  return UI.heroAcronyms[name] or HERO_ACRONYM[name] or "?"
end

local function base_max()
  local best = { 0, 0, 0 }
  for _, name in ipairs(ROSTER) do
    local e = HERO_BASE[name]
    if e ~= nil then
      for k = 1, 3 do
        if e[2 * k] > best[k] then best[k] = e[2 * k] end
      end
    end
  end
  return best
end

local function my_seat()
  for i = 1, SLOT_COUNT do
    if LOBBY.slots[i].owner == OWNER then return i end
  end
  return nil
end

local function lobby_seat(i)
  local s = LOBBY.slots[i]
  local slot0 = i - 1
  local head = TEXT.lobby.slotWord .. " " .. tostring(i) .. ": " .. owner_text(s)
  local kids = {}
  local icon = s.hero and hero_picture(s.hero, 40) or nil
  if icon ~= nil then kids[#kids + 1] = icon end
  local tail = ""
  if s.ready then tail = TEXT.lobby.readySuffix end
  local name = TEXT.lobby.noHero
  if s.hero then
    name = hero_name(s.hero)
  elseif s.owner == OWNER then
    name = TEXT.lobby.pickHero
  end
  kids[#kids + 1] = label(head, seat_tone(s))
  kids[#kids + 1] = label(name .. tail, seat_tone(s))
  if s.owner == nil then
    kids[#kids + 1] = button(TEXT.lobby.claim, "claim", obj({ I("slot", slot0) }), seated(OWNER))
  elseif s.owner == OWNER then
    kids[#kids + 1] = button(TEXT.lobby.leave, "release", obj({ I("slot", slot0) }))
  elseif LOBBY.host == OWNER then
    kids[#kids + 1] = button(TEXT.lobby.remove, "remove", obj({ I("slot", slot0) }))
  end
  return box(kids, "col")
end

local function hero_tile(name, mine)
  local shown = hero_name(name)
  local own = mine ~= nil and LOBBY.slots[mine].hero == name
  local allowed = modes.allows(name, hero_group(name), modes.melee(name))
  local faint = not own and ((held(name) and not modes.duplicates()) or not allowed)
  local f = { S("alt", shown), S("badge", acronym(shown)), S("text", ""), S("tip", shown) }
  local model = hero_model(name)
  if model then f[#f + 1] = S("src", "icons/heroes/" .. model .. ".png") end
  if own then f[#f + 1] = B("selected", true) end
  if faint then f[#f + 1] = S("tone", "faint") end
  if mine ~= nil and not LOBBY.slots[mine].ready and not faint then
    f[#f + 1] = S("call", "pick")
    f[#f + 1] = R("args", obj({ I("slot", mine - 1), S("hero", name) }))
  end
  return obj(f)
end

local function attribute_rows(mine)
  local groups = { {}, {}, {} }
  for _, name in ipairs(ROSTER) do
    local g = hero_group(name)
    groups[g][#groups[g] + 1] = name
  end
  local function tiles_of(g)
    local tiles = {}
    for _, name in ipairs(groups[g]) do tiles[#tiles + 1] = hero_tile(name, mine) end
    return tiles
  end
  local kids, titles = {}, {}
  for g = 1, 3 do
    titles[g] = '"' .. TEXT.lobby.attrs[g] .. '"'
    kids[#kids + 1] = node("portraits", { I("cols", 8), B("fit", true), R("items", arr(tiles_of(g))) })
  end
  return { node("regions", { S("id", "attrs"), S("dir", "row"), B("swipe", true), R("titles", arr(titles)), R("kids", arr(kids)) }) }
end

local function team_strip(t)
  local seats, filled = {}, 0
  for i = 1, TEAM_SIZE do
    local slot = t * TEAM_SIZE + i
    seats[#seats + 1] = lobby_seat(slot)
    if LOBBY.slots[slot].owner ~= nil then filled = filled + 1 end
  end
  return box({
    label(TEXT.lobby.teamNames[t + 1], "accent"),
    bar(filled, TEAM_SIZE, t == 0 and "accent" or "warn"),
    box(seats, "row"),
  }, "col")
end

local function hero_card(mine)
  local kids = { label(TEXT.lobby.chooseHero, "accent") }
  if mine == nil then
    kids[#kids + 1] = label(TEXT.lobby.noSeat, "muted")
    return box(kids, "col")
  end
  local s = LOBBY.slots[mine]
  if s.hero == nil then
    kids[#kids + 1] = label(TEXT.lobby.pickHero, "muted")
  else
    local icon = hero_picture(s.hero, 128)
    if icon ~= nil then kids[#kids + 1] = icon end
    kids[#kids + 1] = label(hero_name(s.hero), nil)
    local base = HERO_BASE[s.hero]
    if base == nil then
      local fill = ui_fill.base[s.hero]
      if fill ~= nil then kids[#kids + 1] = label(base_line(fill), "muted") end
    else
      local max = base_max()
      for k = 1, 3 do
        kids[#kids + 1] = label(TEXT.lobby.attrs[k], nil)
        kids[#kids + 1] = bar(base[2 * k], max[k], "ok")
      end
    end
  end
  local roles = s.hero ~= nil and ui_fill.roles[s.hero] or nil
  if roles ~= nil then kids[#kids + 1] = label(roles, "faint") end
  local lock_text = TEXT.lobby.lockIn
  if s.ready then lock_text = TEXT.lobby.unready end
  kids[#kids + 1] = button(lock_text, "ready", obj({ I("slot", mine - 1), B("ready", not s.ready) }), not s.ready and s.hero == nil and not modes.picks_free())
  kids[#kids + 1] = button(TEXT.lobby.random, "pick", obj({ I("slot", mine - 1), S("hero", "") }), s.ready or not any_free())
  return box(kids, "col")
end

local function composition()
  local parts = {}
  for t = 0, 1 do
    local names = {}
    for i = 1, TEAM_SIZE do
      local s = LOBBY.slots[t * TEAM_SIZE + i]
      if s.hero then names[#names + 1] = hero_name(s.hero) end
    end
    parts[#parts + 1] = TEXT.lobby.teamNames[t + 1] .. ": " .. join(names, ", ")
  end
  return TEXT.lobby.composition .. ": " .. join(parts, "  |  ")
end

local function mode_row(host)
  local names = (TEXT.lobby.modes or {})
  local kids = { label(names.title or "", "muted") }
  for _, code in ipairs(modes.BUTTONS) do
    local text = (modes.has(code) and "[x] " or "") .. (names[code] or code)
    kids[#kids + 1] = button(text, "mode", obj({ S("code", code) }), not host)
  end
  return box(kids, "row")
end

local function lobby_tree()
  if LOBBY.started then return nil end
  local mine = my_seat()
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
    box({ team_strip(0), team_strip(1) }, "row"),
    box({ box(attribute_rows(mine), "col"), hero_card(mine) }, "row"),
    mode_row(host),
    box({
      label(composition(), "muted"),
      label(status, "muted"),
      button(TEXT.lobby.fillBots, "fill", "{}", not (host and any_open())),
      button(LOBBY.worldReady and TEXT.lobby.startMatch or TEXT.lobby.loadingWorld, "start", "{}", not (host and all_ready() and LOBBY.worldReady)),
    }, "row"),
  })
end

local function emit_all()
  update_heroes()
  emit("lobby", "modal", lobby_tree())
  emit("hud", "hud", hud_tree())
  emit("win", "win", (not WIN_CLOSED) and win_tree() or nil)
  emit("team", "top", team_tree())
  emit("scoreboard", "centre", OPEN.scoreboard and scoreboard_tree() or nil)
  scoreboard_688g.refresh(OPEN.scoreboard, LOCAL)
  emit("shop", "right", OPEN.shop and shop_tree() or nil)
  emit("controls", "centre", OPEN.controls and TEXT.menu ~= nil and controls_tree() or nil)
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

local function refresh_lobby()
  if ROSTER == nil then
    if UI.teams == nil then return end
    ROSTER = roster_names()
  end
  if TEXT.lobby == nil then return end
  emit("lobby", "modal", lobby_tree())
end

local function ui_call(who, call, a)
  if call == "buy" then
    local hero = hero_of(who)
    if hero ~= nil then game.cmd_buy(hero, a.item) end
  elseif call == "toggle" then
    if OPEN[a.panel] ~= nil then OPEN[a.panel] = not OPEN[a.panel] end
  elseif call == "sell" then
    local hero = hero_of(who)
    local refusal = nil
    if hero ~= nil then refusal = sell_refusal(hero, a.slot) end
    if refusal ~= nil then
      game.ui_event("feedback", obj({ I("player", who), S("kind", "refusal"), S("reason", refusal), S("text", TEXT.menu.refund.denied[refusal]) }))
    elseif hero ~= nil and SELL_ARMED == a.slot then
      SELL_ARMED = nil
      game.cmd_sell(hero, a.slot)
    elseif hero ~= nil then
      SELL_ARMED = a.slot
      inventory_cache = {}
    end
  elseif call == "unstash" then
    local hero = hero_of(who)
    if hero ~= nil then game.cmd_unstash(hero, a.slot) end
  elseif call == "skill_up" then
    local hero = hero_of(who)
    if hero ~= nil then game.cmd_skill_up(hero, a.slot) end
  elseif call == "group_page" then
    GROUP_PAGE = math.max(0, GROUP_PAGE + (a.dir or 0))
  elseif call == "win_close" then
    WIN_CLOSED = true
  elseif call == "shop_pick" then
    local kind = shop_kind(who)
    if kind ~= nil then SHOP_PICK[kind] = a.item end
  elseif call == "pin" then
    local kind = shop_kind(who)
    if kind ~= nil then
      PINS[kind] = PINS[kind] or {}
      if PINS[kind][a.item] then PINS[kind][a.item] = nil else PINS[kind][a.item] = true end
    end
  elseif call == "grab_all" then
    local hero = hero_of(who)
    if hero ~= nil then
      local stored = {}
      for _, row in ipairs(game.unit_stash(hero)) do stored[row.slot] = true end
      for s = STASH_SLOTS - 1, 0, -1 do
        if stored[s] then game.cmd_unstash(hero, s) end
      end
    end
  elseif call == "draft_clock" then
    LOBBY.error = draft.clock(a.pubkey or OWNER, a.ticks) or ""
  elseif call == "claim" or call == "release" or call == "remove" or call == "pick" or call == "ready" or call == "fill" or call == "start" or call == "world_ready" or call == "mode" then
    lobby_act(call, a)
    if call ~= "start" then lobby_seats_event() end
    if call == "world_ready" then emit("lobby", "modal", lobby_tree()) end
    pcall(refresh_lobby)
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
    pcall(refresh_lobby)
  elseif name == "key" then
    if args.phase == "down" and (args.player == nil or args.player == LOCAL) then
      if key_in(KEYS.shop, args.key) then OPEN.shop = not OPEN.shop end
      if key_in(KEYS.scoreboard, args.key) then OPEN.scoreboard = not OPEN.scoreboard end
      if key_in(KEYS.controls, args.key) then OPEN.controls = not OPEN.controls end
    end
  elseif name == "peer_left" then
    if not LOBBY.started then
      for i = 1, SLOT_COUNT do
        if LOBBY.slots[i].owner == args.pubkey and args.pubkey ~= nil then
          lobby_act("release", { slot = i - 1, pubkey = args.pubkey })
          lobby_seats_event()
          pcall(refresh_lobby)
        end
      end
    end
  elseif name == "selection" then
    if args.player == nil or args.player == LOCAL then
      SEL = args.ids or {}
      GROUP_PAGE = 0
    end
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

local lobby_api = {
  state = function() return LOBBY end,
  owner = function() return OWNER end,
  act = lobby_act,
  roster = function()
    if ROSTER == nil then ROSTER = roster_names() end
    return ROSTER
  end,
  group = hero_group,
  hero_name = hero_name,
  changed = function()
    lobby_seats_event()
    pcall(refresh_lobby)
  end,
}

chat_lobby.attach(lobby_api)
draft.attach(lobby_api)

game.timer("panels", 1, refresh, REFRESH_TICKS)

return { on_action = panels_on_action }

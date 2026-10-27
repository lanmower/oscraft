local modes = {}

local NAMES = {
  ap = "allpick", ar = "allrandom", lm = "leaguemode", mm = "mirrormatch", tr = "teamrandom", dm = "deathmatch",
  mr = "moderandom", sp = "shuffleplayers", aa = "allagility", ai = "allintelligence", as = "allstrength",
  id = "itemdrop", np = "nopowerups", sc = "supercreeps", em = "easymode", du = "duplicatemode", sh = "samehero",
  vr = "voterandom", rv = "reverse", rd = "randomdraft", om = "onlymid", xl = "extendedleague", nm = "nomid",
  nt = "notop", nb = "nobot", ns = "noswap", nr = "norepick", cd = "captainsdraft", sd = "singledraft",
  pm = "poolingmode", oi = "observerinfo", mi = "miniheroes", cm = "captainsmode", fr = "fastrespawn",
  mo = "meleeonly", ro = "rangeonly", er = "experimentalrunes", rs = "randomside", so = "switchon", zm = "zoommode",
  cp = "capturepoint", ub = "unban", tt = "tagteam", x = "extended",
}
local SUPPORTED = { sc = true, oi = true, ap = true, ar = true, aa = true, ai = true, as = true, ro = true, mo = true, du = true, sh = true, fr = true, np = true, so = true, em = true, mi = true, id = true, cm = true, cd = true, rd = true, sd = true, rv = true, x = true }

local seed = 12345
local function roll(span)
  seed = (seed * 1103515245 + 12345) % 2147483648
  return math.floor(seed / 65536) % span
end

function modes.reseed(n) seed = 12345 + n * 7919 end

local MELEE = nil

local FLAGS = {}
local LIST = {}

function modes.name(code) return NAMES[code] end
function modes.supported(code) return SUPPORTED[code] == true end
function modes.melee(hero)
  if MELEE == nil then
    MELEE = {}
    for _, k in ipairs(game.unit_kinds()) do MELEE[k.name] = k.melee end
  end
  return MELEE[hero] == true
end
function modes.picks_free() return FLAGS.ar == true or FLAGS.sh == true or FLAGS.cm == true or FLAGS.cd == true or FLAGS.rd == true or FLAGS.sd == true or FLAGS.rv == true end

local restriction = nil

function modes.restrict(fn) restriction = fn end
function modes.rand(span) return roll(span) end
function modes.has(code) return FLAGS[code] == true end
function modes.flags() return FLAGS end
function modes.codes() return LIST end

local function hit(f, list)
  for _, c in ipairs(list) do
    if f[c] then return true end
  end
  return false
end

local CONFLICTS = {
  { "ar", "invalid", { "cm", "ap", "tr", "mr", "rv", "rd", "cd", "sd", "tt" } },
  { "ap", "invalid", { "tr", "mr" } },
  { "tr", "invalid", { "mr" } },
  { "mm", "invalid", { "sh" } },
  { "aa", "invalid", { "ai", "as", "dm" } },
  { "as", "invalid", { "ai", "dm" } },
  { "ai", "invalid", { "dm" } },
  { "ro", "invalid", { "mo", "dm" } },
  { "mo", "invalid", { "dm" } },
  { "rv", "invalid", { "cm", "ar", "tr", "mr", "dm", "sh", "tt" } },
  { "dm", "deathmatch", { "cm", "tr", "mr", "sh", "rv", "rd", "cd", "sd", "tt", "mm" } },
  { "sh", "deathmatch", { "dm" } },
  { "cm", "captains", { "tt", "sp", "ap", "rd", "cd", "ar", "sd", "mm", "tr", "dm", "mr", "aa", "ai", "as", "id", "np", "sc", "em", "du", "sh", "rv", "om", "nb", "nm", "nt", "mo", "ro" } },
  { "rd", "invalid", { "tt", "cd", "cm", "ap", "ar", "sd", "mm", "tr", "dm", "mr", "aa", "ai", "as", "sh", "rv", "ro", "mo" } },
  { "cd", "invalid", { "tt", "rd", "cm", "ap", "ar", "sd", "mm", "tr", "dm", "mr", "aa", "ai", "as", "sh", "rv", "ro", "mo" } },
  { "mr", "invalid", { "tt", "cm", "ap", "ar", "sd", "mm", "tr", "dm", "aa", "ai", "as", "sh", "rv", "ro", "mo" } },
  { "sd", "invalid", { "tt", "cm", "ap", "ar", "rd", "cd", "mm", "tr", "dm", "mr", "aa", "ai", "as", "sh", "rv", "ro", "mo" } },
  { "vr", "invalid", { "tt", "cm", "rd", "cd", "sd", "ar", "mm", "tr", "dm", "mr", "aa", "ai", "as", "sh", "rv", "ro", "mo" } },
  { "tt", "invalid", { "so", "vr", "cm", "rd", "cd", "sd", "ar", "mm", "tr", "dm", "mr", "aa", "ai", "as", "sh", "rv", "ro", "mo" } },
}

function modes.compatible(f)
  if f.x and not f.cd then return "invalid" end
  for _, row in ipairs(CONFLICTS) do
    if f[row[1]] and hit(f, row[3]) then return row[2] end
  end
  return nil
end

function modes.set_codes(codes)
  local f = {}
  for _, c in ipairs(codes or {}) do
    if NAMES[c] == nil then return false, "unknown" end
    if not SUPPORTED[c] then return false, "unsupported" end
    f[c] = true
  end
  local why = modes.compatible(f)
  if why ~= nil then return false, why end
  FLAGS = f
  LIST = {}
  for _, c in ipairs(codes or {}) do LIST[#LIST + 1] = c end
  return true, nil
end

function modes.toggle(code)
  local codes = {}
  local found = false
  for _, c in ipairs(LIST) do
    if c == code then found = true else codes[#codes + 1] = c end
  end
  if not found then codes[#codes + 1] = code end
  return modes.set_codes(codes)
end

function modes.allows(hero, group, melee)
  if restriction ~= nil and hero ~= nil and not restriction(hero) then return false end
  if FLAGS.aa and group ~= 2 then return false end
  if FLAGS.ai and group ~= 3 then return false end
  if FLAGS.as and group ~= 1 then return false end
  if FLAGS.ro and melee then return false end
  if FLAGS.mo and not melee then return false end
  return true
end

function modes.duplicates() return FLAGS.du == true or FLAGS.sh == true end

function modes.respawn(secs)
  if FLAGS.fr then return math.floor(secs / 2) end
  return secs
end

function modes.assign(slots, host, roster, group_of, melee_of)
  local pool = {}
  for _, h in ipairs(roster) do
    if modes.allows(h, group_of(h), melee_of(h)) then pool[#pool + 1] = h end
  end
  if #pool == 0 then return end
  if FLAGS.sh then
    local hero = nil
    for _, s in ipairs(slots) do
      if s.owner == host and s.hero ~= nil then hero = s.hero end
    end
    if hero == nil then hero = pool[roll(#pool) + 1] end
    for _, s in ipairs(slots) do
      if s.owner ~= nil then s.hero = hero end
    end
    return
  end
  if not FLAGS.ar then return end
  local used = {}
  for _, s in ipairs(slots) do
    if s.owner ~= nil then
      local free = {}
      for _, h in ipairs(pool) do
        if not used[h] then free[#free + 1] = h end
      end
      if #free == 0 then free = pool end
      local hero = free[roll(#free) + 1]
      used[hero] = true
      s.hero = hero
    end
  end
end

local DISPLAY = {
  ar = "All Random", aa = "All Agility", ai = "All Intelligence", as = "All Strength", ro = "Range Only", mo = "Melee Only",
  du = "Duplicate Mode", sh = "Same Hero", sc = "Super Creeps", fr = "Fast Respawn", np = "No Powerups", em = "Easy Mode", mi = "Mini Heroes", id = "Item Drop",
  cm = "Captains Mode", cd = "Captains Draft", rd = "Random Draft", sd = "Single Draft", rv = "Reverse",
}
local announced = false

modes.BUTTONS = { "ar", "aa", "ai", "as", "ro", "mo", "du", "sh", "sc", "fr", "np", "em", "mi", "id", "cm", "cd", "rd", "sd", "rv" }

function modes.on_tick()
  if announced or game.ticks() < 1 then return end
  announced = true
  local names = ""
  for _, c in ipairs(LIST) do
    names = names .. (DISPLAY[c] or c) .. " "
  end
  if names == "" then
    game.message(-1, "Game mode automatically set to All Pick", 20)
  else
    game.message(-1, "Game mode: " .. names, 20)
  end
end

return modes

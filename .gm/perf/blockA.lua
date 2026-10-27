-- Unit kinds for the class order: HERO for a hero, STRUCTURE for a structure, 0 for a plain unit. kind_of reads a unit's
-- kind once a tick (bucket_units empties KIND), and AGGRO is the rule aggro_priority, read that same tick.
local HERO, STRUCTURE = 2, 1
local AGGRO = nil
local KIND = {}

local function kind_of(u)
  local k = KIND[u]
  if k == nil then
    k = (units.is_hero(u) and HERO or 0) + (units.is_structure(u) and STRUCTURE or 0)
    KIND[u] = k
  end
  return k
end

-- Priority class of a candidate of kind `cand_kind` for an attacker of kind `att_kind`; lower is preferred. A unit
-- attacker takes plain units, then units, then structures. An armed structure takes its aggro target while it is held
-- (`held`), then plain units, units, structures. Within one class the nearest target wins. Off unless the rule
-- aggro_priority is 1.
local function class_order(forced, att_kind, cand_kind, held)
  if forced >= 0 then return forced end
  if AGGRO ~= 1 then return 0 end
  if (att_kind & HERO) ~= 0 then return 0 end
  local cand_hero = (cand_kind & HERO) ~= 0
  if (att_kind & STRUCTURE) ~= 0 then
    if held then return 0 end
    if cand_kind == 0 then return 1 end
    if cand_hero then return 2 end
    return 3
  end
  if cand_kind == 0 then return 0 end
  if cand_hero then return 1 end
  return 2
end

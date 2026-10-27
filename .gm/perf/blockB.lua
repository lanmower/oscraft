-- One tick reads every living unit's position and player once and buckets the living by player and by the cell they
-- stand in, so a choice scans only the players its unit is hostile to. The cells hold the same candidates in another
-- order; the choice keeps the least (class, distance, id), so that order never changes the pick.
local CELL = 262144                    -- the side of a bucket square, in fixed point
local PX, PY = {}, {}                  -- the position of each living unit, as this tick read it
local GRID = {}                        -- player -> cell column -> cell row -> the living ids of that player in the cell
local PRESENT = {}                     -- the players with a living unit this tick
local FOES = {}                        -- player -> the players it is hostile to, read once a tick
local BUCKETS_FOR = -1                 -- the tick the buckets were built for

-- Buckets the living units by player and cell, once a tick, and reads the rule the class order needs. Only a step moves
-- a unit, so the tick's own read is the scan's.
local function bucket_units()
  local tick = game.ticks()
  if tick == BUCKETS_FOR then return end
  BUCKETS_FOR = tick
  GRID, PRESENT, FOES, KIND = {}, {}, {}, {}
  AGGRO = config.rule("aggro_priority")
  for u = 0, get_unit_count() - 1 do
    if unit_alive(u) then
      local x, y = unit_position(u)
      PX[u], PY[u] = x, y
      local p = unit_player(u)
      local grid = GRID[p]
      if grid == nil then
        grid = {}
        GRID[p] = grid
        PRESENT[#PRESENT + 1] = p
      end
      local cx, cy = math.floor(x / CELL), math.floor(y / CELL)
      local col = grid[cx]
      if col == nil then col = {} grid[cx] = col end
      local cell = col[cy]
      if cell == nil then cell = {} col[cy] = cell end
      cell[#cell + 1] = u
    end
  end
end

-- The players `p` is hostile to among those with a living unit this tick, by the sim's own hostility.
local function foes_of(p)
  local list = FOES[p]
  if list == nil then
    list = {}
    for _, q in ipairs(PRESENT) do
      if units.hostile(p, q) then list[#list + 1] = q end
    end
    FOES[p] = list
  end
  return list
end

-- The target `unit` takes out of the cells its reach covers, among the players it is hostile to: the sim's own terms
-- (can_target) and the class order decide who may be struck, the nearest within a class wins, and the lowest id wins a
-- tie of class and distance.
local function best_in_reach(unit, p, x, y, range, aggro)
  range = math.max(range, 0)
  local r2, best, best_class, best_d = range * range, -1, nil, nil
  local forced, att_kind = nil, nil
  local cx0, cx1 = math.floor((x - range) / CELL), math.floor((x + range) / CELL)
  local cy0, cy1 = math.floor((y - range) / CELL), math.floor((y + range) / CELL)
  for _, q in ipairs(foes_of(p)) do
    local grid = GRID[q]
    for cx = cx0, cx1 do
      local col = grid[cx]
      if col ~= nil then
        for cy = cy0, cy1 do
          local cell = col[cy]
          if cell ~= nil then
            for _, j in ipairs(cell) do
              local dx, dy = PX[j] - x, PY[j] - y
              local d = dx * dx + dy * dy
              if d <= r2 and game.can_target(unit, j) then
                if att_kind == nil then forced, att_kind = game.class_override(unit), kind_of(unit) end
                local class = class_order(forced, att_kind, kind_of(j), aggro == j)
                if best_class == nil or class < best_class
                  or (class == best_class and (d < best_d or (d == best_d and j < best))) then
                  best, best_class, best_d = j, class, d
                end
              end
            end
          end
        end
      end
    end
  end
  return best
end

-- The enemy `unit` strikes this tick, -1 for none. A unit that deals no damage strikes nothing. A taunter in reach is
-- struck first; then the enemy struck last while it stays valid (sticky); otherwise the best class in reach
-- (class_order), the nearest within a class, the lowest id on a tie.
function targeting.strike_choice(unit)
  local damage, range = game.weapon(unit)
  if damage <= 0 then return -1 end
  local taunter = game.taunter(unit)
  if taunter ~= nil and within(unit, taunter, range) and game.can_target(unit, taunter) then return taunter end
  local last = game.strike_of(unit)
  if last >= 0 and unit_alive(last) and within(unit, last, range) and game.can_target(unit, last) then return last end
  bucket_units()
  local x, y = unit_position(unit)
  return best_in_reach(unit, unit_player(unit), x, y, range, game.aggro_target(unit))
end

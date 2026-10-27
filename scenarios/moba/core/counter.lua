local counter = {}

local function store(get, set)
  return {
    ready = function(key, now) return get(key) <= now end,
    arm = function(key, tick) set(key, tick) end,
    get = get,
    set = set,
  }
end

function counter.table()
  local t = {}
  return store(function(k) return t[k] or 0 end, function(k, v) t[k] = v end)
end

function counter.sim(name)
  return store(function(u) return game.counter(u, name) end, function(u, v) game.set_counter(u, name, v) end)
end

return counter

-- Mod contract: on_tick(state) runs once per host tick.
-- `state` is a plain table the host owns; the mod may read and write it.
-- spawn_unit and get_unit_count are host functions; positions are Q16.16.

local KINDS = { "skirmisher", "warden" }

function on_tick(state)
  state.ticks = state.ticks + 1
  local n = state.ticks
  if n % 10 == 0 then
    local kind = KINDS[(n // 10) % #KINDS + 1]
    spawn_unit(kind, (n % 32) * 65536, ((n * 3) % 24) * 65536)
  end
end

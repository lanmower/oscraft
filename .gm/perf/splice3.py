import sys
src_path, out_path = sys.argv[1], sys.argv[2]
def read(p): return open(p, encoding="utf-8").read()
src = read(src_path)
a0 = src.index("-- Priority class of candidate"); a1 = src.index("-- Every armed non-unit unit")
src = src[:a0] + read(".gm/perf/blockA2.lua") + "\n\n" + src[a1:]
b0 = src.index("-- One tick reads every living unit"); b1 = src.index("-- Hook handlers:")
src = src[:b0] + read(".gm/perf/blockB3.lua") + "\n\n" + src[b1:]
old = "function targeting.on_tick()\n  for u = 0"
assert src.count(old) == 1, src.count(old)
new = "function targeting.on_tick()\n  if BUCKETS_FOR ~= game.ticks() then\n    BUCKETS_FOR = game.ticks()\n    AGGRO = config.rule(\"aggro_priority\")\n    KIND = {}\n  end\n  for u = 0"
src = src.replace(old, new, 1)
open(out_path, "w", encoding="utf-8").write(src)
print("splice3 ok", len(src))

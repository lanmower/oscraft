import sys
root = sys.argv[1]
def read(p): return open(p, encoding="utf-8").read()
def write(p, s): open(p, "w", encoding="utf-8").write(s)
def one_sub(src, old, new):
    assert src.count(old) == 1, ("anchor count", src.count(old), old[:60])
    return src.replace(old, new, 1)
# targeting.lua (root = a pack dir)
t = root + "/targeting.lua"
src = read(t)
a0 = src.index("-- Priority class of candidate"); a1 = src.index("-- Every armed non-unit unit")
src = src[:a0] + read(".gm/perf/blockA2.lua") + "\n\n" + src[a1:]
b0 = src.index("-- One tick reads every living unit"); b1 = src.index("-- Hook handlers:")
src = src[:b0] + read(".gm/perf/blockB2.lua") + "\n\n" + src[b1:]
src = one_sub(src, "function targeting.on_tick()\n  for u = 0", "function targeting.on_tick()\n  AGGRO = config.rule(\"aggro_priority\")\n  KIND = {}\n  for u = 0")
src = one_sub(src, "function targeting.on_state_replaced()\n  BUCKETS_FOR = -1\nend", "function targeting.on_state_replaced()\n  KIND = {}\nend")
write(t, src)
print("targeting ok", len(src))

import sys
def read(p): return open(p, encoding="utf-8").read()
def write(p, s): open(p, "w", encoding="utf-8").write(s)
def one_sub(src, old, new):
    assert src.count(old) == 1, ("anchor count", src.count(old), old[:60])
    return src.replace(old, new, 1)
p = "/workspace/oscraft/rust/crates/mod-host/src/piccolo_host.rs"
src = read(p)
anchor = '    native!(ctx, game, "units_near", [sh], |c, s| {\n'
src = one_sub(src, anchor, read("/workspace/oscraft/.gm/perf/strike_reach.rs.txt") + anchor)
write(p, src)
d = "/workspace/oscraft/docs/SCRIPTING.md"
doc = read(d)
idx = doc.index("| `game.units_near(x, y, r)` |")
line_start = doc.rfind("\n", 0, idx) + 1
row = "| `game.strike_reach(unit, r)` | the enemies `unit` can strike this tick whose centre is within raw Q16.16 `r` of it, as a flat list of unit id and squared distance pairs |\n"
doc = doc[:line_start] + row + doc[line_start:]
write(d, doc)
t = "/workspace/oscraft/scenarios/moba/targeting.lua"
print("ok-doc")

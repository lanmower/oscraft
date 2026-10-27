import sys
src_path, dst_path = sys.argv[1], sys.argv[2]
src = open(src_path, encoding="utf-8").read()
anchor = '    native!(ctx, game, "units_near", [sh], |c, s| {\n'
assert src.count(anchor) == 1, src.count(anchor)
new = open("/workspace/oscraft/.gm/perf/strike_reach.rs.txt", encoding="utf-8").read()
src = src.replace(anchor, new + anchor, 1)
open(dst_path, "w", encoding="utf-8").write(src)
print("inserted", len(src))

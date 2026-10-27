import sys
target, block_a, block_b = sys.argv[1], sys.argv[2], sys.argv[3]
src = open(target, encoding="utf-8").read()
a0 = src.index("-- Priority class of candidate")
a1 = src.index("-- Every armed non-unit unit")
src = src[:a0] + open(block_a, encoding="utf-8").read() + "\n\n" + src[a1:]
b0 = src.index("-- One tick reads every living unit")
b1 = src.index("-- Hook handlers:")
src = src[:b0] + open(block_b, encoding="utf-8").read() + "\n\n" + src[b1:]
open(target, "w", encoding="utf-8").write(src)
print("spliced", len(src))

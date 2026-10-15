# Map grid. grid(width, height, cell_milli=, fill=) runs once.
# paint(x0, y0, x1, y1, walk=[...]) is half-open; walk names are "blocked" and "cliff"
# (empty or omitted means walkable). Cells are overwritten, not OR-ed.
grid(32, 24, cell_milli=1000)
paint(0, 0, 32, 1, walk=["blocked"])
paint(8, 8, 12, 12, walk=["cliff"])
paint(12, 8, 14, 10, walk=["blocked", "cliff"])

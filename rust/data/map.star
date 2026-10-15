# Map grid. grid(width, height, cell_milli=, fill=) runs once.
# paint(x0, y0, x1, y1, value) is half-open. Cell bits: 1 blocked, 2 cliff.
# spawn(unit, player, x, y) places a unit at milli-unit coordinates on the grid.
grid(32, 24, cell_milli=1000)
paint(0, 0, 32, 1, 1)
paint(8, 8, 12, 12, 2)
paint(12, 8, 14, 10, 3)
spawn("skirmisher", 0, 4000, 4000)
spawn("warden", 0, 6000, 5000)
spawn("skirmisher", 1, 26000, 19000)
spawn("warden", 1, 28000, 20000)

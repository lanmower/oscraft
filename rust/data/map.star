# Map grid. grid(width, height, cell_milli=, fill=) runs once.
# paint(x0, y0, x1, y1, value) is half-open. Cell bits: 1 blocked, 2 cliff.
# spawn(unit, player, x, y) places a unit at milli-unit coordinates on the grid.
# start(player, x, y) places a player start location. spawn(unit, -1, x, y) is neutral.
# limit(per_cell=) caps the units in one grid cell; 0 or no call means no limit.
grid(32, 24, cell_milli=1000)
paint(0, 0, 32, 1, 1)
paint(8, 8, 12, 12, 2)
paint(12, 8, 14, 10, 3)
limit(per_cell=4)
# height_step(milli) sets the elevation of one level; paint_height(x0, y0, x1, y1, level) sets levels 0..=255.
height_step(500)
paint_height(0, 0, 32, 24, 1)
paint_height(12, 8, 14, 10, 6)
start(0, 4000, 4000)
start(1, 26000, 19000)
spawn("skirmisher", 0, 4000, 4000)
spawn("warden", 0, 6000, 5000)
spawn("skirmisher", 1, 26000, 19000)
spawn("warden", 1, 28000, 20000)
spawn("spore", -1, 16000, 12000)

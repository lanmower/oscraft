# Map for the duel-routes scenario: two sides, three routes, one objective per route.
# grid(fill=1) starts every cell blocked; paint(..., 0) opens a route. Half-open rectangles.
grid(40, 24, cell_milli=1000, fill=1)
paint(2, 3, 38, 6, 0)
paint(2, 11, 38, 14, 0)
paint(2, 19, 38, 22, 0)
paint(2, 3, 4, 22, 0)
paint(36, 3, 38, 22, 0)
start(0, 4000, 12000)
start(1, 36000, 12000)
# Route north (y 3..5): objective beacon owned by side 1, a side 0 skirmisher and a side 1 warden.
spawn("beacon", 1, 20000, 4500)
spawn("skirmisher", 0, 18000, 4500)
spawn("warden", 1, 22000, 4500)
# Route middle (y 11..13).
spawn("beacon", 1, 20000, 12500)
spawn("skirmisher", 0, 18000, 12500)
spawn("warden", 1, 22000, 12500)
# Route south (y 19..21).
spawn("beacon", 1, 20000, 20500)
spawn("skirmisher", 0, 18000, 20500)
spawn("warden", 1, 22000, 20500)
# Workers for the economy, one per side.
spawn("tender", 0, 6000, 12500)
spawn("tender", 1, 34000, 12500)

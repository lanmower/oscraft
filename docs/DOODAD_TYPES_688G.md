# Doodad types 6.88g: inferred shipped kinds

Source: scenarios/moba/doodad-types-688g.tsv (57 unmatched types, 1330 records; 4 tree types are matched by the tree rule and are out of scope). The archive names none of these codes, so each kind is inferred from placement context and the model footprint, not from a name.

## Method

1. Family: grouped by the first letter of the code. The second-letter convention could not be checked against the archive, so families are lexical groups and do not drive any assignment.
2. Context per type (scripts: the inference run, not committed): share of records within 2 cells of a lane route (routes.osd), median distance to a base spawn in cells, share of records within 1 cell of a tree record (baseline across all non-tree records: 0.09), and nearest-neighbour spacing in cells (clustering).
3. Role: lane-edge (median lane distance <= 10 cells and >= 10% of records within 2 cells) gives rock; base-side (median base distance <= 35 cells) gives building; tree-side (>= 20% near trees) gives nature; otherwise prop.
4. Model: the shipped kind of that role (its model role from scripts/doodad-roles.mjs) whose footprint in js/client/doodad_sizes.json is closest, in log scale, to the role target footprint times the mean record scale. Role targets: rock 1.5, nature 1.2, prop 1.0, building 6 cells.
5. Confidence: high = two or more signals; medium = one signal; low = no signal (default prop). Mean record scale is never a confidence signal.

Exclusions kept: tree records are placed by the tree rule (unchanged). Non-tree records within 2 cells of a lane route are not placed (70 records); records within 8 cells of a base spawn are not placed (0 records). 1260 of 1330 records are placed.

## Families

- `A`: 10 types (APms, AOsr, APtv, AOnt, AObd, ARrk, AWfs, ASbc, AOgs, AObr)
- `C`: 6 types (CObl, CPms, CPct, CRrk, CSbl, CPlp)
- `I`: 1 types (IOst)
- `J`: 1 types (JOgr)
- `L`: 19 types (LOth, LOsk, LOtz, LWw0, LOic, LOsm, LOfl, LObz, LPfp, LOh1, LTrc, LOo2, LOpg, LOlp, LPwh, LOgr, LOsp, LOca, LOsc)
- `N`: 9 types (NObt, NOfl, NOfp, NOfg, NOtb, NRfs, NWrd, NWsd, NSct)
- `O`: 1 types (OOsd)
- `V`: 1 types (VOfl)
- `Y`: 2 types (YTpb, YTlb)
- `Z`: 7 types (ZPsh, ZPfw, ZRrk, ZWfs, ZWbg, ZWsf, ZPru)

## Assignments

| Type | Records | Role | Signals | Kind | Footprint (cells) | Confidence |
|---|---|---|---|---|---|---|
| ZPsh | 419 | prop | cluster | bench | 0.994 | medium |
| ZPfw | 165 | prop | none | bench | 0.994 | low |
| YTpb | 109 | building | base+cluster | fountain | 4 | high |
| CObl | 88 | prop | none | bench | 0.994 | low |
| APms | 78 | nature | tree | grass_large | 0.691 | medium |
| YTlb | 48 | prop | cluster | bench | 0.994 | medium |
| AOsr | 36 | prop | none | bench | 0.994 | low |
| APtv | 35 | nature | tree | grass_large | 0.691 | medium |
| LOth | 30 | prop | none | hay_bale_bundled | 1.035 | low |
| LOsk | 29 | prop | none | conveyor_bars_fence | 0.972 | low |
| AOnt | 26 | rock | lane | rocks | 0.566 | medium |
| LOtz | 22 | prop | none | hay_bale | 1.11 | low |
| LWw0 | 18 | prop | cluster | catwalk_junction | 1.215 | medium |
| LOic | 17 | prop | none | cart_high | 1.047 | low |
| NObt | 15 | prop | none | hay_bale_bundled | 1.035 | low |
| ZRrk | 13 | prop | cluster | campfire_stand | 0.946 | medium |
| LOsm | 12 | prop | none | bench | 0.994 | low |
| CPms | 12 | prop | none | hay_bale_bundled | 1.035 | low |
| AObd | 11 | prop | none | bench | 0.994 | low |
| ARrk | 10 | prop | none | campfire_stand | 0.946 | low |
| LOfl | 9 | prop | none | cart_high | 1.047 | low |
| JOgr | 9 | prop | cluster | campfire_stand | 0.946 | medium |
| LObz | 7 | prop | cluster | iron_fence_border_gate | 0.853 | medium |
| LPfp | 7 | prop | none | machine_bed | 1.173 | low |
| VOfl | 6 | building | base+cluster | fountain | 4 | high |
| LOh1 | 6 | rock | lane | rocks | 0.566 | medium |
| CPct | 6 | prop | none | cart_high | 1.047 | low |
| LTrc | 6 | prop | none | bench | 0.994 | low |
| NOfl | 6 | prop | none | catwalk_junction | 1.215 | low |
| CRrk | 6 | prop | cluster | cart_high | 1.047 | medium |
| LOo2 | 6 | prop | none | trunk | 0.798 | low |
| ZWfs | 6 | prop | none | hay_bale | 1.11 | low |
| AWfs | 6 | prop | none | bench_damaged | 1.136 | low |
| LOpg | 6 | prop | none | hay_bale | 1.11 | low |
| CSbl | 5 | prop | none | catwalk_junction | 1.215 | low |
| NOfp | 4 | prop | none | catwalk_stairs_loop | 0.904 | low |
| NOfg | 3 | prop | none | iron_fence_border_gate | 0.853 | low |
| LOlp | 3 | prop | none | machine_bed | 1.173 | low |
| NOtb | 3 | prop | none | campfire_fishing_stand | 0.926 | low |
| ASbc | 3 | prop | none | hay_bale_bundled | 1.035 | low |
| NRfs | 2 | prop | none | bench | 0.994 | low |
| IOst | 2 | building | base | fountain | 4 | medium |
| LPwh | 2 | nature | tree | grass_large | 0.691 | medium |
| ZWbg | 2 | prop | none | machine_bed | 1.173 | low |
| LOgr | 2 | prop | none | catwalk_stairs_loop | 0.904 | low |
| CPlp | 2 | prop | none | catwalk_stairs_loop | 0.904 | low |
| OOsd | 2 | prop | none | catwalk_junction | 1.215 | low |
| LOsp | 1 | prop | none | conveyor_bars_high_slope | 0.82 | low |
| NWrd | 1 | prop | none | bench | 0.994 | low |
| NWsd | 1 | prop | none | conveyor_bars_high_slope | 0.82 | low |
| LOca | 1 | prop | none | bench | 0.994 | low |
| NSct | 1 | prop | none | cart_high | 1.047 | low |
| ZWsf | 1 | prop | none | bench | 0.994 | low |
| LOsc | 1 | prop | none | campfire_stand | 0.946 | low |
| ZPru | 1 | prop | none | dumpster | 1.163 | low |
| AOgs | 1 | building | base | fountain | 4 | medium |
| AObr | 1 | prop | none | bench_damaged | 1.136 | low |

Confidence: 2 high, 14 medium, 41 low. Most types have no context signal (their placements are scattered mid-map, far from bases and trees), so most assignments are low confidence prop placements chosen by footprint.

## Placed per model

| Kind | Records assigned |
|---|---|
| bench | 790 |
| fountain | 118 |
| grass_large | 115 |
| hay_bale_bundled | 60 |
| cart_high | 39 |
| hay_bale | 34 |
| campfire_stand | 33 |
| rocks | 32 |
| catwalk_junction | 31 |
| conveyor_bars_fence | 29 |
| machine_bed | 12 |
| iron_fence_border_gate | 10 |
| catwalk_stairs_loop | 8 |
| bench_damaged | 7 |
| trunk | 6 |
| campfire_fishing_stand | 3 |
| conveyor_bars_high_slope | 2 |
| dumpster | 1 |

## Regeneration

- the archive decoder removed in the rename commit writes doodad-types-688g.tsv and keeps the shipped_kind of each type across runs.
- scripts/place-doodads-688g.mjs (called by the generator) appends the mapped doodad lines (id 10000 and up) to map.osd from doodads-688g.tsv, applying the lane and base exclusions in scripts/doodad-kinds.mjs.
- Full archive re-decode was not run here (no extracted refmap.* members in this environment). The placement stage was run from the committed decoded placements, which the generator also writes.

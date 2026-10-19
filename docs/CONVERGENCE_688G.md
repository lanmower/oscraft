# DotA Allstars 6.88g: convergence report

Subject: `/workspace/oscraft/DotA v6.88g Allstars.w3x` (DotA Allstars 6.88g), compared with the MOBA scenario data in `scenarios/moba/`. Every value below is labelled DotA Allstars 6.88g. No game code or scenario data was changed.

## 1. Archive inventory

- File size: 41,467,779 bytes. MPQ format version 1, header at offset 0, sector size 4096, hash table 65,536 entries, block table 2,328 valid blocks.
- Member names: the `(listfile)` member lists 2,326 names. All 2,328 valid blocks map to a listed name or a standard name (`(listfile)`, `(attributes)`, `(signature)`).
- Extracted: 2,284 members read through libmpq and a stdlib reader cross-check. Only the first 30 of the 51 read errors were printed in the run. The errors were zero-length `Units\*Func.txt` and `Units\*Strings.txt` entries, which libmpq rejects with a format error. Two members (`NEW_LOADSCREEN.dds`, `SlowTarget.mdl`) use PKWARE implode and were read by libmpq.
- Standard Warcraft III members present: `war3map.w3i` (796 B), `war3map.w3e` (116,568 B), `war3map.doo` (259,494 B), `war3mapUnits.doo` (246 B), `war3map.w3a` (15,336 B, implode), `war3map.w3b` (2,636 B), `war3map.w3d` (5,355 B), `war3map.wts` (20,067 B, implode), `war3map.wpm` (262,160 B), `war3map.shd` (262,144 B), `war3map.wct` (310 B), `war3map.wtg` (567 B), `war3map.w3c` (8 B), `war3map.w3r` (8 B), `war3map.j` (3,065,090 B), `war3map.w3h` (37 B, implode).
- Standard members absent from the archive: `war3map.w3u` (units), `war3map.w3t` (items), `war3map.w3q` (upgrades), `war3map.lua`, `war3map.imp`, `war3map.mmp`, `Scripts\war3map.j`, `Scripts\common.j`, `Scripts\blizzard.j`. The map ships no custom unit or item object data.
- Asset members: about 2,200 textures and models (`war3mapImported\*.mdx`, `*.blp`, `*.mp3`), which are out of scope for this report.

## 2. Extracted tables

- Units (`w3u`): absent. Unit count, hero records and creep records (health, armour, damage, attack period, move speed, range, sight, bounty, cost) cannot be extracted.
- Items (`w3t`): absent. Item count, recipes and cooldowns cannot be extracted.
- Abilities (`w3a`): 16 original-table records, all of them edits of standard ability ids (first ids `A21X`, `A21W`, `A21V`, `A0VV`). Each record carries string modifications such as `atar` (target area) and `atp1`, and numeric modifications such as `aran`, `ahdu`, `adur`, `amcs`. The record stream could not be decoded past the fourth record, so the custom-table count and the per-field values are not reported.
- Upgrades (`w3h`): 1 original record (`BNeg`).
- Destructables (`w3b`): 6 original records.
- Doodads (`w3d`): 42 original records.
- Strings (`wts`): 20,067 bytes of trigger strings (decoded with libmpq; not parsed into a table).

## 3. Comparison with our data

Counts: MATCH 0, DIFFERENT 1, MISSING everything else. The archive has no unit, item or ability stat values that could be checked against our files, so the comparison is mostly MISSING.

| Category | Value (DotA Allstars 6.88g) | Our value | Status |
| --- | --- | --- | --- |
| Map grid | 129 x 129 terrain corners (`w3e`), 128-unit tiles, origin -8192 | `scenarios/moba/map.osd`: `grid(327, 327, cell_milli=1000)`, 64-unit cells | DIFFERENT |
| Tileset | `C` (`w3e` tileset byte) | none recorded in `map.osd` | MISSING |
| Units / heroes / creeps (stats) | no `w3u` in archive | `units.osd` (20,455 B), `units_heroes.osd` (39,212 B), `units_extra.osd` (7,990 B), `neutral.osd` (5,830 B) | MISSING |
| Items (stats, recipes, cooldowns) | no `w3t` in archive | `items.osd` (46,739 B) | MISSING |
| Abilities | 16 original `w3a` edits | `abilities.osd` (11,665 B), `abilities_dota.osd` | MISSING (ids not cross-checked) |
| Upgrades | 1 original `w3h` record | none found in scenario data | MISSING |
| Destructables | 6 original `w3b` records | `map.osd` / `behaviours.osd` | MISSING |
| Doodads | 42 original `w3d` records | `doodad-assets.tsv` | MISSING |
| Lane spawners | JASS-driven, not decoded | `waves.osd`: 6 spawners (3 lanes x 2 sides) | MISSING |
| Map info | `w3i` version 31, map version 5, editor 6112, name/author stored as `TRIGSTR_*` references | none | MISSING |
| Player starts | not decoded from `w3i`/`war3mapunits.doo` | none | MISSING |

## 4. Layout comparison

- Terrain: `war3map.w3e` is 129 x 129 corners (16,641 cells) with origin -8192 on both axes, i.e. a 16,384-unit square at 128 units per tile. Our `map.osd` is 327 x 327 cells at 64 units per cell from the Dota 2 patch 7.41 dump. These are different maps and different scales: DIFFERENT.
- Doodads and placed units: `war3map.doo` is 259,494 B (its header declares the record count, not decoded here). `war3mapUnits.doo` is 246 B, so it holds very few placed units. Lane, tower, barracks, shop and camp positions were not extracted: MISSING.
- Regions and player starts: not decoded: MISSING.

## 5. Trigger script findings (`war3map.j`)

- Size: 102,278 lines, 4,503 functions, 861 `CreateUnit*` calls, 165 `TimerStart` calls, 142 `CreateTimer` calls.
- Obfuscated: the functions use short generated names (`Me`, `MS`, `SIE`, `Ez`, `KZE`). Unit and ability codes are the usual four-character raw ids; 5,755 distinct raw ids appear.
- Wave logic: the only lane-wave hits are `SaveReal(Me,'wave','000x'/'000y', ...)` (about line 22439) and a `-spawncreeps` debug command (about line 31237). The creep wave timers, compositions and upgrade rule could not be isolated from the obfuscated code in this pass. Lane creep waves are MISSING.
- Creep acquisition and attack logic: not isolated: MISSING.

## 6. What could not be read

- `war3map.w3u`, `war3map.w3t`, `war3map.w3q`, `war3map.lua`, `war3map.imp`, `war3map.mmp`, `Scripts\war3map.j`, `Scripts\common.j` and `Scripts\blizzard.j` are not in the archive.
- The `w3a` ability record stream is only decoded through its first four records.
- Placed-unit and doodad positions, player starts and region data are not decoded.
- The obfuscated trigger script was not decoded into wave timings or compositions.

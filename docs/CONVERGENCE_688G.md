# DotA Allstars 6.88g: convergence report

Subject: `/workspace/oscraft/DotA v6.88g Allstars.w3x` (DotA Allstars 6.88g), compared with the MOBA scenario data in `scenarios/moba/`. Every value below is labelled DotA Allstars 6.88g. No game code or scenario data was changed.

Our `scenarios/moba/*.osd` stats are generated from Valve's Dota 2 `npc_units.txt` and `items.txt` (SteamDatabase dump), not from 6.88g. Every comparison below is therefore a cross-game check of the same concept, not a 6.88g-to-6.88g diff.

## 1. Archive inventory

- File size: 41,467,779 bytes. MPQ format version 1, header at offset 0, sector size 4096, hash table 65,536 entries, block table 2,328 valid blocks.
- Member names: the `(listfile)` member lists 2,326 names. Hashing every listed name (and the standard candidates) against the hash table resolves all 2,328 valid blocks: 2,326 listed names plus `(listfile)` and `(attributes)`. No block is unnamed. libmpq's member iterator also yields exactly 2,328 names.
- Readers: libmpq 0.8.1 (ctypes binding over libmpq.so, in `/tmp/mpq688-scripts/venv`) and a stdlib reader (`/tmp/mpq688-analysis/mpqr.py`). Both read the `(listfile)` (98,070 B, plain `\r\n` list). Five members use PKWARE implode (`war3map.w3a`, `war3map.wts`, `war3map.w3h`, `NEW_LOADSCREEN.dds`, `SlowTarget.mdl`); the stdlib reader cannot decode them, libmpq reads all five. 40 `Units\*Func.txt` and `Units\*Strings.txt` members are zero-length (csize 0) and are rejected by libmpq with format error -7. Extracted: 2,288 members with content plus 40 zero-length members.
- StormLib, w3x2lni and ObjectEditor were not built or installed: libmpq and the stdlib reader were sufficient.
- Standard members present: `war3map.w3i` (796 B), `war3map.w3e` (116,568 B), `war3map.doo` (259,494 B), `war3mapUnits.doo` (246 B), `war3map.w3a` (15,336 B), `war3map.w3b` (2,636 B), `war3map.w3d` (5,355 B), `war3map.w3h` (37 B), `war3map.wts` (20,067 B), `war3map.wpm` (262,160 B), `war3map.shd` (262,144 B), `war3map.wct` (310 B), `war3map.wtg` (567 B), `war3map.w3c` (8 B), `war3map.w3r` (8 B), `war3map.j` (3,065,090 B), `war3mapMisc.txt` (1,860 B), `war3mapSkin.txt` (2,006 B), `war3mapImported\templates.toc`.
- Unit, item and ability data are in SLK overrides: `Units\UnitData.slk` (355,210 B), `Units\UnitBalance.slk` (430,025 B), `Units\UnitWeapons.slk` (164,892 B), `Units\UnitAbilities.slk` (114,678 B), `Units\AbilityData.slk` (715,863 B), `Units\UpgradeData.slk` (19,481 B), `units\ItemData.slk` (240,297 B). The lowercase `units\` spelling is also in the archive.
- Standard members absent from the archive: `war3map.w3u` (units), `war3map.w3t` (items), `war3map.w3q` (upgrades), `war3map.lua`, `war3map.imp`, `war3map.mmp`, `Scripts\war3map.j`, `Scripts\common.j`, the base game helper script `Scripts\<helper>.j`. Confirmed: no hash-table entry resolves to any of them under the standard names or the 6.88g candidate names.
- Asset members: about 2,200 textures and models (`war3mapImported\*.mdx`, `*.blp`, `*.mp3`), out of scope for this report.

## 2. Decoded tables

- Units: the object data is in `Units\UnitBalance.slk` (2,032 keyed rows), `Units\UnitData.slk` (2,036 keyed rows) and `Units\UnitWeapons.slk` (480 keyed rows). `war3map.j` calls `CreateUnit*` 737 times with 229 distinct raw ids; 228 of them are rows in UnitBalance. Hero rows (`STR` column set) number 237 in UnitBalance; 6 of the 229 CreateUnit ids are heroes. Stock vs custom rows cannot be separated: the Warcraft III base game stock files are not in the archive.
- Items: `units\ItemData.slk` has 1,169 keyed rows: 412 with `goldcost`, 432 with `abilList`, 21 with `pickRandom`. `war3map.j` calls `CreateItem` with 4 distinct ids (`I00E`, `I00G`, `I02M`, `I0KK`). Recipes are not in the SLK (no recipe column). Cooldowns are an `AbilityData` reference (`cooldownID`), not a number in ItemData.
- Abilities (`war3map.w3a`, version 2): 16 original-table records (`A21X`, `A21W`, `A21V`, `A0VV`, `A0XL`, `A3H0`, `A0V7`, `A0VE`, `A0EF`, `A0NR`, `A0CQ`, `A0AR`, `A087`, `A08C`, `S00A`, `Aamk`) and 1 custom-table record (`ANeg` to `A0CP`, name `TRIGSTR_019`), 572 modifications in total. Each modification is code, type, level, data pointer, value (number, or string), then a 4-byte trailer; the stream ends exactly at the 15,336-byte end of the member. The previous pass stopped at record 4 because it did not read the trailer.
- Upgrades (`war3map.w3h`): 37 bytes, a single record (`BNeg`, buff table).
- Destructables (`war3map.w3b`): 6 original records. Doodads (`war3map.w3d`): 42 original records.
- Strings (`war3map.wts`): 20,067 bytes of trigger strings (libmpq read; not parsed into a table).

## 3. Comparison with our data

Pairing rule: a 6.88g unit is paired with one of ours only when the pairing is stated below. The 6.88g unit names are not in the archive (the `TRIGSTR` strings are not parsed), so the pairs are chosen by the CreateUnit ids and by stat shape, and are marked as guesses.

Values: 6.88g HP, armour and bounty are read directly from the SLK. Damage is `dice + dmgplus` (min) and `dice x sides + dmgplus` (max). Attack period is `cool1` in seconds. Ours are converted from our generated units: 16 ticks per second, 15.625 milli per Dota unit.

Counts: MATCH 6, DIFFERENT 14, MISSING for every ability, item and placed-unit field (no pairing exists).

| Category | Value (DotA Allstars 6.88g) | Our value | Status |
| --- | --- | --- | --- |
| Tower T1 HP | 1800 (`e00S`, `u00N`, `u00D`, 1800 HP) | 1800 (`tower`) | MATCH |
| Tower T1 range | 700 | 700 (10938 milli) | MATCH |
| Tower T1 sight | 1900 | 1900 (29688 milli) | MATCH |
| Tower T1 armour | 22 | 12 | DIFFERENT |
| Tower T1 damage | 120 to 140 (`dice 1 x 21 + 119`) | 90 | DIFFERENT |
| Tower T1 attack period | 0.95 s | 0.8125 s (13 ticks) | DIFFERENT |
| Tower T1 bounty | 200 gold (`e00S`) | not modelled | MISSING |
| Ancient (base) HP | 150000 (`n00C`, `n0DC`, `ncop`: ancient-type rows) | 4500 (`base`) | DIFFERENT |
| Hero HP (pairing guess: `H000`, first CreateUnit hero) | 150 | 120 (`hero`) | DIFFERENT |
| Hero armour (guess: `H000`) | 2 | 0 | DIFFERENT |
| Hero damage (guess: `H000`, `2d2 + 30`) | 32 to 34 | 37 | DIFFERENT |
| Hero attack period (guess: `H000`) | 1.7 s | 1.625 s (26 ticks) | DIFFERENT |
| Hero range (guess: `H000`) | 128 | 150 (2344 milli) | DIFFERENT |
| Hero sight (guess: `H000`) | 1800 | 1800 (28125 milli) | MATCH |
| Hero move speed (guess: `H000`, Dota units/s) | 300 | 310.3 (303 milli per tick) | DIFFERENT |
| Creep HP (pairing guess: `u001`, undead creep, speed 325 and range 100) | 700 | 550 (`creep`) | DIFFERENT |
| Creep armour (guess: `u001`) | 3 | 2 | DIFFERENT |
| Creep damage (guess: `u001`, `1d9 + 35`) | 36 to 44 | 21 | DIFFERENT |
| Creep range (guess: `u001`) | 100 | 100 (1562 milli) | MATCH |
| Creep move speed (guess: `u001`) | 325 | 325 (317 milli per tick) | MATCH |
| Creep sight (guess: `u001`) | 850 | 750 (11719 milli) | DIFFERENT |
| Creep bounty | 22 gold (`u001`) | not modelled | MISSING |
| Items (cost, cooldown, recipe) | 1,169 ItemData rows; recipes not in SLK | `items.osd` (46,739 B) | MISSING (no id pairing) |
| Abilities | 16 original `w3a` edits plus 1 custom (`ANeg`/`A0CP`), 572 mods | `abilities.osd` (11,665 B), `abilities_dota.osd` | MISSING (ids not cross-checked) |
| Upgrades | 1 `w3h` record (`BNeg`) | none found in scenario data | MISSING |
| Destructables | 6 original `w3b` records | `map.osd` / `behaviours.osd` | MISSING |
| Doodads | 42 original `w3d` records | `doodad-assets.tsv` | MISSING |
| Map grid | 129 x 129 terrain corners (`w3e`), 128-unit tiles, origin -8192 | `scenarios/moba/map.osd`: `grid(327, 327, cell_milli=1000)`, 64-unit cells | DIFFERENT |
| Tileset | `C` (`w3e` tileset byte) | none recorded in `map.osd` | MISSING |
| Lane spawners | JASS-driven, not decoded | `waves.osd`: 6 spawners (3 lanes x 2 sides) | MISSING |
| Map info | `w3i` version 31, map version 5, editor 6112, name/author stored as `TRIGSTR_*` references | none | MISSING |
| Player starts | not decoded from `w3i`/`war3mapunits.doo` | none | MISSING |

## 4. Layout comparison

- Terrain: `war3map.w3e` is 129 x 129 corners (16,641 cells) with origin -8192 on both axes, i.e. a 16,384-unit square at 128 units per tile. Our `map.osd` is 327 x 327 cells at 64 units per cell from the Dota 2 patch 7.41 dump. Different maps and scales: DIFFERENT.
- Doodads and placed units: `war3map.doo` is 259,494 B (its header declares the record count, not decoded here). `war3mapUnits.doo` is 246 B, so it holds very few placed units. Lane, tower, barracks, shop and camp positions were not extracted: MISSING.
- Regions and player starts: not decoded: MISSING.

## 5. Trigger script findings (`war3map.j`)

- Size: 102,278 lines, 4,503 functions, 861 `CreateUnit*` calls, 165 `TimerStart` calls, 142 `CreateTimer` calls.
- Obfuscated: the functions use short generated names (`Me`, `MS`, `SIE`, `Ez`, `KZE`). Unit and ability codes are the usual four-character raw ids; 5,755 distinct raw ids appear.
- Wave logic: the only lane-wave hits are `SaveReal(Me,'wave','000x'/'000y', ...)` (about line 22439) and a `-spawncreeps` debug command (about line 31237). The creep wave timers, compositions and upgrade rule could not be isolated from the obfuscated code in this pass. Lane creep waves are MISSING.
- Creep acquisition and attack logic: not isolated: MISSING.

## 6. What could not be read

- `war3map.w3u`, `war3map.w3t`, `war3map.w3q`, `war3map.lua`, `war3map.imp`, `war3map.mmp`, `Scripts\war3map.j`, `Scripts\common.j` and the base game helper script `Scripts\<helper>.j` are not in the archive (confirmed by hashing every listed name and the candidate set).
- Hero and creep names are not resolved: the `TRIGSTR` strings are not parsed, so the pairings in section 3 are guesses.
- Items: recipes are not in the SLK, and cooldown values need the `AbilityData` chain (`cooldownID` to `Cool1`).
- Placed-unit and doodad positions, player starts and region data are not decoded.
- The obfuscated trigger script was not decoded into wave timings or compositions.

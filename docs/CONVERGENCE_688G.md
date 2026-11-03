# reference map 6.88g: convergence report

Subject: `/workspace/oscraft/reference map 6.88g.w3x` (reference map 6.88g; external, not in the repo), compared with the MOBA scenario data in `scenarios/moba/`. Every value below is labelled reference map 6.88g. Sections 7 and 8 (Applied) record the data changes applied so far (69 matched heroes; 32 neutral and Roshan rows); everything else in sections 1 to 6 is still a comparison, not an edit.

Our `scenarios/moba/*.osd` stats are generated from reference ruleset `npc_units.txt` and `items.txt` (SteamDatabase dump), not from 6.88g. Every comparison below is therefore a cross-game check of the same concept, not a 6.88g-to-6.88g diff.

## 1. Archive inventory

- File size: 41,467,779 bytes. MPQ format version 1, header at offset 0, sector size 4096, hash table 65,536 entries, block table 2,328 valid blocks.
- Member names: the `(listfile)` member lists 2,326 names. Hashing every listed name (and the standard candidates) against the hash table resolves all 2,328 valid blocks: 2,326 listed names plus `(listfile)` and `(attributes)`. No block is unnamed. libmpq's member iterator also yields exactly 2,328 names.
- Readers: libmpq 0.8.1 (ctypes binding over libmpq.so, in the scratch venv `/tmp/mpq688-scripts/venv`, not kept) and a stdlib reader (scratch script `/tmp/mpq688-analysis/mpqr.py`, not kept). Both read the `(listfile)` (98,070 B, plain `\r\n` list). Five members use PKWARE implode (`refmap.w3a`, `refmap.wts`, `refmap.w3h`, `NEW_LOADSCREEN.dds`, `SlowTarget.mdl`); the stdlib reader cannot decode them, libmpq reads all five. 40 `Units\*Func.txt` and `Units\*Strings.txt` members are zero-length (csize 0) and are rejected by libmpq with format error -7. Extracted: 2,288 members with content plus 40 zero-length members.
- StormLib, w3x2lni and ObjectEditor were not built or installed: libmpq and the stdlib reader were sufficient.
- Standard members present: `refmap.w3i` (796 B), `refmap.w3e` (116,568 B), `refmap.doo` (259,494 B), `refmapUnits.doo` (246 B), `refmap.w3a` (15,336 B), `refmap.w3b` (2,636 B), `refmap.w3d` (5,355 B), `refmap.w3h` (37 B), `refmap.wts` (20,067 B), `refmap.wpm` (262,160 B), `refmap.shd` (262,144 B), `refmap.wct` (310 B), `refmap.wtg` (567 B), `refmap.w3c` (8 B), `refmap.w3r` (8 B), `refmap.j` (3,065,090 B), `refmapMisc.txt` (1,860 B), `refmapSkin.txt` (2,006 B), `refmapImported\templates.toc`.
- Unit, item and ability data are in SLK overrides: `Units\UnitData.slk` (355,210 B), `Units\UnitBalance.slk` (430,025 B), `Units\UnitWeapons.slk` (164,892 B), `Units\UnitAbilities.slk` (114,678 B), `Units\AbilityData.slk` (715,863 B), `Units\UpgradeData.slk` (19,481 B), `units\ItemData.slk` (240,297 B). The lowercase `units\` spelling is also in the archive.
- Standard members absent from the archive: `refmap.w3u` (units), `refmap.w3t` (items), `refmap.w3q` (upgrades), `refmap.lua`, `refmap.imp`, `refmap.mmp`, `Scripts\refmap.j`, `Scripts\common.j`, `Scripts\<stock>.j`. Confirmed: no hash-table entry resolves to any of them under the standard names or the 6.88g candidate names.
- Asset members: about 2,200 textures and models (`refmapImported\*.mdx`, `*.blp`, `*.mp3`), out of scope for this report.

## 2. Decoded tables

- Units: the object data is in `Units\UnitBalance.slk` (2,032 keyed rows), `Units\UnitData.slk` (2,036 keyed rows) and `Units\UnitWeapons.slk` (480 keyed rows). `refmap.j` calls `CreateUnit*` 737 times with 229 distinct raw ids; 228 of them are rows in UnitBalance. Hero rows (`STR` column set) number 237 in UnitBalance; 6 of the 229 CreateUnit ids are heroes. Stock vs custom rows cannot be separated: the stock engine files are not in the archive.
- Items: `units\ItemData.slk` has 1,169 keyed rows: 412 with `goldcost`, 432 with `abilList`, 21 with `pickRandom`. `refmap.j` calls `CreateItem` with 4 distinct ids (`I00E`, `I00G`, `I02M`, `I0KK`). Recipes are not in the SLK (no recipe column). Cooldowns are an `AbilityData` reference (`cooldownID`), not a number in ItemData.
- Abilities (`refmap.w3a`, version 2): 16 original-table records (`A21X`, `A21W`, `A21V`, `A0VV`, `A0XL`, `A3H0`, `A0V7`, `A0VE`, `A0EF`, `A0NR`, `A0CQ`, `A0AR`, `A087`, `A08C`, `S00A`, `Aamk`) and 1 custom-table record (`ANeg` to `A0CP`, name `TRIGSTR_019`), 572 modifications in total. Each modification is code, type, level, data pointer, value (number, or string), then a 4-byte trailer; the stream ends exactly at the 15,336-byte end of the member. The previous pass stopped at record 4 because it did not read the trailer.
- Upgrades (`refmap.w3h`): 37 bytes, a single record (`BNeg`, buff table).
- Destructables (`refmap.w3b`): 6 original records. Props (`refmap.w3d`): 42 original records.
- Strings (`refmap.wts`): 125 trigger-string entries (`STRING n`); no unit names. Unit display names are in `units/OrcAbilityStrings.txt` (`[code]` sections with `name=`), see section 3.

## 3. Name-resolved comparison

Values are reference map 6.88g unless marked "ours". Each 6.88g value is read from the SLK field named in the table legend; "ours" is our `scenarios/moba/*.osd` value, converted to reference units (1 reference unit = 15.625 milli; speed in reference units/s = milli-per-tick x 16 / 15.625; period in seconds = ticks / 16; range and sight in reference units = milli / 15.625).

Name source: `units/OrcAbilityStrings.txt` (2,301,368 B) holds `[code]` sections with `name=`. 6,019 sections carry a name; 1,945 of the 2,032 `Units\UnitBalance.slk` rows are named. The SLK name fields are empty: `Units\unitUI.slk` has a `name` column with no value in any row, and no SLK cell holds a `TRIGSTR_` reference. `refmap.wts` (125 entries) has no unit names: it holds the game title, credits and command help.

Pairing rule: our unit key is matched to a 6.88g display name by exact normalised name (lowercase, letters and digits only), with the aliases listed below. Hero keys prefer a UnitBalance row with `STR` set; neutral keys prefer rows without `STR`. A name whose only row is a placeholder (no `UnitWeapons` dice1, and `spd` <= 1) is not paired: it goes to the shell list.

Aliases (ours to 6.88g name): `bane` to BE, `zuus` to Zeus, `rattletrap` to Clockwerk, `riki` to Rikimaru, `tusk` to TUS, `brewmaster` to PB, `neutral_furbolg_champion` to Polar Furbolg Champion, `neutral_furbolg_ursa_warrior` to Polar Furbolg URS Warrior, `juggernaut_healing_ward` to Healing Ward.

Counts (name-resolved pairs): 102 pairs (70 heroes, 31 neutral creeps, 1 other: Roshan). Field comparisons: 714: MATCH 323, DIFFERENT 366, MISSING 25. MISSING means one side has no value (our armour is unset on some units; 6.88g sight is absent on some rows). Ambiguous: 58 pairs have more than one 6.88g code with the same name; the code is chosen by the rule above, not verified against the trigger code, so those pairs are provisional.

| Field | 6.88g SLK field | MATCH | DIFFERENT | MISSING |
| --- | --- | ---: | ---: | ---: |
| HP | UnitBalance.HP | 25 | 77 | 0 |
| armour | UnitBalance.def | 24 | 54 | 24 |
| damage (midpoint) | UnitWeapons.dice1/sides1/dmgplus1 | 32 | 70 | 0 |
| attack period | UnitWeapons.cool1 | 55 | 47 | 0 |
| move speed | UnitBalance.spd | 46 | 56 | 0 |
| range | UnitWeapons.rangeN1 | 45 | 57 | 0 |
| sight | UnitBalance.sight | 96 | 5 | 1 |

Largest DIFFERENT values (relative to the 6.88g value; values labelled reference map 6.88g):

| Unit (ours) | 6.88g name (code) | Field | reference map 6.88g | ours | 6.88g source |
| --- | --- | --- | ---: | ---: | --- |
| roshan | Roshan (n00L) | armour | 6 | 30 | UnitBalance.def |
| lone_druid | LD (N014) | range | 128 | 550 | UnitWeapons.rangeN1 |
| neutral_dark_troll_warlord | Dark TW (ndtw) | armour | 1 | 4 | UnitBalance.def |
| legion_commander | LC (E02K) | armour | 1 | -1 | UnitBalance.def |
| enchantress | ENC (UIN1) | armour | -1 | 1 | UnitBalance.def |
| broodmother | BRO (TX78) | armour | 1 | -1 | UnitBalance.def |
| beastmaster | BEA (H00D) | armour | 3 | -1 | UnitBalance.def |
| treant | TRE (e00V) | sight | 850 | 1800 | UnitBalance.sight |
| venomancer | VEN (EC57) | armour | 1 | 0 | UnitBalance.def |
| vengefulspirit | VS (UAG2) | armour | 1 | 0 | UnitBalance.def |
| undying | UND (H00R) | armour | 3 | 0 | UnitBalance.def |
| tidehunter | TID (UST3) | armour | 2 | 0 | UnitBalance.def |
| templar_assassin | TA (E01Y) | armour | 2 | 0 | UnitBalance.def |
| spectre | SPE (E01B) | armour | 1 | 0 | UnitBalance.def |
| skywrath_mage | SM (H0DO) | armour | -1 | -2 | UnitBalance.def |

Name-resolved pairs (cell = reference map 6.88g / ours, then = match, ! different, ? missing). Fields: HP, armour, damage, attack period (s), move speed (reference units/s), range, sight.

| Ours | 6.88g name (code) | HP | armour | damage | period | speed | range | sight |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| neutral_alpha_wolf | Alpha Wolf (n00S) | 600 / 600 = | 3 / 3 = | 31.5 / 28 ! | 1.35 / 1.3125 ! | 350 / 350.2 = | 90 / 90 = | 800 / 800 = |
| neutral_black_dragon | Black Dragon (nbwm) | 2000 / 2000 = | 4 / 4 = | 64.5 / 65 = | 1.5 / 1.5 = | 300 / 300 = | 500 / 300 ! | 800 / 800 = |
| neutral_black_drake | Black Drake (nbdk) | 950 / 950 = | - / 2 ? | 39.5 / 21 ! | 1.8 / 1.6875 ! | 350 / 350.2 = | 500 / 300 ! | 800 / 800 = |
| neutral_centaur_khan | CEN Khan (ncnk) | 1100 / 1100 = | 4 / 4 = | 52 / 52 = | 1.5 / 1.5 = | 320 / 319.5 = | 100 / 100 = | 800 / 800 = |
| neutral_centaur_outrunner | CEN Outrunner (ncen) | 550 / 350 ! | 2 / 1 ! | 19.5 / 19 = | 1.3 / 1.3125 = | 350 / 319.5 ! | 100 / 100 = | 800 / 800 = |
| neutral_dark_troll | Dark Troll (ndtr) | 500 / 500 = | - / 0 ? | 25.5 / 26 = | 1.35 / 1.3125 ! | 270 / 270.3 = | 500 / 250 ! | 800 / 800 = |
| neutral_dark_troll_warlord | Dark TW (ndtw) | 1100 / 1100 = | 1 / 4 ! | 51.5 / 42 ! | 1.35 / 1.3125 ! | 320 / 300 ! | 500 / 250 ! | 800 / 800 = |
| neutral_elder_jungle_stalker | Elder Jungle Stalker (njga) | 2000 / 2000 = | 4 / 5 ! | 67 / 67 = | 1.35 / 1.375 ! | 320 / 319.5 = | 128 / 100 ! | 800 / 800 = |
| neutral_enraged_wildkin | Enraged Wildkin (nowe) | 950 / 950 = | 4 / 4 = | 53 / 53 = | 1.35 / 1.3125 ! | 320 / 319.5 = | 128 / 128 = | 800 / 800 = |
| neutral_fel_beast | Fel Beast (npfl) | 400 / 400 = | 1 / 1 = | 14.5 / 14 = | 1.5 / 1.5 = | 350 / 350.2 = | 100 / 100 = | 800 / 800 = |
| neutral_forest_troll_berserker | Forest Troll Berserker (nftb) | 500 / 500 = | 1 / 1 = | 32.5 / 32 = | 1.6 / 1.625 ! | 270 / 270.3 = | 500 / 500 = | 800 / 800 = |
| neutral_forest_troll_high_priest | Forest Troll High Priest (nfsh) | 450 / 450 = | - / 0 ? | 28.5 / 31 ! | 1.8 / 1.625 ! | 290 / 289.8 = | 600 / 600 = | 1400 / 1400 = |
| neutral_furbolg_champion | Polar Furbolg Champion (nfpc) | 700 / 700 = | 3 / 3 = | 41.5 / 42 = | 1.5 / 1.5 = | 320 / 319.5 = | 100 / 100 = | 800 / 800 = |
| neutral_furbolg_ursa_warrior | Polar Furbolg URS Warrior (nfpu) | 950 / 950 = | 4 / 4 = | 52 / 52 = | 1.55 / 1.5 ! | 320 / 319.5 = | 100 / 100 = | 800 / 800 = |
| neutral_ghost | Ghost (ngh1) | 500 / 500 = | 1 / 2 ! | 32.5 / 40 ! | 1 / 1.1875 ! | 320 / 319.5 = | 400 / 400 = | 800 / 800 = |
| neutral_giant_wolf | Giant Wolf (nwlg) | 500 / 500 = | 1 / 1 = | 22.5 / 16 ! | 1.45 / 1.625 ! | 350 / 350.2 = | 90 / 90 = | 800 / 800 = |
| neutral_gnoll_assassin | Gnoll Assassin (ngns) | 370 / 400 ! | 1 / 1 = | 32 / 26 ! | 1.6 / 1.625 ! | 270 / 270.3 = | 450 / 500 ! | 400 / 400 = |
| neutral_harpy_scout | Harpy Scout (n0HW) | 400 / 400 = | 1 / 1 = | 32.5 / 31 ! | 1.6 / 1.625 ! | 280 / 279.6 = | 300 / 300 = | 1800 / 1200 ! |
| neutral_harpy_storm | Harpy Storm (n0HX) | 550 / 500 ! | 2 / 2 = | 33.5 / 33 = | 1.6 / 1.625 ! | 310 / 310.3 = | 500 / 450 ! | 1800 / 1800 = |
| neutral_jungle_stalker | Jungle Stalker (njg1) | 1300 / 1600 ! | 2 / 3 ! | 55 / 55 = | 1.35 / 1.375 ! | 320 / 319.5 = | 128 / 128 = | 800 / 800 = |
| neutral_kobold | Kobold (nkob) | 240 / 240 = | - / 0 ? | 10.5 / 16 ! | 1.35 / 1 ! | 270 / 289.8 ! | 100 / 100 = | 1400 / 1400 = |
| neutral_kobold_taskmaster | Kobold Taskmaster (nkol) | 400 / 400 = | - / 2 ? | 25.5 / 25 = | 1.35 / 1 ! | 330 / 329.7 = | 100 / 110 ! | 800 / 800 = |
| neutral_kobold_tunneler | Kobold Tunneler (nkot) | 325 / 325 = | 1 / 1 = | 14.5 / 22 ! | 1.35 / 1 ! | 270 / 270.3 = | 100 / 100 = | 800 / 800 = |
| neutral_mud_golem | Mud Golem (n026) | 800 / 750 ! | 0 / 0 = | 31 / 25 ! | 1.35 / 1.3125 ! | 310 / 310.3 = | 100 / 100 = | 800 / 800 = |
| neutral_ogre_magi | OM (nomg) (ambiguous) | 600 / 600 = | - / 0 ? | 25.5 / 19 ! | 1.35 / 1.3125 ! | 270 / 270.3 = | 100 / 100 = | 800 / 800 = |
| neutral_ogre_mauler | Ogre Mauler (nogm) | 850 / 800 ! | 1 / 1 = | 25.5 / 23 ! | 1.35 / 1.3125 ! | 270 / 270.3 = | 100 / 100 = | 800 / 800 = |
| neutral_rock_golem | Rock Golem (ngst) | 800 / 800 = | 4 / 4 = | 31 / 23 ! | 1.35 / 1.3125 ! | 270 / 270.3 = | 100 / 100 = | 800 / 800 = |
| neutral_satyr_hellcaller | Satyr Hellcaller (nsth) | 1100 / 1100 = | - / 2 ? | 52 / 52 = | 1.35 / 1.3125 ! | 290 / 289.8 = | 100 / 100 = | 800 / 800 = |
| neutral_satyr_soulstealer | Satyr Soulstealer (nstl) | 600 / 600 = | 1 / 2 ! | 25.5 / 22 ! | 1.35 / 1.625 ! | 270 / 270.3 = | 100 / 100 = | 800 / 800 = |
| neutral_satyr_trickster | Satyr Trickster (nsat) | 300 / 300 = | - / 0 ? | 8.5 / 11 ! | 1.7 / 1.625 ! | 330 / 300 ! | 300 / 280 ! | 800 / 800 = |
| neutral_wildkin | Wildkin (nowb) | 350 / 350 = | 2 / 2 = | 22.5 / 19 ! | 1.35 / 1.3125 ! | 300 / 300 = | 128 / 128 = | 800 / 800 = |
| roshan | Roshan (n00L) | 7500 / 6000 ! | 6 / 30 ! | 75 / 75 = | 1 / 2 ! | 270 / 270.3 = | 200 / 150 ! | 1400 / 1400 = |
| alchemist | ALC (N01H) (ambiguous) | 150 / 120 ! | 1 / 0 ! | 28.5 / 30 ! | 1.4 / 1.6875 ! | 325 / 289.8 ! | 128 / 150 ! | 1800 / 1800 = |
| ancient_apparition | AA (N0HP) (ambiguous) | 150 / 120 ! | - / -1 ? | 23.5 / 26 ! | 1.7 / 1.6875 = | 295 / 284.7 ! | 600 / 675 ! | 1800 / 1800 = |
| antimage | AM (Edem) (ambiguous) | 150 / 120 ! | 1 / 2 ! | 29 / 31 ! | 1.45 / 1.375 ! | 315 / 315.4 = | 128 / 150 ! | 1800 / 1800 = |
| arc_warden | AW (N0MM) (ambiguous) | 150 / 120 ! | -1 / 0 ! | 30 / 26 ! | 1.7 / 1.6875 = | 295 / 300 ! | 625 / 625 = | 1800 / 1800 = |
| axe | AXE (Opgh) (ambiguous) | 150 / 120 ! | - / 0 ? | 26 / 33 ! | 1.7 / 1.6875 = | 290 / 315.4 ! | 128 / 150 ! | 1800 / 1800 = |
| bane | BE (UIN6) | 150 / 120 ! | 2 / 0 ! | 36 / 27 ! | 1.7 / 1.6875 = | 315 / 305.2 ! | 400 / 425 ! | 1800 / 1800 = |
| batrider | BAT (O016) (ambiguous) | 150 / 120 ! | 1 / 1 = | 16 / 14 ! | 1.7 / 1.6875 = | 290 / 310.3 ! | 375 / 400 ! | 1200 / 1800 ! |
| beastmaster | BEA (H00D) | 150 / 120 ! | 3 / -1 ! | 43 / 25 ! | 1.7 / 1.6875 = | 310 / 305.2 ! | 128 / 150 ! | 1800 / 1800 = |
| bloodseeker | BLO (TX73) (ambiguous) | 150 / 120 ! | 1 / 2 ! | 32 / 38 ! | 1.7 / 1.6875 = | 290 / 284.7 ! | 128 / 150 ! | 1800 / 1800 = |
| bounty_hunter | BH (UAG3) | 150 / 120 ! | 4 / 5 ! | 31 / 37 ! | 1.7 / 1.5 ! | 315 / 324.6 ! | 128 / 150 ! | 1800 / 1800 = |
| brewmaster | PB (Npbm) | 150 / 120 ! | 0 / 1 ! | 32.5 / 30 ! | 1.7 / 1.6875 = | 300 / 310.3 ! | 128 / 150 ! | 1800 / 1800 = |
| bristleback | BRI (H008) (ambiguous) | 150 / 120 ! | 0 / 1 ! | 31 / 34 ! | 1.8 / 1.8125 = | 290 / 294.9 ! | 128 / 150 ! | 1800 / 1800 = |
| broodmother | BRO (TX78) (ambiguous) | 150 / 120 ! | 1 / -1 ! | 29 / 30 ! | 1.7 / 1.6875 = | 295 / 284.7 ! | 128 / 175 ! | 0 / 1800 ! |
| chaos_knight | CK (U00A) | 150 / 120 ! | 3 / 2 ! | 44 / 42 ! | 1.7 / 1.6875 = | 325 / 324.6 = | 128 / 150 ! | 1800 / 1800 = |
| crystal_maiden | CM (Hjai) (ambiguous) | 150 / 120 ! | - / 0 ? | 22 / 31 ! | 1.7 / 1.6875 = | 285 / 279.6 ! | 600 / 600 = | 1800 / 1800 = |
| dark_seer | DS (H00N) (ambiguous) | 150 / 120 ! | 6 / 2 ! | 34 / 34 = | 1.7 / 1.6875 = | 300 / 294.9 ! | 128 / 150 ! | 1800 / 1800 = |
| death_prophet | DP (TX66) (ambiguous) | 150 / 120 ! | 2 / 1 ! | 30 / 27 ! | 1.7 / 1.6875 = | 280 / 289.8 ! | 600 / 600 = | 1800 / 1800 = |
| disruptor | DIS (E02J) (ambiguous) | 150 / 120 ! | - / 0 ? | 29 / 29 = | 1.7 / 1.6875 = | 300 / 300 = | 600 / 625 ! | 1800 / 1800 = |
| doom_bringer | DB (TX32) (ambiguous) | 150 / 120 ! | - / 1 ? | 35 / 37 ! | 1.7 / 1.875 ! | 290 / 289.8 = | 150 / 175 ! | 1800 / 1800 = |
| dragon_knight | DK (H00F) (ambiguous) | 150 / 120 ! | 2 / 0 ! | 30 / 37 ! | 1.7 / 1.625 ! | 315 / 310.3 ! | 500 / 150 ! | 1800 / 1800 = |
| drow_ranger | DR (Nbrn) | 150 / 120 ! | -2 / 0 ! | 23.5 / 31 ! | 1.7 / 1.6875 = | 300 / 310.3 ! | 625 / 625 = | 1800 / 1800 = |
| earth_spirit | ESP (N0MU) (ambiguous) | 150 / 120 ! | 2 / 0 ! | 30 / 27 ! | 1.7 / 1.6875 = | 295 / 289.8 ! | 150 / 150 = | 1800 / 1800 = |
| earthshaker | EAR (TX27) (ambiguous) | 150 / 120 ! | 2 / 2 = | 29 / 32 ! | 1.7 / 1.6875 = | 310 / 315.4 ! | 128 / 150 ! | 1800 / 1800 = |
| ember_spirit | ES (N0M0) (ambiguous) | 150 / 120 ! | -1 / 0 ! | 32 / 32 = | 1.7 / 1.6875 = | 310 / 294.9 ! | 128 / 150 ! | 1800 / 1800 = |
| enchantress | ENC (UIN1) (ambiguous) | 150 / 120 ! | -1 / 1 ! | 36 / 26 ! | 1.7 / 1.6875 = | 335 / 320.5 ! | 550 / 575 ! | 1800 / 1800 = |
| enigma | ENI (UIN5) (ambiguous) | 150 / 120 ! | 3 / 1 ! | 25 / 22 ! | 1.7 / 1.6875 = | 300 / 279.6 ! | 500 / 500 = | 1800 / 1800 = |
| faceless_void | FV (EC45) (ambiguous) | 150 / 120 ! | 2 / 0 ! | 40 / 37 ! | 1.7 / 1.6875 = | 300 / 300 = | 128 / 150 ! | 1800 / 1800 = |
| gyrocopter | GYR (E02N) (ambiguous) | 150 / 120 ! | 2 / 1 ! | 22 / 21 ! | 1.7 / 1.6875 = | 315 / 315.4 = | 365 / 365 = | 1800 / 1800 = |
| invoker | INV (H00U) (ambiguous) | 150 / 120 ! | - / -1 ? | 22 / 22 = | 1.7 / 1.6875 = | 280 / 279.6 = | 600 / 600 = | 1800 / 1800 = |
| juggernaut | JUG (Nbbc) | 150 / 120 ! | 1 / 0 ! | 26 / 23 ! | 1.4 / 1.375 ! | 305 / 305.2 = | 128 / 150 ! | 1800 / 1800 = |
| keeper_of_the_light | KL (H06W) (ambiguous) | 150 / 120 ! | - / 0 ? | 24 / 24 = | 1.7 / 1.6875 = | 315 / 310.3 ! | 600 / 600 = | 1800 / 1800 = |
| legion_commander | LC (E02K) (ambiguous) | 150 / 120 ! | 1 / -1 ! | 37 / 35 ! | 1.7 / 1.6875 = | 320 / 324.6 ! | 128 / 150 ! | 1800 / 1800 = |
| lich | LIC (Ulic) (ambiguous) | 150 / 120 ! | - / -1 ? | 28.5 / 29 = | 1.7 / 1.6875 = | 315 / 289.8 ! | 550 / 550 = | 1800 / 1800 = |
| life_stealer | LIF (TX70) (ambiguous) | 150 / 120 ! | - / 1 ? | 37 / 26 ! | 1.7 / 1.6875 = | 315 / 320.5 ! | 128 / 150 ! | 1800 / 1800 = |
| lone_druid | LD (N014) (ambiguous) | 550 / 120 ! | 7 / 0 ! | 24 / 24 = | 1.6 / 1.625 ! | 280 / 300 ! | 128 / 550 ! | 1800 / 1800 = |
| magnataur | MAG (TX35) (ambiguous) | 150 / 120 ! | 3 / 1 ! | 34 / 35 ! | 1.7 / 1.8125 ! | 315 / 305.2 ! | 128 / 200 ! | 1800 / 1800 = |
| morphling | MOR (O00P) (ambiguous) | 2150 / 120 ! | -1 / -1 = | 17.5 / 17 = | 1.6 / 1.5 ! | 285 / 284.7 = | 350 / 350 = | 1800 / 1800 = |
| naga_siren | NAG (HC49) | 150 / 120 ! | 4 / 1 ! | 24 / 24 = | 1.7 / 1.6875 = | 320 / 329.7 ! | 128 / 150 ! | 1800 / 1800 = |
| necrolyte | NEC (U00E) (ambiguous) | 150 / 120 ! | 1 / 2 ! | 24 / 28 ! | 1.7 / 1.6875 = | 290 / 279.6 ! | 550 / 500 ! | 1800 / 1800 = |
| night_stalker | NS (Udre) | 150 / 120 ! | 4 / 2 ! | 40 / 42 ! | 1.7 / 1.6875 = | 295 / 294.9 = | 128 / 150 ! | 1200 / 800 ! |
| obsidian_destroyer | OD (U00P) | 150 / 120 ! | 3 / 0 ! | 27.5 / 28 = | 1.7 / 1.875 ! | 315 / 315.4 = | 450 / 450 = | 1800 / 1800 = |
| ogre_magi | OM (TX74) (ambiguous) | 150 / 120 ! | 7 / 4 ! | 44 / 48 ! | 1.7 / 1.6875 = | 295 / 289.8 ! | 128 / 150 ! | 1800 / 1800 = |
| omniknight | OMN (Harf) (ambiguous) | 150 / 120 ! | 4 / 2 ! | 36 / 36 = | 1.7 / 1.6875 = | 305 / 310.3 ! | 128 / 150 ! | 1800 / 1800 = |
| oracle | ORA (N0MD) (ambiguous) | 150 / 120 ! | 1 / 1 = | 25 / 16 ! | 1.4 / 1.6875 ! | 305 / 294.9 ! | 620 / 625 ! | 1800 / 1800 = |
| phantom_assassin | PA (Ewar) (ambiguous) | 150 / 120 ! | 2 / 1 ! | 24 / 36 ! | 1.7 / 1.6875 = | 310 / 310.3 = | 128 / 150 ! | 1800 / 1800 = |
| phantom_lancer | PL (Ogrh) (ambiguous) | 150 / 120 ! | 1 / 0 ! | 33 / 31 ! | 1.7 / 1.6875 = | 290 / 284.7 ! | 128 / 225 ! | 1800 / 1800 = |
| phoenix | PHO (E02F) (ambiguous) | 150 / 120 ! | -1 / -1 = | 31 / 26 ! | 1.7 / 1.6875 = | 285 / 279.6 ! | 500 / 500 = | 1800 / 1800 = |
| queenofpain | QOP (UC01) | 150 / 120 ! | - / 0 ? | 29 / 29 = | 1.6 / 1.5 ! | 295 / 289.8 ! | 550 / 550 = | 1800 / 1800 = |
| sand_king | SK (TX40) (ambiguous) | 150 / 120 ! | 1 / 1 = | 33 / 24 ! | 1.7 / 1.6875 = | 300 / 289.8 ! | 128 / 150 ! | 1800 / 1800 = |
| shadow_demon | SD (E02H) (ambiguous) | 150 / 120 ! | 1 / 0 ! | 29 / 31 ! | 1.7 / 1.6875 = | 295 / 289.8 ! | 500 / 500 = | 1800 / 1800 = |
| shadow_shaman | SHS (Orkn) | 150 / 120 ! | - / 2 ? | 29.5 / 44 ! | 1.7 / 1.6875 = | 285 / 284.7 = | 500 / 400 ! | 1800 / 1800 = |
| silencer | SIL (N01A) (ambiguous) | 150 / 120 ! | - / 0 ? | 23 / 25 ! | 1.7 / 1.625 ! | 295 / 300 ! | 600 / 600 = | 1800 / 1800 = |
| skeleton_king | WK (NC00) (ambiguous) | 150 / 120 ! | 1 / 0 ! | 40 / 41 ! | 1.7 / 1.8125 ! | 300 / 310.3 ! | 128 / 150 ! | 1800 / 1800 = |
| skywrath_mage | SM (H0DO) (ambiguous) | 150 / 120 ! | -1 / -2 ! | 17 / 21 ! | 1.7 / 1.6875 = | 325 / 324.6 = | 600 / 625 ! | 1800 / 1800 = |
| spectre | SPE (E01B) (ambiguous) | 150 / 120 ! | 1 / 0 ! | 25 / 25 = | 1.7 / 1.8125 ! | 290 / 294.9 ! | 128 / 150 ! | - / 1800 ? |
| spirit_breaker | SPI (O00J) (ambiguous) | 150 / 120 ! | 4 / 1 ! | 36 / 39 ! | 1.9 / 1.875 ! | 290 / 294.9 ! | 128 / 170 ! | 1800 / 1800 = |
| storm_spirit | SS (H00S) (ambiguous) | 150 / 120 ! | 3 / 1 ! | 27 / 29 ! | 1.7 / 1.625 ! | 285 / 284.7 = | 480 / 480 = | 1800 / 1800 = |
| templar_assassin | TA (E01Y) | 150 / 120 ! | 2 / 0 ! | 33 / 31 ! | 1.7 / 1.625 ! | 305 / 315.4 ! | 140 / 250 ! | 1800 / 1800 = |
| tidehunter | TID (UST3) (ambiguous) | 150 / 120 ! | 2 / 0 ! | 28 / 28 = | 1.7 / 1.6875 = | 305 / 300 ! | 128 / 150 ! | 1800 / 1800 = |
| tinker | TIK (Ntin) (ambiguous) | 150 / 120 ! | 3 / 2 ! | 25 / 25 = | 1.7 / 1.6875 = | 305 / 305.2 = | 500 / 500 = | 1800 / 1800 = |
| treant | TRE (e00V) (ambiguous) | 700 / 120 ! | 3 / 1 ! | 40 / 64 ! | 1 / 1.875 ! | 325 / 279.6 ! | 100 / 150 ! | 850 / 1800 ! |
| troll_warlord | TW (N016) (ambiguous) | 150 / 120 ! | - / -1 ? | 26 / 31 ! | 1.7 / 1.6875 = | 300 / 289.8 ! | 500 / 500 = | 1800 / 1800 = |
| tusk | TUS (E02I) (ambiguous) | 150 / 120 ! | 1 / 1 = | 29 / 29 = | 1.7 / 1.6875 = | 305 / 310.3 ! | 128 / 150 ! | 1800 / 1800 = |
| undying | UND (H00R) (ambiguous) | 150 / 120 ! | 3 / 0 ! | 36 / 37 ! | 1.7 / 1.6875 = | 310 / 294.9 ! | 128 / 150 ! | 1800 / 1800 = |
| vengefulspirit | VS (UAG2) (ambiguous) | 150 / 120 ! | 1 / 0 ! | 19 / 31 ! | 1.7 / 1.5 ! | 300 / 300 = | 400 / 400 = | 1800 / 1800 = |
| venomancer | VEN (EC57) (ambiguous) | 150 / 120 ! | 1 / 0 ! | 20 / 20 = | 1.7 / 1.6875 = | 285 / 279.6 ! | 450 / 450 = | 1800 / 1800 = |
| warlock | WAR (E01C) (ambiguous) | 150 / 120 ! | 2 / 1 ! | 27 / 32 ! | 1.7 / 1.6875 = | 295 / 300 ! | 600 / 600 = | 1800 / 1800 = |
| windrunner | WIN (N0EG) (ambiguous) | 150 / 120 ! | - / 0 ? | 28 / 30 ! | 1.5 / 1.5 = | 295 / 284.7 ! | 600 / 600 = | 1800 / 1800 = |
| winter_wyvern | WW (N0M7) (ambiguous) | 150 / 120 ! | - / 1 ? | 16.5 / 20 ! | 1.7 / 1.6875 = | 285 / 284.7 = | 425 / 450 ! | 1800 / 1800 = |
| witch_doctor | WD (E01A) (ambiguous) | 150 / 120 ! | - / 0 ? | 32 / 31 ! | 1.7 / 1.6875 = | 305 / 300 ! | 600 / 550 ! | 1800 / 1800 = |

Names found only on a placeholder row (no weapon, spd <= 1; the hero shell carries HP 900 or 1200), so no stats were paired: observer_ward (Observer Ward, o004), sentry_ward (Sentry Ward, oeye), mirana (MIR, h04W), nevermore (NEV, h063), puck (PUC, h050), pudge (PUD, h060), razor (RAZ, h05U), sven (SVE, h044), tiny (TIN, h04K), zuus (Zeus, h040), kunkka (KUN, h06T), lina (LIN, h049), lion (LIO, h05C), slardar (SLA, h05P), riki (Rikimaru, h047), sniper (SNI, h04O), pugna (PUG, h05W), viper (VIP, h05T), luna (LUN, h04N), dazzle (DAZ, h06E), rattletrap (Clockwerk, h051), leshrac (LES, h059), furion (FUR, h04I), clinkz (CLI, h05R), huskar (HUS, h04Y), weaver (WEA, h062), jakiro (JAK, h04R), chen (CHE, h04M), ursa (URS, h04F), lycan (LYC, h05H), rubick (RUB, h0CT), wisp (WIS, h0CI), visage (VIS, h05F), slark (SLR, h072), medusa (MED, h05K), centaur (CEN, h04X), abaddon (ABA, h067), terrorblade (TER, h056), techies (TEC, h04L), juggernaut_healing_ward (Healing Ward, ohwd).

Our units with no 6.88g name match (63): hero, tower, base, creep, tender, objective, ironhide, swiftbow, stormcaller, bulwark, shadeblade, emberlord, gale_archer, frostmage, tidecaller, thornwarden, bounty_rune, courier, creep_ranged, creep_siege, creep_super, creep_ranged_super, creep_siege_super, creep_mega, creep_ranged_mega, creep_siege_mega, tower_t2, tower_t3, tower_t4, barracks_melee, barracks_ranged, outpost, neutral_warpine_raider, neutral_prowler_acolyte, neutral_prowler_shaman, neutral_granite_golem, neutral_ice_shaman, neutral_frostbitten_golem, neutral_big_thunder_lizard, neutral_small_thunder_lizard, lane_spawner_top, lane_spawner_mid, lane_spawner_bot, meepo, nyx_assassin, shredder, elder_titan, abyssal_underlord, monkey_king, pangolier, dark_willow, grimstroke, mars, void_spirit, snapfire, hoodwink, dawnbreaker, marci, primal_beast, muerta, ringmaster, kez, largo. Our role keys (`hero`, `tower`, `base`, `creep`, `creep_*`, `tower_t*`, `barracks_*`, `outpost`, `tender`, `objective`, `lane_spawner_*`) carry no display name in our data, and the custom heroes (`ironhide` and others) and the newer reference heroes (`pangolier`, `void_spirit` and others) have no 6.88g name.

Lane creep, tower and ancient (Ghoul u001 / Ziggurat uzig / Ancient Protector e00S, etc.) are not paired by name: our keys are role names and the 6.88g names of the lane creep and tower rows are not the role names. Their 6.88g values are in the tables above only where a name matched.


## 4. Layout comparison

- Terrain: `refmap.w3e` is 129 x 129 corners (16,641 cells) with origin -8192 on both axes, i.e. a 16,384-unit square at 128 units per tile. Our `map.osd` is 327 x 327 cells at 64 units per cell from the reference ruleset patch 7.41 dump. Different maps and scales: DIFFERENT.
- Props and placed units: `refmap.doo` is 259,494 B (its header declares the record count, not decoded here). `refmapUnits.doo` is 246 B, so it holds very few placed units. Lane, tower, barracks, shop and camp positions were not extracted: MISSING.
- Regions and player starts: not decoded: MISSING.

## 5. Trigger script findings (`refmap.j`)

- Size: 102,278 lines, 4,503 functions, 861 `CreateUnit*` calls, 165 `TimerStart` calls, 142 `CreateTimer` calls.
- Obfuscated: the functions use short generated names (`Me`, `MS`, `SIE`, `Ez`, `KZE`). Unit and ability codes are the usual four-character raw ids; 5,755 distinct raw ids appear.
- Wave logic: the creep wave timer and spawn routine are isolated in section 8. Compositions per lane and the siege timing are not isolated. Lane creep wave composition is MISSING.
- Creep acquisition and attack logic: not isolated: MISSING.

## 6. What could not be read

- `refmap.w3u`, `refmap.w3t`, `refmap.w3q`, `refmap.lua`, `refmap.imp`, `refmap.mmp`, `Scripts\refmap.j`, `Scripts\common.j` and `Scripts\<stock>.j` are not in the archive (confirmed by hashing every listed name and the candidate set).
- Lane creep, tower and ancient rows are not resolved by name (our keys are role names; the 6.88g names are in section 3). The 40 names that only match placeholder rows, and the 58 ambiguous names, need the trigger code to confirm.
- Items: recipes are not in the SLK, and cooldown values need the `AbilityData` chain (`cooldownID` to `Cool1`).
- Placed-unit and prop positions, player starts and region data are not decoded.
- The obfuscated trigger script was not decoded into wave timings or compositions.

## 7. Applied: heroes (this pass)

Decisions, each with its reason:

- Source tables: `UnitBalance`, `UnitData`, `UnitWeapons`, `UnitUI`, `UnitAbilities`, `UpgradeData`, `ItemData` and `AbilityData` were decoded from the SLK members (the `ID;PWXL` text tables, parsed cell by cell). Names come from the strings member (section keys with `name=`, 6,019 sections). Extraction used the libmpq binding shipped in the `libmpq` wheel (`mpq.py` with `libmpq.so`), because the pip `libmpq` module name is a different package. The archive listfile is encrypted for the pure-Python reader, so member indices were matched to names by their SLK headers.
- Canonical row per hero name: the balance rows whose `Primary` is STR, AGI or INT and whose name matches our hero id after lowercasing and stripping non-alphanumerics. Among duplicate rows, the most frequent stat tuple wins, then a non-`TX` id (the `TX` prefix marks duplicated test or skin copies). Reason: duplicates carry identical stats, so the choice only fixes the id, not the value.
- Scale (unchanged from `scenarios/moba/tools/gen-heroes.mjs`): 1 reference unit = 15.625 milli; move speed milli per tick = spd x 15.625 / 16; attack period ticks = bat x 16; sight and range milli = reference units x 15.625; radius = collision x 15.625. Reason: the sim already uses this scale for the reference ruleset values, so the 6.88g numbers are expressed in the same model.
- Weapon damage: `dmgplus1 + dice1 x (sides1 + 1) / 2` (average of the dice roll). Reason: the sim models one damage number, so the 6.88g dice average is the closest expressible value (min and max are a gap, see below).
- Armour: the balance `def` base only. Reason: the pack adds agility to armour itself (`scenarios/moba/heroes.lua`, `armour_per_attr_b_x100` = 14 and `armour_base_penalty_x100` = 100 in `scenarios/moba/attr_rules.txt`), so adding it here would double count.
- Hit points: the balance `HP` field is used as `health`. Reason: the pack (`scenarios/moba/heroes.lua`) adds strength on top of this base, as it did for the reference ruleset values.
- Mana and regeneration: `mana0` for mana, `regenMana` and `regenHP` per second (hundredths) for `mana_regen` and `hp_regen`.

Counts for heroes:

| Category | Count | Notes |
|---|---|---|
| Our hero units | 127 | `units_heroes.osd` |
| Matched to a 6.88g canonical row, values applied | 69 | `scenarios/moba/heroes_688g.tsv`, applied by `scripts/apply-heroes-688g.py` |
| Unresolved, left at the reference ruleset values | 58 | pass 2 (below): identity proven for 23, values not applied; 35 with no proof |
| Dropped (no 6.88g hero) | 10 | `ironhide`, `swiftbow`, `stormcaller`, `bulwark`, `shadeblade`, `emberlord`, `gale_archer`, `frostmage`, `tidecaller`, `thornwarden`: no 6.88g name, the DROP set of `scripts/apply-creeps-688g.py`, removed in `ebbafb25` (the ten lines are kept in `units.osd`, see section 9). `hero` is kept as the generic player-hero kind |
| Added (6.88g hero we lack) | 0 | not attempted in this pass |

Pass 2 (row content-hero-conversions-unresolved) took the 58 unresolved heroes. Identity source: the reference ability names of the pinned SteamDatabase tracking repo at 282fad29 (game/dota/pak01_dir/scripts/npc/heroes/npc_dota_hero_<id>.txt, the file gen-heroes.mjs reads), with the hero prefix, trailing digits and non-alphanumerics removed. A 6.88g candidate is a unit.ini section (_parent inherited) that has heroAbilList and a Primary of STR, AGI or INT; its ability names are the normalized Name of each heroAbilList ability section, attributebonus and talents excluded. A hero is proven to a row when that row shares the most names with the reference (at least 2) among all rows with heroAbilList, and every row tied at that count has one stat tuple. Name tokens were not used. Calibration on the 69 applied rows: 32 accepted, all equal to their applied section; 0 false accepts; 20 no-primary, 15 ambiguous, 2 no agreement.

Result for the 58 (none applied; the 69 applied rows and the TSV are unchanged):

- Identity proven, values not applied: 23. Our id = 6.88g section [unit.ini line]: puck=N00B [34573], razor=E002 [21838], zuus=UIN3 [54403], lion=UC18 [53350], slardar=UC91 [53968], riki=HC92 [34424], sniper=UAG6 [52740], viper=EC77 [26665], luna=E005 [22156], dazzle=N01W [36659], rattletrap=H00T [29634], clinkz=E004 [21999], huskar=H00Q [29179], jakiro=E00P [22321], chen=H00A [27412], lycan=U008 [50624], meepo=H00I [28376], treant=UST2 [55265], wisp=O01F [41002], slark=H071 [30800], medusa=H00V [29906], centaur=H000 [26832], shredder=E032 [26190]. Values are not applied: the unit.ini extract (/tmp/g688-lni2/table/unit.ini) omits base values that the 6.88g SLK members hold (AGI, mana0, dice1, sides1, def, sight, turnRate, regenMana and regenHP are missing for many rows), and seven TSV columns disagree with the applied rows on some heroes (str, str_gain, mana, hp_regen, armor, spd, damage). Where the extract has the field, twelve columns agree with every applied row (primary, hp, agi, agi_gain, int, int_gain, bat, collision, range, mana_regen, sight, turn). Clearing action: extract UnitBalance, UnitWeapons and UnitData from /tmp/g688.w3x, then rerun this identity pass against those values.
- Matched row has no Primary in the extract, so the hero stat is unprovable. 14, UNVERIFIED: mirana=N01V, nevermore=UAG4, pudge=U00F, kunkka=H06S, lina=H004, pugna=H00H, leshrac=UIN7, furion=Emns, weaver=UAG5, visage=UC60, abaddon=Udea, elder_titan=O015, terrorblade=Eevi, abyssal_underlord=N00R.
- Ambiguous (tied rows with different stat tuples). 5, UNVERIFIED: sven, tiny, ursa, rubick, nyx_assassin.
- No 6.88g row shares two ability names with the reference. 16, UNVERIFIED: techies, monkey_king, pangolier, dark_willow, grimstroke, mars, void_spirit, snapfire, hoodwink, dawnbreaker, marci, primal_beast, muerta, ringmaster, kez, largo. void_spirit no longer matches FV.

Gaps the sim cannot express (not invented):

- Turn rate (`turnRate` in `UnitData`) has no field in `unit()`.
- Weapon damage min and max (dice and sides) collapse to one number.
- Projectile speed for ranged heroes is not in the SLK rows read here.
- Magic resistance: 6.88g has no field; `mr=25` is kept from the reference ruleset block.
- Hero bounty (level-based in 6.88g) is not in `unit_extra`; kept at 0.
- The pack's attribute factors are the 6.88g misc.ini values (`hp_per_attr_a` = 19, `mana_per_attr_c` = 13, regeneration 0.03 and 0.04, armour 0.14 per agility less 1.0; `scenarios/moba/attr_rules.txt`).

Gate changes (`rust/gate`), each for an old value:

- `hero_axe_level1` and `hero_axe_growth` (`rust/gate/src/hero_checks.rs`, no longer in the tree): AXE now has AGI 20 (was 18), health base 150 (was 120), damage 26 (was 33), mana base 100 (was 75), growth 2.5/2.2/1.6 (was 2.7/1.7/1.6). Reason: the expectations were the reference ruleset values; they now carry the 6.88g numbers.
- `phantom_lancer_illusions` (`rust/gate/src/ability_checks.rs`, no longer in the tree): the victim spawns one unit away instead of two. Reason: 6.88g PL has a 128 range (melee), and the old placement is out of range. The check is about illusions, not range.

Verification (local, at that pass; `moba_match` is no longer in the tree): `osdt-data` loads `scenarios/moba`; `mod_check` on `moba_attack.lua` passes; `oscraft-gate` passes 197 of 197; `oscraft-gate mpq` passes. `moba_match` exits 1 with `no winner by tick=40000` both before and after this change (HEAD baseline gives the same line), so it is a pre-existing condition, not a regression. Superseded: on origin main `fa6862f`, `moba_match ../scenarios/moba 40000` exits 0 with `winner=0 tick=29089` (section 8).

Not done in this pass (the rest of the brief): creeps and neutrals in `units.osd` (19 of its 51 creep, neutral and Roshan unit lines have no 6.88g row and keep their values; `units_extra.osd` has none), the 58 unresolved heroes, and every ability in `abilities.osd` and `abilities_ruleset.osd` (AbilityData `Cool1`, `Cost1`, `Rng1`, `Area1`, `Dur1`, `DataA1`..`DataI1` per level, read by the effect model). The ability mapping needs the `effect` model in `rust/crates/data/src/builtins.rs` matched per kind before values can be written.

Pass 3 (row parity-heroes, hy3d `output/688g-data/hero-base.tsv`, `hero-base.sources.tsv`, `hero-identity.tsv`):

- `heroes_688g.tsv` now has one row per hero unit (127). 112 are PROVEN to a 6.88g unit through `hero-identity.tsv` (pick-table rows first, so alchemist N01I, dragon_knight UST5, keeper_of_the_light UIN2, troll_warlord N016 and winter_wyvern N0M7 carry the pick-table values, not a tied variant); the 15 heroes not in 6.88g (`hero-not-in-688g.tsv`) have a row with `unverified` = `all` and keep every current value.
- Every VERIFIED_688G field was compared with the applied value and set where it differed: 432 field changes over 48 heroes (a derived pair such as `damage` and `weapon_damage` counts once). Per hero: abaddon 10, abyssal_underlord 7, alchemist 5, centaur 11, chen 10, clinkz 11, dazzle 11, dragon_knight 5, elder_titan 7, furion 7, huskar 7, jakiro 9, keeper_of_the_light 1, kunkka 8, leshrac 10, lina 10, lion 9, luna 10, lycan 11, medusa 8, meepo 12, mirana 11, nevermore 9, nyx_assassin 14, puck 11, pudge 9, pugna 11, rattletrap 11, razor 11, riki 10, rubick 10, shredder 12, slardar 7, slark 8, sniper 8, sven 10, techies 14, terrorblade 9, tiny 12, treant 7, troll_warlord 6, ursa 9, viper 8, visage 6, weaver 9, winter_wyvern 2, wisp 10, zuus 9. This closes the 23 proven-but-unapplied and the 14 no-primary heroes of pass 2 (the extract that omitted AGI, mana0, dice1, sides1, def, sight, turnRate, regenMana and regenHP is replaced by `hero-base.tsv`).
- ENGINE_DEFAULT fields (6.88g's unit.ini does not set them; the value comes from the stock table) keep the applied value where it differs: `mana_regen` 21, `armor` 7, `mana` 14, `hp_regen` 6, `damage` 24, `primary` 3, `range` 1. Each is named in the `unverified` and `note` columns of its row, and `scripts/apply-heroes-688g.py` writes a `# UNVERIFIED <id>:` line above the unit. The Custom-versus-Melee ruleset choice for these is pending the user, so they stay ENGINE_DEFAULT.
- Radius: each hero's radius is its collision x 15.625 (108 heroes at collision 24 = 375; phantom_lancer, rattletrap, naga_siren and slark at collision 8 = 125). Healing Ward `juggernaut_healing_ward` (6.88g o00C, collision 16 inherited from stock ohwd, ENGINE_DEFAULT) is 250, was 125. Roshan (n00L, collision 0, VERIFIED_688G) is 0, was 375: `movement.rs` separation uses `radius[i] + radius[j]` with a guarded distance and `strike_measure` reads the radius of structures only, so zero cannot divide or stall; no Lua reads `game.unit_radius`.
- The generic `hero` kind in `units.osd` has no 6.88g unit and keeps its reference ruleset values, flagged UNVERIFIED.
- The 11 tooltip mismatches (`tooltip-crosscheck.tsv`: H004 intplus, H00H agiplus, H00N spd, H00S str, O00P str and strplus, O015 rangen1, QTW4 spd, QTWF spd, TX24 intplus, TX46 spd) keep the unit field; the tooltip text is not the unit value.
- Not carried (no field in `unit()` or `attr`): turn rate (column `turn` is recorded), dice and sides separately, projectile speed, magic resistance (`mr=25` kept, UNVERIFIED), attribute-bonus abilities. `scenarios/moba/panels.lua` `HERO_BASE` and `HERO_PRIMARY` are a hand copy of TSV rows for the heroes they list; no hero in `HERO_BASE` changed in this pass and no listed hero's primary changed; the `primary` of dazzle, nyx_assassin, techies and wisp changed and they are not in `HERO_PRIMARY`, so that table needs rows for them (panels lane).

## 8. Wave, siege, start, day/night and buyback timing (this pass)

Line numbers are in the extracted `refmap.j` (a scratch extract of that pass, not kept), unless marked "ours".

Changed: nothing. Every map-sourced value below either matches our data or has no value in the map to compare with.

Map values:

- Creep wave cadence: 30 s periodic. `TriggerRegisterTimerEventPeriodic(xR,30.)` at line 30050, with `TriggerExecute(xR)` at line 30051, so the first wave fires at game start. Action `JAE` (line 26200, bound at line 101660). `JAE` spawns through `JRE` with raw ids `u001`, `n12Q`, `ugho` and `u002`, `unec`, `n12R`, chosen by flags. Every seventh execution (`ModuloInteger(GetTriggerExecCount(xR),7)==0`) calls `H5E` at line 26201. `H5E` only writes stats, so the seventh-wave rule is an upgrade hook, not a spawn change.
- Ours: `scenarios/moba/lanes.lua` `CREEP_PERIOD = 480` ticks (line 17). At `TICKS_PER_SECOND = 16` (`rust/crates/sim/src/lib.rs:64`) that is 30 s. Match. The first creep is at game timer 1 tick, which is the start, so it matches the map's immediate `TriggerExecute`.
- Siege timing: superseded by the lane creep section at the end of this file (a siege creep spawns with every 7th wave, `wT` in `JAE`). The text below is the earlier pass and is wrong on the point. No trigger in that pass fired a siege or name-based siege spawn. Our `SIEGE_STARTS = {4800, 28800, 57600}` (`scenarios/moba/lanes.lua` line 19, 5, 30 and 60 min at 16 ticks/s) and `SIEGE_PERIOD = 4800` (line 18) have no map source here. The 5-minute value is a public description, not map-sourced, and is kept unchanged.
- Game-start timer: `TimerStart(CreateTimer(),0xB4,false,function POE)` at line 30036 is a one-shot 180 s (`0xB4`) that sets flag `Oo=true` (`POE` at lines 30013-30016). The flag's consumers are not decoded. The countdown before start is `TimerStart(SA,TA+1,false,null)` at line 36638 (`RZX`). `TA` is a global `90` (line 791), reassigned to `0x96` = 150 (line 37252, `I9X`), `0xB4` = 180 (line 39444), `90` (lines 37852, 39662) and `'x'` = 120 (line 40408). The value in effect at start is not resolved, so no countdown length is claimed.
- Day and night: `SetTimeOfDayScale(.625)` at line 30035, and the start is dawn with `SetFloatGameState(GAME_STATE_TIME_OF_DAY,6.)` at line 30033. The map stores a rate, not a day or night length in seconds. Length is NOT in the map. Ours: `day_s=300` and `night_s=300` (`scenarios/moba/attr_rules.txt` lines 79-80, read by `scenarios/moba/clock.lua` as 300 * 16 ticks each, 5 min each). These are unsourced from the map and kept unchanged.
- Buyback cooldown: NOT FOUND. No buyback timer or cooldown value is in the map. The `UnitResetCooldown` calls at lines 26211-26220 and 3735-3745 reset unit cooldowns, not buyback. Ours: `buyback_net_worth_div=13` (`scenarios/moba/attr_rules.txt` line 32, used by `scenarios/moba/heroes.lua`) is a cost divisor. There is no cooldown constant to compare.
- Other map timers, unmatched to a repo value: 450 s periodic `oR` (line 30052, action `JTE` at line 26536, which swaps tech research counts); 75 s one-shot `rR` (line 101706, action `JUE`); 2669 s one-shot `vR` (line 30053, action `HSE` at line 26000, increments `Ji`); 849 s one-shot `eR` (line 30054, action `HTE` at line 26003, increments `ji`). These are not matched to our wave, siege or day data.

Repo sources that are not map data: `scenarios/moba/waves.osd` holds the lane spawner placements and the wave ability definitions (`creep_wave_*` and `siege_wave_*`, periods 480 and 4800 ticks). The wave cadence that spawns is in `scenarios/moba/lanes.lua`. `camp_first_spawn_s=60` (`scenarios/moba/attr_rules.txt` line 66) is a neutral-camp value, not a wave value, so it is not compared.

Verification on origin main `fa6862f` (fresh clone):

- `oscraft-gate` (rust, release): `gate: 203 checks, 0 failed`.
- `moba_match ../scenarios/moba 40000` (rust, release; no longer in the tree): exit 0, `winner=0 tick=29089`, replay hash `35358641158f53c0`. This was the result on `fa6862f`. The earlier `no winner by tick=40000` and the `winner=0 tick=29089` reports came from different builds.

## 8. Applied: neutral creeps and Roshan

Source: `scenarios/moba/creeps_688g.tsv`, applied by `scripts/apply-creeps-688g.py` to `units.osd`. Pairing is by exact display name (`units/OrcAbilityStrings.txt`); neutral keys prefer rows with `Primary` `_`. Formulas as heroes: damage is `dmgplus1 + dice1 x (sides1 + 1) / 2`, period is `cool1 x 16` ticks, regen is `regenHP x 62.5` milli per tick; armour `def` (blank keeps the current value).

32 rows converted (31 neutrals and Roshan). Kept on current values: the 9 lane creep rows (role keys with no 6.88g display name) and 8 neutrals with no 6.88g name or row (`neutral_warpine_raider`, `neutral_prowler_acolyte`, `neutral_prowler_shaman`, `neutral_granite_golem`, `neutral_ice_shaman`, `neutral_frostbitten_golem`, `neutral_big_thunder_lizard`, `neutral_small_thunder_lizard`), and the 2 Roshan pit lines (`roshan_pit_day`, `roshan_pit_night`) have no 6.88g row. The 8 neutrals with a blank armour in `creeps_688g.tsv` (`neutral_kobold`, `neutral_kobold_taskmaster`, `neutral_ogre_magi`, `neutral_satyr_hellcaller`, `neutral_satyr_trickster`, `neutral_dark_troll`, `neutral_forest_troll_high_priest`, `neutral_black_drake`) are converted on every other field and keep their current `defense=`. Counts at origin `ebbafb25`: `units.osd` has 51 creep, neutral and Roshan unit lines (49 without the two Roshan pit lines); 32 have a TSV row, and 0 of their 248 compared fields differ; 19 lines in `units.osd` carry an UNVERIFIED comment.

Not expressible in the sim, not invented: turn rate, weapon min and max, projectile speed, magic resistance, level-based bounty, day and night regeneration.

## 9. Open: why the ten placeholder hero lines matter

Commit `73fd708` removed the ten slot hero unit lines (ironhide, swiftbow, stormcaller, bulwark, shadeblade, emberlord, gale_archer, frostmage, tidecaller, thornwarden) from `units.osd` and their matching lines from `units_extra.osd` and `slot-models.tsv`. Commit `b56043b` restores the `units.osd` lines before `roshan`, which keeps the original unit order.

Evidence (release gate, same Rust code and only scenario data changed):

- Pre-conversion data: `camp-tormentor-timing` 6.9s, `camp-pit-swap` 7.7s, `camp-roshan-window` 2.7s, gate total 29.6s.
- Merged data: 76s, 100s, 8.8s, gate total about 3m24s.
- Only the ten hero lines removed from pre-conversion data: `camp-pit-swap` 89s.
- Ten hero lines restored on merged data: gate 203 of 203 in 29.8s.
- Creep, neutral and Roshan lines were each ruled out by bisection (halves of the neutral set and the Roshan line reverted individually stay slow or fast as the hero lines dictate).

Not known: the sim mechanism. Unit kinds are resolved by name (`unit_index`), so a shifted index alone should not change behaviour. The slow path has not been identified. Until it is, keep the ten lines in `units.osd`.

Debug-profile note: the gate with the merged data was killed at 25 minutes at `camp-pit-swap` (after `camp-tormentor-timing` PASS). Release is the reference for timing.

## Neutral camps (par-world-neutrals)

Source: the 6.88g map script (`war3map.j`): camp pass `KPE` (one `KKE` call per camp), roll `KBE`, composition functions that call `JWE`, per-unit gold and xp in `UUR` (`SaveInteger(ce, id, Xx|Ox|Rx, v)`), kill payout `W_R`. Generated into `scenarios/moba/neutral.osd` by `scenarios/moba/tools/gen-neutral-688g.mjs` (`--script <war3map.j> [--check]`).

- 14 camps, not 28: ids 1 and 2 ancient, 4, 8, 9, 12, 14 and 15 large, 3, 7, 10 and 11 medium, 5 and 6 small. The Dota 2 era camp list (14 per side) is gone. Camp 3 is called twice (medium then large) and the second call never fires (the first pass's creeps fill the check rect), so it is medium.
- Timing: first spawn 30 s after game start (`PXE`), then every whole minute (`PRE` periodic 60 s); a camp spawns only when its check rect (a box roughly 900 to 1500 units a side, much larger than our 4688 milli clear radius) holds no living unit other than the filtered ones. `-neutrals` re-runs the pass by chat.
- Composition: rolled per spawn from 5 or 6 options per size, never the option the camp had last. We hold one stand-in option per camp, ((camp id - 1) mod options) + 1. Members stand on a 42.5 unit ring (the script uses a random 25 to 60 unit offset).
- Gold: uniform random between the `Xx` and `Ox` values of the dying type; we pay the midpoint (`reward`). Xp is the `Rx` value split among heroes within 1325 units (`assist_radius_milli=20703`); for a non-ancient neutral the share goes to every hero in range, for an ancient only to the killer's allies (not modelled).
- Stats: `creeps_688g.tsv`, now 37 rows. Armor that is blank in the map SLK and unit.ini is the stock WC3 base-object value (all 0 for the 8 formerly blank rows). Five units were missing and are added (`neutral_blue_dragonspawn_overseer` nbdo, `neutral_blue_dragonspawn_sorcerer` nbds, `neutral_emerald_golem` nggr, and the existing `neutral_big_thunder_lizard`/`neutral_small_thunder_lizard` now carry n0LC/n0LD). The models of the first three are placeholders (black drake, granite golem).
- Not in 6.88g camps, kept unused and UNVERIFIED: `neutral_warpine_raider`, `neutral_prowler_acolyte`, `neutral_prowler_shaman`, `neutral_granite_golem`, `neutral_ice_shaman`, `neutral_frostbitten_golem`.
- Capability gaps (recorded in `/workspace/hy3d/output/688g-data/world-parity.neutrals.tsv`): per-respawn composition re-roll, rect clear test, whole-camp simultaneous spawn, night sleep (`canSleep`), neutral abilities (auto-cast and passive), ancient-versus-regular xp sharing, per-type acquire range.

## Lane creeps (par-world-creeps)

Sources: `war3map.j` (line numbers of `output/g688-map/war3map.j`), `unit.ini`, `upgrade.ini` and the map archive SLKs (`UnitBalance`, `UnitWeapons`, `UpgradeData`). The comparison table, one row per item with its source line, is `output/688g-data/world-parity.creeps.tsv` in the hy3d project.

Wave rules (`JAE`, 26233-26456):
- Timer `xR`: first execution at game start, then every 30 s (30083-30084). Melee count `ji` = 3, ranged count `Ji` = 1 (100730-100731); `ji` rises at 849, 1759 and 2669 s (`eR`, `HTE`) and `Ji` at 2669 s (`vR`, `HSE`) (30086-30089).
- Every 7th execution (`ModuloInteger(GetTriggerExecCount(xR),7)==0`: 3:00, 6:30, 10:00 ...) one siege creep per ranged slot spawns at the ranged spawn rect of each lane (26330-26356). The earlier "siege not found" note in section 8 missed this.
- Melee creeps spawn at one point (`JRE`, 26224) per lane and side, ranged and siege at another. The points are the centres of the Rect literals at 100207-100219 (Sentinel top Cn/Rn, mid dn/In, bot cn/On; Scourge top En/bn, mid Vn/Nn, bot nn/An).
- Upgrade rule: a Scourge melee creep is `ugho`, `u001` once the Sentinel melee barracks of its lane died, `n12Q` once all six Sentinel barracks died; ranged `unec`/`u002`/`n12R` by the ranged barracks of the lane; siege `umtw`, then `u00R` when the lane's ranged barracks died, with no mega. Sentinel: `esen`/`e00V`/`n12S`, `edry`/`e00W`/`n12T`, `ebal`/`e026` (26261-26454, 3704-3732, 26457-26527). Barracks die gold: 275 melee, 225 ranged, to every player of the killing side (3704-3723).
- 450 s upgrades (`oR`, `JTE`, 26569-26578): each side's research levels R003/R009/R00A/R00B (Scourge) and R008/R00D/R007/R00C (Sentinel) rise by one and apply to living units too. Per level: melee +1 damage +10 hp, super melee +2/+19, ranged +2/+10, super ranged +3/+18; mega and siege creeps carry no upgrade.
- Routes: a wave unit is attack-moved to the waypoint unit of its lane (top gR -5805,5258; mid JR -562,-652; bot hR 5614,-6650), then to the enemy base target (FR 4902,4537 for the Sentinel, GR -5468,-5891 for the Scourge) once in range of the waypoint (25025-26074, 31604-31619, 100983-100993, 101612-101614).
- Bounty and xp are not unit.ini: war3map.j `UUR` (91685-91732) holds bounty_lowest, bounty_highest and expirience per unit type (melee 35-44 gold 62 xp, ranged 40-49 / 41, siege 66-80 / 88, super and mega 18-26 / 25). The xp share radius is 1325 units (`xx`, 141), a denied creep pays half (93566).

Applied: `units.osd` creep rows (hp, damage, siege period 43 ticks and hp 550, mega hp and damage, sight 13281), `units_extra.osd` (gold midpoint, xp, share radius 20703, acquire), `lanes.lua` (spawn table, counts, 7th-wave siege, barracks-based super and mega, 450 s upgrades through the `creep_up_*` researches of `waves.osd`), `routes.osd` (regenerated by `scenarios/moba/tools/gen-lane-routes-688g.mjs`), `waves.osd` (spawner centres; the wave abilities and behaviours that duplicated `lanes.lua` are gone, `creep_wave` in `abilities.osd` stays because it fixes the ability index order).

Not expressed, not invented: the random gold roll, attack and armor type per unit row, the random siege golem event (`KQE`, flag `Xi`), lane-disable modes (`DL`, `hL`, `FL`, `GL`). UNVERIFIED: acquire range of `u001`/`n12Q` and `e00W` (parents `ndmu`, `nqb1` are in no readable file).

## Buildings, shops and starts (par-world-buildings)

Source: `war3map.j` `main()` (the literal `CreateUnit` calls, lines 100536-101307, rebuilt by `scenarios/moba/tools/buildings-688g.mjs`), `unit.ini` with the map's `UnitBalance.slk` and `UnitWeapons.slk` for the stock fields. The comparison table with every position and stat is `/workspace/hy3d/output/688g-data/world-parity.buildings.tsv`.

Changed:

- Positions. Every tower (tier by rawcode: e00R/u00M 1, e011/u00D 2, e00S/u00N 3, e019/u00T 4; lane = creation order top, mid, bot), barracks, fountain and shop is placed from the script. The old tower positions were the Moon Well and Ziggurat positions of the map and the old barracks lines had melee and ranged swapped; the old starts were the editor's slot locations, not the script's hero spawn. `convert-688g-map.mjs` now calls `buildings-688g.mjs`, which derives start, building and shop-depot lines (`node scenarios/moba/tools/buildings-688g.mjs --script <war3map.j>` checks `map.osd` against the script, `--write` rewrites it). `gen-reference-map.mjs` (Dota 2 era) refuses to run without `--legacy-dota2`.
- Starts. Random pick and revive put a Sentinel hero at the centre of rect `Bn` (-6864,-6784) and a Scourge hero at the centre of `Xn` (6300,6096) (`jU`, line 3815). Seats alternate by index (even Sentinel), spread by a fixed slot offset inside the rect. `starts-688g.tsv` lists these spawn points.
- Stats, both sides alike. Towers T1 to T4: HP 1500/1800/1800/1800, armour 20/22/22/30, damage 110/130/152/152 (attack 2, siege), range 700 units, period 16/15/15/15 ticks, regen only on T4 (4 hp/s), truesight 900. Barracks: melee 1500 HP armour 15 regen 5, ranged 1200 HP armour 10. Ancient: 4250 HP, armour 15, regen 6, sight 1800. Fountain (new unit): 50000 HP, damage 195 every 2 ticks, range 1150, invulnerable. Base outbuildings (new units, Moon Well, Ziggurat, Ancient of Wind, Hunter's Hall, Boneyard, Sacrificial Pit): 500 HP, armour 10. Bounty gold 200 tower, 125 barracks, 111 outbuilding, 25 xp, 50 percent xp when denied; deny threshold 10 percent for every structure but the ancient.
- Rules in `protection.lua`: the lane chain (T2 needs the lane's T1 dead, T3 its T2, barracks its T3, T4 and outbuildings any T3, ancient both T4); no backdoor protection (the script has none); glyph covers every structure including the ancient, and a tier 1 tower loss resets the team's glyph cooldown; fountain heal 4 percent of maximum life a second (maximum approximated by the highest life seen) with a 2.5 s grace; tower kills pay every enemy seat 120 + 40 x tier gold (half when denied).
- Shops: one depot prop per script shop (24 home, 6 side: Goblin Laboratory, Goblin Merchant and Furbolg's hut at both map edges, 2 secret: Leragas The Vile).

Capability gaps (each row in the table): per-shop item lists (a depot only carries a tag), mana regeneration at the fountain, night sight (800 against 1900), attack and armour type matrix (siege 0.75 against heroes, 1.50 against fortified), illusion damage against structures (75 percent), per-side hull size of the ancient (unpl 196 against etol 144), Sacrificial Pit regen. UNVERIFIED: the melee and ranged role of eaom/usep and eaoe/utod (read from stats and names), the Scourge temple regen (blank in the slk, 0 kept), the initial courier positions (the script creates no courier at start; couriers are sold units) and the buyback rules.

## Pathing (interior blocking, measured)

The blocked cells of `scenarios/moba/map.osd` are `pathing-wpm-688g.tsv` (the 6.88g `war3map.wpm` unwalkable bit 0x02, a sim cell blocked when any of its 2x2 pathing cells is) joined with the pathTex footprints of the placed props and destructables. `tools/apply-pathing-688g.mjs` rewrites only the `paint(..., 0)` lines from `pathing-wpm-688g.tsv`, `props-688g.tsv` and `pathing-footprints-688g.tsv` (`--check` is the drift test); `convert-688g-map.mjs` calls the same code. `tools/check-pathing.mjs` runs in CI.

- Footprints: a texture pixel is one 32-unit pathing cell, red = unwalkable; the texture is centred on the object, authored at facing 270 and turned counter-clockwise from 270; a doo record with flag bit 0x2 clear is non-solid. The shapes are the real TGAs of War3.mpq (WarRaft/War3.mpq extract/PathTextures); the type to texture map is the map's own destructable.ini/doodad.ini overrides, else stock DestructableData.slk and doodads.slk. Trees ATtr, NTtw, CTtr and LTrc are 4x4 solid (16 cells), NTtc and the pathing blocker YTpb are 2x2 (4 cells), YTlb has no footprint (map override `pathTex=""`), and 40 of the 54 decor prop types have none (stock or map override `""`).
- Cliffs: no extra rule. The wpm already holds the editor's cliff pathing (2,404 of 2,450 non-ramp cliff tiles are fully unwalkable in it, 45 partly, 1 open, and 193 of 321 ramp-flag tiles are open), so a whole-tile cliff rule would over-block.
- wpm bits, measured on the 6.88g `war3map.wpm` (512 x 512, one byte per 32-unit cell): 0x02 unwalkable, 0x04 unflyable (set on the map-boundary tiles: 38,660 of 41,449 cells lie in w3e boundary tiles, the rest on their edge), 0x08 unbuildable, 0x10 never set, 0x20 blight (exactly the 32 cells of the 2 blighted w3e corners), 0x40 no-water (set on every non-water cell, clear on exactly the 8,832 water cells), 0x80 terrain-baked block (set on boundary and cliff cells: 32,532 of 32,660 unwalkable interior cells, and no prop footprint is in the wpm). The sim reads 0x02 only; the others carry no gameplay here. Cell-to-w3e-corner mapping is `(x + 2) >> 2`, corners sit at multiples of 4 cells. Footprint centring: every one of the map's 3,647 solid footprint records is already on the grid (offset 0 for all), and `footprintCells` rounds an odd footprint to the cell containing the centre and an even one to the nearest corner, which is openwar3's `footprintOrigin` rule, so an off-grid centre needs no extra rule.
- Water: the wpm leaves the river walkable (0x08 unbuildable, 0x02 clear); the few deep cells (0x0a) are in the wpm baseline. Open water cells stay open.
- Fountains: each sits in a ring of eight YTpb blockers around a 2x2-cell pocket, so a fountain cell is open but unreachable, as in 6.88g.
- Trees are destructible in 6.88g (war3map.j pU 3984 names the types; 61 call sites of hy 4806, the clear-trees-under-a-moved-unit function; Tango and Quelling Blade eat a tree; Timber Chain; every felled tree regrows after 300 s, 450 s for the first 20 in one mode, unless a player's unit is within 175, Y7R 94710 and Y6R 94688; Sprout, Ironwood Branch, Fissure and Ice Shards create temporary blockers). What exists now: the sim holds obstacles (`game.tree_add`, `tree_kill`, `tree_restore`, `set_tree_guard`, `trees_near`, `blocker_add`; a TREE cell bit beside BLOCKED, `docs/SCRIPTING.md`), and the pack drives them. `scenarios/moba/tools/gen-trees-688g.mjs` derives the 3475 trees of ATtr, NTtw, CTtr and NTtc (footprints from pathing-footprints-688g.tsv at the props-688g.tsv positions) into `trees_688g.lua`, which `trees.lua` registers once at start with the 175-unit guard. map.osd no longer paints tree-only cells (10735 of the 68351 blocked cells); apply-pathing-688g.mjs paints the wpm plus the non-tree footprints and check-pathing.mjs counts live tree cells as blocked (painted blocked union tree cells equals the previous blocked set, 0 differences). LTrc (6 placed) is not in pU, so its footprint stays painted. Consumers: Tango and Quelling Blade are point items whose cast picks the tree under the click (`spells.lua` allow_cast and `trees.cut`), Tango heals 115 over 16 s; Scroll of Teleportation, Boots of Travel and Nether Swap clear trees at their landing (the 4 of the 61 hy sites the pack moves in Lua); a unit standing inside a live tree cuts it, which covers the engine-effect moves; Sprout, Ironwood Branch (item_branches, now a point item), Fissure and Ice Shards place blockers for their 6.88g durations; Timber Chain pulls the caster to the first tree on its line. Missing: a per-site clear radius for the 57 hy sites whose move is an `.osd` effect (the engine move effects have no tree hook), the Timber Chain damage and the 250 range talent, the ward target of Tango and Quelling Blade, and slot reuse for expired blockers (obstacles.rs push_obstacle appends, MAX_OBSTACLES is 16384).

# DotA Allstars 6.88g: convergence report

Subject: `/workspace/oscraft/DotA v6.88g Allstars.w3x` (DotA Allstars 6.88g), compared with the MOBA scenario data in `scenarios/moba/`. Every value below is labelled DotA Allstars 6.88g. Section 7 records the one data change applied so far (66 matched heroes); everything else in sections 1 to 6 is still a comparison, not an edit.

Our `scenarios/moba/*.osd` stats are generated from Valve's Dota 2 `npc_units.txt` and `items.txt` (SteamDatabase dump), not from 6.88g. Every comparison below is therefore a cross-game check of the same concept, not a 6.88g-to-6.88g diff.

## 1. Archive inventory

- File size: 41,467,779 bytes. MPQ format version 1, header at offset 0, sector size 4096, hash table 65,536 entries, block table 2,328 valid blocks.
- Member names: the `(listfile)` member lists 2,326 names. Hashing every listed name (and the standard candidates) against the hash table resolves all 2,328 valid blocks: 2,326 listed names plus `(listfile)` and `(attributes)`. No block is unnamed. libmpq's member iterator also yields exactly 2,328 names.
- Readers: libmpq 0.8.1 (ctypes binding over libmpq.so, in `/tmp/mpq688-scripts/venv`) and a stdlib reader (`/tmp/mpq688-analysis/mpqr.py`). Both read the `(listfile)` (98,070 B, plain `\r\n` list). Five members use PKWARE implode (`war3map.w3a`, `war3map.wts`, `war3map.w3h`, `NEW_LOADSCREEN.dds`, `SlowTarget.mdl`); the stdlib reader cannot decode them, libmpq reads all five. 40 `Units\*Func.txt` and `Units\*Strings.txt` members are zero-length (csize 0) and are rejected by libmpq with format error -7. Extracted: 2,288 members with content plus 40 zero-length members.
- StormLib, w3x2lni and ObjectEditor were not built or installed: libmpq and the stdlib reader were sufficient.
- Standard members present: `war3map.w3i` (796 B), `war3map.w3e` (116,568 B), `war3map.doo` (259,494 B), `war3mapUnits.doo` (246 B), `war3map.w3a` (15,336 B), `war3map.w3b` (2,636 B), `war3map.w3d` (5,355 B), `war3map.w3h` (37 B), `war3map.wts` (20,067 B), `war3map.wpm` (262,160 B), `war3map.shd` (262,144 B), `war3map.wct` (310 B), `war3map.wtg` (567 B), `war3map.w3c` (8 B), `war3map.w3r` (8 B), `war3map.j` (3,065,090 B), `war3mapMisc.txt` (1,860 B), `war3mapSkin.txt` (2,006 B), `war3mapImported\templates.toc`.
- Unit, item and ability data are in SLK overrides: `Units\UnitData.slk` (355,210 B), `Units\UnitBalance.slk` (430,025 B), `Units\UnitWeapons.slk` (164,892 B), `Units\UnitAbilities.slk` (114,678 B), `Units\AbilityData.slk` (715,863 B), `Units\UpgradeData.slk` (19,481 B), `units\ItemData.slk` (240,297 B). The lowercase `units\` spelling is also in the archive.
- Standard members absent from the archive: `war3map.w3u` (units), `war3map.w3t` (items), `war3map.w3q` (upgrades), `war3map.lua`, `war3map.imp`, `war3map.mmp`, `Scripts\war3map.j`, `Scripts\common.j`, `Scripts\<stock>.j`. Confirmed: no hash-table entry resolves to any of them under the standard names or the 6.88g candidate names.
- Asset members: about 2,200 textures and models (`war3mapImported\*.mdx`, `*.blp`, `*.mp3`), out of scope for this report.

## 2. Decoded tables

- Units: the object data is in `Units\UnitBalance.slk` (2,032 keyed rows), `Units\UnitData.slk` (2,036 keyed rows) and `Units\UnitWeapons.slk` (480 keyed rows). `war3map.j` calls `CreateUnit*` 737 times with 229 distinct raw ids; 228 of them are rows in UnitBalance. Hero rows (`STR` column set) number 237 in UnitBalance; 6 of the 229 CreateUnit ids are heroes. Stock vs custom rows cannot be separated: the stock engine files are not in the archive.
- Items: `units\ItemData.slk` has 1,169 keyed rows: 412 with `goldcost`, 432 with `abilList`, 21 with `pickRandom`. `war3map.j` calls `CreateItem` with 4 distinct ids (`I00E`, `I00G`, `I02M`, `I0KK`). Recipes are not in the SLK (no recipe column). Cooldowns are an `AbilityData` reference (`cooldownID`), not a number in ItemData.
- Abilities (`war3map.w3a`, version 2): 16 original-table records (`A21X`, `A21W`, `A21V`, `A0VV`, `A0XL`, `A3H0`, `A0V7`, `A0VE`, `A0EF`, `A0NR`, `A0CQ`, `A0AR`, `A087`, `A08C`, `S00A`, `Aamk`) and 1 custom-table record (`ANeg` to `A0CP`, name `TRIGSTR_019`), 572 modifications in total. Each modification is code, type, level, data pointer, value (number, or string), then a 4-byte trailer; the stream ends exactly at the 15,336-byte end of the member. The previous pass stopped at record 4 because it did not read the trailer.
- Upgrades (`war3map.w3h`): 37 bytes, a single record (`BNeg`, buff table).
- Destructables (`war3map.w3b`): 6 original records. Doodads (`war3map.w3d`): 42 original records.
- Strings (`war3map.wts`): 125 trigger-string entries (`STRING n`); no unit names. Unit display names are in `units/OrcAbilityStrings.txt` (`[code]` sections with `name=`), see section 3.

## 3. Name-resolved comparison

Values are DotA Allstars 6.88g unless marked "ours". Each 6.88g value is read from the SLK field named in the table legend; "ours" is our `scenarios/moba/*.osd` value, converted to Dota units (1 Dota unit = 15.625 milli; speed in Dota units/s = milli-per-tick x 16 / 15.625; period in seconds = ticks / 16; range and sight in Dota units = milli / 15.625).

Name source: `units/OrcAbilityStrings.txt` (2,301,368 B) holds `[code]` sections with `name=`. 6,019 sections carry a name; 1,945 of the 2,032 `Units\UnitBalance.slk` rows are named. The SLK name fields are empty: `Units\unitUI.slk` has a `name` column with no value in any row, and no SLK cell holds a `TRIGSTR_` reference. `war3map.wts` (125 entries) has no unit names: it holds the game title, credits and command help.

Pairing rule: our unit key is matched to a 6.88g display name by exact normalised name (lowercase, letters and digits only), with the aliases listed below. Hero keys prefer a UnitBalance row with `STR` set; neutral keys prefer rows without `STR`. A name whose only row is a placeholder (no `UnitWeapons` dice1, and `spd` <= 1) is not paired: it goes to the shell list.

Aliases (ours to 6.88g name): `bane` to Bane Elemental, `zuus` to Zeus, `rattletrap` to Clockwerk, `riki` to Rikimaru, `tusk` to Tuskarr, `brewmaster` to Pandaren Brewmaster, `neutral_furbolg_champion` to Polar Furbolg Champion, `neutral_furbolg_ursa_warrior` to Polar Furbolg Ursa Warrior, `juggernaut_healing_ward` to Healing Ward.

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

Largest DIFFERENT values (relative to the 6.88g value; values labelled DotA Allstars 6.88g):

| Unit (ours) | 6.88g name (code) | Field | DotA Allstars 6.88g | ours | 6.88g source |
| --- | --- | --- | ---: | ---: | --- |
| roshan | Roshan (n00L) | armour | 6 | 30 | UnitBalance.def |
| lone_druid | Lone Druid (N014) | range | 128 | 550 | UnitWeapons.rangeN1 |
| neutral_dark_troll_warlord | Dark Troll Warlord (ndtw) | armour | 1 | 4 | UnitBalance.def |
| legion_commander | Legion Commander (E02K) | armour | 1 | -1 | UnitBalance.def |
| enchantress | Enchantress (UIN1) | armour | -1 | 1 | UnitBalance.def |
| broodmother | Broodmother (TX78) | armour | 1 | -1 | UnitBalance.def |
| beastmaster | Beastmaster (H00D) | armour | 3 | -1 | UnitBalance.def |
| treant | Treant (e00V) | sight | 850 | 1800 | UnitBalance.sight |
| venomancer | Venomancer (EC57) | armour | 1 | 0 | UnitBalance.def |
| vengefulspirit | Vengeful Spirit (UAG2) | armour | 1 | 0 | UnitBalance.def |
| undying | Undying (H00R) | armour | 3 | 0 | UnitBalance.def |
| tidehunter | Tidehunter (UST3) | armour | 2 | 0 | UnitBalance.def |
| templar_assassin | Templar Assassin (E01Y) | armour | 2 | 0 | UnitBalance.def |
| spectre | Spectre (E01B) | armour | 1 | 0 | UnitBalance.def |
| skywrath_mage | Skywrath Mage (H0DO) | armour | -1 | -2 | UnitBalance.def |

Name-resolved pairs (cell = DotA Allstars 6.88g / ours, then = match, ! different, ? missing). Fields: HP, armour, damage, attack period (s), move speed (Dota units/s), range, sight.

| Ours | 6.88g name (code) | HP | armour | damage | period | speed | range | sight |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| neutral_alpha_wolf | Alpha Wolf (n00S) | 600 / 600 = | 3 / 3 = | 31.5 / 28 ! | 1.35 / 1.3125 ! | 350 / 350.2 = | 90 / 90 = | 800 / 800 = |
| neutral_black_dragon | Black Dragon (nbwm) | 2000 / 2000 = | 4 / 4 = | 64.5 / 65 = | 1.5 / 1.5 = | 300 / 300 = | 500 / 300 ! | 800 / 800 = |
| neutral_black_drake | Black Drake (nbdk) | 950 / 950 = | - / 2 ? | 39.5 / 21 ! | 1.8 / 1.6875 ! | 350 / 350.2 = | 500 / 300 ! | 800 / 800 = |
| neutral_centaur_khan | Centaur Khan (ncnk) | 1100 / 1100 = | 4 / 4 = | 52 / 52 = | 1.5 / 1.5 = | 320 / 319.5 = | 100 / 100 = | 800 / 800 = |
| neutral_centaur_outrunner | Centaur Outrunner (ncen) | 550 / 350 ! | 2 / 1 ! | 19.5 / 19 = | 1.3 / 1.3125 = | 350 / 319.5 ! | 100 / 100 = | 800 / 800 = |
| neutral_dark_troll | Dark Troll (ndtr) | 500 / 500 = | - / 0 ? | 25.5 / 26 = | 1.35 / 1.3125 ! | 270 / 270.3 = | 500 / 250 ! | 800 / 800 = |
| neutral_dark_troll_warlord | Dark Troll Warlord (ndtw) | 1100 / 1100 = | 1 / 4 ! | 51.5 / 42 ! | 1.35 / 1.3125 ! | 320 / 300 ! | 500 / 250 ! | 800 / 800 = |
| neutral_elder_jungle_stalker | Elder Jungle Stalker (njga) | 2000 / 2000 = | 4 / 5 ! | 67 / 67 = | 1.35 / 1.375 ! | 320 / 319.5 = | 128 / 100 ! | 800 / 800 = |
| neutral_enraged_wildkin | Enraged Wildkin (nowe) | 950 / 950 = | 4 / 4 = | 53 / 53 = | 1.35 / 1.3125 ! | 320 / 319.5 = | 128 / 128 = | 800 / 800 = |
| neutral_fel_beast | Fel Beast (npfl) | 400 / 400 = | 1 / 1 = | 14.5 / 14 = | 1.5 / 1.5 = | 350 / 350.2 = | 100 / 100 = | 800 / 800 = |
| neutral_forest_troll_berserker | Forest Troll Berserker (nftb) | 500 / 500 = | 1 / 1 = | 32.5 / 32 = | 1.6 / 1.625 ! | 270 / 270.3 = | 500 / 500 = | 800 / 800 = |
| neutral_forest_troll_high_priest | Forest Troll High Priest (nfsh) | 450 / 450 = | - / 0 ? | 28.5 / 31 ! | 1.8 / 1.625 ! | 290 / 289.8 = | 600 / 600 = | 1400 / 1400 = |
| neutral_furbolg_champion | Polar Furbolg Champion (nfpc) | 700 / 700 = | 3 / 3 = | 41.5 / 42 = | 1.5 / 1.5 = | 320 / 319.5 = | 100 / 100 = | 800 / 800 = |
| neutral_furbolg_ursa_warrior | Polar Furbolg Ursa Warrior (nfpu) | 950 / 950 = | 4 / 4 = | 52 / 52 = | 1.55 / 1.5 ! | 320 / 319.5 = | 100 / 100 = | 800 / 800 = |
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
| neutral_ogre_magi | Ogre Magi (nomg) (ambiguous) | 600 / 600 = | - / 0 ? | 25.5 / 19 ! | 1.35 / 1.3125 ! | 270 / 270.3 = | 100 / 100 = | 800 / 800 = |
| neutral_ogre_mauler | Ogre Mauler (nogm) | 850 / 800 ! | 1 / 1 = | 25.5 / 23 ! | 1.35 / 1.3125 ! | 270 / 270.3 = | 100 / 100 = | 800 / 800 = |
| neutral_rock_golem | Rock Golem (ngst) | 800 / 800 = | 4 / 4 = | 31 / 23 ! | 1.35 / 1.3125 ! | 270 / 270.3 = | 100 / 100 = | 800 / 800 = |
| neutral_satyr_hellcaller | Satyr Hellcaller (nsth) | 1100 / 1100 = | - / 2 ? | 52 / 52 = | 1.35 / 1.3125 ! | 290 / 289.8 = | 100 / 100 = | 800 / 800 = |
| neutral_satyr_soulstealer | Satyr Soulstealer (nstl) | 600 / 600 = | 1 / 2 ! | 25.5 / 22 ! | 1.35 / 1.625 ! | 270 / 270.3 = | 100 / 100 = | 800 / 800 = |
| neutral_satyr_trickster | Satyr Trickster (nsat) | 300 / 300 = | - / 0 ? | 8.5 / 11 ! | 1.7 / 1.625 ! | 330 / 300 ! | 300 / 280 ! | 800 / 800 = |
| neutral_wildkin | Wildkin (nowb) | 350 / 350 = | 2 / 2 = | 22.5 / 19 ! | 1.35 / 1.3125 ! | 300 / 300 = | 128 / 128 = | 800 / 800 = |
| roshan | Roshan (n00L) | 7500 / 6000 ! | 6 / 30 ! | 75 / 75 = | 1 / 2 ! | 270 / 270.3 = | 200 / 150 ! | 1400 / 1400 = |
| alchemist | Alchemist (N01H) (ambiguous) | 150 / 120 ! | 1 / 0 ! | 28.5 / 30 ! | 1.4 / 1.6875 ! | 325 / 289.8 ! | 128 / 150 ! | 1800 / 1800 = |
| ancient_apparition | Ancient Apparition (N0HP) (ambiguous) | 150 / 120 ! | - / -1 ? | 23.5 / 26 ! | 1.7 / 1.6875 = | 295 / 284.7 ! | 600 / 675 ! | 1800 / 1800 = |
| antimage | Anti-Mage (Edem) (ambiguous) | 150 / 120 ! | 1 / 2 ! | 29 / 31 ! | 1.45 / 1.375 ! | 315 / 315.4 = | 128 / 150 ! | 1800 / 1800 = |
| arc_warden | Arc Warden (N0MM) (ambiguous) | 150 / 120 ! | -1 / 0 ! | 30 / 26 ! | 1.7 / 1.6875 = | 295 / 300 ! | 625 / 625 = | 1800 / 1800 = |
| axe | Axe (Opgh) (ambiguous) | 150 / 120 ! | - / 0 ? | 26 / 33 ! | 1.7 / 1.6875 = | 290 / 315.4 ! | 128 / 150 ! | 1800 / 1800 = |
| bane | Bane Elemental (UIN6) | 150 / 120 ! | 2 / 0 ! | 36 / 27 ! | 1.7 / 1.6875 = | 315 / 305.2 ! | 400 / 425 ! | 1800 / 1800 = |
| batrider | Batrider (O016) (ambiguous) | 150 / 120 ! | 1 / 1 = | 16 / 14 ! | 1.7 / 1.6875 = | 290 / 310.3 ! | 375 / 400 ! | 1200 / 1800 ! |
| beastmaster | Beastmaster (H00D) | 150 / 120 ! | 3 / -1 ! | 43 / 25 ! | 1.7 / 1.6875 = | 310 / 305.2 ! | 128 / 150 ! | 1800 / 1800 = |
| bloodseeker | Bloodseeker (TX73) (ambiguous) | 150 / 120 ! | 1 / 2 ! | 32 / 38 ! | 1.7 / 1.6875 = | 290 / 284.7 ! | 128 / 150 ! | 1800 / 1800 = |
| bounty_hunter | Bounty Hunter (UAG3) | 150 / 120 ! | 4 / 5 ! | 31 / 37 ! | 1.7 / 1.5 ! | 315 / 324.6 ! | 128 / 150 ! | 1800 / 1800 = |
| brewmaster | Pandaren Brewmaster (Npbm) | 150 / 120 ! | 0 / 1 ! | 32.5 / 30 ! | 1.7 / 1.6875 = | 300 / 310.3 ! | 128 / 150 ! | 1800 / 1800 = |
| bristleback | Bristleback (H008) (ambiguous) | 150 / 120 ! | 0 / 1 ! | 31 / 34 ! | 1.8 / 1.8125 = | 290 / 294.9 ! | 128 / 150 ! | 1800 / 1800 = |
| broodmother | Broodmother (TX78) (ambiguous) | 150 / 120 ! | 1 / -1 ! | 29 / 30 ! | 1.7 / 1.6875 = | 295 / 284.7 ! | 128 / 175 ! | 0 / 1800 ! |
| chaos_knight | Chaos Knight (U00A) | 150 / 120 ! | 3 / 2 ! | 44 / 42 ! | 1.7 / 1.6875 = | 325 / 324.6 = | 128 / 150 ! | 1800 / 1800 = |
| crystal_maiden | Crystal Maiden (Hjai) (ambiguous) | 150 / 120 ! | - / 0 ? | 22 / 31 ! | 1.7 / 1.6875 = | 285 / 279.6 ! | 600 / 600 = | 1800 / 1800 = |
| dark_seer | Dark Seer (H00N) (ambiguous) | 150 / 120 ! | 6 / 2 ! | 34 / 34 = | 1.7 / 1.6875 = | 300 / 294.9 ! | 128 / 150 ! | 1800 / 1800 = |
| death_prophet | Death Prophet (TX66) (ambiguous) | 150 / 120 ! | 2 / 1 ! | 30 / 27 ! | 1.7 / 1.6875 = | 280 / 289.8 ! | 600 / 600 = | 1800 / 1800 = |
| disruptor | Disruptor (E02J) (ambiguous) | 150 / 120 ! | - / 0 ? | 29 / 29 = | 1.7 / 1.6875 = | 300 / 300 = | 600 / 625 ! | 1800 / 1800 = |
| doom_bringer | Doom Bringer (TX32) (ambiguous) | 150 / 120 ! | - / 1 ? | 35 / 37 ! | 1.7 / 1.875 ! | 290 / 289.8 = | 150 / 175 ! | 1800 / 1800 = |
| dragon_knight | Dragon Knight (H00F) (ambiguous) | 150 / 120 ! | 2 / 0 ! | 30 / 37 ! | 1.7 / 1.625 ! | 315 / 310.3 ! | 500 / 150 ! | 1800 / 1800 = |
| drow_ranger | Drow Ranger (Nbrn) | 150 / 120 ! | -2 / 0 ! | 23.5 / 31 ! | 1.7 / 1.6875 = | 300 / 310.3 ! | 625 / 625 = | 1800 / 1800 = |
| earth_spirit | Earth Spirit (N0MU) (ambiguous) | 150 / 120 ! | 2 / 0 ! | 30 / 27 ! | 1.7 / 1.6875 = | 295 / 289.8 ! | 150 / 150 = | 1800 / 1800 = |
| earthshaker | Earthshaker (TX27) (ambiguous) | 150 / 120 ! | 2 / 2 = | 29 / 32 ! | 1.7 / 1.6875 = | 310 / 315.4 ! | 128 / 150 ! | 1800 / 1800 = |
| ember_spirit | Ember Spirit (N0M0) (ambiguous) | 150 / 120 ! | -1 / 0 ! | 32 / 32 = | 1.7 / 1.6875 = | 310 / 294.9 ! | 128 / 150 ! | 1800 / 1800 = |
| enchantress | Enchantress (UIN1) (ambiguous) | 150 / 120 ! | -1 / 1 ! | 36 / 26 ! | 1.7 / 1.6875 = | 335 / 320.5 ! | 550 / 575 ! | 1800 / 1800 = |
| enigma | Enigma (UIN5) (ambiguous) | 150 / 120 ! | 3 / 1 ! | 25 / 22 ! | 1.7 / 1.6875 = | 300 / 279.6 ! | 500 / 500 = | 1800 / 1800 = |
| faceless_void | Faceless Void (EC45) (ambiguous) | 150 / 120 ! | 2 / 0 ! | 40 / 37 ! | 1.7 / 1.6875 = | 300 / 300 = | 128 / 150 ! | 1800 / 1800 = |
| gyrocopter | Gyrocopter (E02N) (ambiguous) | 150 / 120 ! | 2 / 1 ! | 22 / 21 ! | 1.7 / 1.6875 = | 315 / 315.4 = | 365 / 365 = | 1800 / 1800 = |
| invoker | Invoker (H00U) (ambiguous) | 150 / 120 ! | - / -1 ? | 22 / 22 = | 1.7 / 1.6875 = | 280 / 279.6 = | 600 / 600 = | 1800 / 1800 = |
| juggernaut | Juggernaut (Nbbc) | 150 / 120 ! | 1 / 0 ! | 26 / 23 ! | 1.4 / 1.375 ! | 305 / 305.2 = | 128 / 150 ! | 1800 / 1800 = |
| keeper_of_the_light | Keeper of the Light (H06W) (ambiguous) | 150 / 120 ! | - / 0 ? | 24 / 24 = | 1.7 / 1.6875 = | 315 / 310.3 ! | 600 / 600 = | 1800 / 1800 = |
| legion_commander | Legion Commander (E02K) (ambiguous) | 150 / 120 ! | 1 / -1 ! | 37 / 35 ! | 1.7 / 1.6875 = | 320 / 324.6 ! | 128 / 150 ! | 1800 / 1800 = |
| lich | Lich (Ulic) (ambiguous) | 150 / 120 ! | - / -1 ? | 28.5 / 29 = | 1.7 / 1.6875 = | 315 / 289.8 ! | 550 / 550 = | 1800 / 1800 = |
| life_stealer | Lifestealer (TX70) (ambiguous) | 150 / 120 ! | - / 1 ? | 37 / 26 ! | 1.7 / 1.6875 = | 315 / 320.5 ! | 128 / 150 ! | 1800 / 1800 = |
| lone_druid | Lone Druid (N014) (ambiguous) | 550 / 120 ! | 7 / 0 ! | 24 / 24 = | 1.6 / 1.625 ! | 280 / 300 ! | 128 / 550 ! | 1800 / 1800 = |
| magnataur | Magnataur (TX35) (ambiguous) | 150 / 120 ! | 3 / 1 ! | 34 / 35 ! | 1.7 / 1.8125 ! | 315 / 305.2 ! | 128 / 200 ! | 1800 / 1800 = |
| morphling | Morphling (O00P) (ambiguous) | 2150 / 120 ! | -1 / -1 = | 17.5 / 17 = | 1.6 / 1.5 ! | 285 / 284.7 = | 350 / 350 = | 1800 / 1800 = |
| naga_siren | Naga Siren (HC49) | 150 / 120 ! | 4 / 1 ! | 24 / 24 = | 1.7 / 1.6875 = | 320 / 329.7 ! | 128 / 150 ! | 1800 / 1800 = |
| necrolyte | Necrolyte (U00E) (ambiguous) | 150 / 120 ! | 1 / 2 ! | 24 / 28 ! | 1.7 / 1.6875 = | 290 / 279.6 ! | 550 / 500 ! | 1800 / 1800 = |
| night_stalker | Night Stalker (Udre) | 150 / 120 ! | 4 / 2 ! | 40 / 42 ! | 1.7 / 1.6875 = | 295 / 294.9 = | 128 / 150 ! | 1200 / 800 ! |
| obsidian_destroyer | Obsidian Destroyer (U00P) | 150 / 120 ! | 3 / 0 ! | 27.5 / 28 = | 1.7 / 1.875 ! | 315 / 315.4 = | 450 / 450 = | 1800 / 1800 = |
| ogre_magi | Ogre Magi (TX74) (ambiguous) | 150 / 120 ! | 7 / 4 ! | 44 / 48 ! | 1.7 / 1.6875 = | 295 / 289.8 ! | 128 / 150 ! | 1800 / 1800 = |
| omniknight | Omniknight (Harf) (ambiguous) | 150 / 120 ! | 4 / 2 ! | 36 / 36 = | 1.7 / 1.6875 = | 305 / 310.3 ! | 128 / 150 ! | 1800 / 1800 = |
| oracle | Oracle (N0MD) (ambiguous) | 150 / 120 ! | 1 / 1 = | 25 / 16 ! | 1.4 / 1.6875 ! | 305 / 294.9 ! | 620 / 625 ! | 1800 / 1800 = |
| phantom_assassin | Phantom Assassin (Ewar) (ambiguous) | 150 / 120 ! | 2 / 1 ! | 24 / 36 ! | 1.7 / 1.6875 = | 310 / 310.3 = | 128 / 150 ! | 1800 / 1800 = |
| phantom_lancer | Phantom Lancer (Ogrh) (ambiguous) | 150 / 120 ! | 1 / 0 ! | 33 / 31 ! | 1.7 / 1.6875 = | 290 / 284.7 ! | 128 / 225 ! | 1800 / 1800 = |
| phoenix | Phoenix (E02F) (ambiguous) | 150 / 120 ! | -1 / -1 = | 31 / 26 ! | 1.7 / 1.6875 = | 285 / 279.6 ! | 500 / 500 = | 1800 / 1800 = |
| queenofpain | Queen of Pain (UC01) | 150 / 120 ! | - / 0 ? | 29 / 29 = | 1.6 / 1.5 ! | 295 / 289.8 ! | 550 / 550 = | 1800 / 1800 = |
| sand_king | Sand King (TX40) (ambiguous) | 150 / 120 ! | 1 / 1 = | 33 / 24 ! | 1.7 / 1.6875 = | 300 / 289.8 ! | 128 / 150 ! | 1800 / 1800 = |
| shadow_demon | Shadow Demon (E02H) (ambiguous) | 150 / 120 ! | 1 / 0 ! | 29 / 31 ! | 1.7 / 1.6875 = | 295 / 289.8 ! | 500 / 500 = | 1800 / 1800 = |
| shadow_shaman | Shadow Shaman (Orkn) | 150 / 120 ! | - / 2 ? | 29.5 / 44 ! | 1.7 / 1.6875 = | 285 / 284.7 = | 500 / 400 ! | 1800 / 1800 = |
| silencer | Silencer (N01A) (ambiguous) | 150 / 120 ! | - / 0 ? | 23 / 25 ! | 1.7 / 1.625 ! | 295 / 300 ! | 600 / 600 = | 1800 / 1800 = |
| skeleton_king | Skeleton King (NC00) (ambiguous) | 150 / 120 ! | 1 / 0 ! | 40 / 41 ! | 1.7 / 1.8125 ! | 300 / 310.3 ! | 128 / 150 ! | 1800 / 1800 = |
| skywrath_mage | Skywrath Mage (H0DO) (ambiguous) | 150 / 120 ! | -1 / -2 ! | 17 / 21 ! | 1.7 / 1.6875 = | 325 / 324.6 = | 600 / 625 ! | 1800 / 1800 = |
| spectre | Spectre (E01B) (ambiguous) | 150 / 120 ! | 1 / 0 ! | 25 / 25 = | 1.7 / 1.8125 ! | 290 / 294.9 ! | 128 / 150 ! | - / 1800 ? |
| spirit_breaker | Spiritbreaker (O00J) (ambiguous) | 150 / 120 ! | 4 / 1 ! | 36 / 39 ! | 1.9 / 1.875 ! | 290 / 294.9 ! | 128 / 170 ! | 1800 / 1800 = |
| storm_spirit | Storm Spirit (H00S) (ambiguous) | 150 / 120 ! | 3 / 1 ! | 27 / 29 ! | 1.7 / 1.625 ! | 285 / 284.7 = | 480 / 480 = | 1800 / 1800 = |
| templar_assassin | Templar Assassin (E01Y) | 150 / 120 ! | 2 / 0 ! | 33 / 31 ! | 1.7 / 1.625 ! | 305 / 315.4 ! | 140 / 250 ! | 1800 / 1800 = |
| tidehunter | Tidehunter (UST3) (ambiguous) | 150 / 120 ! | 2 / 0 ! | 28 / 28 = | 1.7 / 1.6875 = | 305 / 300 ! | 128 / 150 ! | 1800 / 1800 = |
| tinker | Tinker (Ntin) (ambiguous) | 150 / 120 ! | 3 / 2 ! | 25 / 25 = | 1.7 / 1.6875 = | 305 / 305.2 = | 500 / 500 = | 1800 / 1800 = |
| treant | Treant (e00V) (ambiguous) | 700 / 120 ! | 3 / 1 ! | 40 / 64 ! | 1 / 1.875 ! | 325 / 279.6 ! | 100 / 150 ! | 850 / 1800 ! |
| troll_warlord | Troll Warlord (N016) (ambiguous) | 150 / 120 ! | - / -1 ? | 26 / 31 ! | 1.7 / 1.6875 = | 300 / 289.8 ! | 500 / 500 = | 1800 / 1800 = |
| tusk | Tuskarr (E02I) (ambiguous) | 150 / 120 ! | 1 / 1 = | 29 / 29 = | 1.7 / 1.6875 = | 305 / 310.3 ! | 128 / 150 ! | 1800 / 1800 = |
| undying | Undying (H00R) (ambiguous) | 150 / 120 ! | 3 / 0 ! | 36 / 37 ! | 1.7 / 1.6875 = | 310 / 294.9 ! | 128 / 150 ! | 1800 / 1800 = |
| vengefulspirit | Vengeful Spirit (UAG2) (ambiguous) | 150 / 120 ! | 1 / 0 ! | 19 / 31 ! | 1.7 / 1.5 ! | 300 / 300 = | 400 / 400 = | 1800 / 1800 = |
| venomancer | Venomancer (EC57) (ambiguous) | 150 / 120 ! | 1 / 0 ! | 20 / 20 = | 1.7 / 1.6875 = | 285 / 279.6 ! | 450 / 450 = | 1800 / 1800 = |
| warlock | Warlock (E01C) (ambiguous) | 150 / 120 ! | 2 / 1 ! | 27 / 32 ! | 1.7 / 1.6875 = | 295 / 300 ! | 600 / 600 = | 1800 / 1800 = |
| windrunner | Windrunner (N0EG) (ambiguous) | 150 / 120 ! | - / 0 ? | 28 / 30 ! | 1.5 / 1.5 = | 295 / 284.7 ! | 600 / 600 = | 1800 / 1800 = |
| winter_wyvern | Winter Wyvern (N0M7) (ambiguous) | 150 / 120 ! | - / 1 ? | 16.5 / 20 ! | 1.7 / 1.6875 = | 285 / 284.7 = | 425 / 450 ! | 1800 / 1800 = |
| witch_doctor | Witch Doctor (E01A) (ambiguous) | 150 / 120 ! | - / 0 ? | 32 / 31 ! | 1.7 / 1.6875 = | 305 / 300 ! | 600 / 550 ! | 1800 / 1800 = |

Names found only on a placeholder row (no weapon, spd <= 1; the hero shell carries HP 900 or 1200), so no stats were paired: observer_ward (Observer Ward, o004), sentry_ward (Sentry Ward, oeye), mirana (Mirana, h04W), nevermore (Nevermore, h063), puck (Puck, h050), pudge (Pudge, h060), razor (Razor, h05U), sven (Sven, h044), tiny (Tiny, h04K), zuus (Zeus, h040), kunkka (Kunkka, h06T), lina (Lina, h049), lion (Lion, h05C), slardar (Slardar, h05P), riki (Rikimaru, h047), sniper (Sniper, h04O), pugna (Pugna, h05W), viper (Viper, h05T), luna (Luna, h04N), dazzle (Dazzle, h06E), rattletrap (Clockwerk, h051), leshrac (Leshrac, h059), furion (Furion, h04I), clinkz (Clinkz, h05R), huskar (Huskar, h04Y), weaver (Weaver, h062), jakiro (Jakiro, h04R), chen (Chen, h04M), ursa (Ursa, h04F), lycan (Lycan, h05H), rubick (Rubick, h0CT), wisp (Wisp, h0CI), visage (Visage, h05F), slark (Slark, h072), medusa (Medusa, h05K), centaur (Centaur, h04X), abaddon (Abaddon, h067), terrorblade (Terrorblade, h056), techies (Techies, h04L), juggernaut_healing_ward (Healing Ward, ohwd).

Our units with no 6.88g name match (63): hero, tower, base, creep, tender, objective, ironhide, swiftbow, stormcaller, bulwark, shadeblade, emberlord, gale_archer, frostmage, tidecaller, thornwarden, bounty_rune, courier, creep_ranged, creep_siege, creep_super, creep_ranged_super, creep_siege_super, creep_mega, creep_ranged_mega, creep_siege_mega, tower_t2, tower_t3, tower_t4, barracks_melee, barracks_ranged, outpost, neutral_warpine_raider, neutral_prowler_acolyte, neutral_prowler_shaman, neutral_granite_golem, neutral_ice_shaman, neutral_frostbitten_golem, neutral_big_thunder_lizard, neutral_small_thunder_lizard, lane_spawner_top, lane_spawner_mid, lane_spawner_bot, meepo, nyx_assassin, shredder, elder_titan, abyssal_underlord, monkey_king, pangolier, dark_willow, grimstroke, mars, void_spirit, snapfire, hoodwink, dawnbreaker, marci, primal_beast, muerta, ringmaster, kez, largo. Our role keys (`hero`, `tower`, `base`, `creep`, `creep_*`, `tower_t*`, `barracks_*`, `outpost`, `tender`, `objective`, `lane_spawner_*`) carry no display name in our data, and the custom heroes (`ironhide` and others) and the newer Dota heroes (`pangolier`, `void_spirit` and others) have no 6.88g name.

Lane creep, tower and ancient (Ghoul u001 / Ziggurat uzig / Ancient Protector e00S, etc.) are not paired by name: our keys are role names and the 6.88g names of the lane creep and tower rows are not the role names. Their 6.88g values are in the tables above only where a name matched.


## 4. Layout comparison

- Terrain: `war3map.w3e` is 129 x 129 corners (16,641 cells) with origin -8192 on both axes, i.e. a 16,384-unit square at 128 units per tile. Our `map.osd` is 327 x 327 cells at 64 units per cell from the Dota 2 patch 7.41 dump. Different maps and scales: DIFFERENT.
- Doodads and placed units: `war3map.doo` is 259,494 B (its header declares the record count, not decoded here). `war3mapUnits.doo` is 246 B, so it holds very few placed units. Lane, tower, barracks, shop and camp positions were not extracted: MISSING.
- Regions and player starts: not decoded: MISSING.

## 5. Trigger script findings (`war3map.j`)

- Size: 102,278 lines, 4,503 functions, 861 `CreateUnit*` calls, 165 `TimerStart` calls, 142 `CreateTimer` calls.
- Obfuscated: the functions use short generated names (`Me`, `MS`, `SIE`, `Ez`, `KZE`). Unit and ability codes are the usual four-character raw ids; 5,755 distinct raw ids appear.
- Wave logic: the creep wave timer and spawn routine are isolated in section 8. Compositions per lane and the siege timing are not isolated. Lane creep wave composition is MISSING.
- Creep acquisition and attack logic: not isolated: MISSING.

## 6. What could not be read

- `war3map.w3u`, `war3map.w3t`, `war3map.w3q`, `war3map.lua`, `war3map.imp`, `war3map.mmp`, `Scripts\war3map.j`, `Scripts\common.j` and `Scripts\<stock>.j` are not in the archive (confirmed by hashing every listed name and the candidate set).
- Lane creep, tower and ancient rows are not resolved by name (our keys are role names; the 6.88g names are in section 3). The 40 names that only match placeholder rows, and the 58 ambiguous names, need the trigger code to confirm.
- Items: recipes are not in the SLK, and cooldown values need the `AbilityData` chain (`cooldownID` to `Cool1`).
- Placed-unit and doodad positions, player starts and region data are not decoded.
- The obfuscated trigger script was not decoded into wave timings or compositions.

## 7. Applied: heroes (this pass)

Decisions, each with its reason:

- Source tables: `UnitBalance`, `UnitData`, `UnitWeapons`, `UnitUI`, `UnitAbilities`, `UpgradeData`, `ItemData` and `AbilityData` were decoded from the SLK members (the `ID;PWXL` text tables, parsed cell by cell). Names come from the strings member (section keys with `name=`, 6,019 sections). Extraction used the libmpq binding shipped in the `libmpq` wheel (`mpq.py` with `libmpq.so`), because the pip `libmpq` module name is a different package. The archive listfile is encrypted for the pure-Python reader, so member indices were matched to names by their SLK headers.
- Canonical row per hero name: the balance rows whose `Primary` is STR, AGI or INT and whose name matches our hero id after lowercasing and stripping non-alphanumerics. Among duplicate rows, the most frequent stat tuple wins, then a non-`TX` id (the `TX` prefix marks duplicated test or skin copies). Reason: duplicates carry identical stats, so the choice only fixes the id, not the value.
- Scale (unchanged from `scripts/gen-heroes.mjs`): 1 WC3 unit = 15.625 milli; move speed milli per tick = spd x 15.625 / 16; attack period ticks = bat x 16; sight and range milli = WC3 units x 15.625; radius = collision x 15.625. Reason: the sim already uses this scale for the Dota 2 values, so the 6.88g numbers are expressed in the same model.
- Weapon damage: `dmgplus1 + dice1 x (sides1 + 1) / 2` (average of the dice roll). Reason: the sim models one damage number, so the 6.88g dice average is the closest expressible value (min and max are a gap, see below).
- Armour: the balance `def` base only. Reason: the sim adds agility to armour itself (gate: `q16(agi/6)`), so adding it here would double count.
- Hit points: the balance `HP` field is used as `health`. Reason: the sim adds strength on top of this base, as it did for the Dota 2 values.
- Mana and regeneration: `mana0` for mana, `regenMana` and `regenHP` per second (hundredths) for `mana_regen` and `hp_regen`.

Counts for heroes:

| Category | Count | Notes |
|---|---|---|
| Our hero units | 127 | `units_heroes.osd` |
| Matched to a 6.88g canonical row, values applied | 69 | `scenarios/moba/heroes_688g.tsv`, applied by `scripts/apply-heroes-688g.py` |
| Unresolved, left at the Dota 2 values | 58 | see gaps: the name exists only on `h0xx`/`n0xx` rows with `Primary` `_` (no playable stat row found) |
| Dropped (no 6.88g hero) | 10 | `ironhide`, `swiftbow`, `stormcaller`, `bulwark`, `shadeblade`, `emberlord`, `gale_archer`, `frostmage`, `tidecaller`, `thornwarden`: no 6.88g name, dropped by `scripts/apply-creeps-688g.py`. `hero` is kept as the generic player-hero kind |
| Added (6.88g hero we lack) | 0 | not attempted in this pass |

Gaps the sim cannot express (not invented):

- Turn rate (`turnRate` in `UnitData`) has no field in `unit()`.
- Weapon damage min and max (dice and sides) collapse to one number.
- Projectile speed for ranged heroes is not in the SLK rows read here.
- Magic resistance: 6.88g has no field; `mr=25` is kept from the Dota 2 block.
- Hero bounty (level-based in 6.88g) is not in `unit_extra`; kept at 0.
- The sim's per-strength health (22) and per-intelligence mana (12) factors are the Dota 2 constants; 6.88g's factors are not in the SLK rows.

Gate changes (`rust/gate`), each for an old value:

- `hero_axe_level1` and `hero_axe_growth` (`rust/gate/src/hero_checks.rs`): Axe now has AGI 20 (was 18), health base 150 (was 120), damage 26 (was 33), mana base 100 (was 75), growth 2.5/2.2/1.6 (was 2.7/1.7/1.6). Reason: the expectations were the Dota 2 values; they now carry the 6.88g numbers.
- `phantom_lancer_illusions` (`rust/gate/src/ability_checks.rs`): the victim spawns one unit away instead of two. Reason: 6.88g Phantom Lancer has a 128 range (melee), and the old placement is out of range. The check is about illusions, not range.

Verification (local): `osdt-data` loads `scenarios/moba`; `mod_check` on `moba_attack.lua` passes; `oscraft-gate` passes 197 of 197; `oscraft-gate mpq` passes. `moba_match` exits 1 with `no winner by tick=40000` both before and after this change (HEAD baseline gives the same line), so it is a pre-existing condition, not a regression. Superseded: on origin main `fa6862f`, `moba_match ../scenarios/moba 40000` exits 0 with `winner=0 tick=29089` (section 8).

Not done in this pass (the rest of the brief): creeps and neutrals in `units.osd` and `units_extra.osd` (77 rows), the 61 unresolved heroes, and every ability in `abilities.osd` and `abilities_dota.osd` (AbilityData `Cool1`, `Cost1`, `Rng1`, `Area1`, `Dur1`, `DataA1`..`DataI1` per level, read by the effect model). The ability mapping needs the `effect` model in `rust/crates/data/src/builtins.rs` matched per kind before values can be written.

## 8. Wave, siege, start, day/night and buyback timing (this pass)

Line numbers are in the extracted `war3map.j` (`/tmp/mpq688/out`), unless marked "ours".

Changed: nothing. Every map-sourced value below either matches our data or has no value in the map to compare with.

Map values:

- Creep wave cadence: 30 s periodic. `TriggerRegisterTimerEventPeriodic(xR,30.)` at line 30050, with `TriggerExecute(xR)` at line 30051, so the first wave fires at game start. Action `JAE` (line 26200, bound at line 101660). `JAE` spawns through `JRE` with raw ids `u001`, `n12Q`, `ugho` and `u002`, `unec`, `n12R`, chosen by flags. Every seventh execution (`ModuloInteger(GetTriggerExecCount(xR),7)==0`) calls `H5E` at line 26201. `H5E` only writes stats, so the seventh-wave rule is an upgrade hook, not a spawn change.
- Ours: `mods/moba_waves.lua` `CREEP_PERIOD = 480` ticks. At `TICKS_PER_SECOND = 16` (`rust/crates/sim/src/lib.rs:65`) that is 30 s. Match. The first creep is at game timer 1 tick, which is the start, so it matches the map's immediate `TriggerExecute`.
- Siege timing: NOT FOUND in the map. No trigger in this pass fires a siege or name-based siege spawn. Our `SIEGE_STARTS = {4800, 28800, 57600}` (`mods/moba_waves.lua` line 13, 5, 30 and 60 min at 16 ticks/s) and `SIEGE_PERIOD = 4800` have no map source here. The 5-minute value is a public description, not map-sourced, and is kept unchanged.
- Game-start timer: `TimerStart(CreateTimer(),0xB4,false,function POE)` at line 30036 is a one-shot 180 s (`0xB4`) that sets flag `Oo=true` (`POE` at lines 30013-30016). The flag's consumers are not decoded. The countdown before start is `TimerStart(SA,TA+1,false,null)` at line 36638 (`RZX`). `TA` is a global `90` (line 791), reassigned to `0x96` = 150 (line 37252, `I9X`), `0xB4` = 180 (line 39444), `90` (lines 37852, 39662) and `'x'` = 120 (line 40408). The value in effect at start is not resolved, so no countdown length is claimed.
- Day and night: `SetTimeOfDayScale(.625)` at line 30035, and the start is dawn with `SetFloatGameState(GAME_STATE_TIME_OF_DAY,6.)` at line 30033. The map stores a rate, not a day or night length in seconds. Length is NOT in the map. Ours: `DAY_TICKS = 300 * 16` and `NIGHT_TICKS = 300 * 16` (`rust/crates/sim/src/lib.rs:70-71`, 5 min each). These are unsourced from the map and kept unchanged.
- Buyback cooldown: NOT FOUND. No buyback timer or cooldown value is in the map. The `UnitResetCooldown` calls at lines 26211-26220 and 3735-3745 reset unit cooldowns, not buyback. Ours: `BUYBACK_NET_WORTH_DIV = 13` (`rust/crates/sim/src/hero.rs:25`) is a cost divisor. There is no cooldown constant to compare.
- Other map timers, unmatched to a repo value: 450 s periodic `oR` (line 30052, action `JTE` at line 26536, which swaps tech research counts); 75 s one-shot `rR` (line 101706, action `JUE`); 2669 s one-shot `vR` (line 30053, action `HSE` at line 26000, increments `Ji`); 849 s one-shot `eR` (line 30054, action `HTE` at line 26003, increments `ji`). These are not matched to our wave, siege or day data.

Repo sources that are not map data: `scenarios/moba/waves.osd` holds only lane spawner placements. The wave cadence is in `mods/moba_waves.lua`, not in `waves.osd`. `rust/crates/sim/src/camp.rs` `CAMP_FIRST_SPAWN_S = 60` is a neutral-camp value, not a wave value, so it is not compared.

Verification on origin main `fa6862f` (fresh clone):

- `oscraft-gate` (rust, release): `gate: 203 checks, 0 failed`.
- `moba_match ../scenarios/moba 40000` (rust, release): exit 0, `winner=0 tick=29089`, replay hash `35358641158f53c0`. This is the result that holds on current main. The earlier `no winner by tick=40000` and the `winner=0 tick=29089` reports came from different builds.

## 8. Applied: neutral creeps and Roshan

Source: `scenarios/moba/creeps_688g.tsv`, applied by `scripts/apply-creeps-688g.py` to `units.osd`. Pairing is by exact display name (`units/OrcAbilityStrings.txt`); neutral keys prefer rows with `Primary` `_`. Formulas as heroes: damage is `dmgplus1 + dice1 x (sides1 + 1) / 2`, period is `cool1 x 16` ticks, regen is `regenHP x 62.5` milli per tick; armour `def` (blank keeps the current value).

32 rows converted (31 neutrals and Roshan). Kept on current values: the 9 lane creep rows (role keys with no 6.88g display name) and 8 neutrals with no 6.88g name or row (`neutral_warpine_raider`, `neutral_prowler_acolyte`, `neutral_prowler_shaman`, `neutral_granite_golem`, `neutral_ice_shaman`, `neutral_frostbitten_golem`, `neutral_big_thunder_lizard`, `neutral_small_thunder_lizard`).

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

# 6.88f proxy map data analysis

> Historical: this note describes the MOBA-era design (the 6.88 reference map work). It is kept for the record, not as a description of the current generic engine.

Source label: every value taken from the reference proxy repository (owner BEN-HUR5, repo name withheld) is labelled "6.88f proxy". The 6.88g archive (wc3maps.com) is behind a Cloudflare challenge and returns 403, so the proxy is the only reachable copy. Our values come from `scenarios/moba/*` in this repository (reference ruleset npc dump at SteamDatabase tracking repo @282fad29f0f74504fad465c52d197fb61f4d612b).

Read method: `gh api repos/<proxy-owner>/<proxy-repo>/contents/<path>?ref=main` with the raw media type, parsed into scratch storage outside the repository.

## 1. Inventory (6.88f proxy)

| Location | Entries | Notes |
| --- | --- | --- |
| repo root | 22 files, 13 directories | 2,398,871 bytes of root files; includes `refmap.w3a` (878,361 bytes), `refmap.doo` (275,324), `refmap.w3e` (116,568), `refmap.shd` (262,144) |
| `Units/` | 51 files | 8,196,474 bytes; 9 `.slk` tables: AbilityBuffData, AbilityData, ItemData, UnitAbilities, UnitBalance, UnitData, UnitUI, UnitWeapons, UpgradeData |
| `Units/` unit and item strings | 6 `*UnitStrings.txt`, `ItemStrings.txt` and the race `*Func.txt` scripts | Used to resolve unit ids to names (Chinese for most entries, English for a few) |
| `Scripts/` | 1 file: `refmap.j`, 5,199,936 bytes, 135,555 lines | Obfuscated Jass (function names are hashed, for example `LX7`, `R1X`). Most of its text is hero-help tip strings |

Parsed tables (rows excluding the header): UnitData 1,898, UnitBalance 1,898, UnitUI 1,898, UnitWeapons 1,897, ItemData 1,103, AbilityData 4,476, UpgradeData 87. Localized name strings: 3,058 unit ids and 1,177 item strings.

## 2. Extracted tables

- Heroes: hero base stats for 16 shipped heroes. 11 map to proxy codes (see 2.1). The other five are UNMAPPED.
- Items: 313 items in `scenarios/moba/items.osd`. 69 match a 6.88f proxy English item name with a gold cost, 244 do not (proxy strings are localized).
- Creeps, towers and barracks: the proxy has stock towers (`hgtw` Guard Tower 500 HP, `hctw` Cannon Tower 600 HP, `owtw` Watch Tower 500 HP). The reference lane creep, tower and barracks roles are not identifiable from the obfuscated script, so these rows are MISSING.
- Waves: `refmap.j` does not expose a readable wave timer, composition or spawn-point table. The frequent `CreateUnit` codes resolve to helper or dummy units (`e00E` Spellcaster 194 calls, `n00C` region indicator 30 calls). MISSING.
- Abilities: cooldown and mana per level exist in `AbilityData.slk` for the proxy ability codes listed in 2.3, but the proxy ability names are localized, so they cannot be paired with our ability ids. MISSING.
- Recipes: `ItemData.slk` has no component columns in the proxy, so recipes are MISSING.

### 2.1 Hero identity used for the mapping (6.88f proxy)

Mapping is by the proxy's localized name in `Units/*UnitStrings.txt` for the unit id in `UnitBalance.slk` and the `R1W` hero table in `refmap.j`. Chinese names: 斧王 Axe (`Opgh`), 水晶室女 Crystal Maiden (`Hjai`), 主宰 Juggernaut (`Nbbc`), 奥林匹斯之王 Zeus (`Hmbr`), 矮人狙击手 Sniper (`Usyl`), 幻影长矛手 Phantom Lancer (`Ogrh`), 魅惑魔女 Dazzle (`Emoo`), 巫妖 Lich (`Ulic`), 潮汐猎人 Tidehunter (`Ofar`), 复仇之魂 Vengeful Spirit (`Hvwd`), 敌法师 Anti-Mage (`Edem`).

Unmapped: Sven, Lion, Omniknight, Lina, Slardar. Candidate ids exist (for example `Ucrl` STR, `Ekee` INT, `Hpal`/`Hhkl`/`Hpb1` "Paladin", `Hblm` INT, `Oshd` INT) but none is confirmed, so they are not compared.

Label: 6.88f proxy. Stat conventions: health and armor in the proxy are reference fields, and in our data `hero_rules.txt` stores armor in hundredths. Our range and sight are scaled by 15.625 milli per reference unit (`range_milli / 15.625`, `sight_milli / 15.625`).

### 2.2 Hero base stats (6.88f proxy vs ours)

Columns: ours first, proxy second. Health: ours is the reference `StatusHealth` base (`units_heroes.osd` line 1 field), proxy is reference base life 200.

| Ours | Proxy | Health | STR | AGI | INT | Mana | Armor | Move speed | Attack period | Range | Sight | Collision |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| axe | Opgh | DIFFERENT (120 vs 200) | MATCH (25 vs 25) | DIFFERENT (18 vs 20) | MATCH (18 vs 18) | DIFFERENT (75 vs 50) | MATCH (0 vs 0) | DIFFERENT (308 vs 290) | MATCH (1.7 vs 1.7) | MATCH (150 vs 150) | MATCH (1800 vs 1800) | MATCH (24 vs 24) |
| crystal_maiden | Hjai | DIFFERENT (120 vs 200) | DIFFERENT (17 vs 16) | MATCH (16 vs 16) | DIFFERENT (20 vs 16) | DIFFERENT (75 vs 50) | MATCH (0 vs 0) | DIFFERENT (273 vs 280) | MATCH (1.7 vs 1.7) | MATCH (600 vs 600) | MATCH (1800 vs 1800) | MATCH (24 vs 24) |
| juggernaut | Nbbc | DIFFERENT (120 vs 200) | MATCH (20 vs 20) | DIFFERENT (32 vs 26) | MATCH (14 vs 14) | DIFFERENT (75 vs 50) | DIFFERENT (0 vs 2) | DIFFERENT (298 vs 305) | MATCH (1.4 vs 1.4) | MATCH (150 vs 150) | MATCH (1800 vs 1800) | MATCH (24 vs 24) |
| zuus | Hmbr | DIFFERENT (120 vs 200) | DIFFERENT (21 vs 19) | MATCH (11 vs 11) | DIFFERENT (23 vs 20) | DIFFERENT (75 vs 50) | MATCH (1 vs 1) | DIFFERENT (298 vs 295) | MATCH (1.7 vs 1.7) | DIFFERENT (380 vs 350) | MATCH (1800 vs 1800) | MATCH (24 vs 24) |
| sniper | Usyl | DIFFERENT (120 vs 200) | DIFFERENT (19 vs 16) | DIFFERENT (27 vs 21) | MATCH (15 vs 15) | DIFFERENT (75 vs 50) | MATCH (0 vs 0) | DIFFERENT (278 vs 290) | MATCH (1.7 vs 1.7) | MATCH (550 vs 550) | MATCH (1800 vs 1800) | MATCH (24 vs 24) |
| phantom_lancer | Ogrh | DIFFERENT (120 vs 200) | MATCH (21 vs 21) | DIFFERENT (26 vs 29) | MATCH (21 vs 21) | DIFFERENT (75 vs 50) | DIFFERENT (0 vs 1) | DIFFERENT (278 vs 290) | MATCH (1.7 vs 1.7) | DIFFERENT (225 vs 150) | MATCH (1800 vs 1800) | DIFFERENT (24 vs 8) |
| dazzle | Emoo | DIFFERENT (120 vs 200) | DIFFERENT (19 vs 16) | DIFFERENT (20 vs 19) | DIFFERENT (25 vs 16) | DIFFERENT (75 vs 50) | MATCH (0 vs 0) | DIFFERENT (298 vs 335) | MATCH (1.7 vs 1.7) | DIFFERENT (575 vs 550) | MATCH (1800 vs 1800) | MATCH (24 vs 24) |
| lich | Ulic | DIFFERENT (120 vs 200) | DIFFERENT (20 vs 18) | DIFFERENT (17 vs 15) | DIFFERENT (28 vs 18) | DIFFERENT (75 vs 50) | DIFFERENT (-1 vs 0) | DIFFERENT (283 vs 315) | MATCH (1.7 vs 1.7) | DIFFERENT (550 vs 600) | MATCH (1800 vs 1800) | MATCH (24 vs 24) |
| tidehunter | Ofar | DIFFERENT (120 vs 200) | DIFFERENT (25 vs 22) | MATCH (15 vs 15) | DIFFERENT (20 vs 16) | DIFFERENT (75 vs 50) | DIFFERENT (0 vs 2) | DIFFERENT (293 vs 305) | MATCH (1.7 vs 1.7) | MATCH (150 vs 150) | MATCH (1800 vs 1800) | MATCH (24 vs 24) |
| vengefulspirit | Hvwd | DIFFERENT (120 vs 200) | DIFFERENT (20 vs 18) | DIFFERENT (23 vs 27) | DIFFERENT (19 vs 15) | DIFFERENT (75 vs 50) | DIFFERENT (0 vs 1) | DIFFERENT (293 vs 300) | DIFFERENT (1.5 vs 1.7) | MATCH (400 vs 400) | MATCH (1800 vs 1800) | MATCH (24 vs 24) |
| antimage | Edem | DIFFERENT (120 vs 200) | DIFFERENT (21 vs 22) | DIFFERENT (25 vs 22) | DIFFERENT (12 vs 15) | DIFFERENT (75 vs 50) | DIFFERENT (2 vs 0) | DIFFERENT (308 vs 315) | DIFFERENT (1.4 vs 1.45) | MATCH (150 vs 150) | MATCH (1800 vs 1800) | MATCH (24 vs 24) |

Damage for the 11 mapped heroes is UNVERIFIED: our `damage` is a hundredths midpoint, while the proxy uses `dmgplus` plus `dice` and `sides`, and the proxy `mindmg` and `maxdmg` fields do not reconcile with those columns, so no comparison is claimed.

### 2.3 Ability cooldown and mana (6.88f proxy, unpaired)

| Hero | Proxy ability code | Cooldown per level | Mana per level |
| --- | --- | --- | --- |
| axe | A0I6 | - | - |
| axe | A0S1 | - | - |
| axe | A0C6 | - | - |
| axe | A0E2 | - | - |
| crystal_maiden | A1E9 | - | - |
| crystal_maiden | A04C | - | - |
| crystal_maiden | AHab | - | - |
| crystal_maiden | A03R | - | - |
| juggernaut | A047 | - | - |
| juggernaut | A05G | - | - |
| juggernaut | A00K | - | - |
| juggernaut | A0M1 | - | - |
| zuus | A020 | - | - |
| zuus | A0N5 | - | - |
| zuus | A0JC | - | - |
| zuus | A29G | - | - |
| sniper | A04P | - | - |
| sniper | A03U | - | - |
| sniper | A03S | - | - |
| sniper | A064 | - | - |
| phantom_lancer | A10D | - | - |
| phantom_lancer | A2W7 | - | - |
| phantom_lancer | A2W1 | - | - |
| phantom_lancer | A0DB | - | - |
| dazzle | A0DX | - | - |
| dazzle | A01B | - | - |
| dazzle | A0DW | - | - |
| dazzle | A0DY | - | - |
| lich | A053 | - | - |
| lich | A07F | - | - |
| lich | A08R | - | - |
| lich | A05T | - | - |
| tidehunter | A29I | - | - |
| tidehunter | A226 | - | - |
| tidehunter | A046 | - | - |
| tidehunter | A04E | - | - |
| vengefulspirit | A02A | - | - |
| vengefulspirit | ACac | - | - |
| vengefulspirit | A17O | - | - |
| vengefulspirit | A0IN | - | - |
| antimage | A0KY | - | - |
| antimage | A022 | - | - |
| antimage | AEbl | - | - |
| antimage | A0E3 | - | - |

### 2.4 Items (6.88f proxy gold cost vs ours)

Name match is the English name in `ItemStrings.txt`. Cost is `goldcost` of the first proxy id with a cost that is not a recipe. The 244 unmatched items are MISSING.

| Ours (items.osd) | Ours gold | Proxy id | Proxy gold | Result |
| --- | --- | --- | --- | --- |
| mystic_staff | 2800 | I04N | 2700 | DIFFERENT |
| blades_of_attack | 450 | I045 | 420 | DIFFERENT |
| broadsword | 1000 | I046 | 1200 | DIFFERENT |
| chainmail | 500 | I047 | 550 | DIFFERENT |
| claymore | 1350 | I048 | 1400 | DIFFERENT |
| helm_of_iron_will | 975 | I04E | 900 | DIFFERENT |
| javelin | 900 | I055 | 1500 | DIFFERENT |
| mithril_hammer | 1600 | I04M | 1600 | MATCH |
| ring_of_protection | 175 | I04U | 175 | MATCH |
| hyperstone | 2000 | I04F | 2000 | MATCH |
| moon_shard | 4000 | I0OO | 4000 | MATCH |
| ogre_axe | 1000 | I04O | 1000 | MATCH |
| blade_of_alacrity | 1000 | I044 | 1000 | MATCH |
| staff_of_wizardry | 1000 | I050 | 1000 | MATCH |
| ultimate_orb | 2800 | I052 | 2100 | DIFFERENT |
| sobi_mask | 175 | I04Z | 325 | DIFFERENT |
| talisman_of_evasion | 1300 | I0J6 | 1700 | DIFFERENT |
| cheese | 1000 | I0B1 | 1000 | MATCH |
| magic_stick | 200 | I0GC | 200 | MATCH |
| magic_wand | 460 | I0HB | 465 | DIFFERENT |
| flying_courier | 100 | I01D | 250 | DIFFERENT |
| phase_boots | 1450 | I0GJ | 1240 | DIFFERENT |
| demon_edge | 2200 | I049 | 2400 | DIFFERENT |
| ring_of_health | 700 | I04T | 850 | DIFFERENT |
| void_stone | 700 | I054 | 850 | DIFFERENT |
| energy_booster | 800 | I04B | 900 | DIFFERENT |
| point_booster | 1200 | I04R | 1200 | MATCH |
| vitality_booster | 1000 | I053 | 1100 | DIFFERENT |
| soul_booster | 3000 | I091 | 3300 | DIFFERENT |
| hand_of_midas | 2200 | I06D | 2050 | DIFFERENT |
| oblivion_staff | 1625 | I06F | 1650 | DIFFERENT |
| orb_of_venom | 350 | oven | 400 | DIFFERENT |
| bracer | 505 | I06H | 525 | DIFFERENT |
| wraith_band | 505 | I06J | 485 | DIFFERENT |
| null_talisman | 505 | I06L | 470 | DIFFERENT |
| mekansm | 1775 | I093 | 2275 | DIFFERENT |
| ring_of_basilius | 425 | I067 | 500 | DIFFERENT |
| urn_of_shadows | 825 | I0KY | 875 | DIFFERENT |
| bloodthorn | 6400 | I0TE | 7195 | DIFFERENT |
| force_staff | 2200 | I0HI | 2225 | DIFFERENT |
| dagon | 3000 | I09L | 7720 | DIFFERENT |
| black_king_bar | 4050 | I09D | 3975 | DIFFERENT |
| veil_of_discord | 1700 | I0O3 | 2270 | DIFFERENT |
| bloodstone | 4700 | I0BK | 4875 | DIFFERENT |
| crimson_guard | 3725 | I0Q0 | 3550 | DIFFERENT |
| blade_mail | 2400 | I08O | 2200 | DIFFERENT |
| monkey_king_bar | 5000 | I0A5 | 5400 | DIFFERENT |
| radiance | 4700 | I0A8 | 5150 | DIFFERENT |
| yasha | 2100 | I08G | 2050 | DIFFERENT |
| sange | 2100 | I08H | 2050 | DIFFERENT |
| sange_and_yasha | 4200 | I095 | 4100 | DIFFERENT |
| satanic | 5050 | I0AB | 5900 | DIFFERENT |
| maelstrom | 2950 | I08P | 2800 | DIFFERENT |
| mjollnir | 5500 | I0BE | 5700 | DIFFERENT |
| helm_of_the_dominator | 2550 | I08T | 1800 | DIFFERENT |
| rod_of_atos | 2250 | I0OL | 2400 | DIFFERENT |
| blight_stone | 300 | I07V | 300 | MATCH |
| mask_of_madness | 1900 | I08Y | 1800 | DIFFERENT |
| diffusal_blade | 2500 | I08R | 3150 | DIFFERENT |
| ethereal_blade | 5200 | I0LT | 4700 | DIFFERENT |
| soul_ring | 805 | I0LL | 800 | DIFFERENT |
| arcane_boots | 1500 | I0MI | 1300 | DIFFERENT |
| octarine_core | 5100 | I0SD | 5900 | DIFFERENT |
| tranquil_boots | 900 | I0OG | 900 | MATCH |
| medallion_of_courage | 0 | I0N0 | 1175 | DIFFERENT |
| solar_crest | 2575 | I0QJ | 2875 | DIFFERENT |
| guardian_greaves | 4450 | I0QP | 5275 | DIFFERENT |
| abyssal_blade | 6250 | I0OX | 6400 | DIFFERENT |
| glimmer_cape | 2150 | I0QM | 1850 | DIFFERENT |

## 3. Comparison counts

| Category | MATCH | DIFFERENT | MISSING | UNVERIFIED |
| --- | --- | --- | --- | --- |
| Hero base stats, 11 mapped heroes (11 fields each) | 52 | 69 | 0 | 11 (damage) |
| Hero base stats, 5 unmapped heroes (11 fields each) | 0 | 0 | 55 | 0 |
| Items with a proxy English name match (gold cost) | 12 | 57 | 0 | 0 |
| Items with no proxy English name match | 0 | 0 | 244 | 0 |
| Lane creep, neutral creep, tower and barracks stats | 0 | 0 | all | 0 |
| Wave timing, composition and spawn points | 0 | 0 | all | 0 |
| Ability cooldown and mana | 0 | 0 | all | 0 |
| Recipes | 0 | 0 | all | 0 |

Our file and line references for the comparison: `scenarios/moba/units_heroes.osd` lines 5 onward (`unit("<hero>", health, speed, radius, ..., weapon_range_milli, weapon_period, sight_milli, armor)`); `scenarios/moba/hero_rules.txt` `hero <id> ...` lines from line 30 on; `scenarios/moba/items.osd` item lines (313 entries); `scenarios/moba/units.osd` lines 5 to 47 (tower, base, creep, barracks, outpost); `scenarios/moba/waves.osd`.

## 4. Differences that look like real parity gaps

Caveat: our data is the reference ruleset dump and the proxy is 6.88f, so most differences are expected patch drift. The list ranks differences that are per-unit gameplay values rather than encoding or scale.

| # | Item | Ours | Proxy |
| --- | --- | --- | --- |
| 1 | Phantom Lancer collision (radius) | ours 375 (24 reference units), units_heroes.osd line 27 | proxy 8 (Ogrh) |
| 2 | Dazzle move speed | ours 298, units_heroes.osd line 101 | proxy 335 (Emoo) |
| 3 | Lich move speed | ours 283, units_heroes.osd line 53 | proxy 315 (Ulic) |
| 4 | Lich armor | ours -1, hero_rules.txt (armor=-100) | proxy 0 (Ulic) |
| 5 | Juggernaut armor | ours 0, hero_rules.txt (armor=0) | proxy 2 (Nbbc) |
| 6 | Starting mana, all 11 mapped heroes | ours 75, hero_rules.txt (mana=75) | proxy 50 (manaN, e.g. Opgh) |
| 7 | Phantom Lancer attack range | ours 225 (3516 milli / 15.625) | proxy 150 (Ogrh) |
| 8 | Lich attack range | ours 550 (8594 milli / 15.625) | proxy 600 (Ulic) |
| 9 | Zeus attack range | ours 380 (5938 milli / 15.625) | proxy 350 (Hmbr) |
| 10 | Javelin gold cost | ours 900, items.osd | proxy 1500 (I055) |

Full per-hero and per-item rows are in the tables above. Other differences that look like drift and not gaps: all 11 hero move speeds (`units_heroes.osd` speed 273 to 308 against proxy 280 to 335), all 11 hero attack periods that differ (for example Vengeful Spirit 1.5 against 1.7), and item costs such as Sobi Mask 175 against 325.

## 5. Not reachable from the proxy

- Map layout, doodads, regions, starting positions and creep camps: these live in `refmap.w3e`, `refmap.doo` and `refmap.w3i` binary data, and the 6.88g `.w3x` archive is behind Cloudflare. Only the text tables above are reachable.
- Wave logic: `refmap.j` is obfuscated (hashed function names) and the wave routines were not isolated in this pass.
- Hero identity for Sven, Lion, Omniknight, Lina and Slardar: no confirmed proxy id.
- Tower, barracks and lane creep roles: no confirmed proxy id.

Default applied: hero identity by localized name (one line, stated above). No game data was changed.

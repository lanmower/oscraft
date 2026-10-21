# reference map 6.88w3 map research

> Historical: this note describes the MOBA-era design (the 6.88 reference map work). It is kept for the record, not as a description of the current generic engine.

Source: "reference map 6.88w3" (internal version string "reference map 6.88w3", from refmap.wts STRING 50000). File: `reference map 6.88w3.w3x`, 22,729,587 bytes. Every value below is from this map unless it names a repo file.

Extracted files live only under `/tmp/w3map` (not in the repo). Parsers: `/tmp/w3scripts`.

## Archive layout

- Map header `HM3W` at offset 0. MPQ header at offset 512, format v1 (header size 32), sector shift 13 (4096-byte sectors).
- Archive size field 22,729,075 bytes.
- Hash table: 4096 entries at 22,630,067. Block table: 2092 entries at 22,695,603. All 2092 hash entries point to a block (no free or deleted slots).
- Hash and block tables decrypt with key `(hash table)` / `(block table)` and verify.
- `(listfile)` is present but holds one name only (`NEW_LOADSCREEN.tga`, 20 bytes). The archive has no full file list, so the other names were found by hashing candidate names.

## Archive inventory (named members)

| Member | Packed | Unpacked | Flags | Notes |
|---|---|---|---|---|
| (listfile) | 28 | 20 | enc, zlib | one entry |
| refmap.w3i | 274 | 641 | zlib | map info |
| refmap.w3e | 29,764 | 116,584 | zlib | terrain |
| refmap.doo | 55,995 | 205,974 | zlib | doodads |
| refmap.j | 564,596 | 3,278,936 | enc, zlib | script (decrypted, obfuscated) |
| refmap.shd | 1,223 | 262,144 | zlib | shadow map |
| refmap.wts | 1,367 | 2,592 | enc, zlib | string table (decrypted) |
| refmap.w3a | 1,526 | 9,109 | zlib | ability modifications |
| refmap.w3b | 1,192 | 4,106 | zlib | destructable modifications |
| refmap.w3d | 1,778 | 12,523 | zlib | doodad modifications |
| refmap.wpm | 3,006 | 262,160 | zlib | pathing map |
| refmap.mmp | 147 | 408 | zlib | minimap |
| Units\UnitData.slk | 9,488 | 142,171 | zlib | base reference unit data |
| Units\UnitBalance.slk | 14,783 | 174,501 | zlib | base reference unit balance |
| Units\UnitUI.slk | 21,450 | 233,286 | zlib | base reference unit UI and model paths |
| Units\UnitWeapons.slk | 9,046 | 111,703 | zlib | base reference weapons |
| Units\UnitAbilities.slk | 9,349 | 56,769 | zlib | base reference unit abilities |
| Units\UpgradeData.slk | 1,929 | 14,278 | zlib | base reference upgrades |
| Units\ItemData.slk | 10,737 | 182,641 | zlib | base reference items |
| Units\AbilityData.slk | 90,441 | 607,516 | zlib | base reference abilities |

Not in the archive (lookup miss): refmap.w3u (units), refmap.w3t (items), refmap.w3h (upgrades), refmap.w3q, refmap.lua, refmapunits.doo, refmap.w3r, refmap.w3c, refmap.w3s, refmap.w3p.

Unnamed members: 2072 blocks. 2014 use zlib (sector mask 0x02), 41 are empty, 17 have no standard sector table. No member used bzip2 (0x10), PKWARE implode (0x08), or Huffman (0x01). No member needed an unsupported method, so none was skipped.

Encrypted members (flag 0x10000, FIX_KEY absent) decrypted with the filename key: refmap.j, refmap.wts, (listfile).

## Object data

- The archive has no refmap.w3u or refmap.w3t. Unit and item object data are the base reference game SLK tables (Units\UnitData.slk, UnitBalance, UnitUI, ItemData, AbilityData). The 6.88w3 map does not override them with its own binary object files.
- refmap.w3a (9,109 bytes, version 2, 14 original-object records) holds ability modifications. Its record layout (object id, new id, modification count, then id, level, data, and a value that is a 4-byte number or a null-terminated string followed by 4 bytes of padding) was parsed for the header only. The full value decode is not finished.
- Base SLK values for hero-class codes present in the map's script (base reference stats, not reference-specific): Hblm HP 200, move 335, STR 14, AGI 15, INT 25; Naka def 4, move 315; Ulic move 315; Ntin move 305, INT 30; Ogrh move 290, AGI 29. These are the stock base values for those codes. Their binding to the 16 reference heroes is not confirmed (see below).

## Wave logic (refmap.j)

- The script is obfuscated: identifiers are random (for example `MMRRI`, `IIOPI`), and unit codes appear as integers. The 3.28 MB text decodes to 26,430 lines, but no lane timer, composition, super or siege rule could be bound to a readable name.
- Creature and building codes appear as integer FourCC literals. Many decoded values are constants, not codes, so counts from the script cannot be used as unit lists.
- Lane spawn coordinates are not readable from the script without the object binding above.

## Comparison with repo data

Repo data is listed for reference. The header of `scenarios/moba/units_heroes.osd` says the hero values are generated from reference ruleset `npc_heroes` (SteamDatabase tracking repo), not from this map. That is a different game data source, so direct equality with this map is not expected.

The comparison has no bound pairs. No hero, creep, tower, barracks or item in the repo maps by name to a code in this archive, and the archive has no name strings for them. Every row is therefore MISSING, not MATCH or DIFFERENT.

| Category | Repo entries | MATCH | DIFFERENT | MISSING |
|---|---|---|---|---|
| 16 shipped heroes (units_heroes.osd) | 16 | 0 | 0 | 16 |
| Hero unit rows, all (units_heroes.osd) | 254 | 0 | 0 | 254 |
| Creeps, towers, barracks, neutrals (units.osd, units_extra.osd, neutral.osd) | 77 + 73 + 89 | 0 | 0 | 239 |
| Items (items.osd) | 426 | 0 | 0 | 426 |
| Abilities (abilities.osd, abilities_ruleset.osd) | 65 + 349 | 0 | 0 | 414 |
| Wave spawns (waves.osd) | 6 | 0 | 0 | 6 |

Repo files referenced by the request that are not in the repo: `mods/moba_match.lua`. `mods/moba_waves.lua` exists and is the wave script the spawn file names.

## Top differences

None bound. The top ten differences with both values cannot be produced because no repo entry maps to a map code. The one verified difference is the source: the repo uses reference ruleset values, and this map uses base reference game SLK and its own script.

## What could not be read

- Unit and item reference-specific stats: no refmap.w3u or w3t in the archive; base SLK only.
- Name-to-code binding for heroes, creeps, towers, barracks and items.
- refmap.w3a value fields beyond the header (layout decoded to the record start only).
- refmap.j wave timers, composition, super and siege rules, and spawn coordinates (obfuscated).
- Hash names of the 2072 unnamed members. Candidate names were not found, so their contents are unknown.

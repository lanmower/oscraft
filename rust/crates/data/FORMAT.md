# OSDT data file, format version 25

Binary image written by `osdt-data` and read by the sim crate. All integers are little-endian. There is no padding between sections or records beyond the explicit reserved bytes listed below. A reader rejects any other version: a reader of an older version must reject the file.

The layout is fixed by this file and by `src/format.rs`. The content is not: names and the meaning of most numeric fields come from the scenario data (its `.osd` files and its Lua mods). Effect kinds, stat names, filter bits, mod conditions and doodad kinds are name lists in the image (`SNAM`, `EKND`, `FNAM`, `MCND`, `DKND`), so a code is an index into a list the scenario declares, not a value this document enumerates.

## Header (16 bytes)

| offset | size | field                                              |
|--------|------|----------------------------------------------------|
| 0      | 4    | magic `OSDT` (0x4F 0x53 0x44 0x54)                  |
| 4      | 2    | version, `25`                                      |
| 6      | 2    | header length, `16`                                 |
| 8      | 4    | section count, `27`                                |
| 12     | 4    | total file length in bytes                          |

A reader rejects the file on a magic mismatch, an unknown version, a section count other than `27`, or a total length that does not equal the file size.

## Section table (16 bytes per entry, immediately after the header)

| offset | size | field                                              |
|--------|------|----------------------------------------------------|
| 0      | 4    | tag (ASCII)                                        |
| 4      | 4    | absolute byte offset of the section body           |
| 8      | 4    | record count                                       |
| 12     | 4    | record size in bytes                               |

Sections appear in this order, and the reader requires all twenty-eight tags:

| tag    | record size | count means                                        |
|--------|-------------|----------------------------------------------------|
| `STRS` | 1           | byte length                                        |
| `EFCT` | 20          | effect records                                     |
| `ABIL` | 32          | ability records                                    |
| `UABL` | 2           | ability index refs                                 |
| `UNIT` | 86          | unit records                                       |
| `BHVR` | 16          | behaviour records                                  |
| `PLCE` | 16          | placement records                                  |
| `REGN` | 12          | one per unit; count = unit count                   |
| `LIMT` | 4           | always 1                                           |
| `ECON` | 32          | always 1                                           |
| `UEXT` | 48          | one per unit; count = unit count                   |
| `ACHN` | 8           | one per ability; count = ability count             |
| `RSCH` | 32          | research records                                   |
| `PREQ` | 8           | prerequisite records                               |
| `DMGF` | 8           | damage factor records                              |
| `DODA` | 20          | doodad records                                     |
| `GRID` | 1           | byte length                                        |
| `ITEM` | 192         | item records                                       |
| `SKIL` | 72          | one per ability; count = ability count             |
| `AXTN` | 256         | one per ability; count = ability count             |
| `ROUT` | 136         | route records                                      |
| `TUNE` | 128         | always 1                                           |
| `SNAM` | 1           | byte length (stat name list)                       |
| `EKND` | 1           | byte length (effect kind name list)                |
| `FNAM` | 1           | byte length (filter name list)                     |
| `MCND` | 1           | byte length (mod condition name list)              |
| `DKND` | 1           | byte length (doodad kind name list)                |

Section bodies follow the table back to back. A section body occupies `count * record size` bytes, and the next body starts at the end of the previous one.

## Names

A name is a pair `(offset u32, length u16)` into `STRS`. Names are UTF-8 bytes matching `[a-z][a-z0-9_]*`. Records store names as this pair, never inline.

A name-list section (`SNAM`, `EKND`, `FNAM`, `MCND`, `DKND`) is a byte stream: each name is one length byte (1 to 255) then its UTF-8 bytes. Its count is the byte length. A name's index is its place in the list. A list holds at most 1024 names, and each index fits a `u16`.

## Section bodies

All offsets below are byte offsets within a record.

### STRS (record size 1)
Concatenated name bytes. No terminator.

### EFCT (20 bytes per record)
| offset | size | field                                                       |
|--------|------|-------------------------------------------------------------|
| 0      | 2    | effect kind: index into `EKND`                              |
| 2      | 2    | reserved, write 0                                           |
| 4      | 4    | param 0, i32                                                |
| 8      | 4    | param 1, i32                                                |
| 12     | 4    | param 2, i32                                                |
| 16     | 4    | param 3, i32                                                |

Effects are stored contiguously: first the effects of each ability in ability order, then one effect per research (RSCH order).

Parameters are plain `i32` integers. Fractions are Q16 (`65536` = 1.0). Radii and distances are milli-units. Durations, periods and lifetimes are game ticks and must be integers. Unused parameters are written 0 and not read. What each parameter means is defined by the effect kind's behaviour in the sim and the scenario that names it.

### ABIL (32 bytes per record)
| offset | size | field                                              |
|--------|------|----------------------------------------------------|
| 0      | 4    | name offset                                        |
| 4      | 2    | name length                                        |
| 6      | 2    | flags: bit 0 = projectile                          |
| 8      | 4    | cooldown, game ticks                               |
| 12     | 4    | energy cost                                        |
| 16     | 1    | target code: index into `TARGETS`                  |
| 17     | 1    | effect count, `0..=255`                            |
| 18     | 2    | target filter mask (bit n = `FNAM` entry n)        |
| 20     | 4    | first effect index into `EFCT`                     |
| 24     | 4    | projectile speed, milli-units per tick (0 if none) |
| 28     | 4    | initial cooldown, game ticks, `>= 0`               |

A projectile flag is set exactly when the ability declares a projectile speed. The filter mask is the set of unit filters a valid target must carry (`0` = no filter).

### ACHN (8 bytes per record, one per ability in `ABIL` order)
| offset | size | field                                                           |
|--------|------|-----------------------------------------------------------------|
| 0      | 2    | flags: bit 0 = channelled (the caster is held for the duration) |
| 2      | 2    | reserved, write 0                                               |
| 4      | 4    | channel duration, game ticks; `>= 1` when bit 0 is set, else 0 |

### UABL (2 bytes per record)
A `u16` ability index into `ABIL`. A unit's abilities are a contiguous run here.

### UNIT (86 bytes per record)
| offset | size | field                                                 |
|--------|------|-------------------------------------------------------|
| 0      | 4    | name offset                                           |
| 4      | 2    | name length                                           |
| 6      | 2    | flags: bit 0 = has weapon (weapon fields valid)       |
| 8      | 4    | health, `> 0`                                         |
| 12     | 4    | speed, milli-units per tick (integer, no float)       |
| 16     | 4    | radius, milli-units                                   |
| 20     | 4    | resource_a cost                                       |
| 24     | 4    | gas cost                                              |
| 28     | 4    | build time, game ticks                                |
| 32     | 4    | first ability index into `UABL`                       |
| 36     | 2    | ability count                                         |
| 38     | 2    | capacity cost                                         |
| 40     | 4    | filter mask (bit n = `FNAM` entry n)                  |
| 44     | 4    | shield amount, `>= 0`                                 |
| 48     | 4    | armor, i32, flat reduction per hit                    |
| 52     | 4    | weapon damage, i32 (0 if no weapon)                   |
| 56     | 4    | weapon range, milli-units (0 if no weapon)            |
| 60     | 4    | weapon period, game ticks (0 if no weapon)            |
| 64     | 4    | sight, milli-units                                    |
| 68     | 2    | product unit index into `UNIT`, `0xFFFF` = none       |
| 70     | 1    | damage response code: index into `DAMAGE_RESPONSES`   |
| 71     | 1    | resource kind code: index into `RESOURCE_KINDS`       |
| 72     | 4    | harvest amount per trip, resource units (0 if none)   |
| 76     | 4    | model name offset into `STRS`                         |
| 80     | 2    | model name length, `0` = no model                     |
| 82     | 4    | reserved, write 0                                     |

The model name is the file stem, without `.glb`, that a renderer draws the unit with. It uses lowercase letters, digits, hyphens and underscores.

### UEXT (48 bytes per record, one per unit in `UNIT` order)
| offset | size | field                                                              |
|--------|------|--------------------------------------------------------------------|
| 0      | 2    | capacity provided by this unit, `>= 0`                             |
| 2      | 2    | transport capacity in size units (`0` = not a transport)           |
| 4      | 1    | size category: index into `SIZE_CATEGORIES`                        |
| 5      | 1    | weapon damage type code: index into `DAMAGE_TYPES`                 |
| 6      | 1    | cloak flags: bit 0 = cloaked, bit 1 = can burrow, bit 2 = needs a detector to target |
| 7      | 1    | reserved, write 0                                                  |
| 8      | 4    | detector radius, milli-units (0 = not a detector)                  |
| 12     | 4    | ranged miss chance, Q16, `0..=65536`                               |
| 16     | 4    | burrow time, game ticks, `>= 0`                                    |
| 20     | 4    | rally point offset x from the unit, milli-units, i32               |
| 24     | 4    | rally point offset y from the unit, milli-units, i32               |
| 28     | 4    | bounty, resource units paid on destruction, `>= 0`                 |
| 32     | 1    | deny threshold: health percent at or below which an allied unit may deny it, `0..=100` |
| 33     | 1    | deny share percent, `0..=100`                                      |
| 34     | 1    | flags: bit 0 = player-controlled; other bits 0                     |
| 35     | 1    | reserved, write 0                                                  |
| 36     | 4    | xp bounty, `>= 0`                                                  |
| 40     | 4    | assist radius, milli-units, `>= 0`                                 |
| 44     | 4    | acquire range, milli-units (0 = none): a unit with no hold order chases a visible enemy inside it |

Transport capacity is spent in size units: the size category sets the cost (small 1, medium 2, large 4).

### BHVR (16 bytes per record)
| offset | size | field                                                   |
|--------|------|---------------------------------------------------------|
| 0      | 4    | name offset                                             |
| 4      | 2    | name length                                             |
| 6      | 1    | trigger code: index into `TRIGGERS`                     |
| 7      | 1    | action code: index into `ACTIONS`                       |
| 8      | 4    | param, i32, action specific                             |
| 12     | 2    | ability index into `ABIL`, `0xFFFF` = none              |
| 14     | 2    | reserved, write 0                                       |

Whether `param` is read and whether `ability` is required depend on the trigger and action. `use_ability` requires an ability index other than `0xFFFF`. `flee` reads a distance in milli-units. `attack_nearest` is removed; its targeting is in `scenarios/moba/targeting.lua`.

### PLCE (16 bytes per record)
One record per placed object: a unit placed for a player, a neutral unit, or a start location.

| offset | size | field                                                  |
|--------|------|--------------------------------------------------------|
| 0      | 2    | unit index into `UNIT`, `0xFFFF` for a start location  |
| 2      | 2    | player index `0..=15`, or `0xFFFF` = neutral           |
| 4      | 4    | x, milli-units, `0 <= x < width * cell_milli`          |
| 8      | 4    | y, milli-units, `0 <= y < height * cell_milli`         |
| 12     | 4    | flags: bit 0 = start, bit 1 = team, bits 8..15 = team index when bit 1 is set; other bits 0 |

A start record names a player (`0..=15`) and carries unit `0xFFFF`. Each player has at most one start. A start with bit 1 set also names its team. A neutral object uses player `0xFFFF`. Records are in declaration order. A position at the grid edge (`x` or `y` equal to the grid extent) is refused.

### PREQ (8 bytes per record)
| offset | size | field                                                           |
|--------|------|-----------------------------------------------------------------|
| 0      | 1    | target kind: `0 unit`, `1 research`                             |
| 1      | 1    | requirement kind: `0 unit`, `1 research`                        |
| 2      | 2    | target index: `UNIT` index for kind 0, `RSCH` index for kind 1  |
| 4      | 2    | requirement index: `UNIT` or `RSCH` index by requirement kind   |
| 6      | 2    | reserved, write 0                                               |

A target needs every requirement listed for it, and is refused until each is complete.

### RSCH (32 bytes per record)
| offset | size | field                                                       |
|--------|------|-------------------------------------------------------------|
| 0      | 4    | name offset                                                 |
| 4      | 2    | name length                                                 |
| 6      | 2    | building unit index into `UNIT` (the research is run there) |
| 8      | 4    | research time, game ticks                                   |
| 12     | 4    | resource_a cost                                             |
| 16     | 4    | gas cost                                                    |
| 20     | 4    | effect index into `EFCT` (the research's effect slot)       |
| 24     | 4    | applies-to filter mask (bit n = `FNAM` entry n)             |
| 28     | 4    | reserved, write 0                                           |

A building runs one research at a time. The research effect applies to every matching unit when it completes.

### REGN (12 bytes per record, one per unit in `UNIT` order)
| offset | size | field                                                        |
|--------|------|--------------------------------------------------------------|
| 0      | 4    | shield regeneration per tick, `>= 0`                         |
| 4      | 4    | delay after the last hit before regeneration starts, ticks, `>= 0` |
| 8      | 4    | health regeneration per tick, `>= 0`                         |

### LIMT (4 bytes, count 1)
| offset | size | field                                                          |
|--------|------|----------------------------------------------------------------|
| 0      | 4    | per-cell unit limit: most units that may occupy one grid cell at once; `0` = no limit |

### ECON (32 bytes, count 1)
| offset | size | field                                                              |
|--------|------|--------------------------------------------------------------------|
| 0      | 1    | workers per resource patch that add income (saturation), `>= 1`   |
| 1      | 1    | workers per extractor that add gas income (saturation), `>= 1`     |
| 2      | 1    | resource patches per base                                          |
| 3      | 1    | extractors per base                                                |
| 4      | 4    | gas per trip from one extractor, `>= 0`                            |
| 8      | 4    | opening resource_a per start, `>= 0`                               |
| 12     | 4    | opening gas per start, `>= 0`                                      |
| 16     | 2    | opening workers per start                                          |
| 18     | 2    | capacity cap at the start, `>= 0`                                  |
| 20     | 2    | capacity cap maximum (the ceiling providers can raise to)          |
| 22     | 2    | reserved, write 0                                                  |
| 24     | 4    | resource_a per resource patch, `>= 0`                              |
| 28     | 4    | gas per extractor, `>= 0`                                          |

A unit charges its capacity cost (`UNIT` offset 38). Production is refused past the capacity cap. The cap is the base plus the capacity provided by every completed provider, bounded by the maximum. Queued production reserves its capacity, released on cancel.

### DMGF (8 bytes per record)
| offset | size | field                                                        |
|--------|------|--------------------------------------------------------------|
| 0      | 1    | damage type code: index into `DAMAGE_TYPES`                  |
| 1      | 1    | size category code: index into `SIZE_CATEGORIES`             |
| 2      | 2    | reserved, write 0                                            |
| 4      | 4    | final damage factor, Q16, `>= 0` (65536 = 1.0)               |

One record per (damage type, size category) pair that differs from 1.0. A pair without a record scales by 1.0.

### DODA (20 bytes per record, count = doodad count)
Decorative objects on the map. Doodads do not block movement; walkability is in GRID.

| offset | size | field                                                                  |
|--------|------|------------------------------------------------------------------------|
| 0      | 4    | doodad id, unique per map                                              |
| 4      | 2    | kind code: index into `DKND`                                           |
| 6      | 2    | reserved, write 0                                                      |
| 8      | 4    | x, milli-units, `0 <= x < width * cell_milli`                          |
| 12     | 4    | y, milli-units, `0 <= y < height * cell_milli`                         |
| 16     | 4    | rotation, milli-turns, `0 <= rotation < 1000` (1000 = a full turn)     |

### ITEM (192 bytes per record; the item id is the record index)
| offset | size | field                                                              |
|--------|------|--------------------------------------------------------------------|
| 0      | 4    | name offset into `STRS`                                            |
| 4      | 2    | name length                                                        |
| 6      | 2    | flags: 1 consumable, 2 neutral, 4 not for sale, 8 cannot be sold, 16 enhancement |
| 8      | 4    | cost, `>= 0`                                                       |
| 12     | 2    | active or passive ability index into `ABIL`, or `0xFFFF` for none  |
| 14     | 1    | neutral tier `0..=5` (0 not a neutral drop)                        |
| 15     | 1    | sale places: bit 0 home, bit 1 side, bit 2 secret (0 sells nowhere) |
| 16     | 12   | six component item indices (u16 each), `0xFFFF` for none          |
| 28     | 2    | charges one purchase grants (0 means one item)                     |
| 30     | 2    | most charges one slot stacks (0 means not stackable)               |
| 32     | 1    | stock maximum (0 means unlimited)                                  |
| 33     | 1    | stock at the start, `<=` the maximum                               |
| 34     | 2    | seconds to restock one unit                                        |
| 36     | 2    | seconds before the first restock (0 means the restock time)        |
| 38     | 2    | reserved, write 0                                                  |
| 40     | 152  | 38 stat bonuses, i32 each, in the order of `ITEM_STATS`            |

`ITEM_STATS` (fixed in `format.rs`, index = position): `armor`, `damage`, `health`, `mana`, `strength`, `agility`, `intelligence`, `move_speed`, `attack_speed`, `health_regen`, `mana_regen`, `magic_resist`, `evasion`, `spell_amp`, `lifesteal`, `spell_lifesteal`, `cast_range`, `attack_range`, `status_resist`, `cooldown_reduction`, `gold_per_min`, `xp_per_min`, `crit_chance`, `crit_multiplier`, `cleave`, `cleave_distance`, `cleave_start_width`, `cleave_end_width`, `bash_chance`, `bash_duration`, `speed_pct`, `slow_resist`, `hp_regen_amp`, `mana_regen_amp`, `mana_cost_reduction`, `night_vision`, `scepter`, `shard`. Negative values are legal. A recipe component must be declared before the item it builds.

### SKIL (72 bytes per record, one per ability in `ABIL` order)
| offset | size | field                                                                 |
|--------|------|-----------------------------------------------------------------------|
| 0      | 1    | has skill: `0` none, `1` a skill tree                                 |
| 1      | 1    | max rank, `1..=4` when has skill, else 0                              |
| 2      | 1    | ultimate: `0` no, `1` yes; an ultimate takes at most 3 ranks         |
| 3      | 5    | reserved, write 0                                                     |
| 8      | 16   | level gate per rank, i32 x 4: an ultimate's rank `k` needs level 6, 12, 18 for `k` = 1, 2, 3; other ranks need a level `1..=30`, not below the previous rank's |
| 24     | 16   | damage per rank, i32 x 4, `>= 0`                                      |
| 40     | 16   | cooldown per rank, game ticks, i32 x 4, `>= 0`: replaces `ABIL` cooldown |
| 56     | 16   | energy cost per rank, i32 x 4, `>= 0`: replaces `ABIL` energy cost    |

Entries past the max rank are written 0. A rank of 0 means the ability is not learned and cannot be cast.

### AXTN (256 bytes per record, one per ability in `ABIL` order)
| offset | size | field                                                                 |
|--------|------|-----------------------------------------------------------------------|
| 0      | 2    | flags, bits 0..9 (mask `1023`): 1 passive, 2 aura, 4 proc on attack, 8 proc when attacked, 16 pierce spell immunity, 32 ground pulse, 64 counted proc, 128 passive stats, 256 ally only, 512 enemy only |
| 2      | 2    | cast point, ticks                                                     |
| 4      | 4    | period, ticks                                                         |
| 8      | 4    | cast range, milli-units; 0 = none                                     |
| 12     | 4    | proc chance, percent                                                  |
| 16     | 8    | scatter min and max, milli-units                                      |
| 24     | 4    | charges: maximum charges, 0 = none                                    |
| 28     | 4    | charge restore time, ticks                                            |
| 32     | 4    | break range, milli-units                                              |
| 36     | 4    | line extra, milli-units                                               |
| 40     | 120  | 6 scaled entries of 20 bytes: used u8, effect u8, param u8, pad u8, 4 x i32 values by rank |
| 160    | 96   | 8 modifier entries of 12 bytes: used u8, cond u8, effect u8, param u8, mode u8, 3 pad, delta i32 |

A modifier's `effect` values from 250 up name an ability field (cooldown, energy, cast range, cast point, charge restore). Its `cond` names a mod condition by index into `MCND`. Mode 0 adds the delta; mode 1 adds that percent of the base.

### ROUT (136 bytes per record)
| offset | size | field                                                       |
|--------|------|-------------------------------------------------------------|
| 0      | 2    | player index                                                |
| 2      | 2    | route id, `1..=3`                                           |
| 4      | 2    | point count, `0..=64`                                       |
| 6      | 2    | reserved, write 0                                           |
| 8      | 512  | 64 waypoints, each x and y as u32 milli-units               |

A spawn effect carries its route id in the two top bits of its p3.

### TUNE (128 bytes, count 1)
Tuning values the sim reads at load, set by the scenario's tuning call.

| offset | size | field                                                              |
|--------|------|--------------------------------------------------------------------|
| 0      | 1    | structure scan cells past weapon range, `0..=64`                   |
| 1      | 1    | teleport ring points, `1..=8`                                      |
| 2      | 1    | teleport ring steps, `>= 1`                                        |
| 3      | 1    | reserved, write 0                                                  |
| 4      | 8    | best-only stat mask (u64): bit k set for item stat k of `ITEM_STATS` |
| 12     | 116  | 29 cumulative level thresholds of u32, nondecreasing               |

### GRID (16-byte header, then three arrays)
| offset | size | field                                                                    |
|--------|------|--------------------------------------------------------------------------|
| 0      | 4    | width in cells, `>= 1`                                                   |
| 4      | 4    | height in cells, `>= 1`                                                  |
| 8      | 4    | cell size, milli-units                                                   |
| 12     | 4    | height step, milli-units per height level, `>= 1`                        |
| 16     | w*h  | walkability, row-major (`index = y * width + x`), one byte per cell      |
| 16+w*h | w*h  | height level, same indexing, `0..=255`                                   |
| 16+2*w*h | w*h | tile id, same indexing, `0..=255`, author-defined                       |

The body is exactly `16 + 3 * width * height` bytes, and the record size is 1. A map has exactly one grid. A cell's elevation in milli-units is `level * height step`.

Cell walkability bits: `1` blocked, `2` cliff, `4` ramp (walkable, drawn with the slope it belongs to). Bits 3 to 7 are written 0 and rejected. A cell with no bit is walkable.

## Name lists

| section | list | index means |
|---------|------|-------------|
| `SNAM`  | stat names (`stat` and `timed_stat` parameter, `ITEM` stat order is fixed separately) | a stat's code |
| `EKND`  | effect kind names | `EFCT.kind` |
| `FNAM`  | filter names, up to 15 bits | bit n of a filter mask |
| `MCND`  | mod condition names | `AXTN` modifier `cond` |
| `DKND`  | doodad kind names | `DODA.kind` |

## Fixed code lists

These lists are fixed in `format.rs`. A code is the zero-based index. Changing or reordering an entry is a format change and needs a version bump. Adding one at the end is compatible only with a bump too.

- Targets (`ABIL.target`): `none`, `unit`, `point`
- Triggers (`BHVR.trigger`): `idle`, `attacked`, `spawned`, `ability_ready`, `destroyed`
- Actions (`BHVR.action`): `flee` (0), `use_ability` (1). Any other name, including the removed `attack_nearest` and `retired_declare_winner`, fails to load.
- Status codes: `stun`, `slow`, `silence`, `root`, `invisibility`, `taunt`, `hex`, `disarm`, `spell_immunity`, `invulnerable`, `break`, `mute`, `physical_immunity`, `death_prevention`
- Spell damage types: `legacy`, `physical`, `magical`, `pure`
- Order kinds: `stop`, `attack_move`, `hold`; order targets: `caster_relative`, `point`
- Vitals: `life`, `shield`; vital modes: `drain`, `set`; vital targets: `current`, `max`
- Damage responses: `normal`, `reduced`, `immune`
- Resource kinds: `none`, `resource_a`, `gas`
- Damage types (`UEXT`, `DMGF`): `normal`, `concussive`, `explosive`
- Size categories: `small`, `medium`, `large`
- Cloak flags: `cloaked`, `burrowable`, `detector_required`
- Requirement kinds: `unit`, `research`
- Tile ids (`GRID`): `0..=255`, author-defined; the sim maps each id to terrain and art.

## Rules for readers

- Read every field at its offset. Reserved bytes are written 0 and ignored on read.
- Bounds-check every index (`EFCT`, `UABL`, `ABIL`, `UNIT`, `RSCH`, `STRS`, `EKND`, `DKND`, `FNAM`, `MCND`) against its section or list.
- Reject an index whose code is past its fixed list above, and an effect kind past the `EKND` list.
- Reject a PLCE record whose unit index is past `UNIT` (unless it is a start, which carries `0xFFFF`), whose player is neither `0..=15` nor `0xFFFF`, whose position is outside the grid extent, or whose flags have bits other than 0, 1 and the team byte set.
- Reject two start records for the same player.
- Reject a PREQ record with a kind above `1` or an index past its section.
- Reject an RSCH record whose building is past `UNIT` or whose effect is past `EFCT`.
- Reject a UEXT record with a size category or damage type past its list, cloak flags above `0b111`, or a miss chance above `65536`.
- Reject a DMGF record with a negative factor.
- Reject an ACHN record with the channel flag and a duration of `0`.
- Reject a section whose count does not match its rule: REGN and UEXT equal the UNIT count, ACHN and SKIL and AXTN equal the ABIL count, ECON, LIMT and TUNE are 1.
- Reject a GRID body whose length is not `16 + 3 * width * height`, a height step of `0`, or a cell byte with bits outside `0b111`.
- Reject a name list that holds more than 1024 names.
- Reject a DODA record whose rotation is `1000` or more, whose position is off the grid, or whose id repeats.
- The tick path never parses names. It uses only numeric fields.

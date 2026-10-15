# OSDT data file, format version 11

Binary image written by `osdt-data` and read by the sim crate. All integers are
little-endian. There is no padding between sections or records beyond the explicit
reserved bytes listed below. Version 11 adds the production, transport, channelling,
combat and cloak/burrow parameters as new sections. Every record size from version 10
is unchanged. A reader of an older version must reject the file.

## Header (16 bytes)

| offset | size | field                                              |
|--------|------|----------------------------------------------------|
| 0      | 4    | magic `OSDT` (0x4F 0x53 0x44 0x54)                  |
| 4      | 2    | version, `11`                                      |
| 6      | 2    | header length, `16`                                 |
| 8      | 4    | section count, `17`                                 |
| 12     | 4    | total file length in bytes                          |

A reader rejects the file on a magic mismatch, an unknown version, or a total
length that does not equal the file size.

## Section table (16 bytes per entry, immediately after the header)

| offset | size | field                                              |
|--------|------|----------------------------------------------------|
| 0      | 4    | tag (ASCII)                                        |
| 4      | 4    | absolute byte offset of the section body           |
| 8      | 4    | record count                                       |
| 12     | 4    | record size in bytes                               |

Sections appear in this order, and the reader requires all seventeen tags:

| tag    | record size | count means                                  |
|--------|-------------|----------------------------------------------|
| `STRS` | 1           | byte length                                  |
| `EFCT` | 20          | effect records                               |
| `ABIL` | 32          | ability records                              |
| `UABL` | 2           | ability index refs                           |
| `UNIT` | 80          | unit records                                 |
| `BHVR` | 16          | behaviour records                            |
| `PLCE` | 16          | placement records                            |
| `UPGR` | 16          | upgrade records                              |
| `REGN` | 12          | one per unit; count = unit count             |
| `LIMT` | 4           | always 1                                     |
| `ECON` | 32          | always 1                                     |
| `UEXT` | 32          | one per unit; count = unit count             |
| `ACHN` | 8           | one per ability; count = ability count       |
| `RSCH` | 32          | research records                             |
| `PREQ` | 8           | prerequisite records                         |
| `DMGF` | 8           | damage factor records                        |
| `GRID` | 1           | byte length                                  |

Section bodies follow the table back to back. A section body occupies
`count * record size` bytes, and the next body starts at the end of the previous one.

## Names

A name is a pair `(offset u32, length u16)` into `STRS`. Names are UTF-8 bytes
matching `[a-z][a-z0-9_]*`. Records store names as this pair, never inline.

## Section bodies

All offsets below are byte offsets within a record.

### STRS (record size 1)
Concatenated name bytes. No terminator.

### EFCT (20 bytes per record)
| offset | size | field                                  |
|--------|------|----------------------------------------|
| 0      | 2    | effect kind code (`0..=10`, see below) |
| 2      | 2    | reserved, write 0                      |
| 4      | 4    | param 0, i32                           |
| 8      | 4    | param 1, i32                           |
| 12     | 4    | param 2, i32                           |
| 16     | 4    | param 3, i32                           |

Effects are stored contiguously: first the effects of each ability in ability
order, then one effect per upgrade (UPGR order), then one effect per research
(RSCH order).

Parameters are plain `i32` integers. Fractions are Q16 (`65536` = 1.0). Radii and
distances are milli-units. Durations, periods and lifetimes are game ticks and
must be integers. Unused parameters are written 0 and not read.

Effect parameters by kind:

| code | kind               | p0                                                                 | p1                                     | p2                          | p3                               |
|------|--------------------|--------------------------------------------------------------------|----------------------------------------|-----------------------------|----------------------------------|
| 0    | `damage`           | amount, `>= 0`                                                     | radius, milli-units, `>= 0` (0 = target only) | –                    | –                                |
| 1    | `heal`             | amount, `>= 0`                                                     | radius, milli-units, `>= 0` (0 = target only) | –                    | –                                |
| 2    | `move`             | distance cap, milli-units, `> 0` and `<=` the grid diagonal        | –                                      | –                           | –                                |
| 3    | `spawn`            | unit index into `UNIT`, `>= 0`                                     | –                                      | –                           | –                                |
| 4    | `buff`             | buff kind code (sim-defined), `>= 0`                               | duration, ticks, `>= 0`                | –                           | –                                |
| 5    | `issue_order`      | order kind: `0 stop`, `1 attack_move`, `2 hold`                    | target: `0 caster_relative`, `1 point` | –                           | –                                |
| 6    | `remove_behaviour` | buff kind code to remove, `>= 0`                                   | radius, milli-units, `>= 0`            | –                           | –                                |
| 7    | `create_healer`    | heal amount per pulse, `>= 0`                                      | radius, milli-units, `>= 0` (0 = target only) | period, ticks, `>= 1`       | lifetime, ticks, integer, `>= 1` |
| 8    | `use_calldown`     | calldown ability index into `ABIL`, `>= 0`, `< ability count`     | –                                      | –                           | –                                |
| 9    | `vital_fraction`   | vital: `0 life`, `1 shield`                                        | fraction, Q16, `0..=65536`             | mode: `0 drain`, `1 set`    | target: `0 current`, `1 max`     |
| 10   | `shield_bonus`     | flat bonus amount, `>= 0`                                          | –                                      | –                           | –                                |

### Receiver

Every effect acts on one receiver, chosen by the cast:

- When the cast names a target (an ability whose `target` code is `1 unit` or
  `2 point`, with a target supplied by the cast), the receiver is that target.
- Otherwise the receiver is the caster. An ability with target `0 none` always
  acts on the caster.

The receiver rule applies to every effect kind. It is stated here because three
effects are defined by the receiver: `issue_order`, `shield_bonus` and
`vital_fraction` act on the receiver.

- `issue_order`: the receiver is the unit that receives the order. Its `p1`
  target (`caster_relative` or `point`) is resolved from the caster or the cast
  point, and is independent of who receives the order.
- `shield_bonus`: the receiver's shield gains the flat amount.
- `vital_fraction`: the receiver's vital (life or shield) is changed by the
  fraction of the target value, as set by mode and target below.

`vital_fraction` mode and target are explicit: `drain` subtracts
`fraction * target value` from the vital, `set` writes `fraction * target value`
to it, where the target value is the current vital (`0 current`) or its maximum
(`1 max`).

### Radius rule

`damage`, `heal`, `remove_behaviour` and `create_healer` use p1 as a radius in
milli-units. A radius of 0 means the effect applies to the receiver only. A
radius above 0 applies it to every unit within that radius of the receiver. This
rule is the same for all four kinds.

`move` uses p0 as a cap on how far the caster moves toward the cast point, in
milli-units. A cast point closer than the cap is reached exactly. A cast point
beyond the cap is clamped to the cap, placed along the same line from the caster.
The cap must be greater than 0, and it must not exceed the grid diagonal
`sqrt((width * cell_milli)^2 + (height * cell_milli)^2)`, so that every point
on the map is within reach of some legal cap. The generator refuses a cap that
breaks either rule. The check compares squares in integers, so no float is read.

## ABIL (32 bytes per record)
| offset | size | field                                              |
|--------|------|----------------------------------------------------|
| 0      | 4    | name offset                                        |
| 4      | 2    | name length                                        |
| 6      | 2    | flags: bit 0 = projectile                          |
| 8      | 4    | cooldown, game ticks                               |
| 12     | 4    | energy cost                                        |
| 16     | 1    | target code (see below)                            |
| 17     | 1    | effect count, `0..=255`                            |
| 18     | 2    | target filter mask (12 bits, `FILTERS` bit n)      |
| 20     | 4    | first effect index into `EFCT`                     |
| 24     | 4    | projectile speed, milli-units per tick (0 if none) |
| 28     | 4    | reserved, write 0                                  |

A projectile flag is set exactly when the ability declares a projectile speed.
The target filter mask is the set of unit filters a valid target must carry
(`0` = no filter). Bits 12 to 15 are written 0 and rejected by the generator.

### ACHN (8 bytes per record, one per ability in `ABIL` order)
| offset | size | field                                                        |
|--------|------|--------------------------------------------------------------|
| 0      | 2    | flags: bit 0 = channelled (the caster is held for the duration) |
| 2      | 2    | reserved, write 0                                            |
| 4      | 4    | channel duration, game ticks; `>= 1` when bit 0 is set, else 0 |

An interrupted channel ends early; the interrupt rule is the sim's, the duration
is the data.

### UABL (2 bytes per record)
A `u16` ability index into `ABIL`. A unit's abilities are a contiguous run here.

### UNIT (80 bytes per record)
| offset | size | field                                                 |
|--------|------|-------------------------------------------------------|
| 0      | 4    | name offset                                           |
| 4      | 2    | name length                                           |
| 6      | 2    | flags: bit 0 = has weapon (weapon fields valid)        |
| 8      | 4    | health, `> 0`                                         |
| 12     | 4    | speed, milli-units per tick (integer, no float)       |
| 16     | 4    | radius, milli-units                                   |
| 20     | 4    | minerals cost                                         |
| 24     | 4    | gas cost                                              |
| 28     | 4    | build time, game ticks                                |
| 32     | 4    | first ability index into `UABL`                       |
| 36     | 2    | ability count                                         |
| 38     | 2    | supply cost                                           |
| 40     | 4    | filter mask (bit per `FILTERS` entry)                 |
| 44     | 4    | shield amount, `>= 0`                                 |
| 48     | 4    | armor, i32, flat reduction per hit                    |
| 52     | 4    | weapon damage, i32 (0 if no weapon)                   |
| 56     | 4    | weapon range, milli-units (0 if no weapon)            |
| 60     | 4    | weapon period, game ticks (0 if no weapon)            |
| 64     | 4    | sight, milli-units                                    |
| 68     | 2    | product unit index into `UNIT`, `0xFFFF` = none       |
| 70     | 1    | damage response code (see below)                      |
| 71     | 1    | resource kind code (see below)                        |
| 72     | 4    | harvest amount per trip, resource units (0 if none)   |
| 76     | 4    | reserved, write 0                                     |

### UEXT (32 bytes per record, one per unit in `UNIT` order)
| offset | size | field                                                              |
|--------|------|--------------------------------------------------------------------|
| 0      | 2    | supply provided by this unit (buildings), `>= 0`                   |
| 2      | 2    | transport capacity in size units (`0` = not a transport)           |
| 4      | 1    | size category: `0 small`, `1 medium`, `2 large`                    |
| 5      | 1    | weapon damage type code (see below)                                |
| 6      | 1    | cloak flags: bit 0 = cloaked, bit 1 = can burrow, bit 2 = needs a detector to target |
| 7      | 1    | reserved, write 0                                                  |
| 8      | 4    | detector radius, milli-units (0 = not a detector)                  |
| 12     | 4    | ranged miss chance, Q16, `0..=65536` (applies to ranged attacks)  |
| 16     | 4    | burrow time, game ticks, `>= 0`                                    |
| 20     | 4    | rally point offset x from the building, milli-units, i32           |
| 24     | 4    | rally point offset y from the building, milli-units, i32           |
| 28     | 4    | reserved, write 0                                                  |

Transport capacity is spent in size units: a small unit costs 1, a medium unit 2,
a large unit 4 (the sim applies these costs; the table is part of the registry).

### DMGF (8 bytes per record)
| offset | size | field                                       |
|--------|------|---------------------------------------------|
| 0      | 1    | damage type code                            |
| 1      | 1    | size category code (`0 small`, `1 medium`, `2 large`) |
| 2      | 2    | reserved, write 0                           |
| 4      | 4    | final damage factor, Q16, `>= 0` (65536 = 1.0) |

One record per (damage type, size category) pair that differs from 1.0. A pair
without a record scales by 1.0.

### BHVR (16 bytes per record)
| offset | size | field                                    |
|--------|------|------------------------------------------|
| 0      | 4    | name offset                              |
| 4      | 2    | name length                              |
| 6      | 1    | trigger code (see below)                 |
| 7      | 1    | action code (see below)                  |
| 8      | 4    | param, i32, action specific              |
| 12     | 2    | ability index into `ABIL`, `0xFFFF` = none |
| 14     | 2    | reserved, write 0                        |

### PLCE (16 bytes per record)
One record per placed object: a unit placed for a player, a neutral unit, or a
player start location.

| offset | size | field                                                  |
|--------|------|--------------------------------------------------------|
| 0      | 2    | unit index into `UNIT`, `0xFFFF` for a start location  |
| 2      | 2    | player index `0..=15`, or `0xFFFF` = neutral           |
| 4      | 4    | x, milli-units, `0 <= x < width * cell_milli`          |
| 8      | 4    | y, milli-units, `0 <= y < height * cell_milli`         |
| 12     | 4    | flags: bit 0 = start location; other bits write 0      |

Flags:
- bit 0 (`start`): the record is a player start location. A start record carries
  unit `0xFFFF` and a player index `0..=15`. Each player has at most one start,
  and a map has at most one start per player.
- Neutral objects use player `0xFFFF`. A neutral object belongs to no player; it
  is a unit record with a unit index and no owner.

Records are in declaration order. A position at the grid edge (`x` or `y` equal
to the grid extent) is refused; positions are on the map, not past its border.

### PREQ (8 bytes per record)
| offset | size | field                                                       |
|--------|------|-------------------------------------------------------------|
| 0      | 1    | target kind: `0 unit`, `1 research`                         |
| 1      | 1    | requirement kind: `0 unit (a building)`, `1 research`        |
| 2      | 2    | target index: `UNIT` index for kind 0, `RSCH` index for kind 1 |
| 4      | 2    | requirement index: `UNIT` index for requirement kind 0, `RSCH` index for kind 1 |
| 6      | 2    | reserved, write 0                                           |

A target needs every requirement listed for it. The target is refused until each
requirement is complete.

### RSCH (32 bytes per record)
| offset | size | field                                                       |
|--------|------|-------------------------------------------------------------|
| 0      | 4    | name offset                                                 |
| 4      | 2    | name length                                                 |
| 6      | 2    | building unit index into `UNIT` (the research is run there) |
| 8      | 4    | research time, game ticks                                   |
| 12     | 4    | minerals cost                                               |
| 16     | 4    | gas cost                                                     |
| 20     | 4    | effect index into `EFCT` (the research's effect slot)       |
| 24     | 4    | applies-to filter mask (bit per `FILTERS` entry); the effect applies to every unit whose mask contains these bits |
| 28     | 4    | reserved, write 0                                           |

A building runs one research at a time. The research effect applies to every
matching unit when it completes.

### REGN (12 bytes per record, one per unit in `UNIT` order)
| offset | size | field                                                        |
|--------|------|--------------------------------------------------------------|
| 0      | 4    | shield regeneration per tick, `>= 0`                         |
| 4      | 4    | delay after the last hit before regeneration starts, ticks, `>= 0` |
| 8      | 4    | health regeneration per tick, `>= 0`                         |

### UPGR (16 bytes per record)
One record per upgrade. An upgrade applies one effect to a target unit or
ability, at a level, to new units only or to existing units too, with a stacking rule.

| offset | size | field                                                             |
|--------|------|-------------------------------------------------------------------|
| 0      | 4    | name offset (upgrade id)                                          |
| 4      | 2    | name length                                                       |
| 6      | 1    | target kind: `0 unit`, `1 ability`                                |
| 7      | 1    | level, `1..=255`                                                  |
| 8      | 1    | applies to: `0 new units only`, `1 existing units too`            |
| 9      | 1    | stacking rule: `0 replace`, `1 add`, `2 refuse if present`        |
| 10     | 2    | target index: `UNIT` index for kind 0, `ABIL` index for kind 1    |
| 12     | 4    | effect index into `EFCT` (the upgrade's effect slot)              |

### LIMT (4 bytes, count 1)
| offset | size | field                                                          |
|--------|------|----------------------------------------------------------------|
| 0      | 4    | per-cell unit limit: most units that may occupy one grid cell at once; `0` = no limit |

### ECON (32 bytes, count 1)
Production economy for the map. Every player starts from the same opening values.

| offset | size | field                                                              |
|--------|------|--------------------------------------------------------------------|
| 0      | 1    | workers per mineral patch that add income (saturation), `>= 1`    |
| 1      | 1    | workers per extractor that add gas income (saturation), `>= 1`    |
| 2      | 1    | mineral patches per base                                           |
| 3      | 1    | extractors (geysers) per base                                      |
| 4      | 4    | gas per trip from one extractor, `>= 0`                            |
| 8      | 4    | opening minerals per start, `>= 0`                                 |
| 12     | 4    | opening gas per start, `>= 0`                                      |
| 16     | 2    | opening workers per start                                          |
| 18     | 2    | supply cap at the start (base), `>= 0`                             |
| 20     | 2    | supply cap maximum per race (the ceiling providers can raise to)  |
| 22     | 2    | reserved, write 0                                                  |
| 24     | 4    | minerals per mineral patch, `>= 0`                                 |
| 28     | 4    | gas per geyser, `>= 0`                                             |

Supply: a unit charges its `supply cost` (UNIT offset 38). Production is refused
past the supply cap. The cap is `supply_base` plus the `supply provided` of every
completed provider, bounded by `supply_max`. Queued production reserves its supply,
released on cancel.

## GRID (record size 1, count = byte length)
| offset | size | field                                                    |
|--------|------|----------------------------------------------------------|
| 0      | 4    | width in cells, `>= 1`                                   |
| 4      | 4    | height in cells, `>= 1`                                  |
| 8      | 4    | cell size, milli-units                                   |
| 12     | w*h  | cells, row-major (`index = y * width + x`), one byte each |

The body is exactly `12 + width * height` bytes. A map has exactly one grid.
Each cell byte is a walkability bitfield (see below); bits 2 to 7 are written 0
and rejected by the generator.

## Registries

A code is the zero-based index into the list. Adding an entry at the end is
a compatible change only if the version is bumped; reordering is never allowed
without a version bump.

- Effect kinds (`EFCT.kind`): `0 damage`, `1 heal`, `2 move`, `3 spawn`, `4 buff`,
  `5 issue_order`, `6 remove_behaviour`, `7 create_healer`, `8 use_calldown`,
  `9 vital_fraction`, `10 shield_bonus`. Codes above `10` are rejected.
- Targets (`ABIL.target`): `0 none`, `1 unit`, `2 point`
- Triggers (`BHVR.trigger`): `0 idle`, `1 attacked`, `2 spawned`, `3 ability_ready`
- Actions (`BHVR.action`): `0 attack_nearest`, `1 flee`, `2 use_ability`
- Filters (`UNIT.filter mask`, `ABIL.target filter mask`, `RSCH` applies-to mask, bit n): `0 ground`, `1 air`,
  `2 armored`, `3 biological`, `4 mechanical`, `5 psionic`, `6 structure`, `7 light`,
  `8 massive`, `9 worker`, `10 army`, `11 building`
- Damage response (`UNIT.damage response`): `0 normal`, `1 reduced`, `2 immune`
- Resource kind (`UNIT.resource kind`): `0 none`, `1 minerals`, `2 gas`
- Damage type (`UEXT.weapon damage type`, `DMGF.damage type`): `0 normal`, `1 concussive`, `2 explosive`
- Size category (`UEXT.size category`, `DMGF.size category`): `0 small`, `1 medium`, `2 large`
- Cell walkability bits (`GRID` cell byte): `bit 0 = blocked` (value `1`),
  `bit 1 = cliff` (value `2`). A cell with neither bit is walkable. The bits are
  combinable, so the valid cell values are `0..=3`.

`use_ability` requires a behaviour ability index other than `0xFFFF`.

## Rules for readers

- Read every field at its offset. Reserved bytes are written 0 and ignored on read.
- Bounds-check every index (`EFCT`, `UABL`, `ABIL`, `UNIT`, `RSCH`, `UPGR` targets, `STRS`) against its section count.
- Reject an effect kind code above `10`, and any effect whose parameters fall outside the table above.
- Reject a `move` cap of 0 or one whose square exceeds the grid diagonal squared, computed from the GRID header.
- Reject a PLCE record whose unit index is past `UNIT` (unless it is a start, which carries `0xFFFF`), whose player is neither `0..=15` nor `0xFFFF`, whose position is outside the grid extent, or whose flags have bits other than bit 0 set.
- Reject two start records for the same player, and a start record with player `0xFFFF`.
- Reject an UPGR record with a target kind above `1`, a level of `0`, an applies-to above `1`, a stacking rule above `2`, or a target index past its section.
- Reject a PREQ record with a kind above `1` or an index past its section.
- Reject an RSCH record whose building is past `UNIT` or whose effect is past `EFCT`.
- Reject a UEXT record with a size category above `2`, a damage type above `2`, cloak flags above `0b111`, or a miss chance above `65536`.
- Reject a DMGF record with a damage type above `2`, a size above `2`, or a negative factor.
- Reject an ACHN record with a channel flag and a duration of `0`.
- Reject a section whose count does not match its rule: REGN and UEXT equal the UNIT count, ACHN equals the ABIL count, ECON and LIMT are 1.
- Reject a cell byte with bits outside `0b11` and a resource kind code above `2`.
- Apply the receiver, radius, upgrade, research and supply rules above; they are not encoded in the record and must not be inferred from it.
- The tick path never parses names. It uses only numeric fields.

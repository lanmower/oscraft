# OSDT data file, format version 20

Binary image written by `osdt-data` and read by the sim crate. All integers are
little-endian. Version 20 adds the AXTN section (ability extension), effect kinds 20 to 30 and the spell stats and statuses below; it changes the `damage` and `heal` parameters (type in p2) and lets a non-ultimate rank gate be any level from 1 to 30. There is no padding between sections or records beyond the explicit
reserved bytes listed below. Version 14 adds a per-cell tile id to GRID and the DODA section for doodads. Version 13 adds a per-level resource_a and gas cost to UPGR, which grows from 16 to 24 bytes. Version 12 changed the GRID section to carry a per-cell height. Version 11 added the production, transport, channelling,
combat and cloak/burrow parameters as new sections. Every record size from version 10
is unchanged. A reader of an older version must reject the file.

## Header (16 bytes)

| offset | size | field                                              |
|--------|------|----------------------------------------------------|
| 0      | 4    | magic `OSDT` (0x4F 0x53 0x44 0x54)                  |
| 4      | 2    | version, `15`                                      |
| 6      | 2    | header length, `16`                                 |
| 8      | 4    | section count, `18`                                 |
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
| `UPGR` | 24          | upgrade records                              |
| `REGN` | 12          | one per unit; count = unit count             |
| `LIMT` | 4           | always 1                                     |
| `ECON` | 32          | always 1                                     |
| `UEXT` | 44          | one per unit; count = unit count             |
| `ACHN` | 8           | one per ability; count = ability count       |
| `RSCH` | 32          | research records                             |
| `PREQ` | 8           | prerequisite records                         |
| `DMGF` | 8           | damage factor records                        |
| `DODA` | 20          | doodad records                               |
| `GRID` | 1           | byte length                                  |
| `ITEM` | 32          | item records                                 |
| `SKIL` | 72          | one per ability; count = ability count       |

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
| 11   | `stun`             | duration, ticks, `>= 1`                                            | –                                      | –                           | –                                |
| 12   | `slow`             | duration, ticks, `>= 1`                                            | speed multiplier, Q16, `0..=65536`     | –                           | –                                |
| 13   | `silence`          | duration, ticks, `>= 1`                                            | –                                      | –                           | –                                |
| 14   | `root`             | duration, ticks, `>= 1`                                            | –                                      | –                           | –                                |
| 15   | `invisibility`     | duration, ticks, `>= 1`                                            | –                                      | –                           | –                                |
| 16   | `stat`             | stat: `0 damage`, `1 armor`, `2 health`, `3 speed`, `4 range`      | flat amount, `>= 0` (whole units for damage, armor, health; milli-units for speed, range) | – | –            |
| 17   | `ward`             | unit index into `UNIT`, `>= 0`, `< unit count`                     | lifetime, ticks, `>= 1`                | –                           | –                                |
| 18   | `glyph`            | duration, ticks, `>= 1`                                            | team cooldown, ticks, `>= 0`           | filter bits covered, `>= 1` | filter bits excluded, `>= 0`     |
| 19   | `deliver`          | reach, milli-units, `>= 0`                                         | –                                      | –                           | –                                |
| 20   | `dot`              | damage per pulse, whole hit points, `>= 0`                         | period, ticks, `>= 1`                  | duration, ticks, `>= 1`     | damage type `0..=3` (see below)  |
| 21   | `status`           | status code `0..=13` (see Registries)                              | duration, ticks, `>= 1`                | hex only: move speed, milli-units per tick (0 = default) | – |
| 22   | `timed_stat`       | stat code (see Registries)                                         | amount, signed                         | duration, ticks, `>= 1`     | mode: `0 flat`, `1 percent of the receiver's base damage or armour` |
| 23   | `aoe`              | radius, milli-units (the half width of a line)                     | team `0..=3` plus 4 centre on caster, 8 line, 16 caster only | number of effects that follow, `0..=16` | filter bits required (12 bits), bits 16..23 a cap on targets taken nearest first, bit 30 heroes only |
| 24   | `dispel`           | mask `1..=7`: 1 timed buffs, 2 debuffs and damage over time, 4 stuns and hexes as well | – | –                  | –                                |
| 25   | `chain`            | damage, whole hit points                                           | jumps after the first hit              | jump radius, milli-units    | damage type                      |
| 26   | `zone`             | ability index whose effects pulse (`effect(ability=)`)            | period, ticks, `>= 1`                  | lifetime, ticks, `>= 1`     | delay before the first pulse, ticks; bit 30 makes the zone follow its caster |
| 27   | `illusion`         | copies `1..=8` placed around the caster                            | duration, ticks, `>= 1`                | damage dealt, percent       | damage taken, percent            |
| 28   | `mana`             | amount, `>= 0`                                                     | mode: `0 give`, `1 burn`, `2 drain to the caster` | –                | –                                |
| 29   | `if_kill`          | number of effects that follow and run only if the last damage killed its receiver, `1..=16` | – | –               | –                                |
| 30   | `attack`           | percent of the caster's weapon damage, `>= 0`                      | –                                      | –                           | –                                |

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
- `damage` with type p2 `0` is the legacy raw amount (Q16.16) that only the receiver's hero magic resistance reduces; with p2 `1`
  physical, `2` magical or `3` pure the amount is whole hit points, scaled by the caster's spell amplification and then reduced
  by the receiver's armour (physical) or magic resistance (magical, stacked multiplicatively with the hero base); pure is not
  reduced. `heal` with p2 `1` heals whole hit points, with p2 `2` p0 hundredths of a percent of the receiver's maximum.
- Spell immunity: a hostile effect of an ability without the pierce flag skips a receiver that has the `spell_immunity` status;
  invulnerable units take no damage; a unit with `death_prevention` cannot drop below 1 hit point; `physical_immunity` stops weapon
  hits and physical spell damage; `taunt` forces the target to walk to and strike the taunter; `hex` disarms, silences and slows to
  the status speed; `disarm` stops weapon attacks; `mute` stops item use; `break` is held for passives that check it.
- `aoe` runs the p2 effects after it on each unit it selects, with that unit as receiver: units within p0 of the cast point
  (or of the caster with the centre flag, or in a line from the caster through the cast point of length cast range + `line_extra`),
  filtered by team, required filter bits (structures are skipped unless the mask names them), the target cap and the heroes flag.
- `dot` ticks `damage` every `period` ticks for `duration` ticks; reapplying from the same ability refreshes it. `timed_stat`
  adds to the receiver's stat modifier for `duration` ticks and removes it after; the same effect of the same ability refreshes.
- `chain` damages the receiver, then jumps to the nearest other living enemy within p2 every `period` ticks (the ability's
  AXTN period) for p1 jumps. `zone` runs the named ability's effects at the cast point every period ticks; the pulse resolves
  its values at the rank of the ability that started it. `illusion` places copies of the caster that deal p2 percent and take
  p3 percent damage, expire after p1 ticks and pay no bounty. `attack` makes the caster strike the receiver once.
- `stat`: the receiver's stat rises by the flat amount, permanently for that unit's life. A
  passive is a `stat` ability with a very long cooldown that a `spawned` behaviour casts.
- `stun`, `slow`, `silence`, `root`, `invisibility`: status effects on the receiver for
  p0 ticks (the cast tick counts as the first). Reapplying keeps the longer duration; a
  slow keeps the lower multiplier while active. Stun: no orders, cast, attack or movement,
  and it ends a channel. Slow: speed scales by p1. Silence: no ability cast (items still
  work). Root: no movement or `move` effect. Invisibility: hidden from hostile players
  unless one of their detectors covers the unit.
- `ward`: places a unit of type p0 at the cast point for the caster's player, removed after p1
  ticks without a bounty. Sight, a detector radius and health come from the unit's own record. An
  item whose ability holds a `ward` effect loses one charge when the ward is placed; a placement
  that fails (no free unit slot, point does not fit) keeps the charge.
- `glyph`: every unit of the caster's team whose filter holds all of the p2 bits and none of the p3
  bits takes no damage for p0 ticks. The team then cannot glyph again for p1 ticks (a refused cast
  counts as a cooldown refusal).
- `deliver`: the caster walks to the receiver (a living unit friendly to the caster), moves the
  receiver's stash items into its free inventory slots once within p0 milli-units, then walks back
  to where it started.
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
| 28     | 4    | initial cooldown, game ticks, `>= 0`: ticks the ability starts on cooldown when its unit spawns; `ability_ready` fires when they run out |

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

### UNIT (86 bytes per record)
| offset | size | field                                                 |
|--------|------|-------------------------------------------------------|
| 0      | 4    | name offset                                           |
| 4      | 2    | name length                                           |
| 6      | 2    | flags: bit 0 = has weapon (weapon fields valid)        |
| 8      | 4    | health, `> 0`                                         |
| 12     | 4    | speed, milli-units per tick (integer, no float)       |
| 16     | 4    | radius, milli-units                                   |
| 20     | 4    | resource_a cost                                         |
| 24     | 4    | gas cost                                              |
| 28     | 4    | build time, game ticks                                |
| 32     | 4    | first ability index into `UABL`                       |
| 36     | 2    | ability count                                         |
| 38     | 2    | capacity cost                                           |
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
| 76     | 4    | model name offset into `STRS` (see below)             |
| 80     | 2    | model name length, `0` = no model                     |
| 82     | 4    | reserved, write 0                                     |

The model name is the file stem, without `.glb`, that a renderer draws the unit with.
It uses lowercase letters, digits, hyphens and underscores.

### UEXT (44 bytes per record, one per unit in `UNIT` order)
| offset | size | field                                                              |
|--------|------|--------------------------------------------------------------------|
| 0      | 2    | capacity provided by this unit (buildings), `>= 0`                   |
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
| 28     | 4    | kill bounty, resource units paid to the killer when destroyed by an enemy, `>= 0` |
| 32     | 1    | deny threshold: health percent at or below which an allied unit may deny it, `0..=100` |
| 33     | 1    | deny xp percent: share of its xp the enemies get when it is denied, `0..=100` |
| 34     | 1    | flags: bit 0 = player-controlled (a hero), shares assist and xp; other bits 0 |
| 35     | 1    | reserved, write 0                                                  |
| 36     | 4    | xp bounty, shared among the killer and in-range heroes, `>= 0`    |
| 40     | 4    | assist radius, milli-units, `>= 0` (heroes inside it share the xp) |

Transport capacity is spent in size units: a small unit costs 1, a medium unit 2,
a large unit 4 (the sim applies these costs; the table is part of the registry).

### ITEM (32 bytes per record, in `item()` declaration order; the item id is the record index)
| offset | size | field                                                              |
|--------|------|--------------------------------------------------------------------|
| 0      | 4    | name offset into `STRS`                                            |
| 4      | 2    | name length                                                        |
| 6      | 2    | reserved, write 0                                                  |
| 8      | 4    | cost in gold, `>= 0`                                               |
| 12     | 4    | armor bonus, `>= 0`                                                |
| 16     | 4    | damage bonus, `>= 0`                                               |
| 20     | 4    | health bonus, `>= 0`                                               |
| 24     | 2    | active ability index into `ABIL`, or `0xFFFF` for none             |
| 26     | 2    | first recipe component (ITEM index), or `0xFFFF` for none          |
| 28     | 2    | second recipe component (ITEM index), or `0xFFFF` for none         |
| 30     | 2    | reserved, write 0                                                  |

The bonuses are added to the buyer's instance stats when the item is bought. The
active ability is cast from the item's inventory slot with its own cooldown.
A recipe component must be declared before the item. The item's cost must be at least
the sum of its components' costs (the cost rule). Buying a recipe consumes its components
from the buyer's inventory, removes their bonuses, and charges the cost minus the components' cost.
Selling returns half the cost.

### SKIL (72 bytes per record, one per ability in `ABIL` order)
An ability with a skill tree learns ranks with skill points. Abilities without one
(`has skill` 0) cast at their `ABIL` cooldown and energy and need no rank.

| offset | size | field                                                                 |
|--------|------|-----------------------------------------------------------------------|
| 0      | 1    | has skill: `0` none, `1` a skill tree                                 |
| 1      | 1    | max rank, `1..=4` when has skill, else 0                              |
| 2      | 1    | ultimate: `0` no, `1` yes; an ultimate takes at most 3 ranks         |
| 3      | 5    | reserved, write 0                                                     |
| 8      | 16   | level gate per rank, i32 x 4: ultimate rank `k` needs level 6, 12, 18 for `k` = 1, 2, 3; other ranks need a level `1..=30`, not below the previous rank's (Dota: 1, 3, 5, 7) |
| 24     | 16   | damage per rank, i32 x 4, `>= 0`: the amount of the ability's first damage or heal effect |
| 40     | 16   | cooldown per rank, game ticks, i32 x 4, `>= 0`: replaces `ABIL` cooldown |
| 56     | 16   | energy cost per rank, i32 x 4, `>= 0`: replaces `ABIL` energy cost   |

Entries past the max rank are written 0. A rank is spent with a skill point, and
the rank count of one ability cannot exceed its max rank. A unit's character level
is its xp level plus one, so it has one skill point per character level, less the
ranks it has spent. A damage table is nonzero only when the ability has a damage or
heal effect. A rank of 0 means the ability is not learned and cannot be cast.

### AXTN (256 bytes per record, one per ability in `ABIL` order)
Ability extension: cast mechanics, rank-scaled effect parameters and scepter, shard and talent modifiers.

| offset | size | field                                                                 |
|--------|------|-----------------------------------------------------------------------|
| 0      | 2    | flags: 1 passive (never cast), 2 aura (pulses every `period` ticks), 4 proc on attack, 8 proc when attacked, 16 pierce spell immunity, 32 ground pulse, 64 counted proc, 128 passive stats (castable; `stat` effects apply by rank and are skipped on a cast), 256 ally only, 512 enemy only |
| 2      | 2    | cast point, ticks: the caster is held this long before the effects apply |
| 4      | 4    | period, ticks: aura and channel pulse period, chain jump delay, or the attack count of a counted proc |
| 8      | 4    | cast range, milli-units; 0 = none. A cast beyond it walks the caster into range, then casts |
| 12     | 4    | proc chance, percent |
| 16     | 8    | scatter min and max, milli-units: a channel pulse lands at a random point of this ring around the caster |
| 24     | 4    | charges: maximum charges, 0 = none |
| 28     | 4    | charge restore time, ticks |
| 32     | 4    | break range, milli-units: a channel on a unit ends when the target is farther |
| 36     | 4    | line extra, milli-units: length added to the cast range for a line `aoe` |
| 40     | 120  | 6 scaled entries of 20 bytes: used u8, effect u8, param u8, pad u8, 4 x i32 values by rank. Effect 252, 253, 254 name cast range, period and chance |
| 160    | 96   | 8 modifier entries of 12 bytes: used u8, cond u8, effect u8, param u8, mode u8, 3 pad, delta i32. cond 0 scepter, 1 shard, 2..=9 the talents; effect 250..=254 name cooldown, energy, cast range, cast point, charge restore; mode 0 adds the delta, mode 1 adds that percent of the base |

A talent is chosen by a hero at character level 10, 15, 20 or 25, one of two per tier, and sets its cond bit; a stat talent is a
modifier on a passive ability that holds a `stat` effect of amount 0.

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

Trigger fields. Each trigger states when it fires, whether `param` is read, and
whether `ability` is required. An action that reads `param` states its meaning.

| trigger         | fires when                                                   | `param`                          | `ability`                       |
|-----------------|--------------------------------------------------------------|----------------------------------|---------------------------------|
| `idle` (0)      | the unit has no order                                        | not read                         | required only by `use_ability`  |
| `attacked` (1)  | the unit takes damage                                        | not read                         | required only by `use_ability`  |
| `spawned` (2)   | once, when the unit is created                               | not read                         | required only by `use_ability`  |
| `ability_ready` (3) | the named ability's cooldown has ended and its energy is available | not read                  | required (names the ability)    |

Action `param` meanings: `attack_nearest` reads a search radius in milli-units
(`0` = the unit's sight); `flee` reads a distance in milli-units; `use_ability`
does not read `param`.

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
| 12     | 4    | resource_a cost                                               |
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

### UPGR (24 bytes per record)
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
| 16     | 4    | resource_a cost per level, `>= 0`; the cost to reach level `n` is `n` times this |
| 20     | 4    | gas cost per level, `>= 0`; the cost to reach level `n` is `n` times this      |

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
| 8      | 4    | opening resource_a per start, `>= 0`                                 |
| 12     | 4    | opening gas per start, `>= 0`                                      |
| 16     | 2    | opening workers per start                                          |
| 18     | 2    | capacity cap at the start (base), `>= 0`                             |
| 20     | 2    | capacity cap maximum per race (the ceiling providers can raise to)  |
| 22     | 2    | reserved, write 0                                                  |
| 24     | 4    | resource_a per mineral patch, `>= 0`                                 |
| 28     | 4    | gas per geyser, `>= 0`                                             |

Capacity: a unit charges its `capacity cost` (UNIT offset 38). Production is refused
past the capacity cap. The cap is `capacity_base` plus the `capacity provided` of every
completed provider, bounded by `capacity_max`. Queued production reserves its capacity,
released on cancel.

## DODA (20 bytes per record, count = doodad count)
Decorative objects on the map. Doodads do not block movement; walkability is in GRID.
| offset | size | field                                                                  |
|--------|------|------------------------------------------------------------------------|
| 0      | 4    | doodad id, unique per map                                              |
| 4      | 2    | kind code (registry below)                                             |
| 6      | 2    | reserved, write 0                                                      |
| 8      | 4    | x, milli-units, `0 <= x < width * cell_milli`                          |
| 12     | 4    | y, milli-units, `0 <= y < height * cell_milli`                         |
| 16     | 4    | rotation, milli-turns, `0 <= rotation < 1000` (1000 = a full turn)     |

## GRID (record size 1, count = byte length)
| offset | size | field                                                                    |
|--------|------|--------------------------------------------------------------------------|
| 0      | 4    | width in cells, `>= 1`                                                   |
| 4      | 4    | height in cells, `>= 1`                                                  |
| 8      | 4    | cell size, milli-units                                                   |
| 12     | 4    | height step, milli-units per height level, `>= 1`                       |
| 16     | w*h  | walkability, row-major (`index = y * width + x`), one byte each          |
| 16+w*h | w*h  | height level, same indexing, one byte each, `0..=255`                    |

The body is exactly `16 + 3 * width * height` bytes. A map has exactly one grid.
Each walkability byte is a bitfield (see below); bits 2 to 7 are written 0 and
rejected by the generator. A cell's elevation in milli-units is `level * height step`.
A third array of `width * height` tile id bytes follows the height levels, at
offset `16 + 2 * width * height`, indexed the same way. The record size stays 1;
the count is the body length.

## Registries

A code is the zero-based index into the list. Adding an entry at the end is
a compatible change only if the version is bumped; reordering is never allowed
without a version bump.

- Effect kinds (`EFCT.kind`): `0 damage`, `1 heal`, `2 move`, `3 spawn`, `4 buff`,
  `5 issue_order`, `6 remove_behaviour`, `7 create_healer`, `8 use_calldown`,
  `9 vital_fraction`, `10 shield_bonus`, `11 stun`, `12 slow`, `13 silence`, `14 root`,
  `15 invisibility`, `16 stat`, `17 ward`, `18 glyph`, `19 deliver`, `20 dot`, `21 status`, `22 timed_stat`, `23 aoe`, `24 dispel`, `25 chain`, `26 zone`, `27 illusion`, `28 mana`, `29 if_kill`, `30 attack`. Codes above `30` are rejected.
- Stats (`stat` p0, `timed_stat` p0): `0 damage`, `1 armor`, `2 health`, `3 speed` (milli-units per tick), `4 range` (milli-units),
  `5 evasion` %, `6 crit_chance` %, `7 crit_mult` % of damage, `8 cleave` %, `9 cleave_radius` (cone length, milli-units), `10 lifesteal` %,
  `11 magic_resist` %, `12 attack_speed`, `13 mana_regen` (hundredths per second), `14 health_regen` (hundredths per second), `15 max_mana`,
  `16 spell_amp` %, `17 bash_chance` %, `18 bash_ticks`, `19 speed_pct`, `20 cooldown_reduction` %, `21 status_resist` %, `22 strength`,
  `23 agility`, `24 intelligence`, `25 spell_lifesteal` %, `26 armor_x100`, `27 cast_range`, `28 cleave_start_width`, `29 cleave_end_width`, `30 damage_reduction` % of all damage taken
- Status codes (`status` p0, the sim's status counters): `0 stun`, `1 slow`, `2 silence`, `3 root`, `4 invisibility`, `5 taunt`, `6 hex`,
  `7 disarm`, `8 spell_immunity`, `9 invulnerable`, `10 break`, `11 mute`, `12 physical_immunity`, `13 death_prevention`
- Spell damage types (`damage` p2, `dot` p3, `chain` p3): `0 legacy`, `1 physical`, `2 magical`, `3 pure`
- Targets (`ABIL.target`): `0 none`, `1 unit`, `2 point`
- Triggers (`BHVR.trigger`): `0 idle`, `1 attacked`, `2 spawned`, `3 ability_ready`, `4 destroyed`
- Actions (`BHVR.action`): `0 attack_nearest` (strikes the nearest enemy in reach), `1 flee` (refused at load: the engine does not run it yet), `2 use_ability`, `3 declare_winner` (param = FILTERS index named by `tag=`; the rival side wins when the destroyed unit's side has no living unit carrying that tag)
- Filters (`UNIT.filter mask`, `ABIL.target filter mask`, `RSCH` applies-to mask, bit n): `0 ground`, `1 air`,
  `2 armored`, `3 biological`, `4 mechanical`, `5 psionic`, `6 structure`, `7 light`,
  `8 massive`, `9 worker`, `10 army`, `11 building`
- Damage response (`UNIT.damage response`): `0 normal`, `1 reduced`, `2 immune`
- Resource kind (`UNIT.resource kind`): `0 none`, `1 resource_a`, `2 gas`
- Damage type (`UEXT.weapon damage type`, `DMGF.damage type`): `0 normal`, `1 concussive`, `2 explosive`
- Size category (`UEXT.size category`, `DMGF.size category`): `0 small`, `1 medium`, `2 large`
- Doodad kinds (`DODA.kind`): `0 rock`, `1 tree`, `2 crystal`, `3 debris`, `4 base`, `5 shop`, `6 fountain`, `7 lane`, `8 camp`. Codes above `8` are rejected.
- Tile ids (`GRID` tile array): `0..=255`, author-defined; the sim maps each id to terrain and art.
- Cell walkability bits (`GRID` cell byte): `bit 0 = blocked` (value `1`),
  `bit 1 = cliff` (value `2`). A cell with neither bit is walkable. The bits are
  combinable, so the valid cell values are `0..=3`. Cell height (`GRID` height
  array): level `0..=255`, elevation `level * height step`.

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
- Reject a GRID body whose length is not `16 + 3 * width * height`, or a height step of `0`.
- Reject a DODA record with a kind above `8`, a rotation of `1000` or more, a position off the grid, or an id that repeats.
- Reject a cell byte with bits outside `0b11` and a resource kind code above `2`.
- Apply the receiver, radius, upgrade, research and capacity rules above; they are not encoded in the record and must not be inferred from it.
- The tick path never parses names. It uses only numeric fields.

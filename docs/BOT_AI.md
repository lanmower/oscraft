# Bot AI: behaviours read from the AI map

Source: `war3map.j` extracted from `DotA 6.88 AI REBORN.w3x` (the owner's copy, 8,402,417 bytes, one 4,104,779-byte script, single 16 MB sector, zlib). Extraction: a Python reader of the MPQ tables in scratch space (`.gm/scratch`, not committed). Line numbers below are 1-based in that `war3map.j`. Identifiers in the script are obfuscated, so each rule is read from its body; names such as `N9F` are the script's own labels.

Trigger key: `timer` = driven by a clock or sleep, `event` = driven by a game event or trigger, `state` = a check on unit or AI state each tick.

Our implementation is written fresh in `mods/bot_lib.lua` and uses only the mod API. Nothing from the script is copied into the repo.

## 1. Hero AI loop (`N9F`, 87046-90408)

| Rule | Source | Trigger | Our status |
| --- | --- | --- | --- |
| The hero loop runs every 0.1 s per hero. | 87062 `TriggerSleepAction(.1)` | timer | Ours: decision every 8 ticks (`DECIDE_EVERY`), lib constant. |
| Retreat when HP is below 40% and HP is below 300 + 30 x level. | 5425-5427 `ME7`, used at 87101 | state | Implemented as HP below 40% (`RETREAT_FRACTION` 0.4). The level term needs a level query the mod API does not expose, so it is not applied. |
| Engage only at level above 3, HP above 70%, and the AI flag set. | 87121 | state | Not copied. Our bot engages whenever a hostile is in reach. |
| Look for an enemy hero within 800 (`K7`), else a creep within 1100 (`Kl`), then engage. | 87122-87125 | state | Ours: nearest hostile within `ATTACK_REACH` (10 tiles). |
| Engage only when at most 2 enemies are within 750 and none is in `Q77`. | 87131 | state | Not copied. |
| Retarget within 600 only when a candidate has less HP. While the target is above 50% HP, the candidate's HP plus 20 x its level must be below the target's HP; otherwise a plain HP comparison. | 87137-87145 | state | Ours: weakest hostile in reach takes priority (last hit, HP at most 60). The level term is not applied. |
| Disengage at level 6 and above when out of 1500 range, or when own HP% + 20 is below the target's HP%. | 87158-87160 | state | Implemented as the HP% rule (`disengage_from`), without the range term. |
| Disengage below level 6 when out of 750 range, or when own HP% - 20 is below the target's HP%. | 87154-87157 | state | Same HP% rule; range term not applied. |
| A hero target with HP above 50% triggers a search within 700 for another unit. | 87147-87150 | state | Not copied. |
| A new AI order is withheld while a cooldown (`hq[832+p]`) runs, the unit is dead (HP at or below 0.405) or paused, or its current order is a stop order. | 5341-5350 `K57` | timer + state | Ours: no decision for a slot with no live hero. |
| Blink to engage only at 2000 range or more from the target (3000 for two special targets). | 6571-6577 `PB7`, blink order 6592-6593 | state | Not copied (no blink in the mod API). |
| Blink point is 500 short of the target, on the caster-to-target line. | 6583-6585 | state | Not copied. |
| Attack group: enemies within 1200 of the target (2400 when flagged) are ordered to attack the same target. | 66155-66172 `WQE`, orders 66185, 66190 | event | Not copied. Ours: each bot attacks its own target. |
| Pick an ally hero in 1100 range with the lowest HP to help, else the nearest enemy within 800. | 66195-66222 `WTE` | state | Not copied. |

## 2. Movement and lane (`J3`, 6473-6542; `PW7` 6423)

| Rule | Source | Trigger | Our status |
| --- | --- | --- | --- |
| No target: order a walk to the lane waypoint (`h9`), order `0xD0012`. | 6536-6540 | state | Ours: `walk_lane` to a per-lane point. |
| Engage the hero's target: a marked target (`XMarked`) is approached by point, a lasso target gets an attack-move, otherwise attack order `0xD000F` on the target. | 6493-6509 | state | Ours: attack by reach (`game.cmd_attack`). |
| A tethered hero walks back to its tether unit if more than 700 away. | 6523-6526 | state | Not copied. |

## 3. Items and consumables

| Rule | Source | Trigger | Our status |
| --- | --- | --- | --- |
| Consumable state (heal, mana, base) swaps the consumable in the inventory: heal if HP below 30%, mana if mana below 120, a hysteresis back when mana is above 140 / max mana x 100. | 86997-87041 `NSF` | state | Not copied (no inventory swap in the mod API). |
| Use a heal consumable when HP is below 50% with a target or the AI flag set. | 90338 | state | Not copied. Needs item identity. |
| Use a target item on a hero target at HP below 40% (not on mechanical or hero targets as a default). | 90303, 90315 | state | Not copied. |
| Use a target item at HP below 30% when flagged. | 90318 | state | Not copied. |
| Use an item when more than 2 units matching one filter, and more than 1 matching a second filter, are within 900. | 90324 | state | Not copied. |
| Use an item when missing more than 150 mana and the clock passes the item's time. | 90332 | timer + state | Not copied. |

## 4. Shop

The purchase path is not located. `PB7` (6555) is blink, not shopping. The gold and item natives in `QS8`, `WU8` and `B8E` are shop and reward code. Our shopping schedule (`SHOPPING` in `mods/bot_lib.lua`) is ours, not taken from the map. Shopping needs the script's purchase function to be read before its rules can be recorded here.

## 5. Map-level clock (not AI, recorded for the wave lane)

| Rule | Source | Trigger |
| --- | --- | --- |
| 30 s wave handler, first run at game start. | 30050-30051 `JAE` at 26200-26423 | timer |
| 450 s tech swap, 240 s guard swap, 600 s random spawn. | 30052-30063 | timer |

These are already matched by `scenarios/moba/waves.osd` (see `docs/MAP_SCRIPT_688G.md`, section 3). The AI does not own them.

## 6. Not read

- The buy rules (purchase function not located), the consumable item identities (`ZU` table), and the lane assignment function (`eY` state machine). These need a full read before they can be recorded.
- `KKE`, `G0V`, `G1V`, `PXE`, `JXE`: bodies not read.

# Bot AI: behaviours read from the AI map

Source: `refmap.j` extracted from `reference ruleset 6.88 AI REBORN.w3x` (the owner's copy, 8,402,417 bytes, one 4,104,779-byte script, single 16 MB sector, zlib). Extraction: a Python reader of the MPQ tables in scratch space (`.gm/scratch`, not committed). Line numbers below are 1-based in that `refmap.j`. Identifiers in the script are obfuscated, so each rule is read from its body; names such as `N9F` are the script's own labels.

Trigger key: `timer` = driven by a clock or sleep, `event` = driven by a game event or trigger, `state` = a check on unit or AI state each tick.

Our implementation is written fresh in `mods/bot_lib.lua` and uses only the mod API. Nothing from the script is copied into the repo.

Units: the script's ranges are in its own units, 64 to a map cell (`cells(units) = units * 15.625 / 1000`, the same conversion the blink distance check in `rust/gate/src/spell_checks.rs` uses). The sim runs 16 ticks to the second.

## 1. Hero AI loop (`N9F`, 87046-90408)

| Rule | Source | Trigger | Our status |
| --- | --- | --- | --- |
| The hero loop runs every 0.1 s per hero. | 87062 `TriggerSleepAction(.1)` | timer | Done: a decision every 2 ticks (`Bot.DECIDE_EVERY`), 0.125 s, the whole-tick count nearest 0.1 s at 16 ticks a second. |
| Retreat when HP is below 40% and HP is below 300 + 30 x level. | 5425-5427 `ME7`, used at 87101 | state | Done: `Bot.think` retreats below 40% and below 300 + 30 x the hero's level (`unit_level`, section 7); recovers at 80%. |
| Engage only at level above 3, HP above 70%, and the AI flag set. | 87121 | state | Done for enemy heroes. The AI flag is set on every bot slot, so it is not checked. Creep last hits are not gated, so a level-1 hero still last-hits . |
| Look for an enemy hero within 800 (`K7`), else a creep within 1100 (`Kl`), then engage. | 87122-87125 | state | Done: `search_target`, 12.5 cells for a hero, 17.2 cells for a creep. |
| Engage only when at most 2 enemies are within 750 and none is in `Q77`. | 87131 | state | Partly: at most 2 within 750 (11.7 cells) is done. The `Q77` test is not applied: its body is not read (section 6). |
| Retarget within 600 only when a candidate has less HP. While the target is above 50% HP, the candidate's HP plus 20 x its level must be below the target's HP; otherwise a plain HP comparison. | 87137-87145 | state | Done: `retarget`, 9.4 cells, with the level term from `unit_level` (section 7). |
| Disengage at level 6 and above when out of 1500 range, or when own HP% + 20 is below the target's HP%. | 87158-87160 | state | Done for hero targets: 23.4 cells and the HP% rule. Creep targets are not disengaged. |
| Disengage below level 6 when out of 750 range, or when own HP% - 20 is below the target's HP%. | 87154-87157 | state | Done for hero targets: 11.7 cells and the HP% rule. Creep targets are not disengaged. |
| A hero target with HP above 50% triggers a search within 700 for another unit. | 87147-87150 | state | Done: `retarget` searches 10.9 cells around a hero target above 50%. |
| A new AI order is withheld while a cooldown (`hq[832+p]`) runs, the unit is dead (HP at or below 0.405) or paused, or its current order is a stop order. | 5341-5350 `K57` | timer + state | Partly: a slot with no live hero gets no decision. The cooldown value (`hq[832+p]`) is not read, and the mod API has no paused or stop-order state, so those are not applied. |
| Blink to engage only at 2000 range or more from the target (3000 for two special targets). | 6571-6577 `PB7`, blink order 6592-6593 | state | Done for the 2000 term: 31.25 cells. The 3000 term is not applied: the two special targets are not identified in the script. The sim refuses a blink past the blink's own range, so a far point is refused. |
| Blink point is 500 short of the target, on the caster-to-target line. | 6583-6585 | state | Done: `Bot.blink_to` uses `game.cmd_use_item` on the held `blink` item with target -1 and the point 7.8 cells short of the target. |
| Attack group: enemies within 1200 of the target (2400 when flagged) are ordered to attack the same target. | 66155-66172 `WQE`, orders 66185, 66190 | event | Done for 1200: `Bot.alert` sets the target of each same-team bot whose hero is within 18.75 cells of a newly chosen target. The 2400 term is not applied: the flag is not in the API. |
| Pick an ally hero in 1100 range with the lowest HP to help, else the nearest enemy within 800. | 66195-66222 `WTE` | state | Done: `help_target` picks the lowest-HP ally within 17.2 cells and targets the nearest enemy within 12.5 cells of that ally. Gated like a hero engage. |

## 2. Movement and lane (`J3`, 6473-6542; `PW7` 6423)

| Rule | Source | Trigger | Our status |
| --- | --- | --- | --- |
| No target: order a walk to the lane waypoint (`h9`), order `0xD0012`. | 6536-6540 | state | Done: `walk_lane` to a per-lane point. |
| Engage the hero's target: a marked target (`XMarked`) is approached by point, a lasso target gets an attack-move, otherwise attack order `0xD000F` on the target. | 6493-6509 | state | Partly: attack order on the target is done (`game.cmd_attack`). The marked and lasso branches are not applied: the mod API exposes no marked or lasso state. |
| A tethered hero walks back to its tether unit if more than 700 away. | 6523-6526 | state | Not done: the mod API exposes no tether unit. |

## 3. Items and consumables

| Rule | Source | Trigger | Our status |
| --- | --- | --- | --- |
| Consumable state (heal, mana, base) swaps the consumable in the inventory: heal if HP below 30%, mana if mana below 120, a hysteresis back when mana is above 140 / max mana x 100. | 86997-87041 `NSF` | state | Not done: the API has no mana query and no inventory move (only buy and sell). |
| Use a heal consumable when HP is below 50% with a target or the AI flag set. | 90338 | state | Done: `Bot.heal` drinks `flask` (`item_flask`, which heals an ally) on the hero below 50%. The AI flag is set on every bot, so the target test always passes. |
| Use a target item on a hero target at HP below 40% (not on mechanical or hero targets as a default). | 90303, 90315 | state | Not done: the target-item identities in the `ZU` table are not read. |
| Use a target item at HP below 30% when flagged. | 90318 | state | Not done: same cause as above. |
| Use an item when more than 2 units matching one filter, and more than 1 matching a second filter, are within 900. | 90324 | state | Not done: the two filters are not read. |
| Use an item when missing more than 150 mana and the clock passes the item's time. | 90332 | timer + state | Not done: the API has no mana query, and the item's time is not read. |

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
- `Q77` (87131), `KKE`, `G0V`, `G1V`, `PXE`, `JXE`: bodies not read.
- The cooldown `hq[832+p]` (5341-5350): its length is not read.

## 7. Mod API used

- The level is not a game call: `game.unit_level` was removed. The level is `xp.level_of` (and `xp.character_level`, the level plus one) in `scenarios/moba/shared/xp.lua`, which counts the `xp_curve` thresholds the unit's xp has reached. `unit_level` in `mods/bot_lib.lua` carries a copy of that rule, because the bot chunk cannot require `shared/xp.lua`. It reads `game.xp(unit)` and `game.xp_curve()`, both read-only.
- Blink uses the existing `game.cmd_use_item(unit, slot, -1, x, y)`, the same path as the item blink in `rust/gate/src/item_checks.rs`. No new cast or sim mechanic was added.

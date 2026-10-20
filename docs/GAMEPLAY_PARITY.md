# Gameplay parity list

Parity with the gameplay list means every GP row is covered by a CAP row whose gate check passes on CI. A row marked MISSING has no covering CAP row in docs/CAPABILITIES.md (read from remote main). Coverage here is a mapping check; CAP rows marked `claimed` are not yet verified by a passing gate.

Sources (read through the fetch port; mechanics restated in our own words, no page text or tables copied, no reference numbers used):
- broodwar.live mechanics wiki, https://broodwar.live/wiki/mechanics/ (worker-management, attack-mechanics, damage-types, armor-types, economy, resources, supply, splash-damage, cloaking-detection, fog-of-war, terrain, transport, game-speed, building-placement, control-groups)
- Wikipedia: reference game: The Frozen Throne and Reign of Chaos, (URL removed)
- Wikipedia: the reference game's article (title and link omitted)
- Wikipedia: Lockstep (computing), https://en.wikipedia.org/wiki/Lockstep_(computing)
- Wikipedia: Replay (video games), https://en.wikipedia.org/wiki/Replay_(video_games)
- Not used: Liquipedia and the game's fan wiki returned bot-challenge responses (HTTP 403) to the fetch port.

Columns: id, mechanic (one sentence), covering CAP row or MISSING.

## Economy (ECO)

| id | mechanic | CAP |
|---|---|---|
| GP-ECO-1 | Workers take a load from a patch, carry it to a drop-off building, and return for the next load in a repeating cycle. | CAP-PROD-2 |
| GP-ECO-2 | Income per patch rises with added workers only up to a saturation count, after which extra workers add little or nothing. | CAP-PROD-5 |
| GP-ECO-3 | Workers whose patch is exhausted search for another nearby patch. | CAP-PROD-2 |
| GP-ECO-4 | Gas comes only from a geyser that has an extraction building on it, and that geyser has its own worker saturation count. | CAP-PROD-6 |
| GP-ECO-5 | Each unit, building, upgrade and research has a mineral and gas cost that is deducted before production starts and refused when unaffordable. | CAP-PROD-3 |
| GP-ECO-6 | Each start has a fixed opening resource amount and a fixed opening worker count. | CAP-PROD-7 |
| GP-ECO-7 | Each base has a fixed number of mineral patches and geysers defined by the map. | CAP-PROD-8 |

## Capacity and population (SUP)

| id | mechanic | CAP |
|---|---|---|
| GP-SUP-1 | Each unit consumes a fixed capacity amount, and production is refused when the owner would exceed the capacity cap. | CAP-PROD-9 |
| GP-SUP-2 | Capacity providers raise the cap, and the cap is bounded by a per-race maximum. | CAP-PROD-10 |
| GP-SUP-3 | Capacity is reserved when production is queued and released when it is cancelled. | CAP-PROD-11 |

## Production queues and rally (PROD)

| id | mechanic | CAP |
|---|---|---|
| GP-PROD-1 | A building produces one unit at a time from a queue, and each unit takes a fixed time to finish. | CAP-PROD-1 |
| GP-PROD-2 | Cancelling a queued unit returns its full cost. | CAP-PROD-4 |
| GP-PROD-3 | A newly finished unit moves to the building's rally point. | CAP-PROD-12 |

## Tech requirements and research (TECH)

| id | mechanic | CAP |
|---|---|---|
| GP-TECH-1 | Units, buildings and upgrades name prerequisite buildings or research that must exist before they can be made. | CAP-PROD-13 |
| GP-TECH-2 | Research occupies one building at a time, takes a fixed time, and costs resources. | CAP-PROD-14 |
| GP-TECH-3 | Completed research applies its effect to every matching unit at once. | CAP-PROD-15 |

## Upgrades (UPG)

| id | mechanic | CAP |
|---|---|---|
| GP-UPG-1 | Upgrades raise a stat for all units of a class in levels, with each level costing more than the one before. | CAP-BEH-6 |
| GP-UPG-2 | Upgrade levels of the same class stack for each class of unit. | CAP-BEH-7 |
| GP-UPG-3 | An upgrade applies to units already on the field, not only to units made after it finishes. | CAP-BEH-8 |

## Combat (CMB)

| id | mechanic | CAP |
|---|---|---|
| GP-CMB-1 | Each weapon has a damage type, and the target's size category scales the final damage. | CAP-CMB-5 |
| GP-CMB-2 | Armour subtracts a flat amount from each weapon hit before any size multiplier is applied. | CAP-SHLD-3 |
| GP-CMB-3 | Each attack starts a cooldown measured in simulation ticks before the unit may attack again. | CAP-CMB-1 |
| GP-CMB-4 | Each weapon has a range; a target outside it is not attacked and the unit moves into range first. | CAP-CMB-6 |
| GP-CMB-5 | A ranged attack launches a projectile that spends time travelling before it deals damage. | CAP-CMB-3 |
| GP-CMB-6 | Splash damage hits every enemy in an area around the impact point. | CAP-CMB-2 |
| GP-CMB-7 | A ranged attack can miss a target on higher ground or under cover, at a fixed chance. | CAP-CMB-7 |
| GP-CMB-8 | Cloaked and burrowed units cannot be targeted by enemies unless a detector covers them. | CAP-CMB-8 |
| GP-CMB-9 | A killed unit is removed from play, and its alive state reads as false afterwards. | CAP-UNIT-2 |

## Abilities (ABL)

| id | mechanic | CAP |
|---|---|---|
| GP-ABL-1 | Casting spends energy from a regenerating pool, and a cast is refused when energy is short. | CAP-ABIL-1 |
| GP-ABL-2 | An ability has a cooldown that must expire before it can be cast again. | CAP-ABIL-1 |
| GP-ABL-3 | A channelled ability holds the caster for its duration and ends early when interrupted. | CAP-ABIL-5 |
| GP-ABL-4 | Casting range is checked against the target, and the caster moves into range first when the ability allows it. | CAP-ABIL-2 |
| GP-ABL-5 | Target filters decide which units an ability may affect, including by team relation. | CAP-ABIL-2 |

## Buffs and debuffs (BUF)

| id | mechanic | CAP |
|---|---|---|
| GP-BUF-1 | A timed buff applies to a unit and expires after its duration. | CAP-BEH-1 |
| GP-BUF-2 | A buff can be cleared before it expires. | CAP-BEH-2 |
| GP-BUF-3 | Buffs of the same kind follow a stated stacking rule rather than adding without limit. | CAP-BEH-9 |
| GP-BUF-4 | A behaviour declared with a trigger fires on idle, attacked, spawned or ability-ready events. | CAP-BEH-4 |

## Health and shield regeneration (REG)

| id | mechanic | CAP |
|---|---|---|
| GP-REG-1 | Shields regenerate after a delay in which the unit takes no damage. | CAP-SHLD-5 |
| GP-REG-2 | Hit points of some unit classes regenerate over time. | CAP-SHLD-6 |
| GP-REG-3 | A shield bonus effect increases the unit's maximum shield. | CAP-SHLD-2 |

## Pathing and collision (PATH)

| id | mechanic | CAP |
|---|---|---|
| GP-PATH-1 | Units route around unwalkable cells toward a goal. | CAP-ORD-4 |
| GP-PATH-2 | Units that collide push each other apart rather than overlapping. | CAP-ORD-7 |
| GP-PATH-3 | Narrow passages and blocked cells limit how many units can pass at once. | CAP-ORD-8 |
| GP-PATH-4 | Units can be stopped, set to attack-move, set to patrol, or set to hold position. | CAP-ORD-5 |
| GP-PATH-5 | A cell can be blocked or unblocked at runtime, which invalidates routes through it. | CAP-MAP-3 |

## Air versus ground (AIR)

| id | mechanic | CAP |
|---|---|---|
| GP-AIR-1 | Air units cross terrain that ground units cannot cross. | CAP-UNIT-5 |
| GP-AIR-2 | Ground-only weapons cannot target air units. | CAP-UNIT-6 |
| GP-AIR-3 | Air units are classified separately from ground units for rules and filters. | CAP-UNIT-4 |

## Transport loading (TRN)

| id | mechanic | CAP |
|---|---|---|
| GP-TRN-1 | A transport holds a limited number of ground units by size and can load and unload them. | CAP-UNIT-7 |
| GP-TRN-2 | Loaded units can be unloaded at a chosen point, including behind enemy lines. | CAP-UNIT-8 |

## Vision and fog of war (VIS)

| id | mechanic | CAP |
|---|---|---|
| GP-VIS-1 | Each unit reveals the cells within its sight radius, and cells outside all friendly sight stay hidden. | CAP-MAP-7 |
| GP-VIS-2 | Elevated cells give sight over lower ground and deny sight from below. | CAP-MAP-8 |
| GP-VIS-3 | An enemy unit seen once is remembered at its last known position until it is seen again. | CAP-MAP-9 |

## Map rules (MAP)

| id | mechanic | CAP |
|---|---|---|
| GP-MAP-1 | Each map defines one start position per player. | CAP-MAP-5 |
| GP-MAP-2 | Neutral objects such as mineral patches, geysers and critters are placed by the map and belong to no player. | CAP-MAP-6 |
| GP-MAP-3 | Units are kept inside map bounds. | CAP-MAP-4 |
| GP-MAP-4 | Walkability of each cell is loaded from map data. | CAP-MAP-1 |
| GP-MAP-5 | A map data file carries a format version and is rejected when the version does not match. | CAP-PERS-1 |

---

# Section A: mod capabilities (MOD)

What a mod of the reference-game style can do, in our terms.

| id | mechanic | CAP |
|---|---|---|
| GP-MOD-1 | A mod defines new unit kinds with per-kind stats in data. | CAP-UNIT-3 |
| GP-MOD-2 | A mod spawns a unit of a catalog kind for a player at chosen coordinates. | CAP-UNIT-1 |
| GP-MOD-3 | A mod defines an ability as a list of effects with its own cooldown, energy and range. | CAP-ABIL-3 |
| GP-MOD-4 | A mod declares a trigger that fires when a unit enters or leaves a rectangular region. | CAP-EVT-2 |
| GP-MOD-5 | A mod receives game events for units that are spawned, attacked or died. | CAP-EVT-3 |
| GP-MOD-6 | A mod runs a script that binds a per-tick handler and keeps state across ticks. | CAP-LUA-1 |
| GP-MOD-7 | A mod reloads when its script file changes on disk. | CAP-LUA-2 |
| GP-MOD-8 | A script error reports the file and line where it occurred. | CAP-LUA-4 |
| GP-MOD-9 | A mod calls host functions to order units or players, read unit state, and set buffs. | CAP-LUA-6 |
| GP-MOD-10 | A mod script runs under instruction and memory limits. | CAP-LUA-7 |
| GP-MOD-11 | A mod schedules named delayed actions and cancels them. | CAP-TIME-3 |
| GP-MOD-12 | A mod edits terrain cells at runtime. | CAP-MAP-3 |
| GP-MOD-13 | A mod defines rectangular regions and tests units against them. | CAP-EVT-1 |
| GP-MOD-14 | A mod authors custom abilities in a graphical editor. | CAP-ABIL-6 |
| GP-MOD-15 | A mod authors custom triggers and events in a graphical editor. | CAP-EVT-4 |
| GP-MOD-16 | A mod authors custom terrain tiles and doodads in an editor. | CAP-MAP-10 |
| GP-MOD-17 | A mod shows custom UI panels and dialogs. Headless servers refuse this; the refusal name is pending in code. | CAP-PRES-3 |
| GP-MOD-18 | A mod plays custom sounds. Headless servers refuse this; the refusal name is pending in code. | CAP-PRES-2 |
| GP-MOD-19 | A mod drives the viewer's camera. Headless servers refuse this; the refusal name is pending in code. | CAP-PRES-1 |
| GP-MOD-20 | A mod posts custom chat messages. Headless servers refuse this; the refusal name is pending in code. | CAP-PRES-4 |
| GP-MOD-21 | A mod's persistent state survives a save and load of the match. | CAP-PERS-3 |
| GP-MOD-22 | A map author writes terrain and placement into a versioned map data image. | CAP-PERS-2 |
| GP-MOD-23 | A map author places neutral objects and start positions through the map data. | CAP-MAP-11 |
| GP-MOD-24 | A mod's filters distinguish allied, team and hostile units. | CAP-PLR-3 |

---

# Section B: general game-server additions (SRV)

What a general game server needs beyond the gameplay list.

| id | mechanic | CAP |
|---|---|---|
| GP-REP-1 | Every order accepted during a match is recorded with its tick, so the match can be replayed. | CAP-DET-4 |
| GP-REP-2 | Replaying a recording reproduces the same state hashes as the original match. | CAP-DET-3 |
| GP-REP-3 | A replay can be played at a chosen speed, paused, and seeked. | CAP-DET-5 |
| GP-SPEC-1 | A spectator receives a read-only snapshot of the match state and cannot issue orders. | CAP-SRV-1 |
| GP-SPEC-2 | A spectator receives only the changes since its last tick instead of the full state. | CAP-SRV-4 |
| GP-PERS-1 | A live match can be saved and loaded with its state. | CAP-PERS-3 |
| GP-PERS-2 | A loaded save continues with the same state hashes as uninterrupted play. | CAP-PERS-4 |
| GP-PERS-3 | Map data is written to a versioned image and read back unchanged. | CAP-PERS-2 |
| GP-DET-1 | The simulation uses integer fixed-point math, so results do not depend on the platform's float behaviour. | CAP-DET-1 |
| GP-DET-2 | Clients advance in lockstep by exchanging inputs per tick and comparing state hashes. | CAP-DET-6 |
| GP-DET-3 | A state hash covers simulation state so divergence between runs is detected. | CAP-DET-2 |
| GP-DET-4 | The same mod and the same state reproduce the same hashes in every run. | CAP-DET-3 |
| GP-PKG-1 | A mod ships as a versioned package that the host checks before loading. | CAP-PERS-5 |
| GP-PKG-2 | A map image is refused when its format version does not match the reader. | CAP-PERS-1 |
| GP-HOST-1 | One process hosts several rooms, each with its own state. | CAP-SRV-5 |
| GP-HOST-2 | Each room advances at a fixed tick rate on its own ticker. | CAP-TIME-1 |
| GP-HOST-3 | Process and room counters are exposed for monitoring. | CAP-SRV-6 |
| GP-ORD-1 | Remote clients submit orders over HTTP with a player index and coordinates. | CAP-SRV-3 |
| GP-ORD-2 | Order intake is rate-limited per player. | CAP-ORD-6 |
| GP-ORD-3 | Order intake checks authorization before an order is applied. | CAP-ORD-6 |
| GP-ORD-4 | A malformed order or out-of-bounds coordinate is rejected before it reaches the simulation. | CAP-ORD-1 |

## Control citations

Each row names its source. A row is confirmed only when a 6.88 page or patch note names the control; none does. Fandom pages return HTTP 402/403 to direct fetch, and the 6.88 searches (Liquipedia Version 6.88 (URL removed) and GosuGamers (URL removed) cover hero and item balance only.

| Control | reference ruleset behaviour | Source | 6.88 status |
| --- | --- | --- | --- |
| Move command | Right click moves selected units | (URL removed) (search snippet) | not confirmed: page not fetchable; no 6.88 control note found |
| Attack move (A) | Default key A; attacks enemies met on the way | (URL removed) (search snippet) | not confirmed: page not fetchable; no 6.88 control note found |
| Hold position (H) | Holds position; H in the legacy preset column | (URL removed) (search snippet) | not confirmed: fetch returned 402; no 6.88 control note found |
| Targeted abilities and cast cursor | Targeted cast arms a cursor, cast on click | (URL removed) (search snippet) | not confirmed: page not fetchable; no 6.88 control note found |
| Ability cooldown | Ability on cooldown after a cast | (URL removed) | not confirmed: fetch returned 402; no 6.88 note found |
| Item use and slots | Six active inventory slots; backpack items inactive | (URL removed) (search snippet) | not confirmed: page not fetchable; slot counts disagree between sources |
| Consumable charges | Charges spent per use; permanent items keep the item | (URL removed) (search snippet) | not confirmed: page not fetchable; no 6.88 note found |
| Refusal "Not enough mana" | Spoken voice line, not on-screen text | (URL removed) | not confirmed: fetch returned 403; on-screen text not found |
| Refusal "Ability is on cooldown" | Spoken voice line, not on-screen text | (URL removed) | not confirmed: fetch returned 403; on-screen text not found |
| Refusal "Target out of range" | No source found | none | not confirmed: no source; the sim's wording is move-range for item casts |

## Wave creep acquire (decision)

Wave creeps now chase an enemy inside their acquire range and then resume the route (`rust/crates/sim/src/acquire.rs`, commit b92f8a9). Acquire ranges come from the 6.88 creep unit data: creep Ghoul 500, creep_ranged Necromancer 600, creep_siege Meat Wagon 800. Structures are excluded as creep acquire targets; the 6.88 data does not say this, so it is a judgement call made to keep matches from stalling.

Measured shift on the 3600-tick match (b92f8a9 against its parent): non-hero kills side 0 to 1 went from 43 to 36, side 1 to 0 from 72 to 70, and the win tick from 28655 to 18953. The shift is kept as the intended behaviour. Revisit it only with a new 6.88 source for creep targeting.

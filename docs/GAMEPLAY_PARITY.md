# Gameplay parity list

Parity with the gameplay list means every GP row is covered by a CAP row whose gate check passes when `oscraft-gate` runs. The `oscraft-gate` job of `.github/workflows/tests.yml` (workflow_dispatch) runs it. A row marked MISSING has no covering CAP row in docs/CAPABILITIES.md (read from remote main). Coverage here is a mapping check; CAP rows marked `claimed` are not yet verified by a passing gate, and CAP rows whose gate reads `none (gate removed)` have no gate at all.

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

## In-game chat commands (CHT)

Script: scenarios/moba/chat_cmds.lua (dispatch), chat_lobby.lua (pick flow), chat_core.lua, chat_text.lua (6.88g strings). Teams are the lobby's `teams` list from moba_ui.json; seat ids in messages are the 6.88g ids 1..5 and 7..11.

| id | command | behaviour | gate check |
|---|---|---|---|
| GP-CHT-1 | `-names [0-2]` | ZDE: no argument cycles default, numbers, hero names; 0, 1, 2 select directly; the confirmation is TX1K, TX1L or TX1M. The mode is the caller's `names` setting; the page's per-viewer rename is UNVERIFIED (the host name is global), chat_core.display gives the per-viewer name for lines the pack writes. | chat-names |
| GP-CHT-2 | `-gameinfo` | V0X: one block per active mode with its real label and text; `Normal Mode` when no mode is set (the La flag is read as no mode: UNVERIFIED). Death Match's hero count (Lt) and Captains/League picks are unreachable because the pack supports neither. | chat-gameinfo |
| GP-CHT-3 | `-matchup, -ma` | EHX: the opposing team's seats that hold a hero, `<name> controls <hero> (Level N)` and `(Left at M:SS)` for a seat that left (OIX). | chat-matchup |
| GP-CHT-4 | `-creepstats, -cs, -csa` | EPX, Y7E: `Creeps Killed`, `Creeps Denied`, `Neutrals Killed` of the caller's hero, or of every seat of the caller's team. Whether Br counts neutral kills is UNVERIFIED: the pack's st_creep counts them. | chat-creepstats |
| GP-CHT-5 | `-cson, -csoff, -showdeny, -hidedeny` | EQX, ESX: silent; the caller's `creepstats` and `showdeny` settings (both default on, main 101983). -cson and -csoff set both, -showdeny and -hidedeny only showdeny. | chat-creepstats, chat-deny-flags |
| GP-CHT-6 | `-denyinfo, -di` | ETX: n0HG through OQV (centred, 2 s). | chat-deny-flags |
| GP-CHT-7 | `-weather snow|rain|off|random|wind|moonlight` | V3X: the caller's `weather` setting (off 0, snow 1, rain 2, moonlight 3, wind 4, random 5); random re-rolls `weather_kind` 1..4 every 300 s for every random seat (OHX). The map special string SPCL is assumed `normal`. The page painting weather is UNVERIFIED. | chat-weather |
| GP-CHT-8 | `-music <theme>` | EOX: the caller's `music` setting, 0 off, 1..22 in the xK table order; random draws 1..22; an unknown theme prints n0CW and the theme list. The spam gate in EOX saves the current time, so it never blocks (kept as is). | chat-music |
| GP-CHT-9 | `-courier` | EYX: enumerates the caller's couriers and clears the group; it has no observable effect in 6.88g and none here. | chat-courier |
| GP-CHT-10 | `-share, -unshare, -rmshare` | E9X, XVX, XEX: team seat 1..5; the grant is `units.shared[owner][grantee]`, which units.own reads, so the grantee selects and commands the owner's units; the caller's `share<raw id>` setting mirrors it. Commands from a grantee are accepted by the pack's selection and order path; whether the sim refuses a foreign owner is UNVERIFIED beyond the pack. | chat-share, chat-shared-control |
| GP-CHT-11 | `-drop N, -yes` | E8X, OQX, OTX, OSX: only seat 1 starts a vote; seats 1..10; four more yes votes within 60 s remove the target (the pack's leave path), else `Kick time has expired`. | chat-drop-vote |
| GP-CHT-12 | `-kickafk [id]` | EWX: lists seats idle more than 300 s once each, then `-kickafk <raw id>` boots an allied listed seat. Idle time is the last page action of the seat. | chat-kickafk |
| GP-CHT-13 | `-ms, -movespeed` | EJX: every hero of the caller, then the Lone Druid's bear. The bear unit kind does not exist in the pack yet (UNVERIFIED), so only heroes print. | chat-movespeed |
| GP-CHT-14 | `-random [int|agi|str]` | ECX, XQX: lobby only; refused with n079 once a hero is held, after a repick, in all-random and same-hero modes or with no free hero; cost 50 (100 with an attribute) is taken from the seat's gold on the first tick of the match (UNVERIFIED: the clamp at zero). | chat-lobby-random, chat-lobby-random-cost, chat-match-repick |
| GP-CHT-15 | `-repick` | EDX, XSX: lobby: clears the seat's hero and ready flag and charges 150 at start (ni, main 0x96); in a match: `Too late to repick` (the window CIX opens is UNVERIFIED). | chat-lobby-repick, chat-match-repick |
| GP-CHT-16 | `-rollhero, -rh` | EIX: `You randomly rolled <hero>`, one per second. | chat-rollhero |
| GP-CHT-17 | `-swap, -swaphero, -swap N, -swapall, -swapcancel` | OEX, X9X, X8X, X7X, XZX: lobby: both allies request each other and the heroes exchange; a match answers `Too late to use -swaphero.` | chat-lobby-swap, chat-match-swap |
| GP-CHT-18 | `-switch, -switch N, -ok, -no` | E0X, RKX, RMX, RJX, RHX, RFX: needs the switch-on mode (`so`, now supported); a 60 s vote, accepted when fewer than two seats wait and half of the seats agreed; the two seats exchange sides (lobby slots, or game.set_team in a match: UNVERIFIED). | chat-switch-flow |
| GP-CHT-19 | `-hold` | ZEE: toggles the caller's Double Hold (`doublehold` setting, default off); the confirmation is TX0H or TX0I. | ctl-double-hold |

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
| GP-MOD-12 | A mod edits terrain cells at runtime. | MISSING |
| GP-MOD-13 | A mod defines rectangular regions and tests units against them. | CAP-EVT-1 |
| GP-MOD-14 | A mod authors custom abilities in a graphical editor. | MISSING |
| GP-MOD-15 | A mod authors custom triggers and events in a graphical editor. | MISSING |
| GP-MOD-16 | A mod authors custom terrain tiles and props in an editor. | MISSING |
| GP-MOD-17 | A mod shows custom UI panels and dialogs. The headless sim names a refusal for this. | CAP-PRES-3 |
| GP-MOD-18 | A mod plays custom sounds. The headless sim names a refusal for this. | CAP-PRES-2 |
| GP-MOD-19 | A mod drives the viewer's camera. The headless sim names a refusal for this. | CAP-PRES-1 |
| GP-MOD-20 | A mod posts custom chat messages. The headless sim names a refusal for this. | CAP-PRES-4 |
| GP-MOD-21 | A mod's persistent state survives a save and load of the match. | CAP-PERS-3 |
| GP-MOD-22 | A map author writes terrain and placement into a versioned map data image. | CAP-PERS-2 |
| GP-MOD-23 | A map author places neutral objects and start positions through the map data. | CAP-MAP-11 |
| GP-MOD-24 | A mod's filters distinguish allied, team and hostile units. | CAP-PLR-3 |

---

# Section B: general engine additions

What the engine needs beyond the gameplay list: replay, persistence, determinism and packaging.

| id | mechanic | CAP |
|---|---|---|
| GP-REP-1 | Every order accepted during a match is recorded with its tick, so the match can be replayed. | CAP-DET-4 |
| GP-REP-2 | Replaying a recording reproduces the same state hashes as the original match. | CAP-DET-3 |
| GP-REP-3 | A replay can be played at a chosen speed, paused, and seeked. | CAP-DET-5 |
| GP-PERS-1 | A live match can be saved and loaded with its state. | CAP-PERS-3 |
| GP-PERS-2 | A loaded save continues with the same state hashes as uninterrupted play. | CAP-PERS-4 |
| GP-PERS-3 | Map data is written to a versioned image and read back unchanged. | CAP-PERS-2 |
| GP-DET-1 | The simulation uses integer fixed-point math, so results do not depend on the platform's float behaviour. | CAP-DET-1 |
| GP-DET-2 | Clients advance in lockstep by exchanging inputs per tick and comparing state hashes. | CAP-DET-6 |
| GP-DET-3 | A state hash covers simulation state so divergence between runs is detected. | CAP-DET-2 |
| GP-DET-4 | The same mod and the same state reproduce the same hashes in every run. | CAP-DET-3 |
| GP-PKG-1 | A mod ships as a versioned package that the host checks before loading. | CAP-PERS-5 |
| GP-PKG-2 | A map image is refused when its format version does not match the reader. | CAP-PERS-1 |
| GP-HOST-2 | Each room advances at a fixed tick rate on its own ticker. | CAP-TIME-1 |

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

Wave creeps now chase an enemy inside their acquire range and then resume the route. The acquire scan, the chase and the return to the saved order are pack rules in `scenarios/moba/lanes.lua` (wave_creep_acquire; game.acquire_of, game.order_of, game.issue_order, game.chase_get, game.chase_set). The sim keeps only the acquire data and the chase storage, in `rust/crates/sim/src/acquire.rs`. Acquire ranges come from the 6.88 creep unit data: creep Ghoul 500, creep_ranged Necromancer 600, creep_siege Meat Wagon 800. Structures are excluded as creep acquire targets; the 6.88 data does not say this, so it is a judgement call made to keep matches from stalling.

Measured shift on the 3600-tick match (b92f8a9 against its parent): non-hero kills side 0 to 1 went from 43 to 36, side 1 to 0 from 72 to 70, and the win tick from 28655 to 18953. The shift is kept as the intended behaviour. Revisit it only with a new 6.88 source for creep targeting.

## Score board and observer info (6.88g multiboards)

`scenarios/moba/scoreboard.lua` draws the 6.88g score board as a `game.panel` matrix (id `scoreboard_688g`, the viewing seat only, sent when the board is open and changed). Title `DotA Scores  - <mode codes> - <sentinel kills>/<scourge kills>` (DVv 27762-27767, MVE 28750; kills are summed hero kills per side). Rows: level and hero, Bonus Gold (assist gold, D9V 14252), Bonus XP (the per-recipient bonus term, D9V 14251), Time Dead (respawn seconds summed, F_V 14625, `m:ss`), Consumables (clarity 1, healing salve 1, sentry and observer wards 1, tango 4, dust 2; LAV 16509), MultiKills (double/triple/ultra/rampage with the script's decrement chain, F_V 14582-14602) and the Kill Details grid (kills/deaths of each hero against each opponent, F_V 14579-14580, L6E 28617). The counters live on the hero (`st_bonus_gold`, `st_bonus_xp`, `st_time_dead`, `st_consumable`, `st_multi2..5`, `st_pk<seat>`, `st_pd<seat>`), so they replay and save. Not ported: the respawn-timer title prefix, hero icons (acronym text instead), and the rows Wards, Hero/Towers damage, XPM.

`scenarios/moba/observer.lua` is the `-oi` (Observer Info) counterpart, on unless mode `oi` is set: per side and per hero `Name (K/D | creeps-denies)` (DTX 42573) and each side's net worth (DUX 42585; the script draws it as map text tags). The panel goes to seats 10..15 and to the spectator seat 255 (`game.panel`, `game.message` and `game.sound` accept -1, 0..15 and 255; the page filters events by its seat and a peer without a seat takes 255). Seat decision (6.88g): the script's referee players are `qd` = Player(0) and `Qd` = Player(6) (L3 8740-8741, `IsPlayerObserver` main 101450-101453, the camera and text-tag observer views D2X 42635, D5X 42672, DUX 42632) plus Players 12..16 (main 101456-101460); the 10 hero seats are Players 1..5 and 7..11, so Players 0 and 6 and 12..16 are the non-hero seats, which the pack numbers 10..15, and a spectator without a seat is 255 (GUESS: the pack has no seat for a viewer who joined past the roster).

## Seat colours

`scenarios/moba/seat_colours.lua` sets `game.player_name` (`Player N`) and `game.player_colour` for every seated seat on the first tick, from the 6.88g `Tr` table (f3 8514-8525 text colours, f3 8526-8537 `SetPlayerColor` with PLAYER_COLOR_RED, BLUE, CYAN, PURPLE, YELLOW, ORANGE, GREEN, PINK, LIGHT_GRAY, LIGHT_BLUE, AQUA, BROWN for Players 0..5 and 6..11): the hero seats are Players 1..5 (blue 0042ff, teal 1ce6b9, purple 540081, yellow fffc01, orange ff8000) and 7..11 (pink e55bb0, gray 959697, light blue 7ebff1, dark green 106246, brown 4e2a04), seat p even is Player 1+p/2 and odd is 7+(p-1)/2; Players 0 (red ff0303) and 6 (green 20c000) are the referee seats (6.88g). Orange is `ff8000` in the script's text table while the engine constant PLAYER_COLOR_ORANGE is fe8a0e (web, from the common palette, not fetched), and the table value is used. The page reads the `seat` UI event `{seat, name, colour}` and tints that seat's units and effects with it, falling back to its two team colours.

## Game modes: Easy, Mini Heroes, Item Drop

`scenarios/moba/mode_rules.lua` reads the lobby flags `em`, `mi` and `id` (modes.lua). Easy Mode (war3map.j NXX 37421, passive gold CXV 11688): every seated player's xp award is multiplied by 1.5 (`game.set_xp_rate`, SetPlayerHandicapXP 1.5) and the passive gold grows from 1 to 2 per 0.6 s tick (kill_gold.lua doubles the period amount). The script also researches upgrade R004 for the two base players; the effect names of R004 (ratx -30, rhpx -300 on the Ancient Protector and Spirit Tower units) are UNVERIFIED and not ported. Mini Heroes (A5X 42658, D4X, y4 10641): each hero is drawn at 50 percent (`game.set_unit_scale`); 6.88g multiplies by a per-hero scale from its own table (Qu 3348), extracted from the P3 and Pu registrations in U3 (8960-9244) into `scenarios/moba/hero_scale_688g.lua` by `tools/gen-hero-scale-688g.mjs` (6.88g); a hero without an entry keeps 1, so the drawn scale is Qu x 0.5 (bane 150 percent x 50 = 75, juggernaut 50). Item Drop (ASX 42511, F_V 14370): a dying hero loses the item in one random slot of six to the ground at its death point; the script's unit H00J (6.88g unit.ini name Geomancer, propernames Meepo, created by SPO 68488 as the Divided We Stand clone; the original Meepo is H00I, P3 id 89) is the only exempt unit, so the pack exempts the clone kind `meepo_clone` (none exists yet) and the original `meepo` drops (6.88g). The item leaves by `UnitRemoveItemFromSlot`, which puts it at the unit's death point (GUESS: engine behaviour, not witnessed). The script's swap of item I00K for I00G is unreachable here: I00K is the Double Damage rune (item.ini, droppable 0, consumed on pickup by QKV 18653, never held in a slot) and I00G is the Lina copy of the Aghanim's Scepter (items_688g_na.tsv), so no replacement is ported (6.88g). Super Creeps (AUX 42518, KQE 26847) needs the Siege Golem, Scary Fish and Ancient Hydra unit kinds, which the pack lacks, and stays refused.

## Courier, glyph, hold and camera keys (CTL)

Scripts: scenarios/moba/courier.lua (commands), controls.lua (key map), glyph.lua, announce_sounds.lua; page: js/client/zoom_keys.js (node check js/client/zoom-check.mjs). Keys are `moba_ui.json` `keys.courier`, `keys.glyph`, `camera.zoomKeys`, `camera.rotationLockKeys`.

| id | behaviour | gate check |
|---|---|---|
| GP-CTL-1 | Courier Find Secret Shop (A2KS, ERE 23631): the courier walks to the secret depot on its team's side. | ctl-courier-secret-shop |
| GP-CTL-2 | Courier Drop Items (A138, LHV 16581): every slot except the filled bottles (N0 6385: the six Magical Bottle states; the empty bottle drops) and the Aegis (qg, I0AW); T0 7023 lists items QWV 18774 froze as undroppable for a holder with ability A0A5, a hero helper or a departed player, which the pack has no state for. A courier inside its team's base rect (yn -7936..-5920 x -7776..-5760, Yn 5504..7744 x 5184..7136) sends an allied owner's item to that owner's hero start (`pd`/`Pd`, F3 8642-8662; the pack's `starts-688g.tsv`), any other drop stays at the courier (6.88g). Pickup Items (A14D, SPV 19349, SAV 19145, SLV 19328): the caster's items on every courier within 1300 (SMV 19346) go to the caster's hero start and, 0.1 s and 0.2 s later, are added to the hero (6.88g; the pack's ground items carry no owner, so only the items that action moved are collected, GUESS). | ctl-courier-drop-pickup, ctl-courier-drop-filters, ctl-courier-pickup-owner |
| GP-CTL-3 | Courier ping (A14F, RCE 40391): the owner's minimap is pinged on the courier for 1 s; 5 s cooldown. | ctl-courier-ping |
| GP-CTL-4 | Courier Burst (A0JZ): the flying courier casts its burst slot; the ground courier refuses. | ctl-courier-burst |
| GP-CTL-5 | Change Courier Type (A0OT, U7V 20946 starts U1V 20747, ULV 20637 applies the Srtt morph): key G (`keys.courier.skin`, 0.1 s cooldown) morphs the selected courier with `game.morph` into the next skin of 6.88g order: ground chicken, penguin, raccoon, crab, seal, skink, dog, dune worm, carrion beetle, fel boar, rabbit, pack horse; flying crow, zeppelin, vulture, owl, viper, frost wyrm, flying panda. On a ground press three draws can override (U1V 20874-20882): 1 in 10 mini pudge, 2 in 10 grunt, 2 in 10 pandaren, and from those the next skin is mini techies then rabbit. Skins are `courier_*` kinds in `units_courier_skins.osd` (generated by `tools/gen-courier-skins.mjs`, stats identical to the base courier, 6.88g), `courier_kinds.lua` holds the order. Every skin draws the one courier model (GUESS, needed_characters.md lists the requests). | units-courier-skin-order, units-courier-skin-flying |
| GP-CTL-6 | Glyph of Fortification (A141/A1WI, Z4V 22906): the team's structures take no damage for 5 s, 300 s team cooldown, reset by the loss of a tier one tower. | ctl-glyph-fortification |
| GP-CTL-7 | Double Hold: two hold presses within 0.2 s hold fire when `-hold` has turned the feature on. | ctl-double-hold |
| GP-CTL-8 | Camera distance steps of 150 within 1500..3000 (A33L/A33M, OIE 24657) and the rotation lock, on by default (Q_V 18808, which forces rotation 90). 6.88g keys: A33L Camera distance plus has hotkey S and A33M minus has hotkey A, command-card hotkeys of the Circle of Power card only (ability.ini 106578, 106631), so they cannot be page-wide keys here (A is attack move, S is stop); the page keeps PageUp and PageDown for the step (GUESS). The rotation has no key in 6.88g: the chat command `-w` (E7X 33447) frees it (distance 1650, angle 304, rotation 90, then the Esc menu sliders drive the camera, Z9R 98622) and a second `-w` locks it, so `rotationLockKeys` is empty and `-w` calls `game.camera_lock`. `-cam [n]` (Y6E 33057): 1500..3000 sets the distance, 0 < n < 3 multiplies 1650, n <= 0 resets with the text `Incorrect multiplier, camera changed to default.`, anything else `Incorrect camera distance.`; the bare `-cam` readout is page-side and not ported (6.88g). | js/client/zoom-check.mjs, unv-camera-chat |
| GP-CTL-9 | `game.sound(name[, player])` targets one client or all; announcer events play the table entry in announce_sounds.lua and an unknown id plays nothing. | ctl-sound-targeting, ctl-announce-sounds |

## Draft game modes (DRF)

Script: scenarios/moba/draft.lua (state machine, panels), draft_data.lua (pool), hooks in panels.lua (`lobby_act`), modes.lua (`modes.restrict`, `picks_free`) and chat_lobby.lua (`-repick` and `-swap` are refused while a draft runs). In 6.88g the draft happens in the match on hero units sold from circles; here it runs in the lobby on the hero grid plus `game.panel` matrices (`draft`, `draft_pool`). The lobby Start press begins the draft; the host presses Start again once every seat holds a hero. Time is the draft clock in sim ticks from that press (the sim does not tick in the lobby: the page must tick it or send ui `draft_clock {ticks}`). Pool: the 112 heroes of `P3` (U3 war3map.j 8962-9113), so heroes the 6.88g map lacks (monkey_king, pangolier ...) are never offered. Team A is `game.random(2)` where the JASS uses `GetRandomInt(1,2)`; bots and empty captain seats act with the timeout rule at once (GUESS: the JASS has no computer-player branch in these modes: its only `MAP_CONTROL` and slot-state reads are PLAYER_SLOT_STATE_EMPTY, f3 8539, QKE 30992, CWX 41443).

| id | mode | behaviour | gate check |
|---|---|---|---|
| GP-DRF-1 | Captains Mode `cm` | ALX 38752-38822, BEX 38732-38751, stages BVX..NQX 37923-38731, display NFX 37436-37770, NKX 37837-37891. First turn at 15 s. Twenty turns, captain = first seat of the team, A = random team: ban A B A B (30 s), pick A B B A (40 s), ban A B A B (30 s), pick B A B A (40 s), ban A B (30 s), pick B A (40 s). Each team has 110 s of bonus time (bK = 'n') used once a turn is past its base time; with it spent the turn auto-acts: a ban becomes no ban (NPX -1), a pick a random free hero (NMX over RU, Pr false). After the last pick each seat takes one of its team's five picks (NHX), claim window 90 s (TA); a held hero is gone from the others. GUESS: the JASS has no assignment for a seat that claims nothing (the claim window is the pre-game timer SA/TA, PRE 30050-30145, and at 0 it assigns no hero; a heroless seat keeps buying from its circle, and in the `ei` mode loses 1 gold a second over the last 75 s, PRE 30129-30141), but the lobby draft needs every seat to hold a hero at Start, so leftovers take the first free team pick in seat order; the random assignments that exist are the `-random` commands (XQX 33824, KU 3916: a random unpicked hero of the seat's half of the pool); bots claim at once; picks outside the sell events (a sold hero counting as an evaluation) are not modelled. | draft-captains-mode, draft-captains-mode-timeout, draft-modes |
| GP-DRF-2 | Captains Draft `cd`, Extended `x` | A8X 40123-40351, CNX 40087-40122, CAX 39997-40086, B9X 39834-39910, COX 39938-39956, CXX 39926-39937, B2X 39648-39699. 28 distinct cards by `lU` (coin flip between the halves, then a uniform index; no techies exclusion); extended `x` 42 cards. First turn at 15 s on a random team, ten turns alternating teams: ban 1 four times, pick 1, pick 2 four times, pick 1 (extended: six bans, pick 1, pick 2 four times, pick 1). Each team has a 150 s reserve (180 s extended) that drops each second of its turn; a turn handed over starts with at least 6 s; at 0 each second the turn acts on a random free card (ban or pick). Only the captain (first seat) acts, and only once the turn's actions are open. After the draft the picks go to the claim phase (90 s, starting 6 s later). GUESS: the leftover and bot rules of cm apply (no 6.88g assignment either). | draft-captains-draft, draft-captains-draft-timeout, draft-modes |
| GP-DRF-3 | Random Draft `rd` | AMX 39428-39615, B0X 39405-39427, BYX 39171-39368, B_X 39380-39404, BQX 39028-39057, BSX 39058-39068. 24 distinct heroes by `lU` without index 23 (techies). Stages every 20 s from 15 s: 1 seat of team A, 2 of B, 2 of A, 2 of B, 2 of A, 1 of B (seats in team order). A seat picks once, only in its stage; at the next stage's start the previous stage's seats that did not pick take the lowest free card in deal order (BSX); the draft ends 20 s after the last stage starts. | draft-random-draft |
| GP-DRF-4 | Single Draft `sd` | APX 40432-40516, CDX/CFX 40352-40390. At the start each seat is offered three heroes: an agility hero (list Zd, group 2), a strength hero (eD, group 1) and an intelligence hero (oD, group 3), drawn in seat order sentinel 1, scourge 1, sentinel 2 ...; no hero is offered twice unless duplicate mode is set. A seat takes one; 120 s window (`TA` 'x'). GUESS: a seat with no pick takes its first offer (no JASS assignment: the window is the same timer); a seat sees only its own offers in its panel but the lobby grid marks every offered hero as pickable. | draft-single-draft |
| GP-DRF-5 | Reverse `rv` | AKX 41490-41552, CWX 41439-41483, CYX 41484-41489, BTV 11446-11450. Seats are paired by place (sentinel n with scourge n); a seat picks the hero its partner will play (heroes picked once, ya/Wa flags). Picking closes at 57 s; at 60 s every seat that was not given a hero gets a random free hero of its own half of the pool (KU, indices 1..59 sentinel, 60..112 scourge) and keeps it. | draft-reverse-draft |
| GP-DRF-6 | Super Creeps `sc` | KQE 26847-26971, registered by PRE 30093-30097 when the flag is set: every 600 s of match time one special creep per side spawns at the chosen lane and walks it. `scenarios/moba/super_creeps.lua` draws kind and lane once with `game.random` (JT 1..9: golem n003, fish n00E, hydra n00D by the lane west, middle, east; DL is false so all nine rows apply), spawns the same kind for both sides through `lanes.spawn_special` (owner is the side's base player, route is the lane's) and tells everyone `<kind> has spawned for the <side> and is taking the <lane> path` for 10 s (unit-text rawcodes n051..n05C Sentinel, n0CC..n0CK Scourge, all one template). Kinds `super_creep_golem|fish|hydra` in `units_super_creeps.osd` carry the 6.88g hit points, armor, damage, cooldown, range, backswing and bounty dice (provenance sidecar); speed of golem and fish (stock nsgg, nmtw), xp, collision, sight and acquire are GUESS; the spawn point is the lane's melee spawn (GUESS, the rect centre is not in the data); models are placeholders (GUESS, needed_characters.md); the Slow Poison ability Aspo of the hydra is not modelled (GUESS). | units-super-creeps-round, units-super-creeps-message, units-super-creeps-off |
| GP-DRF-7 | draft clock | open: the page does not tick the sim while a lobby is open (docs/ARCHITECTURE.md match start gate), so the draft only advances in a real lobby when the page ticks the sim or sends ui `draft_clock {ticks}` from the host. | none |

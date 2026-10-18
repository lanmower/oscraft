# How a match plays

The `scenarios/moba` scenario is a data-only match: the engine has no scenario-specific code, and every number here comes from the `.osd` files in that directory. The full tables are in [REFERENCE.md](REFERENCE.md), generated from the same files.

**Provenance.** A figure is UNVERIFIED unless its `.osd` comment cites a source. Cited figures: tower health, armor and damage; creep health, armor, damage, gold and xp bounty; Roshan health, damage, armor, gold and xp; the cost of the blink item. Everything else (speeds, radii, ranges, sight, hero stats, ability numbers, item costs and stats, timers, positions) is a placeholder. Where this page marks a figure UNVERIFIED, expect it to change.

Time: the sim runs 16 ticks per second. Distances are milli-units; one map cell is 1000.

## Map

A 64 x 60 cell grid (`map.osd`) with three full-width lanes (top y 7-10, middle y 29-32, bottom y 51-54), two base pads at the ends of the middle lane joined to all three lanes, a walkable river down the middle (x 30-34, height level 0), and four raised jungle areas (level 3), two per side. Shops, fountains, lane markers and scenery are doodads. The west side is team 0, the east side team 1.

## Sides and players

Ten players, two teams of five: even players on team 0, odd on team 1 (`start(...)`). Each player starts with one hero and one tender at their base. The lobby pool holds the ten hero types, five per side.

## Lanes and creeps

Each base carries the `creep_wave` ability. It fires when the base spawns and again each time its 600-tick (37.5 s) cooldown ends. It spawns one creep per lane (top, middle, bottom). Each creep marches along its lane to the nearest enemy building and fights what it meets. Idle units with a weapon (creeps, towers, bases) strike the nearest enemy in reach.

Creep: health 550, armor 2, damage 21 (cited); speed 2000, radius 350, range 1000, sight 7000 (UNVERIFIED). Bounty 36 gold and 57 xp (cited).

## Towers and bases

Three towers per side per lane, outer, middle and inner (x 12000, 16000, 22000 for team 0; mirrored for team 1). Tower: health 1800, armor 12, damage 90 (cited), range 6000, period 40 ticks (UNVERIFIED). A tower pays no bounty.

The base: health 1500, armor 0 (UNVERIFIED; cited ancient values were tried and made the CI siege check fail, so they were reverted). A base carries the `building` filter. When the last living building of a side dies, the rival wins (`win_on_base_loss` in `behaviours.osd`).

## Heroes

Ten types plus the base `hero`, each with five abilities: four ordinary and an ultimate in slot R. Health 260-360, damage 12-18, armor 1-3 (UNVERIFIED). Heroes are `player_controlled`, share kill and assist xp within the victim's assist radius (6000), and gain attributes: 20 health per strength point, base mana 75 plus 12 per intelligence point, 1 damage per strength point (constants in `rust/crates/sim/src/hero.rs`, UNVERIFIED).

Skills: a hero spends one skill point per character level. Ordinary abilities learn to rank 4; the ultimate learns to rank 3 at levels 6, 12 and 18. The xp curve reaches level 18 (cumulative xp 100, 250, 500, ... 15200; UNVERIFIED). Mechanics covered by the roster: nuke, projectile, area damage, blink, heal over time, aura, passive stat, summon, channel, and the status effects stun, slow, silence, root and invisibility. Passives are stat abilities with cooldown 100000 cast once at spawn by a `spawned` behaviour.

## Respawn and buyback

`hero_rules.txt` (all UNVERIFIED): a dead hero respawns after `5 + 2 * level` seconds, capped at 60. Buyback costs `100 + 20 * level` gold with a 150 s cooldown. Each hero earns 1 passive gold per second, and `team_share_pct=20` governs the team share of gold.

## Items and shops

A hero holds six items and a stash of six. Buying needs gold, a free slot (overflow goes to the stash) and standing within 4000 milli of a shop doodad (one beside each base, plus two outer shops). Selling returns half the cost. A recipe names up to two earlier components; buying it consumes them and charges its cost minus theirs. Eleven items: three plain stat items, a healing item, a blink dagger (cost 2250 cited, blink range 5000 UNVERIFIED), a banner with an active, an axe, and three recipes (iron mail, heart ring, battle fury axe), plus the aegis. All stats and all other costs are UNVERIFIED.

## Neutrals

Twelve neutral creeps in four camps of three (player -1, positions near the four camp doodads), answering attacks through the shared `attacked` behaviour. A camp unit that strays more than 6000 from home walks back; a dead one respawns at home after 60 s (UNVERIFIED).

## Roshan

A large neutral boss at the middle pit (32000, 32000; position UNVERIFIED). Health 6000, damage 75, armor 30, bounty 245 gold (midpoint of a cited 200-290 range) and 400 xp (cited). Respawns 480 s after death (UNVERIFIED). The killer receives the `aegis` item (cost 300, UNVERIFIED).

## Runes

Two bounty runes at the river points (32000, 26000) and (32000, 38000), positions UNVERIFIED. A living hero within 1500 takes 50 gold and the rune dies; it respawns after 180 s. All four numbers are UNVERIFIED.

## Denying

A unit may deny an allied unit (never itself) whose health is at or below its deny threshold, a percentage of max health set by `deny_pct`: hero 25, creep 50, tender 50, objective 50, tower 10, base 0. A denied unit pays no gold. A denied lane creep gives enemy heroes 50 percent of its xp; a denied hero gives 0 percent. The 50 and 0 percent xp rules and the no-gold rule are verified; every threshold is UNVERIFIED.

## Economy

Tenders harvest `resource_a`, which is the gold of this match. Creep and Roshan kills pay their bounty to the killer's side.

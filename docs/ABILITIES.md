# Hero abilities

Parity target: Dota 2 at SteamDatabase/GameTracking-Dota2 `282fad29f0f74504fad465c52d197fb61f4d612b`. Every ability number of the
heroes below comes from that commit's `game/dota/pak01_dir/scripts/npc/heroes/npc_dota_hero_<hero>.txt` (`AbilityDefinitions`:
cooldown, mana, cast range, cast point, channel time, `AbilityValues` by level, scepter, shard and talent deltas) and the generic
talents from `npc/npc_abilities.txt`. The heroes' ability values are not in `npc_abilities.txt` at this commit; they live in the
per-hero files.

`scripts/gen-abilities.mjs` reads those files and writes `scenarios/moba/abilities_dota.osd` and `units_summons.osd`;
`scripts/dota-recipes.mjs` says which engine effect carries which Valve key. A key a recipe does not name is listed in the
generated file as `# <ability> unmodelled Valve keys`, and a talent with no effect as `# <hero>: talents with no modelled effect`.
Run `node scripts/gen-abilities.mjs` to regenerate. Scale: 1 Dota unit = 15.625 milli, 16 ticks a second.

## Heroes done

axe, crystal_maiden, sven, juggernaut, zuus (Zeus), sniper, lion, omniknight (their full kits), phantom_lancer (Spirit Lance and
Juxtapose), dazzle (Poison Touch and Shallow Grave), lina (Dragon Slave, Light Strike Array, Laguna Blade), lich (Frost Nova,
Frost Shield, Chain Frost), tidehunter (Gush, Kraken Shell, Ravage), slardar (all four), antimage (Mana Break, Blink, Counterspell) and vengefulspirit (Magic Missile,
Wave of Terror, Command Aura): the abilities, the
generic talents (`<hero>_talents`) and the unique talents wherever the key they change is modelled. Slots: basics, then the
ultimate, then the talent modifiers. The other 120 roster heroes in `units_heroes.osd` have no abilities yet.
Not done: Phantom Lancer's Doppelganger and Phantom Rush, Dazzle's Shadow Wave and Weave and the ultimate that moves the soul,
Anti-Mage's Mana Void, Lina's Fiery Soul, Lich's Sinister Gaze, Tidehunter's Anchor Smash (its radius is not in the dump), Vengeful Spirit's Nether Swap.

## Mechanics added to the engine

Effect kinds `dot`, `status`, `timed_stat`, `aoe`, `dispel`, `chain`, `zone`, `illusion`, `mana`, `if_kill`, `attack` and typed
`damage` (physical, magical, pure) and `heal` (whole hit points, percent of maximum); see `rust/crates/data/FORMAT.md`. Around them:

- Rank scaling of any effect parameter, cooldown, mana, cast range, proc chance and period; scepter, shard and eight talent
  modifiers per ability (`AXTN`); talents picked at levels 10, 15, 20, 25 (`Sim::choose_talent`); skill gates 1, 3, 5, 7 and 6, 12, 18.
- Cast point (the caster is held, a stun ends it), cast range with walking into range, charges, passive stat abilities, auras,
  procs on attack and when attacked (by chance or every Nth hit), channels that pulse (random ring) and end by stun, silence or
  hex rather than damage (`channel_damage_interrupt=0` in `hero_rules.txt`).
- Illusions (`illusion`), dispels (`dispel`), summons (`ward` with an aura on the placed unit), blinks (`move`).
- Statuses: taunt, hex, disarm, spell immunity, invulnerable, break, mute, physical immunity, death prevention.
- Attack modifiers from stats: evasion, critical strike, cleave (a cone), lifesteal, bash, attack speed, range, magic resistance,
  spell amplification, cooldown reduction, mana and health regeneration, attributes.
- Armour, magic resistance and spell amplification apply to typed spell damage; hero magic resistance 25 percent.

## What differs from Dota and why

- Charges exist; Zeus's Heavenly Jump talent charges are not modelled.
- Facets are deprecated in the dump and not modelled. Shard and scepter values are modelled only where a recipe names the key;
  the shard and scepter abilities themselves (Zeus's cloud and lightning hands, Juggernaut's swift slash, trinity and vaulted
  strike, Crystal Maiden's crystal clone and glacial guard) are not.
- Projectile speeds of Lion's Impale and travel times of instant-effect bolts are not modelled (Impale lands along its line at once).
- Heavenly Jump hops toward the cast point, not along the facing (the sim has no facing).
- Freezing Field fires 80 explosions of 1.25 times the damage instead of 100 (a tick is 62.5 ms); Blade Fury and Mana Drain pulse
  every 3 and 4 ticks so every step is a whole number; Omnislash strikes every 9 ticks.
- The healing ward does not follow Juggernaut. Illusion-specific rules beyond damage dealt and taken are not modelled.
- Axe: Battle Hunger's speed bonus talent, Counter Helix's scepter debuff, Culling Blade's armour stacks. Crystal Maiden: the
  barrier talent, mana regeneration amplification. Sven: the slow resistance talent, the shard barrier, Wrath of God. Zeus: Static
  Field, the bolt's true sight and spread talent. Sniper: Keen Scope, the shard grenade, Assassinate's scepter. Lion: To Hell
  and Back, Finger of Death's kill stacks. Omniknight: Hammer of Purity's healing and range, the scepter.
- Lina's Dragon Slave and Vengeful Spirit's Wave of Terror are straight lines of constant width and land at once. Lich's Frost Shield
  gives only the damage reduction, Chain Frost has no slow, Kraken Shell has no damage-triggered cleanse.
- Anti-Mage: Mana Break burns the flat mana only, Counterspell gives the magic resistance without reflecting spells, Blink has no minimum range.
- Spell immunity pierce comes from the ability's own flag (`SpellImmunityType`); dispel strength is a mask, not Valve's types.
- Gate checks: `rust/gate/src/ability_checks.rs`, run alone with `GATE_ONLY=ability- cargo run -p oscraft-gate`.

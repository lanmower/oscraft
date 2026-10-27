# Naming map

This map is the spec for one atomic rename commit across `rust/` and `docs/`. Every term below is replaced in that commit. No backward compatibility: rename freely.

## Rules

1. After the rename commit, no game-publisher names, race names of any game, unit or ability names of any game, or script-language names of any game appear in the repository. The rename is one atomic commit across `rust/` and `docs/`.
2. SDK names describe what they do. They carry no game-publisher or other game's names.
3. The platform uses generic game-engine vocabulary: entity, action, rule, resource type, faction, capacity, status effect, tech level, secondary health pool, deployment, spawning.
4. Sample content in the example mods (`mods/`) uses generic example names. Example entities are named by role, not after any game.
5. Deletions, not renames: MPQ, Mo'PaQ and the `mpq` crate are removed with the MPQ reader. Game names in docs and comments are deleted, not replaced.
6. `docs/NAMING.md` itself contains old terms. The rename commit deletes it, so rule 1 holds after that commit.

## Map

| Old term | New term | Where it appears | Rule |
|---|---|---|---|
| race | faction | `FORMAT.md`, ECON per-race fields, `.osd` | everywhere, including ECON per-race fields |
| minerals | resource-a | `RESOURCE_KINDS` in `format.rs`, `FORMAT.md`, `.osd` | resource type |
| vespene, gas | resource-b | `RESOURCE_KINDS` in `format.rs`, `FORMAT.md`, `.osd` | resource type |
| mineral patch | resource node | `FORMAT.md`, Rust source | resource node |
| geyser | resource site | `FORMAT.md`, `builtins.rs`, `encode.rs`, `model.rs`, `validate.rs`, `arrays.rs` | merged with extractor, one concept |
| extractor | resource site | `FORMAT.md`, `builtins.rs`, `encode.rs`, `model.rs`, `validate.rs`, `economy.osd` | merged with geyser, one concept |
| worker | collector | `FILTERS` in `format.rs`, `FORMAT.md`, `builtins.rs`, `encode.rs`, `format.rs`, `model.rs`, `validate.rs` | role term |
| supply | capacity | `FORMAT.md`, `builtins.rs`, `encode.rs`, `model.rs`, `validate.rs`, `loader.rs` | resource concept |
| ghost | stealth-a | `.osd` data | generic example entity |
| barracks | producer-a | `.osd` data | production structure |
| tender | hauler | `map.osd`, `units.osd`, `units_extra.osd` | generic example entity |
| skirmisher | raider | `unit_mod.lua`, `main.rs`, `map.osd`, `tech.osd`, `units.osd`, `units_extra.osd` | generic example entity |
| warden | guardian | `unit_mod.lua`, `map.osd`, `tech.osd`, `units.osd`, `units_extra.osd`, `upgrades.osd` | generic example entity |
| unit | entity | type and field names in `model.rs`, `builtins.rs`, `encode.rs`, `validate.rs`, `FORMAT.md` | type and field rename |
| calldown | deployment | `format.rs`, `reader.rs`, `validate.rs`, `FORMAT.md`, `abilities.osd` | deployment |
| use_calldown | use_deployment | `EFFECT_KINDS` in `format.rs` | effect kind |
| buff, buffs | status effect, status_effects | `EFFECT_KINDS`, `format.rs`, `arrays.rs`, `buffs.rs`, `hash.rs`, `lib.rs`, `FORMAT.md` | buff becomes status effect; `buffs.rs` becomes `status_effects.rs` |
| feedback | energy-burn | `.osd` data | action; not `drain`, which names a vital mode |
| bulwark | bastion | `abilities.osd` | action |
| blink | blink | all | keep, generic name |
| behaviour, behavior | rule | `FORMAT.md`, `builtins.rs`, `encode.rs`, `format.rs`, `model.rs`, `validate.rs` | behaviour becomes rule |
| remove_behaviour | remove_rule | `EFFECT_KINDS` in `format.rs` | effect kind |
| upgrade | tech level | `FORMAT.md`, `builtins.rs`, `encode.rs`, `format.rs`, `model.rs`, `reader.rs` | upgrade becomes tech level |
| shield | secondary health pool | `VITALS` in `format.rs`, `FORMAT.md`, `builtins.rs` | secondary health pool |
| shield_bonus | secondary_pool_bonus | `EFFECT_KINDS` in `format.rs` | effect kind |
| production, produce | spawning | `FORMAT.md` | production becomes spawning; `production.rs` was deleted, its queue rules are `scenarios/duel-routes/production.lua` |
| MPQ, Mo'PaQ, mpq crate | deleted | `rust/crates/mpq/`, `lib.rs`, `Cargo.toml` | deletion |
| OSDT | OSDT | `format.rs`, `loader.rs`, `data/Cargo.toml`, `FORMAT.md` | keep: file magic, OScraft Data Table |
| ABIL | ACTN | `format.rs`, `loader.rs`, `FORMAT.md` | section code: actions |
| UABL | ENTA | `format.rs`, `FORMAT.md` | section code: entity actions |
| UNIT | ENTY | `format.rs`, `loader.rs`, `FORMAT.md` | section code: entities |
| BHVR | RULE | `format.rs`, `loader.rs`, `FORMAT.md` | section code: rules |
| UPGR | TECH | `format.rs`, `loader.rs`, `FORMAT.md` | section code: tech levels |
| UEXT | ENTX | `format.rs`, `loader.rs`, `FORMAT.md` | section code: entity extensions |
| DODA | DECO | `format.rs`, `loader.rs`, `FORMAT.md` | section code: decor objects (props) |
| STRS | STRS | `format.rs`, `loader.rs` | keep: strings |
| EFCT | EFCT | `format.rs`, `loader.rs` | keep: effects |
| PLCE | PLCE | `format.rs`, `loader.rs` | keep: placements |
| REGN | REGN | `format.rs`, `loader.rs` | keep: regions |
| LIMT | LIMT | `format.rs`, `loader.rs` | keep: limits |
| ECON | ECON | `format.rs`, `loader.rs`, `FORMAT.md` | keep: economy; per-race fields become per-faction |
| ACHN | ACHN | `format.rs`, `loader.rs` | keep: achievements |
| RSCH | RSCH | `format.rs`, `loader.rs` | keep: research |
| PREQ | PREQ | `format.rs`, `loader.rs` | keep: prerequisites |
| DMGF | DMGF | `format.rs`, `loader.rs` | keep: damage formulas |
| GRID | GRID | `format.rs`, `loader.rs` | keep: pathing grid |
| `osdt-data` | `oscraft-data` | `crates/data/Cargo.toml` | crate name; library `osdt_data` becomes `oscraft_data` |
| `mod-host` | `oscraft-host` | `crates/mod-host/Cargo.toml` | crate name |
| `sim` | `oscraft-sim` | `crates/sim/Cargo.toml` | crate name |
| `spawn_unit(kind, x, y)` | `game.spawn_entity(kind, x, y)` | `rust/crates/mod-host/src/piccolo_host.rs`, `rust/crates/mod-host/src/lib.rs`, `docs/SCRIPTING.md` | Lua host function, namespace `game`, verb_noun |
| `get_unit_count()` | `game.count_entities()` | `rust/crates/mod-host/src/piccolo_host.rs`, `docs/SCRIPTING.md` | Lua host function |
| CLI flags, env vars | `OSCRAFT_` prefix | `rust/gate/src/checks.rs` reads `GATE_ONLY`; the binaries (`mapexport`, `osdt-data`, `mod_check`) parse their own arguments | new env vars use the `OSCRAFT_` prefix and binaries use `oscraft-*` names |
| `harvest.rs` | removed | `sim/src/harvest.rs` (deleted) | the gather rule is `scenarios/duel-routes/harvest.lua` |
| `arrays.rs`, `hash.rs`, `lib.rs` | unchanged | `sim/src/` | keep |
| `production.rs` | removed | `sim/src/` (deleted) | no `spawning.rs`; the queue rules are `scenarios/duel-routes/production.lua` |
| `buffs.rs` | `status_effects.rs` | `sim/src/` | file name |
| `FORMAT.md` | `docs/FORMAT.md` | `rust/crates/data/` | doc moves to `docs/`; doc names are UPPERCASE.md |
| `Unit` (type) | `Entity` | `sim`, `data` | type rename |
| `Buff` (type) | `StatusEffect` | `sim`, `data` | type rename |
| `Upgrade` (type) | `TechLevel` | `sim`, `data` | type rename |
| `Behaviour` (type) | `Rule` | `sim`, `data` | type rename |
| `Ability` (type) | `Action` | `sim`, `data` | type rename |

## Naming conventions

- Crates: `oscraft-<area>`, lowercase with hyphens. Library names follow with underscores.
- Lua host: one namespace `game`, `verb_noun` functions.
- Section codes: four uppercase letters with a stated meaning in the table above.
- Rust files: snake_case. Types: CamelCase.
- Doc files in `docs/`: UPPERCASE.md.

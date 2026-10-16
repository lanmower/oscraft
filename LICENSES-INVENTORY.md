# License inventory: SC2-derived formats and assets

Scope: the owned crates `rust/crates/mpq` and `rust/crates/data`, plus a project-wide literal codesearch on main for Blizzard-owned terms (Blizzard, StarCraft, SC2, Galaxy, GameData, Storm, Mo'PaQ, MPQ, StormLib, internal/mpq).

## Assets

The tree on main (125 blobs) has no SC2 or Blizzard asset files: no `.sc2`, `.s2ma`, `.sc2map`, `.mpq`, `.m3`, `.mdx`, `.blp`, `.dds`, `.scx`, `.fnt`, `.wav`, `.ogg` or `.mp3`, and no path naming storm, blizz, sc2 or starcraft. Disposition: none present.

## Entries

| # | Path | Term | Hits | Disposition | Note |
|---|---|---|---|---|---|
| 1 | rust/crates/mpq/Cargo.toml | Mo'PaQ, Storm | 2 | replaced | description now says MPQ and sector codecs |
| 2 | rust/crates/mpq/src/lib.rs | Mo'PaQ | 1 | replaced | module doc |
| 3 | rust/crates/mpq/src/lib.rs | Storm | 3 | replaced | module doc, name lookup doc, name form doc |
| 4 | rust/crates/mpq/src/lib.rs | internal/mpq | 1 | removed | the cited path is absent on main; provenance claim dropped |
| 5 | rust/crates/mpq/src/bzip2.rs | Storm | 1 | replaced | doc comment |
| 6 | rust/crates/mpq/src/crypt.rs | Storm | 3 | replaced | HashString and cipher docs |
| 7 | rust/crates/mpq/src/het.rs | Storm | 2 | replaced | doc comments |
| 8 | rust/crates/mpq/src/huffman.rs | Storm | 1 | replaced | doc comment |
| 9 | rust/crates/mpq/src/lzma.rs | Storm | 1 | replaced | doc comment |
| 10 | rust/crates/mpq/src/huffman_tables.rs | StormLib | 1 | justified permissive | attribution kept: StormLib is MIT (Ladislav Zezula); constants are format facts |
| 11 | rust/crates/mpq/src/sparse.rs | StormLib | 1 | justified permissive | attribution kept: MIT reference behaviour |
| 12 | rust/crates/mpq (lib.rs, het.rs, jenkins.rs, Cargo.toml) | MPQ 10, HET 22, BET 22 (shared lines) | 54 | justified original | format names for the container and its tables; reader is an original Rust implementation on the standard library. Deletion per docs/NAMING.md rule 5 is deferred: the crate name is used by the workspace and rust/gate, outside owned paths |
| 13 | rust/crates/data (11 files) | all terms | 0 | none found | no hits |
| 14 | rust/gate/src/mpq_roundtrip.rs | Storm | 1 | deferred | owned by the gate lane |
| 15 | docs/NAMING.md | Blizzard, SC2, StarCraft, Mo'PaQ, MPQ | 8 | deferred | rule 6: deleted in the atomic rename commit |
| 16 | docs/GAMEPLAY_PARITY.md | StarCraft | 3 | deferred | docs lane |
| 17 | README.md | MPQ | 1 | deferred | root README owner |
| 18 | .github/workflows/ci.yml | MPQ | 1 | deferred | CI workflow owner |

## Counts by disposition

- replaced: 8 entries (rows 1, 2, 3, 5, 6, 7, 8, 9)
- removed: 1 entry (row 4)
- justified permissive: 2 entries (rows 10, 11)
- justified original: 1 entry (row 12)
- none found: 1 entry (row 13)
- deferred to owning lane: 5 entries (rows 14 to 18)

## Notes

- Acceptance reading: `Storm` as a literal substring also matches `StormLib`, a third-party MIT project. After this change the owned crates contain no `Blizzard`, `StarCraft`, `SC2`, `Galaxy`, `GameData`, `Mo'PaQ` or Blizzard `Storm` identifiers; the remaining `StormLib` hits are rows 10 and 11.
- Owner decision pending: docs/NAMING.md rule 5 deletes the MPQ reader and the `mpq` crate. That deletion touches the workspace manifest and rust/gate, which this row does not own.

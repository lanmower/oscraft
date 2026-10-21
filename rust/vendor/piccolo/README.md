# Vendored piccolo 0.3.3

The Lua VM the mod host runs on (crates.io piccolo 0.3.3, MIT or CC0), vendored with two fixes: `src/table/raw.rs` and
`src/compiler/compiler.rs`.

The grow path of `RawTable::set` had two arms, each missing half of what the following insert needs. The array arm
purged dead keys and moved array keys into the array, but reserved no map room for the new key, so the insert rehashed
and panicked "map slot must be pre-reserved". The map arm reserved room without purging dead keys, and its rehash hasher
panics on a dead key ("all keys must be live when table is grown"). The fix purges dead keys and reserves room in both
arms. The same change is upstream in piccolo master (commit f617091a, "A lot of improvements to RawTable"), which is not
in a crates.io release.

Reproduced as a flaky failure of the mod-host panels test (about one run in five, from the hash layout). Remove this
directory and the `[patch.crates-io]` entry once a release carries the upstream change.

The assignment statement (`assignment_statement` in `src/compiler/compiler.rs`) stored each target before the next value
was evaluated, so `a, b = b, a` gave `b, b`. A single field target evaluated its value before its table and key, so
`t[f()] = g()` called g first. Single and multiple assignments both follow the Lua 5.4 order now: the table and key of
every field target, then every value, then the stores. The fix copies those into fresh registers and stores after them.
The regression tests are rust/crates/mod-host/tests/vm_assignment.rs.

The build allows the newer lint `dangerous_implicit_autorefs` for this crate (upstream code in `string.rs`): a path
dependency is not capped to warnings the way a registry crate is, and the lint is an error on current rustc.

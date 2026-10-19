//! Lua mod host. A mod is a chunk that defines `on_tick(state)`. The host owns `state` and the
//! sim; a mod reload never resets them. The native host runs mods on mlua (Lua 5.4, C objects);
//! the wasip3 guest runs them on piccolo (pure Rust), a reduced surface in piccolo_host.rs.

#[cfg(all(feature = "mlua", feature = "piccolo"))]
compile_error!("mod-host: the mlua and piccolo features are exclusive");

mod shared;

const MAX_UNITS: usize = 1024;

// The events a mod handles, shared by both backends so the names match on mlua and piccolo. Each
// entry is (name, the Lua arguments it receives). A mod defines a global function of the same
// name, and game.trigger takes the same names. on_tick runs once per step with the state table.
pub const EVENTS: [(&str, &str); 7] = [
    ("on_spawned", "unit"),
    ("on_attacked", "unit"),
    ("on_died", "unit, killer, killer_player, bounty, victim_player, killer_hero"),
    ("on_enter", "region, unit"),
    ("on_leave", "region, unit"),
    ("on_player_joined", "player"),
    ("on_tick", "state"),
];

// The OSDT image for a directory of `.osd` files, from the generate path `osdt-data <dir> <out>` runs.
// Behind the `build` feature so a guest that only loads images never links the starlark toolchain.
#[cfg(feature = "build")]
pub fn build_image(src_dir: &std::path::Path) -> Result<Vec<u8>, String> {
    osdt_data::generate(src_dir).map_err(|e| format!("{e:#}"))
}

#[cfg(feature = "mlua")]
mod mlua_host;
#[cfg(feature = "mlua")]
pub use mlua_host::*;

#[cfg(feature = "piccolo")]
mod piccolo_host;
#[cfg(feature = "piccolo")]
pub use piccolo_host::*;

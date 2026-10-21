//! Lua mod host. A mod is a chunk that defines `on_tick(state)`. The host owns `state` and the
//! sim; a mod reload never resets them. Mods run on piccolo (pure Rust), in piccolo_host.rs.

mod shared;

const MAX_UNITS: usize = 1024;

// Every global the mod environment registers, the list the editor completes from. The piccolo host
// registers these names; the mod-lsp completion reads this list.
pub const HOST_GLOBALS: &[&str] = &[
    "spawn_unit", "cmd_add_gold", "cmd_add_stat", "cmd_add_unit_gold", "cmd_add_xp", "cmd_set_respawn", "get_unit_count",
    "define_region", "in_region", "region_bounds", "map_size", "schedule", "cancel", "order_unit", "order_player",
    "unit_position", "unit_hp", "unit_player", "unit_alive", "edit_terrain", "damage_unit", "order_attack", "list_abilities",
    "ability_info", "list_events", "set_tile", "list_doodads", "remove_doodad", "place_doodad", "buff_unit", "game",
];

// The events a mod handles, shared by the host and mod-lsp so the names match. Each
// entry is (name, the Lua arguments it receives). A mod defines a global function of the same
// name, and game.trigger takes the same names. on_tick runs once per step with the state table.
pub const EVENTS: [(&str, &str); 7] = [
    ("on_spawned", "unit"),
    ("on_attacked", "unit"),
    ("on_died", "unit, killer, killer_player, bounty, victim_player"),
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

#[cfg(feature = "piccolo")]
mod piccolo_host;
#[cfg(feature = "piccolo")]
pub use piccolo_host::*;

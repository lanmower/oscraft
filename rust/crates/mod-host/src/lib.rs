
mod shared;

const MAX_UNITS: usize = 1024;

pub const HOST_GLOBALS: &[&str] = &[
    "spawn_unit", "cmd_add_gold", "cmd_add_stat", "cmd_add_unit_gold", "cmd_add_xp", "cmd_set_respawn", "get_unit_count",
    "define_region", "in_region", "region_bounds", "map_size", "schedule", "cancel", "order_unit",
    "unit_position", "unit_hp", "unit_player", "unit_alive",
    "ability_info", "game",
];

pub const EVENTS: [(&str, &str); 8] = [
    ("on_spawned", "unit"),
    ("on_attacked", "unit"),
    ("on_died", "unit, killer, killer_player, bounty, victim_player, killer_attributed"),
    ("on_enter", "region, unit"),
    ("on_leave", "region, unit"),
    ("on_player_joined", "player"),
    ("on_player_left", "player"),
    ("on_tick", "state"),
];

#[cfg(feature = "build")]
pub fn build_image(src_dir: &std::path::Path) -> Result<Vec<u8>, String> {
    osdt_data::generate(src_dir).map_err(|e| format!("{e:#}"))
}

#[cfg(feature = "piccolo")]
mod piccolo_host;
#[cfg(feature = "piccolo")]
pub use piccolo_host::*;

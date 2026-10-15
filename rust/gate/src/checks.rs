//! The check list. Each entry is a gate check named as docs/CAPABILITIES.md names it
//! (`gate:<name>`), plus two standing checks. A check returns Ok when it passes, or Err with
//! the reason it fails. A check whose API is not on the sim or mod-host public surface returns
//! `missing API: <name>`, so the owning row stays unverified until that API is public.

use std::path::{Path, PathBuf};

use sim::{fixed, Sim};

use crate::{absent, mod_checks, sim_checks};

pub const MAX_UNITS: usize = 1024;
pub const RUN_TICKS: u32 = 2000;

pub type Check = fn(&[u8]) -> Result<(), String>;

pub const CHECKS: &[(&str, Check)] = &[
    ("hash_stable_two_runs", hash_stable_two_runs),
    ("units_ge_map_spawns", units_ge_map_spawns),
    // CAP-MAP
    ("load-map", sim_checks::load_map),
    ("walkable-cells", sim_checks::walkable_cells),
    ("edit-terrain", mod_checks::edit_terrain),
    ("map-bounds", absent::map_bounds),
    // CAP-UNIT
    ("spawn-units", sim_checks::spawn_units),
    ("unit-stats-read", sim_checks::unit_stats_read),
    ("catalog-load", sim_checks::catalog_load),
    ("unit-classes", sim_checks::unit_classes),
    // CAP-ABIL
    ("cast-ability", sim_checks::cast_ability),
    ("ability-target-filter", sim_checks::ability_target_filter),
    ("ability-effects", absent::ability_effects),
    ("unsupported-effect-refuses", absent::unsupported_effect_refuses),
    // CAP-BEH
    ("apply-buff", sim_checks::apply_buff),
    ("clear-buff", absent::clear_buff),
    ("remove-behaviour", absent::remove_behaviour),
    ("behaviour-declared", sim_checks::behaviour_declared),
    ("behaviour-trigger-fires", sim_checks::behaviour_trigger_fires),
    // CAP-ORD
    ("order-parse", absent::order_parse),
    ("issue-move", sim_checks::issue_move),
    ("move-units-tick", sim_checks::move_units_tick),
    ("path-to-goal", sim_checks::path_to_goal),
    ("order-kinds", sim_checks::order_kinds),
    ("order-rate-limit", absent::order_rate_limit),
    // CAP-PROD
    ("produce-unit", sim_checks::produce_unit),
    ("harvest-resources", sim_checks::harvest_resources),
    ("spend-resources", sim_checks::spend_resources),
    ("cancel-production", sim_checks::cancel_production),
    // CAP-CMB
    ("attack-unit", sim_checks::attack_unit),
    ("area-damage", absent::area_damage),
    ("projectile-hit", sim_checks::projectile_hit),
    ("heal-units", absent::heal_units),
    // CAP-SHLD
    ("shield-absorbs", sim_checks::shield_absorbs),
    ("shield-bonus", absent::shield_bonus),
    ("armour-reduces", absent::armour_reduces),
    ("vital-fraction", absent::vital_fraction),
    // CAP-PLR
    ("player-ownership", sim_checks::player_ownership),
    ("player-resources", sim_checks::player_resources),
    ("alliance-filter", sim_checks::alliance_filter),
    // CAP-EVT
    ("region-define", mod_checks::region_define),
    ("trigger-zone", mod_checks::trigger_zone),
    ("event-delivery", absent::event_delivery),
    // CAP-TIME
    ("tick-rate", sim_checks::tick_rate),
    ("cooldown-expires", sim_checks::cooldown_expires),
    ("scheduled-action", mod_checks::scheduled_action),
    // CAP-LUA
    ("lua-load", mod_checks::lua_load),
    ("lua-reload", mod_checks::lua_reload),
    ("lua-state", mod_checks::lua_state),
    ("lua-error-location", mod_checks::lua_error_location),
    ("lua-host-api", mod_checks::lua_host_api),
    ("lua-host-api-full", mod_checks::lua_host_api_full),
    ("lua-sandbox-limits", mod_checks::lua_sandbox_limits),
    // CAP-SRV
    ("http-state", absent::http_state),
    ("http-catalog", absent::http_catalog),
    ("http-order", absent::http_order),
    ("http-delta", absent::http_delta),
    ("multi-room", absent::multi_room),
    ("http-resources", absent::http_resources),
    // CAP-DET
    ("fixed-math", sim_checks::fixed_math),
    ("state-hash-stable", sim_checks::state_hash_stable),
    ("replay-hash-equal", sim_checks::replay_hash_equal),
    // CAP-PERS
    ("map-image-read", sim_checks::map_image_read),
    ("map-image-roundtrip", sim_checks::map_image_roundtrip),
    ("save-load-state", sim_checks::save_load_state),
    // CAP-PRES
    ("presentation-refusal", absent::presentation_refusal),
];

pub fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("data")
}

pub fn missing(api: &str) -> Result<(), String> {
    Err(format!("missing API: {api}"))
}

/// A sim loaded from the generated image, with fog off so every unit is targetable.
pub fn fresh(image: &[u8]) -> Result<Sim, String> {
    let mut s = sim::load(image, MAX_UNITS).map_err(|e| format!("sim::load: {e:?}"))?;
    s.set_fog(false);
    Ok(s)
}

pub fn kind(s: &Sim, name: &str) -> Result<u32, String> {
    s.catalog()
        .unit_index(name)
        .map(|k| k as u32)
        .ok_or_else(|| format!("catalog has no unit {name}"))
}

/// Spawns unit `name` for `player` at whole world units (x, y).
pub fn spawn(s: &mut Sim, player: u8, name: &str, x: i32, y: i32) -> Result<usize, String> {
    let k = kind(s, name)?;
    s.spawn_for(player, k, fixed::from_int(x), fixed::from_int(y))
        .ok_or_else(|| format!("spawn_for refused {name} for player {player} at ({x},{y})"))
}

pub fn run_ticks(s: &mut Sim, n: u32) {
    for _ in 0..n {
        s.tick();
    }
}

/// Runs a plain sim from the image for `ticks` ticks and returns its state hash and unit count.
pub fn state_after(image: &[u8], ticks: u32) -> Result<(u64, usize), String> {
    let mut s = sim::load(image, MAX_UNITS).map_err(|e| format!("sim::load: {e:?}"))?;
    run_ticks(&mut s, ticks);
    Ok((s.state_hash(), s.count()))
}

fn hash_stable_two_runs(image: &[u8]) -> Result<(), String> {
    let (a, _) = state_after(image, RUN_TICKS)?;
    let (b, _) = state_after(image, RUN_TICKS)?;
    if a != b {
        return Err(format!("first={a:016x} second={b:016x}"));
    }
    Ok(())
}

fn units_ge_map_spawns(image: &[u8]) -> Result<(), String> {
    let text = std::fs::read_to_string(data_dir().join("map.star")).map_err(|e| format!("map.star: {e}"))?;
    let spawns = text.lines().filter(|l| l.trim_start().starts_with("spawn(")).count();
    let (_, units) = state_after(image, RUN_TICKS)?;
    if units == 0 || units < spawns {
        return Err(format!("units={units} spawns={spawns}"));
    }
    Ok(())
}

/// Runs every check, prints one line each, and returns how many failed.
pub fn run_all(image: &[u8]) -> usize {
    let mut failed = 0;
    for &(name, check) in CHECKS {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| check(image)))
            .unwrap_or_else(|_| Err("panicked, see stderr".to_string()));
        match outcome {
            Ok(()) => println!("check {name} PASS"),
            Err(reason) => {
                failed += 1;
                println!("check {name} FAIL {}", reason.replace('\n', " "));
            }
        }
    }
    failed
}

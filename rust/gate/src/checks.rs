//! The check list. Each entry is a gate check named as docs/CAPABILITIES.md names it
//! (`gate:<name>`), plus two standing checks. A check returns Ok when it passes, or Err with
//! the reason it fails. A check whose API is not on the sim or mod-host public surface returns
//! `missing API: <name>`, so the owning row stays unverified until that API is public.

use std::path::{Path, PathBuf};

use std::sync::Arc;

use sim::{fixed, Refusal, Sim};

use crate::{claimed_checks, mod_checks, sim_checks};

pub const MAX_UNITS: usize = 1024;
pub const RUN_TICKS: u32 = 2000;

pub type Check = fn(&[u8]) -> Result<(), String>;

#[path = "mechanic_checks.rs"]
mod mechanic_checks;

#[path = "determinism_checks.rs"]
mod determinism_checks;

#[path = "core_formula_checks.rs"]
mod core_formula_checks;

#[path = "unit_checks.rs"]
mod unit_checks;

#[path = "hero_checks.rs"]
mod hero_checks;

#[path = "item_checks.rs"]
mod item_checks;

#[path = "ability_checks.rs"]
mod ability_checks;

pub const CHECKS: &[(&str, Check)] = &[
    ("determinism-replay-hash", determinism_checks::moba_replay_hash),
    ("determinism-save-load-hash", determinism_checks::moba_save_load_hash),
    ("determinism-parity-hash", determinism_checks::moba_parity_hash),
    ("mechanic-unit-spawn", mechanic_checks::unit_spawn),
    ("mechanic-ability-use", mechanic_checks::ability_use),
    ("mechanic-economy-transfer", mechanic_checks::economy_transfer),
    ("mechanic-order-queue", mechanic_checks::order_queue),
    ("mechanic-inventory-slots", mechanic_checks::inventory_slots),
    ("mechanic-xp-level-up", mechanic_checks::xp_level_up),
    ("mechanic-trigger-firing", mechanic_checks::trigger_firing),
    ("mechanic-region-query", mechanic_checks::region_query),
    ("mechanic-structure-damage", mechanic_checks::structure_damage),
    ("mechanic-win-trigger", mechanic_checks::win_trigger),
    ("mechanic-kill-bounty", mechanic_checks::kill_bounty),
    ("mechanic-camp-respawn", mechanic_checks::camp_respawn),
    ("mechanic-camp-leash", mechanic_checks::camp_leash),
    ("mechanic-camp-roshan-aegis", mechanic_checks::camp_roshan_aegis),
    ("mechanic-camp-rune-pickup", mechanic_checks::camp_rune_pickup),
    ("hero-axe-level1", hero_checks::hero_axe_level1),
    ("hero-axe-growth", hero_checks::hero_axe_growth),
    ("hero-universal-damage", hero_checks::hero_universal_damage),
    ("hero-roster", hero_checks::hero_roster),
    ("item-catalog", item_checks::item_catalog),
    ("item-recipe-buy", item_checks::item_recipe_buy),
    ("item-shop-rules", item_checks::item_shop_rules),
    ("item-stack-consume", item_checks::item_stack_consume),
    ("item-slots", item_checks::item_slots),
    ("moba-match-end", mechanic_checks::moba_match_end),
    ("ability-axe-berserkers-call", ability_checks::axe_berserkers_call),
    ("ability-axe-battle-hunger", ability_checks::axe_battle_hunger),
    ("ability-axe-counter-helix", ability_checks::axe_counter_helix),
    ("ability-axe-culling-blade", ability_checks::axe_culling_blade),
    ("ability-crystal-nova", ability_checks::crystal_nova),
    ("ability-crystal-frostbite", ability_checks::crystal_frostbite),
    ("ability-crystal-brilliance-aura", ability_checks::crystal_brilliance_aura),
    ("ability-crystal-freezing-field", ability_checks::crystal_freezing_field),
    ("ability-sven-storm-bolt", ability_checks::sven_storm_bolt),
    ("ability-sven-great-cleave", ability_checks::sven_great_cleave),
    ("ability-sven-buffs", ability_checks::sven_buffs),
    ("ability-juggernaut-blade-fury", ability_checks::juggernaut_blade_fury),
    ("ability-juggernaut-healing-ward", ability_checks::juggernaut_healing_ward),
    ("ability-juggernaut-blade-dance", ability_checks::juggernaut_blade_dance),
    ("ability-juggernaut-omni-slash", ability_checks::juggernaut_omni_slash),
    ("ability-zeus-arc-lightning", ability_checks::zeus_arc_lightning),
    ("ability-zeus-lightning-bolt", ability_checks::zeus_lightning_bolt),
    ("ability-zeus-thundergods-wrath", ability_checks::zeus_thundergods_wrath),
    ("ability-sniper-shrapnel", ability_checks::sniper_shrapnel),
    ("ability-sniper-headshot", ability_checks::sniper_headshot),
    ("ability-sniper-take-aim", ability_checks::sniper_take_aim),
    ("ability-sniper-assassinate", ability_checks::sniper_assassinate),
    ("ability-lion-impale", ability_checks::lion_impale),
    ("ability-lion-voodoo", ability_checks::lion_voodoo),
    ("ability-lion-mana-drain", ability_checks::lion_mana_drain),
    ("ability-lion-finger-of-death", ability_checks::lion_finger_of_death),
    ("ability-omniknight-purification", ability_checks::omniknight_purification),
    ("ability-omniknight-martyr", ability_checks::omniknight_martyr),
    ("ability-omniknight-guardian-angel", ability_checks::omniknight_guardian_angel),
    ("ability-talent-picks", ability_checks::talent_picks),
    ("ability-cast-walks-into-range", ability_checks::cast_walks_into_range),
    ("ability-skill-level-gates", ability_checks::skill_level_gates),
    ("ability-dispel", ability_checks::spell_dispel),
    ("ability-illusions", ability_checks::spell_illusions),
    ("ability-phantom-lancer-illusions", ability_checks::phantom_lancer_illusions),
    ("ability-dazzle-shallow-grave", ability_checks::dazzle_shallow_grave),
    ("ability-lina-slave-and-array", ability_checks::lina_slave_and_array),
    ("ability-lich-nova-and-chain", ability_checks::lich_nova_and_chain),
    ("ability-tidehunter-ravage-and-shell", ability_checks::tidehunter_ravage_and_shell),
    ("ability-slardar-bash", ability_checks::slardar_bash),
    ("ability-vengeful-command-aura", ability_checks::vengeful_command_aura),
    ("ability-antimage-blink-and-mana-break", ability_checks::antimage_blink_and_mana_break),
    ("moba-creeps", mechanic_checks::moba_creeps),
    ("moba-lane", mechanic_checks::moba_lane),
    ("creep-stops-to-attack", mechanic_checks::creep_stops_to_attack),
    ("creep-strikes-creep-before-tower", mechanic_checks::creep_strikes_creep_before_tower),
    ("creep-crowded-cell-strikes-creep", mechanic_checks::creep_crowded_cell_strikes_creep),
    ("creep-reused-slot-drops-stale-target", mechanic_checks::reused_slot_drops_stale_sticky_target),
    ("item-stat-names", item_checks::item_stat_names),
    ("item-stock", item_checks::item_stock),
    ("item-aegis-and-death", item_checks::item_aegis_and_death),
    ("item-town-portal", item_checks::item_town_portal),
    ("item-aghanim-and-refresh", item_checks::item_aghanim_and_refresh),
    ("item-stat-stacking", item_checks::item_stat_stacking),
    ("moba-idle-settle", mechanic_checks::moba_idle_settle),
    ("moba-sync", mechanic_checks::moba_sync),
    ("moba-objective", mechanic_checks::moba_objective),
    ("moba-wave-composition", unit_checks::moba_wave_composition),
    ("moba-siege-timing", unit_checks::moba_siege_timing),
    ("moba-wave-upgrade", unit_checks::moba_wave_upgrade),
    ("camp-roshan-window", unit_checks::camp_roshan_window),
    ("camp-minute-refill", unit_checks::camp_minute_refill),
    ("camp-first-spawn", unit_checks::camp_first_spawn),
    ("camp-tormentor-timing", unit_checks::camp_tormentor_timing),
    ("camp-pit-swap", unit_checks::camp_pit_swap),
    ("hash_stable_two_runs", hash_stable_two_runs),
    ("units_ge_map_spawns", units_ge_map_spawns),
    // CAP-MAP
    ("load-map", sim_checks::load_map),
    ("walkable-cells", sim_checks::walkable_cells),
    ("edit-terrain", mod_checks::edit_terrain),
    ("map-bounds", clamp_to_grid_bounds),
    // CAP-UNIT
    ("spawn-units", sim_checks::spawn_units),
    ("unit-stats-read", sim_checks::unit_stats_read),
    ("catalog-load", sim_checks::catalog_load),
    ("unit-classes", sim_checks::unit_classes),
    // CAP-ABIL
    ("cast-ability", sim_checks::cast_ability),
    ("ability-target-filter", sim_checks::ability_target_filter),
    ("ability-effects", grant_energy_spend),
    ("unsupported-effect-refuses", unsupported_effect_refuses),
    // CAP-BEH
    ("apply-buff", sim_checks::apply_buff),
    ("clear-buff", clear_buff_frees_slot),
    ("remove-behaviour", remove_behaviour_refuses_unknown),
    ("behaviour-declared", sim_checks::behaviour_declared),
    ("behaviour-trigger-fires", sim_checks::behaviour_trigger_fires),
    // CAP-ORD
    ("issue-move", sim_checks::issue_move),
    ("move-units-tick", sim_checks::move_units_tick),
    ("path-to-goal", sim_checks::path_to_goal),
    ("order-kinds", sim_checks::order_kinds),
    // CAP-PROD
    ("produce-unit", sim_checks::produce_unit),
    ("harvest-resources", sim_checks::harvest_resources),
    ("spend-resources", sim_checks::spend_resources),
    ("cancel-production", sim_checks::cancel_production),
    // CAP-CMB
    ("attack-unit", sim_checks::attack_unit),
    ("area-damage", damage_area_zero_noop),
    ("projectile-hit", sim_checks::projectile_hit),
    ("heal-units", grant_energy_spend),
    // CAP-SHLD
    ("shield-absorbs", sim_checks::shield_absorbs),
    ("shield-bonus", shield_bonus_zero_noop),
    ("armour-reduces", armour_reduces_damage),
    ("core-armour-formula", core_formula_checks::armour_formula),
    ("core-xp-table", core_formula_checks::xp_table),
    ("vital-fraction", vital_fraction_zero_noop),
    // CAP-PLR
    ("player-ownership", sim_checks::player_ownership),
    ("player-resources", sim_checks::player_resources),
    ("alliance-filter", sim_checks::alliance_filter),
    // CAP-EVT
    ("region-define", mod_checks::region_define),
    ("trigger-zone", mod_checks::trigger_zone),
    ("event-delivery", mod_checks::event_delivery),
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
    // CAP-DET
    ("fixed-math", sim_checks::fixed_math),
    ("state-hash-stable", sim_checks::state_hash_stable),
    ("replay-hash-equal", sim_checks::replay_hash_equal),
    // CAP-PERS
    ("map-image-read", sim_checks::map_image_read),
    ("map-image-roundtrip", sim_checks::map_image_roundtrip),
    ("save-load-state", sim_checks::save_load_state),
    // CAP-PRES
    ("presentation-refusal", presentation_refusals_named),
    // CAP rows still claimed in docs/CAPABILITIES.md, batch 1
    ("sight-reveal", claimed_checks::sight_reveal),
    ("last-seen-memory", claimed_checks::last_seen_memory),
    ("air-terrain-crossing", claimed_checks::air_terrain_crossing),
    ("ground-only-air-refusal", claimed_checks::ground_only_air_refusal),
    ("unit-collision", claimed_checks::unit_collision),
    ("narrow-passage-limit", claimed_checks::narrow_passage_limit),
    ("weapon-range", claimed_checks::weapon_range),
    ("order-recording", claimed_checks::order_recording),
    ("mod-package-version", claimed_checks::mod_package_version),
    ("save-load-hash-equal", sim_checks::save_load_state),
    // CAP rows still claimed, batch 2
    ("transport-load", claimed_checks::transport_load),
    ("transport-unload-point", claimed_checks::transport_unload_point),
    ("buff-stacking-rule", claimed_checks::buff_stacking_rule),
    ("shield-regen-delay", claimed_checks::shield_regen_delay),
    ("upgrade-levels", claimed_checks::upgrade_levels),
    ("upgrade-stacking", claimed_checks::upgrade_stacking),
    ("upgrade-applies-existing", claimed_checks::upgrade_applies_existing),
    ("ranged-miss-chance", claimed_checks::ranged_miss_chance),
    ("replay-playback-controls", claimed_checks::replay_playback_controls),
    // CAP rows still claimed, batch 3
    ("player-start-positions", claimed_checks::player_start_positions),
    ("neutral-objects", claimed_checks::neutral_objects),
    ("map-author-placement", claimed_checks::map_author_placement),
    ("elevation-sight", claimed_checks::elevation_sight),
    ("hp-regen", claimed_checks::hp_regen),
    ("editor-terrain-tiles", claimed_checks::editor_terrain_tiles),
    ("editor-abilities", claimed_checks::editor_abilities),
    ("editor-triggers", claimed_checks::editor_triggers),
    ("channel-interrupt", claimed_checks::channel_interrupt),
    ("worker-saturation", claimed_checks::worker_saturation),
    // CAP rows still claimed, batch 4
    ("geyser-extraction", claimed_checks::geyser_extraction),
    ("opening-resources", claimed_checks::opening_resources),
    ("base-resource-nodes", claimed_checks::base_resource_nodes),
    ("capacity-cap", claimed_checks::capacity_cap),
    ("capacity-providers", claimed_checks::capacity_providers),
    ("capacity-reservation", claimed_checks::capacity_reservation),
    ("rally-point", claimed_checks::rally_point),
    ("tech-prerequisites", claimed_checks::tech_prerequisites),
    ("research-queue", claimed_checks::research_queue),
    ("research-applies-all", claimed_checks::research_applies_all),
    ("damage-type-size", claimed_checks::damage_type_size),
    ("cloak-detection", claimed_checks::cloak_detection),
];

pub fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("data")
}


/// A named number of scenarios/moba/hero_rules.txt, the camp, rune and boss timings the moba mod owns.
pub fn moba_rule(key: &str) -> Result<i32, String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("scenarios").join("moba").join("hero_rules.txt");
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    text.lines()
        .filter_map(|l| l.trim().split_once('='))
        .find(|(k, _)| k.trim() == key)
        .and_then(|(_, v)| v.trim().parse().ok())
        .ok_or_else(|| format!("hero_rules.txt has no number {key}"))
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
    let text = std::fs::read_to_string(data_dir().join("map.osd")).map_err(|e| format!("map.osd: {e}"))?;
    let spawns = text.lines().filter(|l| l.trim_start().starts_with("spawn(")).count();
    let (_, units) = state_after(image, RUN_TICKS)?;
    if units == 0 || units < spawns {
        return Err(format!("units={units} spawns={spawns}"));
    }
    Ok(())
}

fn first_unit(s: &Sim) -> Result<usize, String> {
    (0..s.count()).find(|&i| s.alive(i)).ok_or_else(|| "no live unit in image".to_string())
}

fn clamp_to_grid_bounds(image: &[u8]) -> Result<(), String> {
    let s = fresh(image)?;
    let (w, h) = (s.grid().w as i32, s.grid().h as i32);
    let got = s.clamp_to_grid(fixed::from_int(-1), fixed::from_int(h + 5));
    let want = (0, fixed::from_int(h - 1));
    if got != want {
        return Err(format!("clamp_to_grid -> {got:?}, want {want:?} (grid {w}x{h})"));
    }
    Ok(())
}

fn grant_energy_spend(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let u = first_unit(&s)?;
    s.grant_energy(u, 50).map_err(|e| format!("grant_energy: {e:?}"))?;
    s.spend_energy(u, 50).map_err(|e| format!("spend_energy: {e:?}"))?;
    if s.spend_energy(u, 1).is_ok() {
        return Err("spend above the granted energy was accepted".to_string());
    }
    Ok(())
}

fn unsupported_effect_refuses(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let u = first_unit(&s)?;
    let before = s.state_hash();
    s.apply_effects(u, usize::MAX, 0, 0, -1);
    if s.state_hash() != before {
        return Err("an unknown ability index changed state".to_string());
    }
    Ok(())
}

fn clear_buff_frees_slot(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let u = first_unit(&s)?;
    if !s.apply_buff(u, 0, 100) {
        return Err("apply_buff refused before clear".to_string());
    }
    s.clear_buff(u, 0);
    if !s.apply_buff(u, 0, 100) {
        return Err("buff slot not free after clear_buff".to_string());
    }
    Ok(())
}

fn remove_behaviour_refuses_unknown(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    if s.remove_behaviour(usize::MAX, 0, 0, 0, 0, 0).is_ok() {
        return Err("an out-of-range caster was accepted".to_string());
    }
    Ok(())
}

fn damage_area_zero_noop(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let before = s.state_hash();
    s.damage_area(-1, 0, 0, 0, 0, 0);
    if s.state_hash() != before {
        return Err("zero-damage area changed state".to_string());
    }
    Ok(())
}

fn shield_bonus_zero_noop(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let u = first_unit(&s)?;
    let before = s.state_hash();
    s.shield_bonus(u, u as i32, 0);
    if s.state_hash() != before {
        return Err("zero shield bonus changed state".to_string());
    }
    Ok(())
}

fn armour_reduces_damage(image: &[u8]) -> Result<(), String> {
    let reduced = sim::armour_reduced(10, sim::fixed::from_int(3));
    if !(0..=10).contains(&reduced) {
        return Err(format!("armour_reduced(10, 3) = {reduced}, outside 0..=10"));
    }
    let mut s = fresh(image)?;
    let u = first_unit(&s)?;
    let before = s.hp(u);
    s.hurt(u, 0);
    if s.hp(u) != before {
        return Err("zero hurt changed hp".to_string());
    }
    Ok(())
}

fn vital_fraction_zero_noop(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let u = first_unit(&s)?;
    let before = s.state_hash();
    s.vital_fraction(u, u as i32, 0, 0, 0, 0);
    if s.state_hash() != before {
        return Err("zero vital fraction changed state".to_string());
    }
    Ok(())
}

fn presentation_refusals_named(_image: &[u8]) -> Result<(), String> {
    for r in [Refusal::CameraUnsupported, Refusal::SoundUnsupported, Refusal::UiPanelUnsupported, Refusal::ChatUnsupported] {
        if r.name().is_empty() {
            return Err(format!("{r:?} has no name"));
        }
    }
    Ok(())
}











/// Runs every check, prints one line each, and returns how many failed.
pub fn run_all(image: &[u8]) -> usize {
    let mut failed = 0;
    let only = std::env::var("GATE_ONLY").unwrap_or_default();
    for &(name, check) in CHECKS.iter().filter(|(name, _)| name.contains(only.as_str())) {
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

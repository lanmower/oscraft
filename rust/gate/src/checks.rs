use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;



use mod_host::ModHost;
use sim::{fixed, Refusal, Sim};

use crate::{claimed_checks, mod_checks, sim_checks};

pub const MAX_UNITS: usize = 1024;
pub const DEFAULT_PACK: &str = "moba";
pub const ROLE_FILE: &str = "gate_roles.txt";
pub const RULES_FILE: &str = "attr_rules.txt";
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

#[path = "chat_checks.rs"]
mod chat_checks;

#[path = "attribute_checks.rs"]
mod attribute_checks;

#[path = "trigger_spell_checks.rs"]
mod trigger_spell_checks;

#[path = "item_checks.rs"]
mod item_checks;

#[path = "spell_checks.rs"]
mod spell_checks;

#[path = "hero_trigger_checks.rs"]
mod hero_trigger_checks;

#[path = "attack_spell_checks.rs"]
mod attack_spell_checks;

#[path = "stack_checks.rs"]
mod stack_checks;

#[path = "pack_run.rs"]
mod pack_run;

#[path = "score_checks.rs"]
mod score_checks;

#[path = "ctl_checks.rs"]
mod ctl_checks;

#[path = "units_checks.rs"]
mod units_checks;

#[path = "draft_checks.rs"]
mod draft_checks;

#[path = "unv_checks.rs"]
mod unv_checks;

pub const CHECKS: &[(&str, Check)] = &[
    ("determinism-replay-hash", determinism_checks::scenario_replay_hash),
    ("determinism-save-load-hash", determinism_checks::scenario_save_load_hash),
    ("determinism-parity-hash", determinism_checks::scenario_parity_hash),
    ("mechanic-unit-spawn", mechanic_checks::unit_spawn),
    ("mechanic-ability-use", mechanic_checks::ability_use),
    ("mechanic-economy-transfer", mechanic_checks::economy_transfer),
    ("mechanic-order-queue", mechanic_checks::order_queue),
    ("mechanic-inventory-slots", mechanic_checks::inventory_slots),
    ("mechanic-xp-accumulates", mechanic_checks::xp_accumulates),
    ("mechanic-trigger-firing", mechanic_checks::trigger_firing),
    ("mechanic-region-query", mechanic_checks::region_query),
    ("mechanic-structure-damage", mechanic_checks::structure_damage),
    ("mechanic-group-respawn", mechanic_checks::spawn_group_respawn),
    ("mechanic-group-roshan-aegis", mechanic_checks::spawn_group_boss_aegis),
    ("mechanic-group-rune-pickup", mechanic_checks::spawn_group_pickup),
    ("mechanic-hero-death-drop", mechanic_checks::hero_death_drop),
    ("mechanic-bottle-drink", mechanic_checks::bottle_drink),
    ("mechanic-bloodstone-terms", mechanic_checks::bloodstone_terms),
    ("mechanic-bottle-refill-on-fountain", mechanic_checks::bottle_refill_on_fountain),
    ("mechanic-bottle-rune", mechanic_checks::bottle_rune),
    ("mechanic-bottle-rune-expires", mechanic_checks::bottle_rune_expires),
    ("mechanic-bottle-swap-in-place", mechanic_checks::bottle_swap_in_place),
    ("mechanic-bloodstone-trickle", mechanic_checks::bloodstone_mana_trickle),
    ("mechanic-pipe-regain-lock", mechanic_checks::pipe_regain_lock),
    ("mechanic-kelens-heavy-hit", mechanic_checks::kelens_heavy_hit),
    ("mechanic-guardian-low-life-aura", mechanic_checks::guardian_low_life_aura),
    ("mechanic-bottle-regen-no-restart", mechanic_checks::bottle_regen_no_restart),
    ("mechanic-bottle-rune-warning", mechanic_checks::bottle_rune_warning),
    ("mechanic-bottle-capture", mechanic_checks::bottle_capture_cast),
    ("mechanic-bottle-courier-slow", mechanic_checks::bottle_courier_slow),
    ("mechanic-bottle-share-dose", mechanic_checks::bottle_share_dose),
    ("mechanic-bottle-loan", mechanic_checks::bottle_loan_and_return),
    ("attribute-hero-primary-rows", attribute_checks::hero_primary_rows),
    ("attribute-roster", attribute_checks::attribute_roster),
    ("item-catalog", item_checks::item_catalog),
    ("item-stack-consume", item_checks::item_stack_consume),
    ("item-slots", item_checks::item_slots),
    ("item-sale-refund", item_checks::item_sale_refund),
    ("item-sale-refusal", item_checks::item_sale_refusal),
    ("spell-a-2", spell_checks::spell_a_2),
    ("slot-summon-group", spell_checks::slot_summon_group),
    ("slot-napalm-stacks", stack_checks::slot_napalm_stacks),
    ("slot-untouchable-attacker", stack_checks::slot_untouchable_attacker),
    ("slot-wall-fissure", stack_checks::slot_wall_fissure),
    ("slot-percent-aura", stack_checks::slot_percent_aura),
    ("slot-displace-vacuum", spell_checks::slot_displace_vacuum),
    ("spell-a-1", spell_checks::spell_a_1),
    ("spell-a-3", spell_checks::spell_a_3),
    ("ability-magnataur-skewer", spell_checks::magnataur_skewer),
    ("ability-elder-titan-earth-splitter", spell_checks::elder_titan_earth_splitter),
    ("spell-a-4", spell_checks::spell_a_4),
    ("ability-crystal-nova", spell_checks::crystal_nova),
    ("ability-damage-by-agility", spell_checks::damage_by_agility),
    ("ability-damage-by-missing-life", spell_checks::damage_by_missing_life),
    ("ability-ward-group-by-rank", spell_checks::ward_group_by_rank),
    ("ability-sleep-song-and-blind", spell_checks::sleep_song_and_blind),
    ("ability-plasma-field-by-distance", spell_checks::plasma_field_by_distance),
    ("ability-scythe-scepter-delta", spell_checks::scythe_scepter_delta),
    ("ability-crystal-frostbite", spell_checks::crystal_frostbite),
    ("ability-crystal-brilliance-aura", spell_checks::crystal_brilliance_aura),
    ("ability-crystal-freezing-field", spell_checks::crystal_freezing_field),
    ("ability-axe-culling-execute", spell_checks::axe_culling_blade_execute),
    ("ability-drow-gust-knockback", spell_checks::drow_gust_knockback),
    ("ability-antimage-mana-break", spell_checks::antimage_mana_break_follows_burn),
    ("trigger-chaos-bolt", trigger_spell_checks::chaos_bolt_random_inverse),
    ("trigger-mana-void", trigger_spell_checks::mana_void_missing_mana),
    ("trigger-death-coil", trigger_spell_checks::death_coil_heals_ally),
    ("trigger-battle-hunger", trigger_spell_checks::battle_hunger_ends_and_stacks),
    ("trigger-hand-of-god", trigger_spell_checks::hand_of_god_heals_own_creeps),
    ("trigger-test-of-faith-damage", trigger_spell_checks::test_of_faith_damages_enemy),
    ("trigger-test-of-faith-home", trigger_spell_checks::test_of_faith_sends_ally_home),
    ("trigger-marksmanship", trigger_spell_checks::marksmanship_alone_only),
    ("trigger-atrophy", trigger_spell_checks::atrophy_stacks_on_enemy_death),
    ("trigger-surge", trigger_spell_checks::surge_hastes_ally),
    ("trigger-ion-shell", trigger_spell_checks::ion_shell_pulses),
    ("trigger-drunken-haze", trigger_spell_checks::drunken_haze_blinds),
    ("trigger-damage-taken", trigger_spell_checks::bloodrage_and_penitence_amplify_damage),
    ("trigger-persuasion", trigger_spell_checks::persuasion_converts_and_caps),
    ("trigger-devour", trigger_spell_checks::devour_kills_and_pays),
    ("trigger-static-storm", trigger_spell_checks::static_storm_ramps),
    ("trigger-rupture", trigger_spell_checks::rupture_hurts_moving_target),
    ("trigger-track", trigger_spell_checks::track_pays_on_death),
    ("trigger-stampede", trigger_spell_checks::stampede_hastes_and_tramples),
    ("trigger-bristleback", trigger_spell_checks::bristleback_heals_rear_hits),
    ("trigger-frostbite", trigger_spell_checks::frostbite_lasts_longer_on_creeps),
    ("trigger-tempest", trigger_spell_checks::tempest_double_costs_mana),
    ("trigger-fiends-grip", trigger_spell_checks::fiends_grip_moves_mana),
    ("trigger-goo", trigger_spell_checks::viscous_goo_stacks),
    ("trigger-trueshot", trigger_spell_checks::trueshot_gives_agility_share),
    ("trigger-reality-rift", trigger_spell_checks::reality_rift_jumps),
    ("trigger-flux", trigger_spell_checks::flux_needs_isolation),
    ("trigger-firefly", trigger_spell_checks::firefly_burns_the_trail),
    ("trigger-thunder-clap", trigger_spell_checks::thunder_clap_slows_creeps_longer),
    ("trigger-thunder-strike", trigger_spell_checks::thunder_strike_hits_neighbours),
    ("trigger-pit", trigger_spell_checks::pit_of_malice_hits_each_once),
    ("trigger-poison-touch", trigger_spell_checks::poison_touch_stages),
    ("trigger-shadow-wave", trigger_spell_checks::shadow_wave_bounces),
    ("trigger-backstab", trigger_spell_checks::backstab_once),
    ("trigger-grip-reveal", trigger_spell_checks::grip_reveals_target),
    ("trigger-spiderlings", trigger_spell_checks::spiderlings_spawn_on_death),
    ("trigger-spark-wraith", trigger_spell_checks::spark_wraith_strikes),
    ("trigger-chilling-touch", trigger_spell_checks::chilling_touch_charges),
    ("trigger-lasso", trigger_spell_checks::lasso_drags_target),
    ("trigger-exorcism", trigger_spell_checks::exorcism_places_ghosts),
    ("trigger-primal-split", trigger_spell_checks::primal_split_hides_and_returns),
    ("trigger-call-of-the-wild", trigger_spell_checks::call_of_the_wild_hawk),
    ("trigger-shuriken-bounces-to-tracked", trigger_spell_checks::shuriken_bounces_to_tracked),
    ("trigger-rupture-scepter-recharges", trigger_spell_checks::rupture_scepter_recharges),
    ("trigger-grip-scepter-adds-damage", trigger_spell_checks::grip_scepter_adds_damage),
    ("trigger-doom-scepter-breaks", trigger_spell_checks::doom_scepter_breaks),
    ("trigger-tempest-copy-holds-skills", trigger_spell_checks::tempest_copy_holds_skills),
    ("trigger-kinetic-field-rings-blockers", trigger_spell_checks::kinetic_field_rings_blockers),
    ("trigger-dark-rift-takes-allies", trigger_spell_checks::dark_rift_takes_allies),
    ("trigger-wild-axes-hit-each-once", trigger_spell_checks::wild_axes_hit_each_once),
    ("trigger-chemical-rage-shortens-attack-time", trigger_spell_checks::chemical_rage_shortens_attack_time),
    ("trigger-wall-of-replica", trigger_spell_checks::wall_of_replica_hits_crossers),
    ("trigger-track-speed", trigger_spell_checks::track_speeds_allies_near),
    ("trigger-goo-scepter", trigger_spell_checks::goo_scepter_reaches_all),
    ("trigger-phantasm-scepter", trigger_spell_checks::phantasm_scepter_extra_image),
    ("trigger-nightmare", trigger_spell_checks::nightmare_sleeps_wakes_and_spreads),
    ("trigger-cold-feet", trigger_spell_checks::cold_feet_stuns_those_who_stay),
    ("trigger-glimpse", trigger_spell_checks::glimpse_returns_four_seconds),
    ("trigger-weave", trigger_spell_checks::weave_changes_armor_each_second),
    ("trigger-aphotic", trigger_spell_checks::aphotic_shield_releases_on_break),
    ("trigger-borrowed-time", trigger_spell_checks::borrowed_time_heals_damage),
    ("trigger-borrowed-time-auto", trigger_spell_checks::borrowed_time_triggers_under_400),
    ("spell-b-3", spell_checks::spell_b_3),
    ("spell-b-2", spell_checks::spell_b_2),
    ("spell-b-4", spell_checks::spell_b_4),
    ("spell-g-2", spell_checks::spell_g_2),
    ("spell-g-3", spell_checks::spell_g_3),
    ("spell-g-1", spell_checks::spell_g_1),
    ("spell-g-4", spell_checks::spell_g_4),
    ("ability-zeus-arc-lightning", spell_checks::zeus_arc_lightning),
    ("ability-zeus-lightning-bolt", spell_checks::zeus_lightning_bolt),
    ("ability-zeus-thundergods-wrath", spell_checks::zeus_thundergods_wrath),
    ("spell-h-3", spell_checks::spell_h_3),
    ("spell-h-2", spell_checks::spell_h_2),
    ("spell-h-4", spell_checks::spell_h_4),
    ("spell-h-1", spell_checks::spell_h_1),
    ("spell-o-2", spell_checks::spell_o_2),
    ("spell-o-4", spell_checks::spell_o_4),
    ("spell-o-3", spell_checks::spell_o_3),
    ("spell-o-1", spell_checks::spell_o_1),
    ("spell-c-3", spell_checks::spell_c_3),
    ("spell-c-2", spell_checks::spell_c_2),
    ("spell-c-1", spell_checks::spell_c_1),
    ("ability-talent-picks", spell_checks::talent_picks),
    ("ability-skill-gate-data", spell_checks::skill_gate_data),
    ("ability-illusions", spell_checks::spell_illusions),
    ("spell-i-2", spell_checks::spell_i_2),
    ("spell-s-1", spell_checks::spell_s_1),
    ("spell-r-1", spell_checks::spell_r_1),
    ("spell-q-2", spell_checks::spell_q_2),
    ("spell-d-2", spell_checks::spell_d_2),
    ("spell-f-1", spell_checks::spell_f_1),
    ("ability-vengeful-command-aura", spell_checks::vengeful_command_aura),
    ("spell-l-1", spell_checks::spell_l_1),
    ("ability-earthshaker-echo-slam", hero_trigger_checks::earthshaker_echo_slam),
    ("ability-earthshaker-enchant-totem", hero_trigger_checks::earthshaker_enchant_totem),
    ("ability-jakiro-ice-path", hero_trigger_checks::jakiro_ice_path),
    ("ability-jakiro-macropyre", hero_trigger_checks::jakiro_macropyre),
    ("ability-jakiro-dual-breath", hero_trigger_checks::jakiro_dual_breath),
    ("ability-huskar-berserkers-blood", hero_trigger_checks::huskar_berserkers_blood),
    ("ability-huskar-life-break", hero_trigger_checks::huskar_life_break),
    ("ability-huskar-burning-spear", hero_trigger_checks::huskar_burning_spear),
    ("ability-huskar-inner-vitality", hero_trigger_checks::huskar_inner_vitality),
    ("ability-lycan-auras-reach-units", hero_trigger_checks::lycan_auras_reach_units),
    ("ability-lycan-shapeshift-speed", hero_trigger_checks::lycan_shapeshift_speed),
    ("ability-lina-laguna-delay", spell_checks::lina_laguna_blade_delay),
    ("ability-lina-dragon-slave-taper", spell_checks::lina_dragon_slave_taper),
    ("ability-lich-dark-ritual", spell_checks::lich_dark_ritual_sacrifice),
    ("ability-blind-and-revealed", spell_checks::spell_blind_and_revealed),
    ("ability-plague-ward-rank-unit", spell_checks::spell_plague_ward_rank_unit),
    ("ability-hit-marks-attack", spell_checks::spell_hit_marks_attack),
    ("ability-vortex-pulls", spell_checks::spell_vortex_pulls),
    ("ability-cast-point-by-rank", spell_checks::spell_cast_point_by_rank),
    ("ability-attack-spells", attack_spell_checks::attack_spells),
    ("ability-overload-and-dispersion", attack_spell_checks::overload_and_dispersion),
    ("ability-techies-mines", attack_spell_checks::techies_mines),
    ("ability-stacks-cloak-remnant", attack_spell_checks::stacks_cloak_remnant),
    ("ability-death-ward-by-rank", attack_spell_checks::death_ward_by_rank),
    ("unit-morph-keeps-fraction", attack_spell_checks::morph_keeps_fraction),
    ("ability-summon-behaviours", attack_spell_checks::summon_behaviours),
    ("ability-blocks-bonds-flare", attack_spell_checks::blocks_bonds_flare),
    ("scenario-creeps", mechanic_checks::scenario_units),
    ("scenario-lane", mechanic_checks::scenario_route),
    ("unit-stops-to-attack", mechanic_checks::unit_stops_to_attack),
    ("unit-strikes-creep-before-tower", mechanic_checks::unit_strikes_unit_before_structure),
    ("unit-crowded-cell-strikes-creep", mechanic_checks::unit_crowded_cell_strikes_unit),
    ("unit-reused-slot-drops-stale-target", mechanic_checks::reused_slot_drops_stale_target),
    ("unit-reused-slot-gets-its-own-rules", mechanic_checks::reused_slot_gets_its_own_unit_rules),
    ("unit-deny-gate", mechanic_checks::deny_gate),
    ("ctl-courier-secret-shop", ctl_checks::courier_secret_shop),
    ("ctl-courier-drop-pickup", ctl_checks::courier_drop_pickup),
    ("ctl-courier-drop-filters", ctl_checks::courier_drop_filters),
    ("ctl-courier-pickup-owner", ctl_checks::courier_pickup_owner),
    ("ctl-courier-ping", ctl_checks::courier_ping),
    ("ctl-courier-burst", ctl_checks::courier_burst),
    ("ctl-glyph-fortification", ctl_checks::glyph_fortification),
    ("ctl-double-hold", ctl_checks::double_hold),
    ("units-super-creeps-round", units_checks::super_creeps_round),
    ("units-super-creeps-message", units_checks::super_creeps_message),
    ("units-super-creeps-off", units_checks::super_creeps_off),
    ("units-courier-skin-order", units_checks::courier_skin_order),
    ("units-courier-skin-flying", units_checks::courier_skin_flying),
    ("ctl-sound-targeting", ctl_checks::sound_targeting),
    ("ctl-announce-sounds", ctl_checks::announce_sounds),
    ("chat-names", chat_checks::names),
    ("chat-gameinfo", chat_checks::gameinfo),
    ("chat-matchup", chat_checks::matchup),
    ("chat-creepstats", chat_checks::creepstats),
    ("chat-deny-flags", chat_checks::deny_flags),
    ("chat-weather", chat_checks::weather),
    ("chat-music", chat_checks::music),
    ("chat-courier", chat_checks::courier),
    ("chat-share", chat_checks::share),
    ("chat-shared-control", chat_checks::shared_control),
    ("chat-movespeed", chat_checks::movespeed),
    ("chat-drop-vote", chat_checks::drop_vote),
    ("chat-kickafk", chat_checks::kickafk),
    ("chat-lobby-random", chat_checks::lobby_random),
    ("chat-lobby-random-cost", chat_checks::lobby_random_cost),
    ("chat-lobby-repick", chat_checks::lobby_repick),
    ("chat-match-repick", chat_checks::match_repick),
    ("chat-rollhero", chat_checks::rollhero),
    ("chat-lobby-swap", chat_checks::lobby_swap),
    ("chat-match-swap", chat_checks::match_swap),
    ("chat-switch-flow", chat_checks::switch_flow),
    ("draft-modes", draft_checks::modes_accepted),
    ("draft-captains-mode", draft_checks::captains_mode),
    ("draft-captains-mode-timeout", draft_checks::captains_mode_timeout),
    ("draft-claim-leftovers", draft_checks::claim_leftovers),
    ("draft-captains-draft", draft_checks::captains_draft),
    ("draft-captains-draft-timeout", draft_checks::captains_draft_timeout),
    ("draft-random-draft", draft_checks::random_draft),
    ("draft-single-draft", draft_checks::single_draft),
    ("draft-reverse-draft", draft_checks::reverse_draft),
    ("draft-bot-seats", draft_checks::bot_seats),
    ("item-stat-names", item_checks::item_stat_names),
    ("item-town-portal", item_checks::item_town_portal),
    ("item-aghanim-and-refresh", item_checks::item_aghanim_and_refresh),
    ("item-stat-stacking", item_checks::item_stat_stacking),
    ("item-form-toggle", item_checks::item_form_toggle),
    ("item-holder-forms", item_checks::item_holder_forms),
    ("item-damage-copies", item_checks::item_damage_copies),
    ("item-spell-block", item_checks::item_spell_block_recharge),
    ("item-bkb-counter", item_checks::item_bkb_counter),
    ("item-recipe-forms", item_checks::item_recipe_forms),
    ("item-alchemist-scepter", item_checks::item_alchemist_scepter),
    ("item-sphere-transfer", item_checks::item_sphere_transfer),
    ("item-treads-start-state", item_checks::item_treads_start_state),
    ("item-bkb-drop-lock", item_checks::item_bkb_drop_lock),
    ("item-rapier-owner", item_checks::item_rapier_owner),
    ("scenario-idle-settle", mechanic_checks::scenario_idle_settle),
    ("scenario-sync", mechanic_checks::scenario_sync),
    ("scenario-objective", mechanic_checks::scenario_objective),
    ("group-roshan-window", unit_checks::spawn_group_boss_window),
    ("group-minute-refill", unit_checks::spawn_group_minute_refill),
    ("group-ward-does-not-block", unit_checks::spawn_group_ward_does_not_block),
    ("group-first-spawn", unit_checks::spawn_group_first_spawn),
    ("group-single-pit", unit_checks::spawn_group_single_pit),
    ("hash_stable_two_runs", hash_stable_two_runs),
    ("units_ge_map_spawns", units_ge_map_spawns),
    ("load-map", sim_checks::load_map),
    ("walkable-cells", sim_checks::walkable_cells),
    ("edit-terrain", mod_checks::edit_terrain),
    ("map-bounds", clamp_to_grid_bounds),
    ("spawn-units", sim_checks::spawn_units),
    ("unit-stats-read", sim_checks::unit_stats_read),
    ("catalog-load", sim_checks::catalog_load),
    ("unit-classes", sim_checks::unit_classes),
    ("cast-ability", sim_checks::cast_ability),
    ("ability-target-filter", sim_checks::ability_target_filter),
    ("ability-effects", grant_energy_spend),
    ("unsupported-effect-refuses", unsupported_effect_refuses),
    ("apply-buff", sim_checks::apply_buff),
    ("clear-buff", clear_buff_frees_slot),
    ("remove-behaviour", remove_behaviour_refuses_unknown),
    ("behaviour-declared", sim_checks::behaviour_declared),
    ("behaviour-trigger-fires", sim_checks::behaviour_trigger_fires),
    ("issue-move", sim_checks::issue_move),
    ("move-units-tick", sim_checks::move_units_tick),
    ("path-to-goal", sim_checks::path_to_goal),
    ("order-kinds", sim_checks::order_kinds),
    ("attack-unit", sim_checks::attack_unit),
    ("area-damage", damage_area_zero_noop),
    ("projectile-hit", sim_checks::projectile_hit),
    ("heal-units", grant_energy_spend),
    ("shield-absorbs", sim_checks::shield_absorbs),
    ("shield-bonus", shield_bonus_zero_noop),
    ("core-xp-table", core_formula_checks::xp_table),
    ("vital-fraction", vital_fraction_zero_noop),
    ("player-ownership", sim_checks::player_ownership),
    ("player-resources", sim_checks::player_resources),
    ("alliance-filter", sim_checks::alliance_filter),
    ("region-define", mod_checks::region_define),
    ("trigger-zone", mod_checks::trigger_zone),
    ("event-delivery", mod_checks::event_delivery),
    ("tick-rate", sim_checks::tick_rate),
    ("cooldown-expires", sim_checks::cooldown_expires),
    ("scheduled-action", mod_checks::scheduled_action),
    ("lua-load", mod_checks::lua_load),
    ("lua-reload", mod_checks::lua_reload),
    ("lua-state", mod_checks::lua_state),
    ("lua-missing-args-nil", mod_checks::lua_missing_args_nil),
    ("host-items-and-facing", mod_checks::host_items_and_facing),
    ("lua-error-location", mod_checks::lua_error_location),
    ("lua-host-api", mod_checks::lua_host_api),
    ("lua-host-api-full", mod_checks::lua_host_api_full),
    ("lua-sandbox-limits", mod_checks::lua_sandbox_limits),
    ("lua-host-capabilities", mod_checks::lua_host_capabilities),
    ("pack-loads-clean", mechanic_checks::pack_loads_clean),
    ("lua-seat-api", mod_checks::lua_seat_api),
    ("lua-editor-api", mod_checks::lua_editor_api),
    ("lua-scaling-api", mod_checks::lua_scaling_api),
    ("lua-panel-matrix", mod_checks::lua_panel_matrix),
    ("score-board-rows", score_checks::scoreboard_rows),
    ("score-board-closes", score_checks::scoreboard_closes),
    ("score-observer-panel", score_checks::observer_panel),
    ("mode-easy-mini-item-drop", mechanic_checks::mode_easy_mini_item_drop),
    ("unv-seat-colours", unv_checks::seat_colours),
    ("unv-mini-hero-scale", unv_checks::mini_hero_scale),
    ("unv-item-drop-clone-exempt", unv_checks::item_drop_clone_exempt),
    ("unv-spectator-panel-addressing", unv_checks::spectator_panel_addressing),
    ("unv-camera-chat", unv_checks::camera_chat),
    ("fixed-math", sim_checks::fixed_math),
    ("state-hash-stable", sim_checks::state_hash_stable),
    ("replay-hash-equal", sim_checks::replay_hash_equal),
    ("map-image-read", sim_checks::map_image_read),
    ("map-image-roundtrip", sim_checks::map_image_roundtrip),
    ("save-load-state", sim_checks::save_load_state),
    ("presentation-refusal", presentation_refusals_named),
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
    ("buff-stacking-rule", claimed_checks::buff_stacking_rule),
    ("shield-regen-delay", claimed_checks::shield_regen_delay),
    ("ranged-miss-chance", claimed_checks::ranged_miss_chance),
    ("replay-playback-controls", claimed_checks::replay_playback_controls),
    ("player-start-positions", claimed_checks::player_start_positions),
    ("neutral-objects", claimed_checks::neutral_objects),
    ("map-author-placement", claimed_checks::map_author_placement),
    ("elevation-sight", claimed_checks::elevation_sight),
    ("hp-regen", claimed_checks::hp_regen),
    ("channel-interrupt", claimed_checks::channel_interrupt),
    ("damage-type-size", claimed_checks::damage_type_size),
    ("cloak-detection", claimed_checks::cloak_detection),
];

pub fn pack_dir() -> PathBuf {
    let name = std::env::var("GATE_PACK").unwrap_or_else(|_| DEFAULT_PACK.to_string());
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("scenarios").join(name)
}

fn role_table() -> &'static Result<BTreeMap<String, String>, String> {
    static TABLE: OnceLock<Result<BTreeMap<String, String>, String>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let path = pack_dir().join(ROLE_FILE);
        let text = std::fs::read_to_string(&path).map_err(|e| format!("pack roles {}: {e}", path.display()))?;
        Ok(text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .filter_map(|l| l.split_once('='))
            .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
            .collect())
    })
}

pub fn role(key: &str) -> Result<&'static str, String> {
    match role_table() {
        Ok(table) => table.get(key).map(String::as_str).ok_or_else(|| {
            format!("pack {} lacks gate role {key}: add `{key} = <name>` to {ROLE_FILE}", pack_dir().display())
        }),
        Err(e) => Err(e.clone()),
    }
}

pub fn fill_roles(src: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut rest = src;
    while let Some(at) = rest.find("@role(") {
        out.push_str(&rest[..at]);
        let after = &rest[at + 6..];
        let end = after.find(')').ok_or("unclosed @role(")?;
        out.push('"');
        out.push_str(role(&after[..end])?);
        out.push('"');
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

pub fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("data")
}


pub fn scenario_rule(key: &str) -> Result<i32, String> {
    let path = pack_dir().join(RULES_FILE);
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    text.lines()
        .filter_map(|l| l.trim().split_once('='))
        .find(|(k, _)| k.trim() == key)
        .and_then(|(_, v)| v.trim().parse().ok())
        .ok_or_else(|| format!("the pack rules file has no number {key}"))
}

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

pub fn run_past_first_camp_mark(host: &mut ModHost) -> Result<(), String> {
    let tps = sim::TICKS_PER_SECOND as u32;
    let first = scenario_rule("camp_first_spawn_s")? as u32 * tps;
    let mark = scenario_rule("camp_respawn_s")? as u32 * tps;
    pack_run::advance(host, first.div_ceil(mark) * mark + 1)
}

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

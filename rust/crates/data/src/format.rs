//! Layout constants and registries shared with the sim loader. See FORMAT.md.

pub const MAGIC: [u8; 4] = *b"OSDT";
pub const VERSION: u16 = 21;
pub const PLCE_FLAG_START: u32 = 1;
/// A start record that names its team: the team index sits in bits 8..16 of the flags.
pub const PLCE_FLAG_TEAM: u32 = 2;
pub const PLCE_TEAM_SHIFT: u32 = 8;
pub const PLCE_TEAM_MASK: u32 = 0xFF << PLCE_TEAM_SHIFT;
pub const NEUTRAL_PLAYER: u16 = u16::MAX;
pub const HEADER_LEN: usize = 16;
pub const SECTION_ENTRY_LEN: usize = 16;
pub const NO_ABILITY: u16 = u16::MAX;
pub const NO_UNIT: u16 = u16::MAX;
pub const MAX_GRID_CELLS: usize = 1 << 24;
pub const MAX_PLAYERS: i32 = 16;

pub const EFFECT_KINDS: &[&str] = &[
    "damage",
    "heal",
    "move",
    "spawn",
    "buff",
    "issue_order",
    "remove_behaviour",
    "create_healer",
    "use_calldown",
    "vital_fraction",
    "shield_bonus",
    "stun",
    "slow",
    "silence",
    "root",
    "invisibility",
    "stat",
    "ward",
    "glyph",
    "deliver",
    "dot",
    "status",
    "timed_stat",
    "aoe",
    "dispel",
    "chain",
    "zone",
    "illusion",
    "mana",
    "if_kill",
    "attack",
    "teleport",
    "refresh",
    "grant_upgrade",
];
pub const MAX_EFFECT_KIND: usize = 33;
/// The stats an effect of kind "stat" may raise, by the code in its first parameter.
pub const STAT_NAMES: &[&str] = &[
    "damage", "armor", "health", "speed", "range", "evasion", "crit_chance", "crit_mult", "cleave", "cleave_radius",
    "lifesteal", "magic_resist", "attack_speed", "mana_regen", "health_regen", "max_mana", "spell_amp", "bash_chance",
    "bash_ticks", "speed_pct", "cooldown_reduction", "status_resist", "strength", "agility", "intelligence", "spell_lifesteal",
    "armor_x100", "cast_range", "cleave_start_width", "cleave_end_width", "damage_reduction",
];
/// Status codes of the `status` effect (and the sim's status counters): the first five have their own effect kinds.
pub const STATUS_CODES: &[&str] = &[
    "stun", "slow", "silence", "root", "invisibility", "taunt", "hex", "disarm", "spell_immunity", "invulnerable", "break", "mute",
    "physical_immunity", "death_prevention",
];
/// Damage types of the `damage`, `dot` and `chain` effects: 0 is the legacy armour-free hit.
pub const SPELL_DAMAGE_TYPES: &[&str] = &["legacy", "physical", "magical", "pure"];
pub const ORDER_KINDS: &[&str] = &["stop", "attack_move", "hold"];
pub const ORDER_TARGETS: &[&str] = &["caster_relative", "point"];
pub const VITALS: &[&str] = &["life", "shield"];
pub const VITAL_MODES: &[&str] = &["drain", "set"];
pub const VITAL_TARGETS: &[&str] = &["current", "max"];
pub const Q16_ONE: i32 = 65536;

pub const TARGETS: &[&str] = &["none", "unit", "point"];
pub const TRIGGERS: &[&str] = &["idle", "attacked", "spawned", "ability_ready", "destroyed"];
pub const ACTIONS: &[&str] = &["attack_nearest", "flee", "use_ability", "declare_winner"];
pub const ACTION_USE_ABILITY: usize = 2;
pub const ACTION_DECLARE_WINNER: usize = 3;
pub const FILTERS: &[&str] = &[
    "ground", "air", "armored", "biological", "mechanical", "psionic", "structure", "light", "massive",
    "worker", "army", "building", "barracks", "melee_rax", "ranged_rax",
];
pub const FILTER_MASK_BITS: u32 = 15;
pub const FILTER_ALL: u32 = (1 << FILTER_MASK_BITS) - 1;
pub const DAMAGE_RESPONSES: &[&str] = &["normal", "reduced", "immune"];
pub const RESOURCE_KINDS: &[&str] = &["none", "resource_a", "gas"];
pub const UPGRADE_TARGET_KINDS: &[&str] = &["unit", "ability"];
pub const UPGRADE_APPLY: &[&str] = &["new_only", "existing"];
pub const UPGRADE_STACKING: &[&str] = &["replace", "add", "refuse"];
pub const DAMAGE_TYPES: &[&str] = &["normal", "concussive", "explosive"];
pub const SIZE_CATEGORIES: &[&str] = &["small", "medium", "large"];
pub const CLOAK_FLAGS: &[&str] = &["cloaked", "burrowable", "detector_required"];
pub const CLOAK_MASK: u8 = 0b111;
pub const REQ_KINDS: &[&str] = &["unit", "research"];
pub const CHANNEL_FLAG: u16 = 1;

pub const CELL_BLOCKED: u8 = 1;
pub const CELL_CLIFF: u8 = 2;
pub const CELL_MASK: u8 = CELL_BLOCKED | CELL_CLIFF;

pub const TAG_STRS: [u8; 4] = *b"STRS";
pub const TAG_EFCT: [u8; 4] = *b"EFCT";
pub const TAG_ABIL: [u8; 4] = *b"ABIL";
pub const TAG_UABL: [u8; 4] = *b"UABL";
pub const TAG_UNIT: [u8; 4] = *b"UNIT";
pub const TAG_BHVR: [u8; 4] = *b"BHVR";
pub const TAG_PLCE: [u8; 4] = *b"PLCE";
pub const TAG_UPGR: [u8; 4] = *b"UPGR";
pub const TAG_REGN: [u8; 4] = *b"REGN";
pub const TAG_LIMT: [u8; 4] = *b"LIMT";
pub const TAG_ECON: [u8; 4] = *b"ECON";
pub const TAG_UEXT: [u8; 4] = *b"UEXT";
pub const TAG_ACHN: [u8; 4] = *b"ACHN";
pub const TAG_RSCH: [u8; 4] = *b"RSCH";
pub const TAG_PREQ: [u8; 4] = *b"PREQ";
pub const TAG_DMGF: [u8; 4] = *b"DMGF";
pub const TAG_DODA: [u8; 4] = *b"DODA";
pub const TAG_GRID: [u8; 4] = *b"GRID";
pub const TAG_ITEM: [u8; 4] = *b"ITEM";
pub const TAG_SKIL: [u8; 4] = *b"SKIL";
pub const TAG_AXTN: [u8; 4] = *b"AXTN";
pub const TAG_ROUT: [u8; 4] = *b"ROUT";

pub const STRS_RECORD: usize = 1;
pub const EFCT_RECORD: usize = 20;
pub const ABIL_RECORD: usize = 32;
pub const UABL_RECORD: usize = 2;
pub const UNIT_RECORD: usize = 86;
pub const BHVR_RECORD: usize = 16;
pub const PLCE_RECORD: usize = 16;
pub const UPGR_RECORD: usize = 24;
pub const REGN_RECORD: usize = 12;
pub const LIMT_RECORD: usize = 4;
pub const ECON_RECORD: usize = 32;
pub const UEXT_RECORD: usize = 44;
pub const ACHN_RECORD: usize = 8;
pub const RSCH_RECORD: usize = 32;
pub const PREQ_RECORD: usize = 8;
pub const DMGF_RECORD: usize = 8;
pub const DODA_RECORD: usize = 20;
pub const GRID_RECORD: usize = 1;
pub const ITEM_RECORD: usize = 192;
/// Recipe components one item may name.
pub const ITEM_COMPONENTS: usize = 6;
/// Per-item stat bonuses, in record order. Armor, health regeneration and mana regeneration are hundredths,
/// percent stats are whole percents, move speed is milli-units per tick, ranges, distances and widths are
/// milli-units, durations are ticks, the rest are whole numbers.
pub const ITEM_STATS: &[&str] = &[
    "armor", "damage", "health", "mana", "strength", "agility", "intelligence", "move_speed", "attack_speed",
    "health_regen", "mana_regen", "magic_resist", "evasion", "spell_amp", "lifesteal", "spell_lifesteal",
    "cast_range", "attack_range", "status_resist", "cooldown_reduction", "gold_per_min", "xp_per_min",
    "crit_chance", "crit_multiplier", "cleave", "cleave_distance", "cleave_start_width", "cleave_end_width",
    "bash_chance", "bash_duration", "speed_pct", "slow_resist", "hp_regen_amp", "mana_regen_amp",
    "mana_cost_reduction", "night_vision", "scepter", "shard",
];
pub const ITEM_STAT_COUNT: usize = 38;
pub const ITEM_FLAG_CONSUMABLE: u16 = 1;
pub const ITEM_FLAG_NEUTRAL: u16 = 2;
pub const ITEM_FLAG_NO_BUY: u16 = 4;
pub const ITEM_FLAG_NO_SELL: u16 = 8;
pub const ITEM_FLAG_ENHANCEMENT: u16 = 16;
pub const ITEM_FLAG_MASK: u16 = 31;
/// Shops that can stock an item, by bit.
pub const ITEM_SHOPS: &[&str] = &["home", "side", "secret"];
pub const ITEM_SHOP_MASK: u8 = 7;
pub const NEUTRAL_TIERS: u8 = 5;
pub const SKIL_RECORD: usize = 72;
pub const AXTN_RECORD: usize = 256;
/// Waypoints one route holds.
pub const MAX_ROUTE_POINTS: usize = 64;
/// Route lanes: 1 up to this value; the lane id of a spawn effect rides the two top bits of its p3.
pub const MAX_ROUTE_LANE: i32 = 3;
/// A route record: player u16, lane u16, count u16, reserved u16, then `MAX_ROUTE_POINTS` x/y pairs of u32 milli-units.
pub const ROUT_RECORD: usize = 8 + MAX_ROUTE_POINTS * 8;
/// Rank-scaled effect parameters and conditional modifiers one ability may carry.
pub const AXTN_SCALED: usize = 6;
pub const AXTN_MODS: usize = 8;
pub const AXTN_FLAG_MASK: u16 = 1023;
/// Upgrade bits a modifier may test: 0 scepter, 1 shard, 2..=9 the eight talents.
pub const MOD_CONDS: usize = 10;
/// Modifier "effect" values from here up name an ability field: cooldown, energy, cast range, cast point.
pub const MOD_FIELD_FIRST: u8 = 250;
/// Ranks a skill tree may hold: an ability's max rank is `1..=SKILL_RANKS`.
pub const SKILL_RANKS: usize = 4;
/// Ultimates take at most this many ranks, each gated by its level in `ULTIMATE_GATES`.
pub const ULTIMATE_RANKS: usize = 3;
pub const ULTIMATE_GATES: [i32; ULTIMATE_RANKS] = [6, 12, 18];
/// Level gate of a non-ultimate rank: any level.
pub const NORMAL_GATE: i32 = 1;
/// Latest level gate of a non-ultimate rank (Dota: 1, 3, 5, 7).
pub const MAX_NORMAL_GATE: i32 = 30;
/// Inventory slots each hero carries.
pub const ITEM_SLOTS: usize = 6;
pub const DOODAD_KINDS: &[&str] = &["rock", "tree", "crystal", "debris", "base", "shop", "fountain", "lane", "camp", "side_shop", "secret_shop", "aid_cane", "aid_cane_blind", "aid_cane_low_vision", "aid_crutch", "aid_defibrillator_green", "aid_defibrillator_red", "aid_glasses", "aid_mask", "aid_sunglasses", "aid_hearing", "altar_stone", "altar_wood", "arrow", "arrow_basic", "arrow_basic_rounded", "arrow_rounded", "balcony_wall", "balcony_wall_fence", "banner", "banner_green", "banner_red", "barrel", "barrel_open", "bedroll", "bedroll_frame", "bedroll_packed", "bench", "bench_damaged", "blade", "border_pillar", "bottle", "bottle_large", "box", "box_large", "box_large_open", "box_long", "box_open", "box_small", "box_wide", "brick_wall", "brick_wall_curve", "brick_wall_curve_small", "brick_wall_end", "bridge", "bridge_draw", "bridge_pillar", "bridge_pillar_wide", "bridge_straight", "bridge_straight_pillar", "bucket", "building_a", "building_b", "building_c", "building_d", "building_e", "building_f", "building_g", "building_h", "building_i", "building_j", "building_k", "building_l", "building_m", "building_n", "building_o", "building_p", "building_platform", "building_q", "building_r", "building_roof", "building_s", "building_structure", "building_t", "button_floor_round", "button_floor_round_small", "button_floor_square", "button_floor_square_small", "campfire_fishing_stand", "campfire_pit", "campfire_stand", "candle", "candle_multiple", "cart", "cart_high", "catwalk_corner", "catwalk_cross", "catwalk_junction", "catwalk_stairs", "catwalk_stairs_loop", "catwalk_straight", "chair", "character_archer", "character_female_a", "character_female_b", "character_female_c", "character_female_d", "character_female_e", "character_female_f", "character_ghost", "character_human", "character_keeper", "character_male_a", "character_male_b", "character_male_c", "character_male_d", "character_male_e", "character_male_f", "character_orc", "character_skeleton", "character_vampire", "character_zombie", "chest", "chimney", "chimney_base", "chimney_basic", "chimney_large", "chimney_medium", "chimney_small", "chimney_top", "coffin", "coffin_old", "cog_a", "cog_b", "cog_c", "cog_d", "cog_e", "coin", "column", "column_large", "cone", "construction_barrier", "construction_cone", "construction_fence", "construction_light", "conveyor", "conveyor_bars", "conveyor_bars_fence", "conveyor_bars_fence_slope", "conveyor_bars_high", "conveyor_bars_high_slope", "conveyor_bars_sides", "conveyor_bars_stripe", "conveyor_bars_stripe_fence", "conveyor_bars_stripe_fence_slope", "conveyor_bars_stripe_high", "conveyor_bars_stripe_high_slope", "conveyor_bars_stripe_side", "conveyor_corner", "conveyor_cross", "conveyor_junction_t", "conveyor_long", "conveyor_long_part_end", "conveyor_long_part_middle", "conveyor_long_sides", "conveyor_long_sides_part_end", "conveyor_long_sides_part_middle", "conveyor_long_stripe", "conveyor_long_stripe_sides", "conveyor_long_stripe_sides_part_end", "conveyor_long_stripe_sides_part_middle", "conveyor_sides", "conveyor_sides_cross", "conveyor_sides_junction_t", "conveyor_stripe", "conveyor_stripe_corner", "conveyor_stripe_cross", "conveyor_stripe_junction_t", "conveyor_stripe_part_end", "conveyor_stripe_part_middle", "conveyor_stripe_sides", "conveyor_stripe_sides_cross", "conveyor_stripe_sides_junction_t", "corridor", "corridor_corner", "corridor_end", "corridor_intersection", "corridor_junction", "corridor_transition", "corridor_wide", "corridor_wide_corner", "corridor_wide_end", "corridor_wide_intersection", "corridor_wide_junction", "crane", "crane_lift", "crane_magnet", "cross", "cross_column", "cross_wood", "crypt", "crypt_a", "crypt_b", "crypt_door", "crypt_large", "crypt_large_door", "crypt_large_roof", "crypt_small", "crypt_small_roof", "debris_wood", "detail_bowl", "detail_chalice", "detail_plate", "detail_tank", "detail_tank_large", "dirt", "door", "door_wide_closed", "door_wide_half", "door_wide_open", "dumpster", "electricity_pole", "electricity_pole_single", "electricity_pole_wide", "electricity_side", "electricity_side_single", "electricity_side_wide", "electricity_wires", "electricity_wires_wide", "fence", "fence_broken", "fence_curved", "fence_damaged", "fence_doorway", "fence_fortified", "fence_gate", "fire_basket", "fish", "fish_large", "flag", "flag_banner_long", "flag_banner_short", "flag_pennant", "flag_wide", "floor", "floor_detail", "floor_hole", "floor_large", "floor_old", "fountain_center", "fountain_corner", "fountain_corner_inner", "fountain_corner_inner_square", "fountain_curved", "fountain_edge", "fountain_round", "fountain_round_detail", "fountain_square", "fountain_square_detail", "gate", "gate_metal_bars", "gate_overhang", "gate_rock", "grass", "grass_large", "grave", "grave_border", "gravestone_bevel", "gravestone_broken", "gravestone_cross", "gravestone_cross_large", "gravestone_debris", "gravestone_decorative", "gravestone_roof", "gravestone_round", "gravestone_wide", "ground", "ground_hills", "ground_pathTile", "ground_riverTile", "hay_bale", "hay_bale_bundled", "hedge", "hedge_curved", "hedge_gate", "hedge_large", "hedge_large_curved", "hedge_large_gate", "hopper_high_round", "hopper_high_square", "hopper_round", "hopper_square", "indicator_special_area", "indicator_special_arrow", "indicator_special_cross", "indicator_special_lines", "iron_fence", "iron_fence_bar", "iron_fence_border", "iron_fence_border_column", "iron_fence_border_curve", "iron_fence_border_gate", "iron_fence_curve", "iron_fence_damaged", "key", "ladder", "lantern", "lantern_candle", "lantern_glass", "lever_double", "lever_single", "light_curved", "light_curved_cross", "light_curved_double", "light_square", "light_square_cross", "light_square_double", "lightpost_all", "lightpost_double", "lightpost_single", "machine", "machine_bed", "machine_connection_hole", "machine_connection_pipe", "machine_fortified", "machine_window", "machine_window_bar", "metal_gate", "metal_panel", "metal_panel_narrow", "metal_panel_screws", "metal_panel_screws_half", "metal_panel_screws_narrow", "oopi", "overhang", "patch_dirt", "patch_grass", "patch_grass_large", "pillar_large", "pillar_obelisk", "pillar_small", "pillar_square", "pillar_stone", "pillar_wood", "pine", "pine_crooked", "pine_fall", "pine_fall_crooked", "pipe_glass_large", "pipe_glass_large_bend", "pipe_glass_large_bump", "pipe_glass_large_cross", "pipe_glass_large_curve", "pipe_glass_large_junction", "pipe_glass_large_long", "pipe_glass_large_side", "pipe_glass_large_valve", "pipe_large", "pipe_large_bend", "pipe_large_bump", "pipe_large_cross", "pipe_large_curve", "pipe_large_junction", "pipe_large_long", "pipe_large_side", "pipe_large_valve", "piston_round", "piston_square", "piston_thin_round", "piston_thin_square", "planks", "planks_half", "planks_opening", "plant", "platform", "poles", "poles_horizontal", "pot", "potion", "pumpkin", "pumpkin_carved", "pumpkin_tall", "pumpkin_tall_carved", "resource_planks", "resource_stone", "resource_stone_large", "resource_wood", "road", "road_bend", "road_bend_barrier", "road_bend_sidewalk", "road_bend_square", "road_bend_square_barrier", "road_bridge", "road_corner", "road_corner_inner", "road_crossing", "road_crossroad", "road_crossroad_barrier", "road_crossroad_line", "road_crossroad_path", "road_curb", "road_curb_end", "road_curve", "road_curve_barrier", "road_curve_intersection", "road_curve_intersection_barrier", "road_curve_pavement", "road_driveway_double", "road_driveway_double_barrier", "road_driveway_single", "road_driveway_single_barrier", "road_edge", "road_edge_slope", "road_end", "road_end_barrier", "road_end_round", "road_end_round_barrier", "road_intersection", "road_intersection_barrier", "road_intersection_line", "road_intersection_path", "road_roundabout", "road_roundabout_barrier", "road_side", "road_side_barrier", "road_side_entry", "road_side_entry_barrier", "road_side_exit", "road_side_exit_barrier", "road_sign_empty", "road_sign_empty_hanging", "road_sign_object_stop", "road_sign_object_street", "road_sign_object_warning", "road_sign_stop", "road_sign_street", "road_sign_warning", "road_slant", "road_slant_barrier", "road_slant_curve", "road_slant_curve_barrier", "road_slant_flat", "road_slant_flat_curve", "road_slant_flat_high", "road_slant_high", "road_slant_high_barrier", "road_slope", "road_split", "road_split_barrier", "road_square", "road_square_barrier", "road_straight", "road_straight_barrier", "road_straight_barrier_end", "road_straight_barrier_half", "road_straight_half", "robot_arm_a", "robot_arm_b", "rock_a", "rock_b", "rock_c", "rock_flat", "rock_flat_grass", "rock_large", "rock_sand_a", "rock_sand_b", "rock_sand_c", "rock_small", "rock_wide", "rocks", "rocks_high", "rocks_large", "rocks_low", "rocks_ramp", "rocks_small", "rocks_tall", "roof", "roof_corner", "roof_corner_inner", "roof_corner_round", "roof_flat", "roof_gable", "roof_gable_detail", "roof_gable_end", "roof_gable_top", "roof_high", "roof_high_corner", "roof_high_corner_round", "roof_high_cornerinner", "roof_high_flat", "roof_high_gable", "roof_high_gable_detail", "roof_high_gable_end", "roof_high_gable_top", "roof_high_left", "roof_high_point", "roof_high_right", "roof_high_window", "roof_left", "roof_point", "roof_right", "roof_window", "room_corner", "room_large", "room_large_variation", "room_small", "room_small_variation", "room_wide", "room_wide_variation", "scanner_high", "scanner_low", "screen_flat", "screen_hanging_small", "screen_hanging_wide", "screen_panel_flat", "screen_panel_small", "screen_panel_wide", "screen_small", "screen_wide", "shield_rectangle", "shield_round", "shipping_container_a", "shipping_container_b", "shipping_container_c", "shovel", "shovel_dirt", "siege_ballista", "siege_ballista_demolished", "siege_catapult", "siege_catapult_demolished", "siege_ram", "siege_ram_demolished", "siege_tower", "siege_tower_demolished", "siege_trebuchet", "siege_trebuchet_demolished", "sign_highway", "sign_highway_detailed", "sign_highway_wide", "signpost", "signpost_single", "solar_panel_flat", "solar_panel_landscape", "solar_panel_landscape_group", "solar_panel_portrait", "solar_panel_portrait_group", "stairs", "stairs_full", "stairs_full_corner_inner", "stairs_full_corner_outer", "stairs_stone", "stairs_stone_corner", "stairs_stone_handrail", "stairs_stone_round", "stairs_stone_square", "stairs_wide", "stairs_wide_stone", "stairs_wide_stone_handrail", "stairs_wide_wood", "stairs_wide_wood_handrail", "stairs_wood", "stairs_wood_handrail", "stall", "stall_bench", "stall_green", "stall_red", "stall_stool", "stone_wall", "stone_wall_column", "stone_wall_curve", "stone_wall_damaged", "stones", "structure", "structure_canvas", "structure_corner_inner", "structure_corner_outer", "structure_doorway", "structure_doorway_wide", "structure_floor", "structure_high", "structure_medium", "structure_metal", "structure_metal_doorway", "structure_metal_floor", "structure_metal_roof", "structure_metal_wall", "structure_roof", "structure_short", "structure_tall", "structure_wall", "structure_window", "structure_window_wide", "structure_yellow_high", "structure_yellow_medium", "structure_yellow_short", "structure_yellow_tall", "table", "target", "template_corner", "template_detail", "template_floor", "template_floor_big", "template_floor_detail", "template_floor_detail_a", "template_floor_layer", "template_floor_layer_hole", "template_floor_layer_raised", "template_wall", "template_wall_corner", "template_wall_detail_a", "template_wall_half", "template_wall_stairs", "template_wall_top", "tent", "tent_canvas", "tent_canvas_half", "tile_high", "tile_low", "tile_slant", "tile_slantHigh", "tool_axe", "tool_axe_upgraded", "tool_hammer", "tool_hammer_upgraded", "tool_hoe", "tool_hoe_upgraded", "tool_pickaxe", "tool_pickaxe_upgraded", "tool_shovel", "tool_shovel_upgraded", "top", "top_large", "top_large_checkerboard", "tower_base", "tower_hexagon_base", "tower_hexagon_mid", "tower_hexagon_roof", "tower_hexagon_roof_secondary", "tower_hexagon_top", "tower_hexagon_top_wood", "tower_slant_roof", "tower_square", "tower_square_arch", "tower_square_base", "tower_square_base_border", "tower_square_base_color", "tower_square_mid", "tower_square_mid_color", "tower_square_mid_door", "tower_square_mid_open", "tower_square_mid_open_simple", "tower_square_mid_windows", "tower_square_roof", "tower_square_top", "tower_square_top_color", "tower_square_top_roof", "tower_square_top_roof_high", "tower_square_top_roof_high_windows", "tower_square_top_roof_rounded", "tower_top", "traffic_light", "traffic_light_hanging", "traffic_light_object_hanging", "traffic_light_object_horizontal", "traffic_light_object_vertical", "trap", "tree_autumn", "tree_autumn_tall", "tree_autumn_trunk", "tree_crooked", "tree_high", "tree_high_crooked", "tree_high_round", "tree_large", "tree_log", "tree_log_small", "tree_small", "tree_tall", "tree_trunk", "trunk", "trunk_long", "urn_round", "urn_square", "wall", "wall_arch", "wall_arch_top", "wall_arch_top_detail", "wall_block", "wall_block_half", "wall_broken", "wall_corner", "wall_corner_detail", "wall_corner_diagonal", "wall_corner_diagonal_half", "wall_corner_edge", "wall_corner_half", "wall_corner_half_tower", "wall_corner_slant", "wall_curved", "wall_detail_cross", "wall_detail_diagonal", "wall_detail_horizontal", "wall_diagonal", "wall_door", "wall_doorway", "wall_doorway_base", "wall_doorway_round", "wall_doorway_square", "wall_doorway_square_wide", "wall_doorway_square_wide_curved", "wall_half", "wall_half_modular", "wall_narrow", "wall_narrow_corner", "wall_narrow_gate", "wall_narrow_stairs", "wall_narrow_stairs_rail", "wall_narrow_wood", "wall_narrow_wood_fence", "wall_opening", "wall_pillar", "wall_rounded", "wall_side", "wall_slope", "wall_stud", "wall_to_narrow", "wall_window_glass", "wall_window_round", "wall_window_shutters", "wall_window_small", "wall_window_stone", "wall_wood", "wall_wood_arch", "wall_wood_arch_top", "wall_wood_arch_top_detail", "wall_wood_block", "wall_wood_block_half", "wall_wood_broken", "wall_wood_corner", "wall_wood_corner_diagonal", "wall_wood_corner_diagonal_half", "wall_wood_corner_edge", "wall_wood_curved", "wall_wood_detail_cross", "wall_wood_detail_diagonal", "wall_wood_detail_horizontal", "wall_wood_diagonal", "wall_wood_door", "wall_wood_doorway_base", "wall_wood_doorway_round", "wall_wood_doorway_square", "wall_wood_doorway_square_wide", "wall_wood_doorway_square_wide_curved", "wall_wood_half", "wall_wood_rounded", "wall_wood_side", "wall_wood_slope", "wall_wood_window_glass", "wall_wood_window_round", "wall_wood_window_shutters", "wall_wood_window_small", "wall_wood_window_stone", "warning_orange", "warning_traffic", "water_tower", "watermill", "watermill_wide", "weapon_arrow", "weapon_bow", "weapon_spear", "weapon_sword", "wheel", "wheelchair", "wheelchair_deluxe", "wheelchair_power", "wheelchair_power_deluxe", "windmill", "windmill_low", "wood_structure", "wood_support", "workbench", "workbench_anvil", "workbench_grind"];
pub const GRID_HEADER_LEN: usize = 16;

/// Section tags and record sizes in the order FORMAT.md requires.
pub const SECTIONS: [([u8; 4], usize); 22] = [
    (TAG_STRS, STRS_RECORD),
    (TAG_EFCT, EFCT_RECORD),
    (TAG_ABIL, ABIL_RECORD),
    (TAG_UABL, UABL_RECORD),
    (TAG_UNIT, UNIT_RECORD),
    (TAG_BHVR, BHVR_RECORD),
    (TAG_PLCE, PLCE_RECORD),
    (TAG_UPGR, UPGR_RECORD),
    (TAG_REGN, REGN_RECORD),
    (TAG_LIMT, LIMT_RECORD),
    (TAG_ECON, ECON_RECORD),
    (TAG_UEXT, UEXT_RECORD),
    (TAG_ACHN, ACHN_RECORD),
    (TAG_RSCH, RSCH_RECORD),
    (TAG_PREQ, PREQ_RECORD),
    (TAG_DMGF, DMGF_RECORD),
    (TAG_DODA, DODA_RECORD),
    (TAG_GRID, GRID_RECORD),
    (TAG_ITEM, ITEM_RECORD),
    (TAG_SKIL, SKIL_RECORD),
    (TAG_AXTN, AXTN_RECORD),
    (TAG_ROUT, ROUT_RECORD),
];

pub fn index_of(list: &[&str], value: &str, what: &str) -> anyhow::Result<usize> {
    list.iter()
        .position(|v| *v == value)
        .ok_or_else(|| anyhow::anyhow!("unknown {what} {value:?}; expected one of {list:?}"))
}

/// Per-kind parameter ranges from FORMAT.md's effect table. Shared by the
/// Starlark builtins and the reference reader. The `move` cap is bounded by
/// the grid, so it is checked by `move_cap_fits_grid` instead.
pub fn effect_params_valid(kind: &str, p: [i32; 4]) -> bool {
    let [p0, p1, p2, p3] = p;
    match kind {
        "damage" => p0 >= 0 && p1 >= 0 && (0..SPELL_DAMAGE_TYPES.len() as i32).contains(&p2),
        "heal" => p0 >= 0 && p1 >= 0 && (0..=2).contains(&p2),
        "move" => p0 > 0,
        "buff" => p0 >= 0 && p1 >= 0,
        "spawn" => p0 >= 0,
        "issue_order" => (0..ORDER_KINDS.len() as i32).contains(&p0) && (0..ORDER_TARGETS.len() as i32).contains(&p1),
        "remove_behaviour" => p0 >= 0 && p1 >= 0,
        "create_healer" => p0 >= 0 && p1 >= 0 && p2 >= 1 && p3 >= 1,
        "use_calldown" => p0 >= 0,
        "vital_fraction" => {
            (0..VITALS.len() as i32).contains(&p0)
                && (0..=Q16_ONE).contains(&p1)
                && (0..VITAL_MODES.len() as i32).contains(&p2)
                && (0..VITAL_TARGETS.len() as i32).contains(&p3)
        }
        "shield_bonus" => p0 >= 0,
        "stat" => (0..STAT_NAMES.len() as i32).contains(&p0),
        "ward" => p0 >= 0 && p1 >= 1,
        "glyph" => p0 >= 1 && p1 >= 0 && p2 >= 1 && p3 >= 0,
        "deliver" => p0 >= 0,
        "dot" => p0 >= 0 && p1 >= 1 && p2 >= 1 && (0..SPELL_DAMAGE_TYPES.len() as i32).contains(&p3),
        "status" => (0..STATUS_CODES.len() as i32).contains(&p0) && p1 >= 1,
        "timed_stat" => (0..STAT_NAMES.len() as i32).contains(&p0) && p2 >= 1 && (0..=1).contains(&p3),
        "aoe" => p0 >= 0 && (0..=31).contains(&p1) && (0..=16).contains(&p2) && p3 >= 0,
        "dispel" => (1..=7).contains(&p0),
        "chain" => p0 >= 0 && p1 >= 0 && p2 >= 0 && (0..SPELL_DAMAGE_TYPES.len() as i32).contains(&p3),
        "zone" => p0 >= 0 && p1 >= 1 && p2 >= 1 && p3 >= 0,
        "illusion" => (1..=8).contains(&p0) && p1 >= 1 && p2 >= 0 && p3 >= 0,
        "mana" => p0 >= 0 && (0..=2).contains(&p1),
        "if_kill" => (1..=16).contains(&p0),
        "attack" => p0 >= 0,
        "teleport" => p0 >= 0,
        "refresh" => (1..=3).contains(&p0),
        "grant_upgrade" => (0..=1).contains(&p0),
        "stun" | "silence" | "root" | "invisibility" => p0 >= 1,
        "slow" => p0 >= 1 && (0..=Q16_ONE).contains(&p1),
        _ => true,
    }
}

/// True when a `move` cap is in `1..=diagonal`, where the diagonal is
/// `sqrt((width * cell)^2 + (height * cell)^2)` in milli-units. Compares squares
/// in integers, so no float is read.
pub fn move_cap_fits_grid(cap: i32, width: u32, height: u32, cell_milli: u32) -> bool {
    if cap <= 0 {
        return false;
    }
    let cap = cap as u128;
    let w = width as u128 * cell_milli as u128;
    let h = height as u128 * cell_milli as u128;
    cap * cap <= w * w + h * h
}

/// True when a placement lies on the map: `0 <= x < width * cell` and `0 <= y < height * cell`.
pub fn position_on_grid(x: i64, y: i64, width: u32, height: u32, cell_milli: u32) -> bool {
    let extent_x = width as i64 * cell_milli as i64;
    let extent_y = height as i64 * cell_milli as i64;
    (0..extent_x).contains(&x) && (0..extent_y).contains(&y)
}

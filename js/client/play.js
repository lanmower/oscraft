// Net client: WebGL2 map and unit view. Map and units come from the same assets as the browser
// page (map.bin and the wasm sim pkg). Joins a wireweave DataSession room named by the URL hash.
// There is no fallback path: a missing or failed asset is shown on the page (see recordAssetError), never drawn as something else.
import { createDataSession, createFSM, NostrAuth, RelayPool } from "wireweave";
import * as NostrTools from "nostr-tools";
import * as xstate from "xstate";
import { encodeFrame, decodeFrame, FRAME, MAX_PLAYERS, SPECTATOR } from "./wire/frame.js";
import { iceServersFrom } from "./ice.js";
import { applyFrame, createSeating, createStateEncoder, startFrame } from "./delta.js";
import { AnimationPlayer, cycleSpeed, jointMatrices, retargetLibrary, skinFloor } from "./anim.js";
import { parseGlb, parseGlbAnimations } from "./glb.js";
import { resolveGlbExtensions } from "./glb_resolve.js";
import { createHud } from "./hud.js";
import { createHud5 } from "./hud5.js";
import { createCamera, createRenderer as createRenderer3d, eyeOf, MAX_JOINTS, meshReach, multiply, rotationY, scaling, translation, viewProjection } from "./render3d.js";
import { buildTerrain, GROUND_UV_PER_CELL, readDoodads, readGrid, vertexNormals } from "./terrain.js";
import { controlMap, fitModel, planarGroundUvs, rampLowCells, SPLAT_LAYERS, SPLAT_TEXTURES, setSplatTexel, TILE_TINTS, walkableBounds, waterMap, waterSurface } from "./tiles.js";
import { loadBaseColorImage, uploadTexture } from "./texture.js";
import { createLobby, heroKindsFor, heroNamesFromRules } from "./lobby.js";
import { createFogView } from "./fogview.js";
import { installDebugApi } from "./debug_api.js";
import { createShop } from "./ui_shop.js";
import { createSkills } from "./ui_skills.js";
import { createInventory } from "./ui_inventory.js";
import { createFx } from "./fx.js";
import { createModUi } from "./modui.js";

// Map size comes from the loaded map image (loadSim); these are the fallbacks before a map is read.
let W = 32;
let H = 24;
const CAMERA_DISTANCE_PER_TILE = 40 / 32;
const CAMERA_MAX_PER_TILE = 1.6;
const CAMERA_MAX_FLOOR = 120;
const CAMERA_PLAY_DISTANCE = 70;
// The match opens on the local hero at this distance: close enough to read a hero, far enough to see its lane.
const CAMERA_START_DISTANCE = 40;
let pendingStartCentre = false;
const CAMERA_FAR_PER_MAX = 2;
const BACKGROUND = [0.114, 0.145, 0.173];
const MAP_COLOR = [0.2, 0.28, 0.22];
const PLAYER_COLORS = [[0.227, 0.482, 0.835], [0.835, 0.329, 0.227]];
const SELECT_COLOR = [0.95, 0.85, 0.3];
const UNIT_HALF = 0.9;
const HERO_DRAW_SCALE = 2;
const DOODAD_HALF = 0.5;
const PICK_RADIUS = 1.2;
const DRAG_SLOP_PX = 4;
const PAN_PX_PER_S = 500;
let orbitOrigin = [W / 2, 0, H / 2];
let cameraBounds = null;
let cameraDistance = Math.min(CAMERA_PLAY_DISTANCE, CAMERA_DISTANCE_PER_TILE * Math.max(W, H));
let cameraMaxDistance = Math.max(CAMERA_MAX_FLOOR, CAMERA_MAX_PER_TILE * Math.max(W, H));
const DOODAD_KINDS = ["rock", "tree", "crystal", "debris", "base", "shop", "fountain", "lane", "camp", "side_shop", "secret_shop", "aid_cane", "aid_cane_blind", "aid_cane_low_vision", "aid_crutch", "aid_defibrillator_green", "aid_defibrillator_red", "aid_glasses", "aid_mask", "aid_sunglasses", "aid_hearing", "altar_stone", "altar_wood", "arrow", "arrow_basic", "arrow_basic_rounded", "arrow_rounded", "balcony_wall", "balcony_wall_fence", "banner", "banner_green", "banner_red", "barrel", "barrel_open", "bedroll", "bedroll_frame", "bedroll_packed", "bench", "bench_damaged", "blade", "border_pillar", "bottle", "bottle_large", "box", "box_large", "box_large_open", "box_long", "box_open", "box_small", "box_wide", "brick_wall", "brick_wall_curve", "brick_wall_curve_small", "brick_wall_end", "bridge", "bridge_draw", "bridge_pillar", "bridge_pillar_wide", "bridge_straight", "bridge_straight_pillar", "bucket", "building_a", "building_b", "building_c", "building_d", "building_e", "building_f", "building_g", "building_h", "building_i", "building_j", "building_k", "building_l", "building_m", "building_n", "building_o", "building_p", "building_platform", "building_q", "building_r", "building_roof", "building_s", "building_structure", "building_t", "button_floor_round", "button_floor_round_small", "button_floor_square", "button_floor_square_small", "campfire_fishing_stand", "campfire_pit", "campfire_stand", "candle", "candle_multiple", "cart", "cart_high", "catwalk_corner", "catwalk_cross", "catwalk_junction", "catwalk_stairs", "catwalk_stairs_loop", "catwalk_straight", "chair", "character_archer", "character_female_a", "character_female_b", "character_female_c", "character_female_d", "character_female_e", "character_female_f", "character_ghost", "character_human", "character_keeper", "character_male_a", "character_male_b", "character_male_c", "character_male_d", "character_male_e", "character_male_f", "character_orc", "character_skeleton", "character_vampire", "character_zombie", "chest", "chimney", "chimney_base", "chimney_basic", "chimney_large", "chimney_medium", "chimney_small", "chimney_top", "coffin", "coffin_old", "cog_a", "cog_b", "cog_c", "cog_d", "cog_e", "coin", "column", "column_large", "cone", "construction_barrier", "construction_cone", "construction_fence", "construction_light", "conveyor", "conveyor_bars", "conveyor_bars_fence", "conveyor_bars_fence_slope", "conveyor_bars_high", "conveyor_bars_high_slope", "conveyor_bars_sides", "conveyor_bars_stripe", "conveyor_bars_stripe_fence", "conveyor_bars_stripe_fence_slope", "conveyor_bars_stripe_high", "conveyor_bars_stripe_high_slope", "conveyor_bars_stripe_side", "conveyor_corner", "conveyor_cross", "conveyor_junction_t", "conveyor_long", "conveyor_long_part_end", "conveyor_long_part_middle", "conveyor_long_sides", "conveyor_long_sides_part_end", "conveyor_long_sides_part_middle", "conveyor_long_stripe", "conveyor_long_stripe_sides", "conveyor_long_stripe_sides_part_end", "conveyor_long_stripe_sides_part_middle", "conveyor_sides", "conveyor_sides_cross", "conveyor_sides_junction_t", "conveyor_stripe", "conveyor_stripe_corner", "conveyor_stripe_cross", "conveyor_stripe_junction_t", "conveyor_stripe_part_end", "conveyor_stripe_part_middle", "conveyor_stripe_sides", "conveyor_stripe_sides_cross", "conveyor_stripe_sides_junction_t", "corridor", "corridor_corner", "corridor_end", "corridor_intersection", "corridor_junction", "corridor_transition", "corridor_wide", "corridor_wide_corner", "corridor_wide_end", "corridor_wide_intersection", "corridor_wide_junction", "crane", "crane_lift", "crane_magnet", "cross", "cross_column", "cross_wood", "crypt", "crypt_a", "crypt_b", "crypt_door", "crypt_large", "crypt_large_door", "crypt_large_roof", "crypt_small", "crypt_small_roof", "debris_wood", "detail_bowl", "detail_chalice", "detail_plate", "detail_tank", "detail_tank_large", "dirt", "door", "door_wide_closed", "door_wide_half", "door_wide_open", "dumpster", "electricity_pole", "electricity_pole_single", "electricity_pole_wide", "electricity_side", "electricity_side_single", "electricity_side_wide", "electricity_wires", "electricity_wires_wide", "fence", "fence_broken", "fence_curved", "fence_damaged", "fence_doorway", "fence_fortified", "fence_gate", "fire_basket", "fish", "fish_large", "flag", "flag_banner_long", "flag_banner_short", "flag_pennant", "flag_wide", "floor", "floor_detail", "floor_hole", "floor_large", "floor_old", "fountain_center", "fountain_corner", "fountain_corner_inner", "fountain_corner_inner_square", "fountain_curved", "fountain_edge", "fountain_round", "fountain_round_detail", "fountain_square", "fountain_square_detail", "gate", "gate_metal_bars", "gate_overhang", "gate_rock", "grass", "grass_large", "grave", "grave_border", "gravestone_bevel", "gravestone_broken", "gravestone_cross", "gravestone_cross_large", "gravestone_debris", "gravestone_decorative", "gravestone_roof", "gravestone_round", "gravestone_wide", "ground", "ground_hills", "ground_pathTile", "ground_riverTile", "hay_bale", "hay_bale_bundled", "hedge", "hedge_curved", "hedge_gate", "hedge_large", "hedge_large_curved", "hedge_large_gate", "hopper_high_round", "hopper_high_square", "hopper_round", "hopper_square", "indicator_special_area", "indicator_special_arrow", "indicator_special_cross", "indicator_special_lines", "iron_fence", "iron_fence_bar", "iron_fence_border", "iron_fence_border_column", "iron_fence_border_curve", "iron_fence_border_gate", "iron_fence_curve", "iron_fence_damaged", "key", "ladder", "lantern", "lantern_candle", "lantern_glass", "lever_double", "lever_single", "light_curved", "light_curved_cross", "light_curved_double", "light_square", "light_square_cross", "light_square_double", "lightpost_all", "lightpost_double", "lightpost_single", "machine", "machine_bed", "machine_connection_hole", "machine_connection_pipe", "machine_fortified", "machine_window", "machine_window_bar", "metal_gate", "metal_panel", "metal_panel_narrow", "metal_panel_screws", "metal_panel_screws_half", "metal_panel_screws_narrow", "oopi", "overhang", "patch_dirt", "patch_grass", "patch_grass_large", "pillar_large", "pillar_obelisk", "pillar_small", "pillar_square", "pillar_stone", "pillar_wood", "pine", "pine_crooked", "pine_fall", "pine_fall_crooked", "pipe_glass_large", "pipe_glass_large_bend", "pipe_glass_large_bump", "pipe_glass_large_cross", "pipe_glass_large_curve", "pipe_glass_large_junction", "pipe_glass_large_long", "pipe_glass_large_side", "pipe_glass_large_valve", "pipe_large", "pipe_large_bend", "pipe_large_bump", "pipe_large_cross", "pipe_large_curve", "pipe_large_junction", "pipe_large_long", "pipe_large_side", "pipe_large_valve", "piston_round", "piston_square", "piston_thin_round", "piston_thin_square", "planks", "planks_half", "planks_opening", "plant", "platform", "poles", "poles_horizontal", "pot", "potion", "pumpkin", "pumpkin_carved", "pumpkin_tall", "pumpkin_tall_carved", "resource_planks", "resource_stone", "resource_stone_large", "resource_wood", "road", "road_bend", "road_bend_barrier", "road_bend_sidewalk", "road_bend_square", "road_bend_square_barrier", "road_bridge", "road_corner", "road_corner_inner", "road_crossing", "road_crossroad", "road_crossroad_barrier", "road_crossroad_line", "road_crossroad_path", "road_curb", "road_curb_end", "road_curve", "road_curve_barrier", "road_curve_intersection", "road_curve_intersection_barrier", "road_curve_pavement", "road_driveway_double", "road_driveway_double_barrier", "road_driveway_single", "road_driveway_single_barrier", "road_edge", "road_edge_slope", "road_end", "road_end_barrier", "road_end_round", "road_end_round_barrier", "road_intersection", "road_intersection_barrier", "road_intersection_line", "road_intersection_path", "road_roundabout", "road_roundabout_barrier", "road_side", "road_side_barrier", "road_side_entry", "road_side_entry_barrier", "road_side_exit", "road_side_exit_barrier", "road_sign_empty", "road_sign_empty_hanging", "road_sign_object_stop", "road_sign_object_street", "road_sign_object_warning", "road_sign_stop", "road_sign_street", "road_sign_warning", "road_slant", "road_slant_barrier", "road_slant_curve", "road_slant_curve_barrier", "road_slant_flat", "road_slant_flat_curve", "road_slant_flat_high", "road_slant_high", "road_slant_high_barrier", "road_slope", "road_split", "road_split_barrier", "road_square", "road_square_barrier", "road_straight", "road_straight_barrier", "road_straight_barrier_end", "road_straight_barrier_half", "road_straight_half", "robot_arm_a", "robot_arm_b", "rock_a", "rock_b", "rock_c", "rock_flat", "rock_flat_grass", "rock_large", "rock_sand_a", "rock_sand_b", "rock_sand_c", "rock_small", "rock_wide", "rocks", "rocks_high", "rocks_large", "rocks_low", "rocks_ramp", "rocks_small", "rocks_tall", "roof", "roof_corner", "roof_corner_inner", "roof_corner_round", "roof_flat", "roof_gable", "roof_gable_detail", "roof_gable_end", "roof_gable_top", "roof_high", "roof_high_corner", "roof_high_corner_round", "roof_high_cornerinner", "roof_high_flat", "roof_high_gable", "roof_high_gable_detail", "roof_high_gable_end", "roof_high_gable_top", "roof_high_left", "roof_high_point", "roof_high_right", "roof_high_window", "roof_left", "roof_point", "roof_right", "roof_window", "room_corner", "room_large", "room_large_variation", "room_small", "room_small_variation", "room_wide", "room_wide_variation", "scanner_high", "scanner_low", "screen_flat", "screen_hanging_small", "screen_hanging_wide", "screen_panel_flat", "screen_panel_small", "screen_panel_wide", "screen_small", "screen_wide", "shield_rectangle", "shield_round", "shipping_container_a", "shipping_container_b", "shipping_container_c", "shovel", "shovel_dirt", "siege_ballista", "siege_ballista_demolished", "siege_catapult", "siege_catapult_demolished", "siege_ram", "siege_ram_demolished", "siege_tower", "siege_tower_demolished", "siege_trebuchet", "siege_trebuchet_demolished", "sign_highway", "sign_highway_detailed", "sign_highway_wide", "signpost", "signpost_single", "solar_panel_flat", "solar_panel_landscape", "solar_panel_landscape_group", "solar_panel_portrait", "solar_panel_portrait_group", "stairs", "stairs_full", "stairs_full_corner_inner", "stairs_full_corner_outer", "stairs_stone", "stairs_stone_corner", "stairs_stone_handrail", "stairs_stone_round", "stairs_stone_square", "stairs_wide", "stairs_wide_stone", "stairs_wide_stone_handrail", "stairs_wide_wood", "stairs_wide_wood_handrail", "stairs_wood", "stairs_wood_handrail", "stall", "stall_bench", "stall_green", "stall_red", "stall_stool", "stone_wall", "stone_wall_column", "stone_wall_curve", "stone_wall_damaged", "stones", "structure", "structure_canvas", "structure_corner_inner", "structure_corner_outer", "structure_doorway", "structure_doorway_wide", "structure_floor", "structure_high", "structure_medium", "structure_metal", "structure_metal_doorway", "structure_metal_floor", "structure_metal_roof", "structure_metal_wall", "structure_roof", "structure_short", "structure_tall", "structure_wall", "structure_window", "structure_window_wide", "structure_yellow_high", "structure_yellow_medium", "structure_yellow_short", "structure_yellow_tall", "table", "target", "template_corner", "template_detail", "template_floor", "template_floor_big", "template_floor_detail", "template_floor_detail_a", "template_floor_layer", "template_floor_layer_hole", "template_floor_layer_raised", "template_wall", "template_wall_corner", "template_wall_detail_a", "template_wall_half", "template_wall_stairs", "template_wall_top", "tent", "tent_canvas", "tent_canvas_half", "tile_high", "tile_low", "tile_slant", "tile_slantHigh", "tool_axe", "tool_axe_upgraded", "tool_hammer", "tool_hammer_upgraded", "tool_hoe", "tool_hoe_upgraded", "tool_pickaxe", "tool_pickaxe_upgraded", "tool_shovel", "tool_shovel_upgraded", "top", "top_large", "top_large_checkerboard", "tower_base", "tower_hexagon_base", "tower_hexagon_mid", "tower_hexagon_roof", "tower_hexagon_roof_secondary", "tower_hexagon_top", "tower_hexagon_top_wood", "tower_slant_roof", "tower_square", "tower_square_arch", "tower_square_base", "tower_square_base_border", "tower_square_base_color", "tower_square_mid", "tower_square_mid_color", "tower_square_mid_door", "tower_square_mid_open", "tower_square_mid_open_simple", "tower_square_mid_windows", "tower_square_roof", "tower_square_top", "tower_square_top_color", "tower_square_top_roof", "tower_square_top_roof_high", "tower_square_top_roof_high_windows", "tower_square_top_roof_rounded", "tower_top", "traffic_light", "traffic_light_hanging", "traffic_light_object_hanging", "traffic_light_object_horizontal", "traffic_light_object_vertical", "trap", "tree_autumn", "tree_autumn_tall", "tree_autumn_trunk", "tree_crooked", "tree_high", "tree_high_crooked", "tree_high_round", "tree_large", "tree_log", "tree_log_small", "tree_small", "tree_tall", "tree_trunk", "trunk", "trunk_long", "urn_round", "urn_square", "wall", "wall_arch", "wall_arch_top", "wall_arch_top_detail", "wall_block", "wall_block_half", "wall_broken", "wall_corner", "wall_corner_detail", "wall_corner_diagonal", "wall_corner_diagonal_half", "wall_corner_edge", "wall_corner_half", "wall_corner_half_tower", "wall_corner_slant", "wall_curved", "wall_detail_cross", "wall_detail_diagonal", "wall_detail_horizontal", "wall_diagonal", "wall_door", "wall_doorway", "wall_doorway_base", "wall_doorway_round", "wall_doorway_square", "wall_doorway_square_wide", "wall_doorway_square_wide_curved", "wall_half", "wall_half_modular", "wall_narrow", "wall_narrow_corner", "wall_narrow_gate", "wall_narrow_stairs", "wall_narrow_stairs_rail", "wall_narrow_wood", "wall_narrow_wood_fence", "wall_opening", "wall_pillar", "wall_rounded", "wall_side", "wall_slope", "wall_stud", "wall_to_narrow", "wall_window_glass", "wall_window_round", "wall_window_shutters", "wall_window_small", "wall_window_stone", "wall_wood", "wall_wood_arch", "wall_wood_arch_top", "wall_wood_arch_top_detail", "wall_wood_block", "wall_wood_block_half", "wall_wood_broken", "wall_wood_corner", "wall_wood_corner_diagonal", "wall_wood_corner_diagonal_half", "wall_wood_corner_edge", "wall_wood_curved", "wall_wood_detail_cross", "wall_wood_detail_diagonal", "wall_wood_detail_horizontal", "wall_wood_diagonal", "wall_wood_door", "wall_wood_doorway_base", "wall_wood_doorway_round", "wall_wood_doorway_square", "wall_wood_doorway_square_wide", "wall_wood_doorway_square_wide_curved", "wall_wood_half", "wall_wood_rounded", "wall_wood_side", "wall_wood_slope", "wall_wood_window_glass", "wall_wood_window_round", "wall_wood_window_shutters", "wall_wood_window_small", "wall_wood_window_stone", "warning_orange", "warning_traffic", "water_tower", "watermill", "watermill_wide", "weapon_arrow", "weapon_bow", "weapon_spear", "weapon_sword", "wheel", "wheelchair", "wheelchair_deluxe", "wheelchair_power", "wheelchair_power_deluxe", "windmill", "windmill_low", "wood_structure", "wood_support", "workbench", "workbench_anvil", "workbench_grind"];
const EDGE_PX = 16;
const RECONNECT_BASE_MS = 1000;
const RECONNECT_CAP_MS = 16000;
const RECONNECT_MAX_ATTEMPTS = 6;
const SIM_TICKS_PER_SECOND = 16;
const SIM_TICK_S = 1 / SIM_TICKS_PER_SECOND;
const SIM_TICK_MS = 1000 * SIM_TICK_S;
const HOST_POLL_MS = 10;
const WIRE_SCALE = 1000;
const ROSTER_EVERY_TICKS = 10;
const U16_MAX = 0xffff;

const params = new URLSearchParams(location.search);
const relayUrls = params.get("relay") ? params.get("relay").split(",") : null;
const namespace = params.get("ns") || "oscraft";
const statusEl = document.getElementById("status");
const canvas = document.getElementById("view");
const room = location.hash.slice(1) || "lobby";
const pkgUrl = new URL(document.body.dataset.pkg, location.href).href;
// The #moba page loads the moba scenario image, the default page loads the duel map image.
const mapUrl = new URL(location.hash === "#moba" ? "moba.bin" : document.body.dataset.map, location.href).href;

let mapGrid = null;
// The WebGL2 renderer and its orbit camera; created once the WebGL2 context exists.
let renderer3d = null;
let mapBytes = null;
let terrainMesh = null;
let waterMesh = null;
// Splat ground: the control map, the four layers and the GPU handles. Built before the match starts; a layer that fails is an error.
let splat = null;
const SPLAT_BAND = 0.4;
// Sun shadows: one setting. ?shadows=0 turns the shadow map off (no shadow pass, no sampling).
const SHADOW_ON = params.get("shadows") !== "0";
const SHADOW_SIZE = 2048;
// Ground lighting: ambient, diffuse and albedo gain. The gain lifts the owner's darker albedo to lit terrain.
const SPLAT_SHADE = [0.42, 0.68, 1.3];
// Per-layer albedo tint, in SPLAT_LAYERS order. The sand (dirt) layer takes the dirt tint so it sits with the grass family;
// the other layers keep their own colour.
const SPLAT_TINTS = SPLAT_LAYERS.map((name) => (name === "dirt" ? TILE_TINTS.dirt : [1, 1, 1]));
// Water tint: multiplies the procedural water colour (1 leaves it as shaded).
const SPLAT_WATER_TINT = [1, 1, 1];
let terrainTiles = null;
let unitMeshes = [];
let arrowMesh = null;
const drawCounts = { terrain: 0, splat: 0, doodad: 0, unit: 0, animated: 0 };
// Every asset that failed to load or build, by asset name (and the unit types with no model). Shown on the status line
// and in body.dataset.assetError; a failed required asset (a hero model, the splat ground, the arrow) stops the start.
const assetErrors = new Map();
// Unit instances drawn into the sun's shadow map this frame (the dynamic casters), published for the browser probe.
let shadowUnitCount = 0;
// Scratch matrices for the per-unit transform and the screen projection, reused every frame.
const unitTransform = new Float32Array(16);
const worldVp = new Float32Array(16);
// Doodads grouped per mesh into static instance batches, and the per-frame batches of unskinned units per mesh.
let doodadBatches = [];
const unitBatches = new Map();
const MOVE_EPS = 1e-4;
const MAX_FRAME_DT_S = 0.1;
// Per-unit clip players keyed by unit id and type, with the last seen position (walking vs standing still).
const animators = new Map();
let lastDrawMs = null;
const peers = new Set();
let netState = null;
let casts = 0;
let attacks = 0;
let localGame = null;
let hud = null;
let hud5 = null;
let fogview = null;
const heroPanels = [];
let fx = null;
let selectedId = -1;
let reconnectAttempt = 0;
let reconnectTimer = 0;
let drag = null;
let hosting = false;
let orders = 0;
let pointer = null;
let lastFrameMs = 0;
let localTickAcc = 0;
const keysDown = new Set();
const seating = createSeating();
const autoSeats = [];
let seatsExplicit = false;
let encoder = createStateEncoder();
let myPlayer = 0;
let lastAppliedTick = 0;
let rosterView = null;
let heroKinds = new Map();
let heroRuleNames = [];
let spawnedSeats = [];
const localPlayerId = () => myPlayer;
const isGuest = () => netTick >= 0;

function setStatus(extra) {
  const mode = netState ? "net" : "local";
  const errors = assetErrors.size ? `\nasset errors:\n${[...assetErrors].map(([name, message]) => `${name}: ${message}`).join("\n")}` : "";
  statusEl.textContent = `room ${room}\npeers ${peers.size} (${mode})\n${extra || ""}${errors}`;
}

// A failed asset is named on the page: console error, the status line, and body.dataset.assetError. Each name is recorded once.
function recordAssetError(name, err) {
  if (assetErrors.has(name)) return;
  const message = err && err.message ? err.message : String(err);
  assetErrors.set(name, message);
  console.error(`[asset] ${name}: ${message}`);
  document.body.dataset.assetError = [...assetErrors].map(([n, m]) => `${n}: ${m}`).join("; ");
  setStatus("assets failed to load");
}

// A start that cannot run (no WebGL2, a required asset) shows its reason on the page and does not continue.
function failStart(err) {
  const message = err && err.message ? err.message : String(err);
  console.error(`[start] ${message}`);
  document.body.dataset.startError = message;
  setStatus(`start failed: ${message}`);
}

async function fetchJson(name) {
  const res = await fetch(new URL(name, location.href));
  if (!res.ok) throw new Error(`${name} HTTP ${res.status}`);
  return res.json();
}

async function loadSim() {
  const mod = await import(/* @vite-ignore */ pkgUrl);
  await mod.default();
  const image = new Uint8Array(await (await fetch(mapUrl)).arrayBuffer());
  mapBytes = image;
  setMapSize(readGrid(image));
  const game = new mod.Game(image);
  // Hero types and their attribute and respawn rules come from the same text the server loads. Without them
  // every hero's attributes are zero at level 0, so heroes are not seen as heroes. The lobby's picker lists the
  // names on every page, so the text is always read for them; only the moba page hands it to the sim.
  const rulesRes = await fetch(new URL("hero_rules.txt", location.href));
  if (!rulesRes.ok) throw new Error(`hero_rules.txt HTTP ${rulesRes.status}`);
  const rulesText = await rulesRes.text();
  if (location.hash === "#moba" && !game.set_hero_rules(rulesText)) throw new Error("hero_rules.txt was refused by the sim");
  heroRuleNames = heroNamesFromRules(rulesText);
  // Fog of war is on for every page, moba included: enemies outside the local team's vision stay hidden.
  game.set_fog(true);
  return game;
}

// The map's grid sets the playfield: orbit origin, default and maximum camera distance scale with its larger side.
function setMapSize(grid) {
  mapGrid = grid;
  W = grid.width;
  H = grid.height;
  const [x0, z0, x1, z1] = walkableBounds(grid);
  cameraBounds = [x0, z0, x1, z1];
  orbitOrigin = [(x0 + x1) / 2, 0, (z0 + z1) / 2];
  cameraDistance = Math.min(CAMERA_PLAY_DISTANCE, CAMERA_DISTANCE_PER_TILE * Math.max(W, H));
  cameraMaxDistance = Math.max(CAMERA_MAX_FLOOR, CAMERA_MAX_PER_TILE * Math.max(W, H));
}

// Skin, node tree and clips for a skinned GLB, with its rest-pose geometry (the joint matrices carry the fit).
// Null when the model has no clips or its skin has more joints than the shader palette.
// The fit is lowered so the rig's lowest skinned vertex stands on y = 0 in its idle, walk and run poses (the fit's own
// floor is the raw first mesh, which the bind pose does not match).
function skinnedRig(animations, glb, fit) {
  const { clips, skins, nodes } = animations;
  const skin = skins[0];
  if (!skin || clips.length === 0 || skin.joints.length > MAX_JOINTS) return null;
  const grounding = clips.filter((clip) => /^(idle|walk|run)/.test(clip.name));
  const grounded = Float32Array.from(fit);
  grounded[13] -= skinFloor(nodes, skin, fit, glb.positions, glb.joints, glb.weights, grounding.length ? grounding : clips);
  return { clips, skin, nodes, fit: grounded, positions: glb.positions, normals: glb.normals || vertexNormals(glb.positions, glb.indices), joints: glb.joints, weights: glb.weights };
}

// Humanoid clip library (anim/library.json, extracted from the spoint animation set). Shared by every rig.
let animLibrary = null;
function humanoidLibrary() {
  if (!animLibrary) animLibrary = fetch(new URL("anim/library.json", location.href)).then((r) => r.json()).catch((error) => { animLibrary = null; throw error; });
  return animLibrary;
}

// A rig without embedded clips is animated from the humanoid library through its bone map (anim/maps/<model>.json).
// A rig with no map, or a map that fails to load, is an error for that model: it is never drawn in its rest pose.
async function libraryClips(name, nodes) {
  const [map, library] = await Promise.all([fetchJson(`anim/maps/${name}.json`), humanoidLibrary()]);
  return retargetLibrary(library, map, nodes);
}

// Each model name is fetched and parsed once: unit types and doodad kinds that name the same GLB share one parse,
// and every fit reads from it. A failed load is dropped from the cache so a later call may retry it.
const parsedModels = new Map();
function parsedModel(name) {
  let entry = parsedModels.get(name);
  if (!entry) {
    entry = (async () => {
      const url = new URL(`models/${name}.glb`, location.href);
      const res = await fetch(url);
      if (!res.ok) throw new Error(`models/${name}.glb HTTP ${res.status}`);
      const bytes = await resolveGlbExtensions(await res.arrayBuffer());
      const glb = parseGlb(bytes);
      const animations = glb.joints && glb.weights ? parseGlbAnimations(bytes) : null;
      if (animations && animations.clips.length === 0) animations.clips = await libraryClips(name, animations.nodes);
      return { url: url.href, glb, animations };
    })();
    parsedModels.set(name, entry);
    entry.catch(() => parsedModels.delete(name));
  }
  return entry;
}

// Loads one GLB by name and returns its fitted model. A missing or unreadable file, or a rig with no clips or too many
// joints, throws with the asset name: nothing is drawn in its place.
async function fetchModel(name, size) {
  try {
    const { url, glb, animations } = await parsedModel(name);
    const fitted = fitModel(glb, size);
    const animation = animations ? skinnedRig(animations, glb, fitted.fit) : null;
    if (animations && !animation) throw new Error("rig has no clips or more joints than the shader palette");
    return { ...fitted, glbUrl: url, animation };
  } catch (err) {
    throw new Error(`model ${name}: ${err && err.message ? err.message : err}`);
  }
}

// One fitted model per unit type, indexed like the wasm unit_types export. A type with no model entry, or a model that
// fails to load, is null: the unit type is then reported on the page once and not drawn. A hero that has no model, or
// whose model fails, throws, so the match does not start. A model with its own size in doodad_sizes.json (Roshan,
// at hero heights) is fitted to it; every other unit is fitted to a hero footprint (UNIT_HALF, the design rule that
// scripts/doodad-sizes.mjs also states).
async function loadModels(game) {
  if (typeof game.unit_model_names !== "function") throw new Error("the wasm build has no unit_model_names export");
  const sizes = await fetchJson("doodad_sizes.json");
  // Heroes (player-controlled unit types) are drawn at HERO_DRAW_SCALE times their fitted size: the owner's intended hero size is twice the fit.
  const heroFlags = typeof game.unit_hero_kinds === "function" ? game.unit_hero_kinds() : [];
  const typeNames = typeof game.unit_type_names === "function" ? game.unit_type_names() : [];
  return Promise.all(game.unit_model_names().map(async (name, type) => {
    const hero = heroFlags[type] === 1;
    if (!name) {
      if (hero) throw new Error(`unit ${typeNames[type] ?? type}: hero has no model`);
      recordAssetError(`unit ${typeNames[type] ?? type}`, new Error("has no model"));
      return null;
    }
    const size = (sizes[name] ?? 2 * UNIT_HALF) * (hero ? HERO_DRAW_SCALE : 1);
    try {
      return await fetchModel(name, size);
    } catch (err) {
      if (hero) throw err;
      recordAssetError(name, err);
      return null;
    }
  }));
}

// Doodads: one fitted model per kind code (DODA kind). The asset catalogue (assets.json) names a kind's GLB when it has
// an entry; doodads.json maps every kind to its GLB path. A placed kind whose model or footprint is missing is recorded
// on the page and its placements are not drawn.
async function loadDoodadModels(usedKinds) {
  const [catalogue, paths, footprints] = await Promise.all([fetchJson("assets.json"), fetchJson("doodads.json"), fetchJson("doodad_sizes.json")]);
  const toStem = (file) => file.replace(/^(js\/client\/)?models\//, "").replace(/\.glb$/, "");
  const modelName = (id) => {
    const file = catalogue.items.find((item) => item.id === id && item.kind === "model")?.path;
    return file ? toStem(file) : null;
  };
  // Each model is fitted to its footprint in cells from doodad_sizes.json (scripts/doodad-sizes.mjs).
  const sizeOf = (name) => footprints[name];
  // Only kinds the map places are fetched; an unplaced kind keeps an empty entry, so indices still match the kind codes.
  return Promise.all(DOODAD_KINDS.map(async (kind, code) => {
    if (usedKinds && !usedKinds.has(code)) return { high: null, low: null };
    const name = modelName(kind) ?? (paths[kind] ? toStem(paths[kind]) : null);
    if (!name) {
      recordAssetError(`doodad ${kind}`, new Error("no model path in assets.json or doodads.json"));
      return { high: null, low: null };
    }
    const lowName = modelName(`${kind}-low`);
    const load = (model) => {
      if (sizeOf(model) === undefined) {
        recordAssetError(model, new Error("no footprint in doodad_sizes.json"));
        return null;
      }
      return modelOrRecord(model, fetchModel(model, sizeOf(model)));
    };
    const [high, low] = await Promise.all([load(name), lowName ? load(lowName) : null]);
    return { high, low };
  }));
}

// Records a model that fails and resolves to null, so only that asset is missing from the page.
function modelOrRecord(name, promise) {
  return promise.catch((err) => {
    recordAssetError(name, err);
    return null;
  });
}

// Uploads a fitted model as a drawable mesh. Its base colour texture is uploaded when the model has one; a texture
// that does not load throws. Units pass animated: their skinned rig replaces the fitted geometry, which the joint matrices carry instead.
async function toMesh(gl, fitted, fallbackColor, { animated = false } = {}) {
  if (!fitted) return null;
  const rig = animated ? fitted.animation : null;
  const mesh = rig
    ? { positions: rig.positions, normals: rig.normals, uvs: fitted.uvs, indices: fitted.indices, joints: rig.joints, weights: rig.weights, animation: rig, color: fallbackColor, height: maxY(fitted.positions) }
    : { positions: fitted.positions, normals: fitted.normals, uvs: fitted.uvs, indices: fitted.indices, color: fallbackColor, height: maxY(fitted.positions) };
  if (fitted.baseColorTexture) {
    const image = await loadBaseColorImage(fitted.baseColorTexture, fitted.glbUrl);
    if (!image) throw new Error(`base colour texture of ${fitted.glbUrl} did not load`);
    mesh.texture = uploadTexture(gl, image);
  }
  return mesh;
}

function maxY(positions) {
  let top = 0;
  for (let v = 1; v < positions.length; v += 3) top = Math.max(top, positions[v]);
  return top;
}

// The ranged attack's projectile: the arrow model, fitted to the footprint doodad_sizes.json gives it. Required: a
// failed arrow stops the start rather than drawing the projectile as something else.
async function loadArrowMesh(gl) {
  const footprints = await fetchJson("doodad_sizes.json");
  if (footprints.arrow === undefined) throw new Error("model arrow: no footprint in doodad_sizes.json");
  return toMesh(gl, await fetchModel("arrow", footprints.arrow), [1, 1, 1]);
}

// Ground height under a tile position; the terrain is flat at 0 until its mesh is built.
function groundAt(x, z) {
  return terrainTiles ? groundY(terrainTiles, x, z) : 0;
}

// Adds `top`, the world height of the unit's model top, so its health bar sits above the model.
function anchored(u) {
  const model = unitMeshes[u.type];
  return { ...u, top: groundAt(u.x, u.z) + (model ? model.height : 0) };
}

function currentUnits() {
  if (netState) return netState.units.map(anchored);
  const out = simUnits();
  return fogview ? fogview.filter(out) : out;
}

// Every living unit of the local sim, unfiltered by fog: the host's view for seats, stats and command ownership.
function simUnits() {
  if (!localGame) return [];
  const flat = localGame.units();
  const ids = localGame.unit_ids();
  // Older wasm builds have no unit_types or unit_hps export: their type is unknown (-1), so no model is drawn for them and the page reports it.
  const types = typeof localGame.unit_types === "function" ? localGame.unit_types() : null;
  const hps = typeof localGame.unit_hps === "function" ? localGame.unit_hps() : null;
  // Older wasm builds lack unit_attacking; their units never report an attack clip.
  const attacking = typeof localGame.unit_attacking === "function" ? localGame.unit_attacking() : null;
  const out = [];
  for (let i = 0; i < ids.length; i++) {
    out.push(anchored({ id: ids[i], x: flat[3 * i], z: flat[3 * i + 1], player: flat[3 * i + 2], type: types ? types[i] : -1, hp: hps ? hps[i] : -1, attacking: attacking ? attacking[i] === 1 : false }));
  }
  return out;
}

// Draw counts per mesh kind for the current frame, published on the body for the browser probe.
function countDraw(kind, mesh, transform, joints = null) {
  drawCounts[kind] += 1;
  if (joints) renderer3d.drawSkinned(mesh, transform, joints);
  else renderer3d.drawMesh(mesh, transform);
}

// Unit meshes are cached per (type, colour key): a textured model keeps its texture whatever the colour. Callers pass
// only a type that has a model (collectUnitDraws reports the rest).
const unitMeshCache = new Map();
function unitMeshFor(type, color) {
  const key = `${type}:${color.join(",")}`;
  let mesh = unitMeshCache.get(key);
  if (!mesh) {
    mesh = { ...unitMeshes[type], color };
    unitMeshCache.set(key, mesh);
  }
  return mesh;
}

// A unit type with no model is reported once (console and status line) and its units are not drawn.
function reportMissingModel(type) {
  const names = localGame && typeof localGame.unit_type_names === "function" ? localGame.unit_type_names() : [];
  recordAssetError(`unit ${names[type] ?? type}`, new Error(`type ${type} has no model`));
}

// Joint palette for a unit this frame: its clip (walk when it moved, attack when it reports attacking, else idle)
// advanced by dt seconds. A unit with no matching clip holds the rest pose.
const EMPTY_POSE = new Map();
// Turn rate of a rigged unit toward its heading, in radians per second scale (the step is eased by dt).
const TURN_RATE = 10;
// Walk and run clip rate bounds, and the ease (per second) of the draw-side speed the rate follows.
const RATE_MIN = 0.25;
const RATE_MAX = 4;
const SPEED_EASE = 8;
function wrapAngle(a) {
  return ((a + Math.PI) % (2 * Math.PI) + 2 * Math.PI) % (2 * Math.PI) - Math.PI;
}
function jointsFor(u, mesh, dt) {
  const key = `${u.id}:${u.type}`;
  let entry = animators.get(key);
  if (!entry) {
    const { skin } = mesh.animation;
    entry = { player: new AnimationPlayer(mesh.animation.clips), x: u.x, z: u.z, seen: false, heading: 0, yaw: 0, speed: 0, cycle: {}, joints: new Float32Array(16 * skin.joints.length) };
    animators.set(key, entry);
  }
  const { nodes, skin, fit } = mesh.animation;
  // Heading comes from the draw-side step since the last frame; an attacking unit keeps its heading (no target is exposed).
  const dx = u.x - entry.x;
  const dz = u.z - entry.z;
  const step = Math.hypot(dx, dz);
  const moving = step > MOVE_EPS;
  if (moving) entry.heading = Math.atan2(dx, dz);
  const turn = wrapAngle(entry.heading - entry.yaw);
  entry.yaw += turn * Math.min(1, dt * TURN_RATE);
  // Draw-side speed, eased so one frame's jitter does not change the clip's rate.
  entry.speed += ((dt > 0 ? step / dt : 0) - entry.speed) * Math.min(1, dt * SPEED_EASE);
  entry.x = u.x;
  entry.z = u.z;
  entry.seen = true;
  entry.player.playState({ attacking: Boolean(u.attacking), moving });
  // A walk or run cycle plays at the unit's speed over the gait's own speed, so its feet keep pace with the ground.
  const clip = entry.player.clip;
  if (clip && entry.cycle[clip.name] === undefined) entry.cycle[clip.name] = cycleSpeed(nodes, fit, clip);
  const cycle = clip ? entry.cycle[clip.name] : 0;
  entry.player.rate = moving && !u.attacking && cycle > 0 ? Math.min(RATE_MAX, Math.max(RATE_MIN, entry.speed / cycle)) : 1;
  entry.player.update(dt);
  return jointMatrices(nodes, skin, entry.player.pose() ?? EMPTY_POSE, fit, entry.joints);
}

// Groups { mesh, lowMesh?, transform } placements into one static instance batch per mesh. lowMesh is the mesh's
// lower-poly model (shared by every placement of that mesh) for the renderer's level-of-detail path.
function batchByMesh(placed) {
  const groups = new Map();
  for (const p of placed) {
    let list = groups.get(p.mesh);
    if (!list) groups.set(p.mesh, (list = { lowMesh: p.lowMesh ?? null, transforms: [] }));
    list.transforms.push(p.transform);
  }
  return [...groups].map(([mesh, { lowMesh, transforms }]) => {
    const matrices = new Float32Array(transforms.length * 16);
    transforms.forEach((t, i) => matrices.set(t, i * 16));
    return { mesh, lowMesh, batch: { matrices, count: transforms.length, dirty: true } };
  });
}

// Appends a unit transform to its mesh's per-frame batch; flushUnitBatches draws one call per mesh.
function queueUnit(mesh, transform) {
  let entry = unitBatches.get(mesh);
  if (!entry) unitBatches.set(mesh, (entry = { matrices: new Float32Array(16 * 64), count: 0, dirty: true }));
  if ((entry.count + 1) * 16 > entry.matrices.length) {
    const grown = new Float32Array(entry.matrices.length * 2);
    grown.set(entry.matrices);
    entry.matrices = grown;
  }
  entry.matrices.set(transform, entry.count * 16);
  entry.count += 1;
}

function flushUnitBatches() {
  for (const [mesh, entry] of unitBatches) {
    entry.dirty = true;
    if (entry.count > 0) renderer3d.drawInstanced(mesh, entry);
    entry.count = 0;
  }
}

// Collects this frame's visible unit draws once: each animator advances here, and the list is replayed by the shadow
// pass and the lit pass. The transform is copied because the scratch matrix is reused by the next unit.
function collectUnitDraws(units, dt) {
  const list = [];
  const clips = [];
  const yaws = [];
  for (const entry of animators.values()) entry.seen = false;
  for (const raw of units) {
    const u = drawnPosition(raw);
    if (!unitMeshes[u.type]) {
      reportMissingModel(u.type);
      continue;
    }
    const color = u.id === selectedId ? SELECT_COLOR : PLAYER_COLORS[u.player] || PLAYER_COLORS[0];
    const mesh = unitMeshFor(u.type, color);
    if (!renderer3d.isVisible(u.x, u.z, meshReach(mesh))) {
      // Off screen: no draw and no clip advance, but its animator survives so the clip resumes where it stopped.
      const kept = animators.get(`${u.id}:${u.type}`);
      if (kept) kept.seen = true;
      continue;
    }
    const transform = new Float32Array(translation(u.x, groundAt(u.x, u.z), u.z, unitTransform));
    if (mesh.animation) {
      const joints = jointsFor(u, mesh, dt);
      const entry = animators.get(`${u.id}:${u.type}`);
      const facing = new Float32Array(multiply(translation(u.x, groundAt(u.x, u.z), u.z), rotationY(entry.yaw)));
      list.push({ kind: "unit", mesh, transform: facing, joints });
      clips.push(`${u.id}:${entry.player.clip?.name ?? ""}:${entry.player.rate.toFixed(2)}`);
      yaws.push(`${u.id}:${entry.yaw.toFixed(2)}`);
    } else list.push({ kind: "unit", mesh, transform, joints: null });
  }
  for (const [key, entry] of animators) if (!entry.seen) animators.delete(key);
  // The clip each rigged unit plays this frame, for the browser probe.
  document.body.dataset.animClips = clips.join(",");
  document.body.dataset.tickAlpha = tickShare().toFixed(3);
  document.body.dataset.animYaw = yaws.join(",");
  return list;
}

// Draw-only interpolation between sim ticks. Each unit glides from its position at the tick before to its position at
// the latest tick, over one tick of wall time, so the sim's positions (and everything that reads them) stay unchanged.
const tickClock = new Map();
let tickStampMs = null;
function advanceTickClock(units) {
  const next = new Map();
  for (const u of units) {
    const old = tickClock.get(u.id);
    next.set(u.id, { px: old ? old.x : u.x, pz: old ? old.z : u.z, x: u.x, z: u.z });
  }
  tickClock.clear();
  for (const [id, v] of next) tickClock.set(id, v);
  tickStampMs = performance.now();
}

// The unit with its draw position: between its previous and latest tick positions by the share of a tick elapsed.
function tickShare() {
  return tickStampMs === null ? 0 : Math.min(1, Math.max(0, (performance.now() - tickStampMs) / SIM_TICK_MS));
}

function drawnPosition(u) {
  const c = tickClock.get(u.id);
  if (!c || tickStampMs === null) return u;
  const a = tickShare();
  return { ...u, x: c.px + (c.x - c.px) * a, z: c.pz + (c.z - c.pz) * a };
}

// Draws a collected unit list. The shadow pass (shadow = true) re-issues the same meshes with the light's matrix and
// returns the number of unit instances it cast; the lit pass counts per category as before.
function emitUnitDraws(list, shadow) {
  let casters = 0;
  for (const d of list) {
    if (shadow) casters += 1;
    if (d.joints) {
      if (shadow) renderer3d.drawSkinned(d.mesh, d.transform, d.joints);
      else {
        countDraw(d.kind, d.mesh, d.transform, d.joints);
        drawCounts.animated += 1;
      }
    } else {
      if (!shadow) drawCounts[d.kind] += 1;
      queueUnit(d.mesh, d.transform);
    }
  }
  flushUnitBatches();
  return casters;
}

// Creep: sets one cell's ground kind (grass, dirt or lane) by rewriting its control texel. Only that texel is uploaded;
// the terrain mesh and the tile batches are not rebuilt. Index is y * width + x in map cells.
window.__setCellKind = (index, kind) => {
  if (!splat || !renderer3d) return false;
  renderer3d.updateSplatCell(splat, index, setSplatTexel(splat.control, index, kind));
  return true;
};

// Terrain and doodads: the casters of the sun's shadow map as well as the lit draws.
function drawGround3d() {
  // The water is drawn on the terrain itself (the splat shader's water blend), so the water quads are not drawn on top of
  // it: at the banks they overlapped the terrain and showed the same ground twice.
  renderer3d.drawSplat(terrainMesh, splat);
  drawCounts.splat += 1;
  drawCounts.terrain += 1;
  for (const d of doodadBatches) {
    drawCounts.doodad += renderer3d.drawInstancedLod(d.mesh, d.batch, d.lowMesh);
  }
}

function drawWorld3d() {
  drawCounts.terrain = 0;
  drawCounts.splat = 0;
  drawCounts.doodad = 0;
  drawCounts.unit = 0;
  drawCounts.animated = 0;
  const now = performance.now();
  const dt = lastDrawMs === null ? 0 : Math.min((now - lastDrawMs) / 1000, MAX_FRAME_DT_S);
  lastDrawMs = now;
  renderer3d.beginFrame(BACKGROUND);
  const units = currentUnits();
  const unitDraws = collectUnitDraws(units, dt);
  // Static casters (terrain, doodads, buildings, towers) are drawn into the cached layer only when it is dirty; the
  // dynamic casters (units) are drawn into the working map every frame, on top of that layer.
  const shadow = renderer3d.beginShadowPass ? renderer3d.beginShadowPass() : null;
  shadowUnitCount = 0;
  if (shadow) {
    const counted = { ...drawCounts };
    if (shadow.staticDirty) drawGround3d();
    renderer3d.beginShadowUnits();
    shadowUnitCount = emitUnitDraws(unitDraws, true);
    renderer3d.endShadowPass();
    Object.assign(drawCounts, counted);
  }
  drawGround3d();
  emitUnitDraws(unitDraws, false);
  // window.__fxOff is the CI probe switch that measures the frame cost of the effects.
  if (fx && window.__fxOff !== true) fx.frame(units, dt);
  renderer3d.endFrame();
}

// Arrow keys and the pointer at the window edge pan the orbit camera; the speed is in pixels per second, so
// the world distance per second scales with the camera distance. A drag pans by itself, so edge scroll waits.
function panByKeys(dtS) {
  if (!renderer3d) return;
  let dx = 0;
  let dz = 0;
  if (keysDown.has("ArrowLeft")) dx -= 1;
  if (keysDown.has("ArrowRight")) dx += 1;
  if (keysDown.has("ArrowUp")) dz -= 1;
  if (keysDown.has("ArrowDown")) dz += 1;
  if (pointer && !drag) {
    if (pointer.x < EDGE_PX) dx -= 1;
    else if (pointer.x > window.innerWidth - EDGE_PX) dx += 1;
    if (pointer.y < EDGE_PX) dz -= 1;
    else if (pointer.y > window.innerHeight - EDGE_PX) dz += 1;
  }
  if (dx === 0 && dz === 0) return;
  // pan() moves the world with the pixel delta (a drag), so a key that moves the view right passes a negative delta.
  renderer3d.pan(-dx * PAN_PX_PER_S * dtS, -dz * PAN_PX_PER_S * dtS);
}

function resize() {
  const dpr = window.devicePixelRatio || 1;
  canvas.width = Math.floor(window.innerWidth * dpr);
  canvas.height = Math.floor(window.innerHeight * dpr);
}

// Ground point (x, z) under a client pixel: the view ray intersected with the terrain surface.
function screenToWorld(clientX, clientY) {
  const cam = renderer3d.camera;
  const eye = eyeOf(cam);
  const fwd = normalize(sub(cam.target, eye));
  const right = normalize(cross(fwd, [0, 1, 0]));
  const up = cross(right, fwd);
  const t = Math.tan(cam.fovY / 2);
  const aspect = window.innerWidth / Math.max(1, window.innerHeight);
  const ndcX = (2 * clientX) / window.innerWidth - 1;
  const ndcY = 1 - (2 * clientY) / window.innerHeight;
  const dir = [0, 1, 2].map((i) => fwd[i] + right[i] * ndcX * t * aspect + up[i] * ndcY * t);
  // Intersect the view ray with the terrain surface: start at y = 0 and re-intersect at the ground height found.
  let y = 0;
  let point = null;
  for (let i = 0; i < 4; i++) {
    const s = (y - eye[1]) / dir[1];
    point = { x: eye[0] + dir[0] * s, z: eye[2] + dir[2] * s };
    y = groundAt(point.x, point.z);
  }
  return point;
}

function sub(a, b) { return [a[0] - b[0], a[1] - b[1], a[2] - b[2]]; }
function cross(a, b) { return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]; }
function normalize(v) { const n = Math.hypot(v[0], v[1], v[2]) || 1; return [v[0] / n, v[1] / n, v[2] / n]; }

function pick(world) {
  let best = null;
  let bestD = PICK_RADIUS * PICK_RADIUS;
  for (const u of currentUnits()) {
    const d = (u.x - world.x) ** 2 + (u.z - world.z) ** 2;
    if (d <= bestD) { bestD = d; best = u; }
  }
  return best;
}

// Client pixel of the world point (x, y, z), y defaulting to the ground: the inverse of screenToWorld. Points behind the camera map far off screen.
function worldToScreen(x, z, y = 0) {
  const clip = worldClip(x, z, y);
  if (clip[3] <= 0) return { x: -1e6, y: -1e6 };
  return {
    x: ((clip[0] / clip[3] + 1) / 2) * window.innerWidth,
    y: ((1 - clip[1] / clip[3]) / 2) * window.innerHeight,
  };
}

// Homogeneous clip coordinates [X, Y, Z, W] of the world point; W <= 0 is behind the camera plane. The fog overlay cuts cells at W so none is projected through the camera.
function worldClip(x, z, y = 0) {
  const vp = viewProjection(renderer3d.camera, window.innerWidth / Math.max(1, window.innerHeight), worldVp);
  return [0, 1, 2, 3].map((r) => vp[r] * x + vp[4 + r] * y + vp[8 + r] * z + vp[12 + r]);
}

// The commands createHud routes input through. Each runs on the host sim, or sends the frame to the host
// when this page is a guest; each returns whether the sim (or the host's send) accepted it.
// A command the local sim refused leaves the sim's refusal name on the page (body.dataset.lastRefusal) for the CI witness.
// An accepted command clears the page's last refusal, so lastRefusal always names the most recent command's outcome.
function noteLocalRefusal(ok, what) {
  if (ok) delete document.body.dataset.lastRefusal;
  else document.body.dataset.lastRefusal = `${what} ${localGame.last_refusal()}`;
  return ok;
}

const commands = {
  order(unitId, x, z) {
    const ok = netState ? (session.broadcast(sendOrder(unitId, x, z)), true) : noteLocalRefusal(localGame.order_move(unitId, x, z), `order_refused unit=${unitId}`);
    if (ok) { orders += 1; publishCommandCounts(); }
    setStatus(`order move unit ${unitId} to ${x.toFixed(2)},${z.toFixed(2)}${netState ? "" : " (local)"}`);
    return ok;
  },
  attackMove(unitId, x, z) {
    const ok = netState ? (session.broadcast(sendAttackMove(unitId, x, z)), true) : noteLocalRefusal(localGame.attack_move(unitId, x, z), `attack_move_refused unit=${unitId}`);
    if (ok) { orders += 1; publishCommandCounts(); }
    setStatus(`attack-move unit ${unitId} to ${x.toFixed(2)},${z.toFixed(2)}${netState ? "" : " (local)"}`);
    return ok;
  },
  attack(unitId, targetId) {
    const ok = netState ? (session.broadcast(sendAttack(unitId, targetId)), true) : noteLocalRefusal(localGame.attack(unitId, targetId), `attack_refused unit=${unitId}`);
    if (ok) { attacks += 1; publishCommandCounts(); }
    return ok;
  },
  cast(unitId, slot, target, x, z) {
    const ok = netState ? (session.broadcast(sendCast(unitId, slot, target, x, z)), true) : noteLocalRefusal(localGame.cast(unitId, slot, target, x, z), `cast_refused unit=${unitId} slot=${slot}`);
    if (ok) { casts += 1; publishCommandCounts(); }
    if (ok && fx) fx.cast(unitId, slot, target, x, z);
    return ok;
  },
  hold(unitId) {
    const ok = netState ? (session.broadcast(sendHold(unitId)), true) : noteLocalRefusal(localGame.hold(unitId), `hold_refused unit=${unitId}`);
    if (ok) { orders += 1; publishCommandCounts(); }
    return ok;
  },
  useItem(unitId, slot, target, x, z) {
    const ok = netState ? (session.broadcast(sendUseItem(unitId, slot, target, x, z)), true) : noteLocalRefusal(localGame.use_item(unitId, slot, target, Math.round(x), Math.round(z)), `item_refused unit=${unitId} slot=${slot}`);
    if (ok) { casts += 1; publishCommandCounts(); }
    return ok;
  },
};

function bindInput() {
  canvas.addEventListener("pointerdown", (e) => {
    drag = { x: e.clientX, y: e.clientY, moved: false };
    canvas.setPointerCapture(e.pointerId);
  });
  canvas.addEventListener("pointermove", (e) => {
    if (!drag) return;
    const dx = e.clientX - drag.x;
    const dy = e.clientY - drag.y;
    if (Math.hypot(dx, dy) > DRAG_SLOP_PX) drag.moved = true;
    if (!drag.moved) return;
    if (renderer3d) {
      // A drag pans the ground under the pointer; shift or the right button orbits instead.
      if (e.shiftKey || e.buttons === 2) renderer3d.orbit(dx, dy);
      else renderer3d.pan(dx, dy);
    }
    drag.x = e.clientX;
    drag.y = e.clientY;
  });
  canvas.addEventListener("pointerup", (e) => {
    const wasDrag = drag && drag.moved;
    drag = null;
    if (e.button === 2) document.body.dataset.rightUp = `${wasDrag ? "drag" : "click"} hud=${!!hud} local=${myPlayer}`;
    if (!wasDrag && hud) hud.onPointerUp(e);
  });
  canvas.addEventListener("contextmenu", (e) => e.preventDefault());
  window.addEventListener("keydown", (e) => {
    if (e.target instanceof HTMLInputElement) return;
    if (e.key.startsWith("Arrow")) {
      keysDown.add(e.key);
      e.preventDefault();
      return;
    }
    if (hud) hud.onKey(e.key, pointer);
  });
  window.addEventListener("keyup", (e) => keysDown.delete(e.key));
  window.addEventListener("blur", () => keysDown.clear());
  window.addEventListener("pointermove", (e) => { pointer = { x: e.clientX, y: e.clientY }; });
  document.documentElement.addEventListener("pointerleave", () => { pointer = null; });
  canvas.addEventListener("wheel", (e) => {
    e.preventDefault();
    if (renderer3d) renderer3d.zoom(e.deltaY);
  }, { passive: false });
  window.addEventListener("resize", resize);
}

// Browser host: this page owns the sim (localGame). Seats come from the lobby START frame (seating.start); before a
// START a joining peer takes the lowest free slot. A seat is kept by its key, so a reconnecting peer gets it back.
// Every SIM_TICK_MS the host ticks the sim and broadcasts that tick's state through the delta encoder (delta.js).
function autoSeat(peerPubkey) {
  if (seatsExplicit || seating.seat(peerPubkey).you !== SPECTATOR) return;
  const taken = new Set(autoSeats.map((s) => s.slot));
  for (let slot = 0; slot < MAX_PLAYERS; slot++) {
    if (!taken.has(slot)) {
      autoSeats.push({ slot, owner: peerPubkey, hero: "" });
      seating.start(autoSeats);
      return;
    }
  }
}

function sendTo(peerPubkey, bytes) {
  if (typeof session.send === "function") session.send(peerPubkey, bytes);
  else session.broadcast(bytes);
}

function sendRoster(peerPubkey) {
  sendTo(peerPubkey, encodeFrame(seating.roster(peerPubkey)));
}

function heroOf(slot) {
  return simUnits().find((u) => u.player === slot && isHeroUnit(u.id)) || null;
}

// The unit a seat starts with: a player-controlled type (the map's generic `hero` placeholder is one, though it has no
// hero attributes until a picked hero replaces it), so the lookup uses the type flag rather than the attributes.
function placeholderOf(slot) {
  const flags = typeof localGame.unit_hero_kinds === "function" ? localGame.unit_hero_kinds() : [];
  return simUnits().find((u) => u.player === slot && flags[u.type] === 1) || null;
}

function isHeroUnit(id) {
  return typeof localGame.attributes === "function" && localGame.attributes(id).some((v) => v !== 0);
}

function refreshSeatStats() {
  for (const { slot } of seating.export()) {
    const hero = heroOf(slot);
    if (!hero) continue;
    seating.setStats(slot, {
      gold: typeof localGame.hero_gold === "function" ? localGame.hero_gold(hero.id) : localGame.gold(slot),
      kills: Math.min(U16_MAX, typeof localGame.kills_of === "function" ? localGame.kills_of(hero.id) : 0),
      deaths: Math.min(U16_MAX, typeof localGame.deaths_of === "function" ? localGame.deaths_of(hero.id) : 0),
    });
  }
}

// Seats a hero for each START seat whose player owns none yet: the picked hero type, at its team's side of the map.
function spawnSeatHeroes(seats) {
  if (typeof localGame.spawn_hero !== "function") return;
  spawnedSeats = seats.map(({ slot, hero }) => ({ slot, hero }));
  for (const seat of seats) {
    const placed = placeholderOf(seat.slot);
    const kind = heroKinds.get(seat.hero);
    if (kind === undefined) {
      if (!placed) console.error(`seat ${seat.slot}: hero ${seat.hero} is not in the roster; no hero spawned`);
      continue;
    }
    if (placed?.type === kind) continue;
    // The map starts each seat with a placeholder hero of the generic `hero` type; the pick replaces it. A removal the
    // sim cannot make is an error, never a silent skip: the placeholder would stand in for the picked hero.
    if (placed) {
      if (typeof localGame.remove_unit !== "function") throw new Error("sim has no remove_unit; the placeholder hero cannot be replaced");
      localGame.remove_unit(placed.id);
    }
    spawnAtSide(seat.slot, kind, placed);
  }
}

// The sim refuses a spawn on a blocked tile, so the hero takes the nearest free tile to `from` (the placeholder it
// replaces, which keeps the map's start) or else to its side's start point.
function spawnAtSide(slot, kind, from = null) {
  const x0 = Math.floor(from ? from.x : slot < 5 ? W * 0.1 : W * 0.9);
  const y0 = Math.floor(from ? from.z : H / 2);
  for (let r = 0; r <= 24; r++) {
    for (let dy = -r; dy <= r; dy++) {
      for (let dx = -r; dx <= r; dx++) {
        if (Math.max(Math.abs(dx), Math.abs(dy)) !== r) continue;
        if (localGame.spawn_hero(slot, kind, x0 + dx, y0 + dy) >= 0) return true;
      }
    }
  }
  console.error(`seat ${slot}: no free tile near its start; no hero spawned`);
  return false;
}

// Applies a START (the lobby's own or a peer's): seats the players, spawns heroes and, on the host, tells every peer.
function applyStart(frame) {
  seating.start(frame.seats);
  seatsExplicit = true;
  const mine = seating.seat(auth.pubkey);
  if (mine.you !== SPECTATOR) myPlayer = mine.you;
  if (!isGuest()) spawnSeatHeroes(frame.seats);
  document.body.dataset.seated = String(seating.size());
  if (hosting) {
    encoder.keyframe();
    for (const key of peers) sendRoster(key);
  }
}

function endLobby() {
  const lobbyEl = document.getElementById("lobby");
  if (lobbyEl) lobbyEl.hidden = true;
  window.dispatchEvent(new Event("oscraft:horn"));
}

function onLobbyStart(state) {
  const frame = startFrame(state);
  setStatus("lobby: match starting");
  if (!isGuest()) applyStart(frame);
  // The match opens on the local hero: the frame loop centres the camera once that hero is in the sim.
  pendingStartCentre = true;
  try { session.broadcast(encodeFrame(frame)); } catch { /* the frame is refused when a seat key is oversize */ }
}

// A peer's move, cast or attack runs on the host only for a unit the peer's player owns. Accepted casts and
// attacks are counted on the host page (data-remote-casts, data-remote-attacks) for the CI witness.
function applyPeerCommand(peerPubkey, command) {
  const player = seating.seat(peerPubkey).you;
  const unit = simUnits().find((u) => u.id === command.unitId);
  if (player === SPECTATOR || !unit || unit.player !== player) {
    setStatus(`order refused not_owner peer=${String(peerPubkey).slice(0, 8)} unit=${command.unitId}`);
    document.body.dataset.lastRefusal = `not_owner unit=${command.unitId}`;
    return false;
  }
  if (command.type === FRAME.ORDER) {
    const ok = localGame.order_move(command.unitId, command.x, command.y);
    if (!ok) document.body.dataset.lastRefusal = `order_refused unit=${command.unitId} x=${command.x} z=${command.y}`;
    if (ok) countRemote("remoteOrders");
    return ok;
  }
  if (command.type === FRAME.CAST) {
    const ok = localGame.cast(command.unitId, command.slot, command.target, command.x, command.y);
    if (!ok) document.body.dataset.lastRefusal = `cast_refused unit=${command.unitId}`;
    if (ok) countRemote("remoteCasts");
    return ok;
  }
  if (command.type === FRAME.ATTACK_MOVE) {
    const ok = localGame.attack_move(command.unitId, command.x, command.y);
    if (!ok) document.body.dataset.lastRefusal = `attack_move_refused unit=${command.unitId} x=${command.x} z=${command.y}`;
    if (ok) countRemote("remoteOrders");
    return ok;
  }
  if (command.type === FRAME.HOLD) {
    const ok = localGame.hold(command.unitId);
    if (!ok) document.body.dataset.lastRefusal = `hold_refused unit=${command.unitId}`;
    if (ok) countRemote("remoteOrders");
    return ok;
  }
  if (command.type === FRAME.USE_ITEM) {
    const ok = localGame.use_item(command.unitId, command.slot, command.target, command.x, command.y);
    if (!ok) document.body.dataset.lastRefusal = `item_refused unit=${command.unitId} slot=${command.slot}`;
    if (ok) countRemote("remoteCasts");
    return ok;
  }
  const ok = localGame.attack(command.unitId, command.targetId);
  if (!ok) document.body.dataset.lastRefusal = `attack_refused unit=${command.unitId}`;
  if (ok) countRemote("remoteAttacks");
  return ok;
}

function countRemote(name) {
  document.body.dataset[name] = String(Number(document.body.dataset[name] || 0) + 1);
}

const SNAPSHOT_EVERY_TICKS = 50;

function snapshotKey() {
  return `oscraft:snapshot:${namespace}:${room}`;
}

function saveSnapshot(tick) {
  if (typeof localGame.save_state !== "function") return;
  const bytes = localGame.save_state();
  let binary = "";
  for (let i = 0; i < bytes.length; i += 0x8000) binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  const record = { tick, hash: localGame.state_hash().toString(), bytes: btoa(binary), seats: seating.export() };
  try {
    localStorage.setItem(snapshotKey(), JSON.stringify(record));
    delete document.body.dataset.snapshotSkipped;
  } catch (e) {
    // The moba state can outgrow the origin quota; the room then runs without a restore point instead of throwing per save.
    if (e && e.name === "QuotaExceededError") { localStorage.removeItem(snapshotKey()); document.body.dataset.snapshotSkipped = String(tick); return; }
    throw e;
  }
}

// Restores the room from its last snapshot. Returns the restored tick, or -1 after an explicit refusal line.
function restoreSnapshot() {
  const raw = localStorage.getItem(snapshotKey());
  if (raw === null) {
    setStatus("snapshot: none stored, starting fresh");
    return 0;
  }
  try {
    const record = JSON.parse(raw);
    const bytes = Uint8Array.from(atob(record.bytes), (c) => c.charCodeAt(0));
    localGame.load_state(bytes);
    const hash = localGame.state_hash().toString();
    if (hash !== record.hash) throw new Error(`hash ${hash} does not match stored ${record.hash}`);
    if (Array.isArray(record.seats) && record.seats.length > 0) {
      seating.restore(record.seats);
      seatsExplicit = true;
    }
    document.body.dataset.restoredTick = String(record.tick);
    document.body.dataset.restoredHash = hash;
    setStatus(`snapshot: restored tick=${record.tick} state_hash=${hash}`);
    return record.tick;
  } catch (err) {
    setStatus(`snapshot refused: ${err.message || err}`);
    document.body.dataset.snapshotRefused = String(err.message || err);
    return -1;
  }
}

function startHost() {
  hosting = true;
  encoder = createStateEncoder();
  const restoredAt = restoreSnapshot();
  let hostTick = restoredAt > 0 ? restoredAt : 0;
  const hostStep = () => {
    localGame.tick();
    advanceTickClock(simUnits());
    hostTick += 1;
    if (hostTick % SNAPSHOT_EVERY_TICKS === 0) saveSnapshot(hostTick);
    const hash = Number(localGame.state_hash() & 0xffffffffn) >>> 0;
    const units = currentUnits().map((u) => ({ id: u.id, kind: Math.max(0, u.type), player: u.player, x: Math.round(u.x * WIRE_SCALE), y: Math.round(u.z * WIRE_SCALE), hp: Math.max(0, u.hp) }));
    if (window.__ciSent) recordTick(window.__ciSent, hostTick, units);
    const frame = encoder.encode(hostTick, hash, units);
    const bytes = encodeFrame(frame);
    if (window.__ciFrames && window.__ciFrames.length < 200) window.__ciFrames.push({ type: frame.type, bytes: bytes.byteLength });
    session.broadcast(bytes);
    if (hostTick % ROSTER_EVERY_TICKS === 0) {
      refreshSeatStats();
      for (const key of peers) sendRoster(key);
    }
  };
  let hostAcc = 0;
  let hostLast = performance.now();
  setInterval(() => {
    const now = performance.now();
    hostAcc += Math.min(now - hostLast, MAX_FRAME_DT_S * 1000);
    hostLast = now;
    while (hostAcc >= SIM_TICK_MS) {
      hostAcc -= SIM_TICK_MS;
      hostStep();
    }
  }, HOST_POLL_MS);
  setStatus("hosting");
}

function scheduleReconnect() {
  if (reconnectAttempt >= RECONNECT_MAX_ATTEMPTS) {
    setStatus("reconnect gave up; reload to retry");
    return;
  }
  const delay = Math.min(RECONNECT_CAP_MS, RECONNECT_BASE_MS * 2 ** reconnectAttempt);
  reconnectAttempt += 1;
  setStatus(`reconnecting in ${delay} ms (attempt ${reconnectAttempt})`);
  clearTimeout(reconnectTimer);
  reconnectTimer = setTimeout(() => joinRoom(), delay);
}

async function joinRoom() {
  try {
    await session.connect(room);
    setStatus("joined");
  } catch (err) {
    setStatus(`join failed: ${err && err.message ? err.message : err}`);
    scheduleReconnect();
  }
}

function sendOrder(unitId, x, z) {
  return encodeFrame({ type: FRAME.ORDER, unitId, x: Math.round(x), y: Math.round(z) });
}

function sendCast(unitId, slot, target, x, z) {
  return encodeFrame({ type: FRAME.CAST, unitId, slot, target, x: Math.round(x), y: Math.round(z) });
}

function sendAttackMove(unitId, x, z) {
  return encodeFrame({ type: FRAME.ATTACK_MOVE, unitId, x: Math.round(x), y: Math.round(z) });
}

function sendAttack(unitId, targetId) {
  return encodeFrame({ type: FRAME.ATTACK, unitId, targetId });
}

function sendHold(unitId) {
  return encodeFrame({ type: FRAME.HOLD, unitId });
}

function sendUseItem(unitId, slot, target, x, z) {
  return encodeFrame({ type: FRAME.USE_ITEM, unitId, slot, target, x: Math.round(x), y: Math.round(z) });
}

const fsm = createFSM(xstate);
const auth = new NostrAuth({ nostrTools: NostrTools });
auth.loadFromStorage() || auth.generateKey();
const relayPool = new RelayPool({
  verifyEvent: NostrTools.verifyEvent,
  ...(relayUrls ? { relays: relayUrls, fallbackRelays: [], autoRotate: false } : {}),
});
relayPool.connect();

// Room announcements over the relay. Each page publishes its room as a parameterized replaceable
// event (kind 30078, tag t=oscraft-room, ns and room tags) every 15 s and lists the rooms other
// pages announce. An announcement expires after three missed announcements (45 s), so a closed page
// drops off the list. Lists feed the "oscraft:rooms" window event that rooms.js renders as a snapshot.
const ROOM_KIND = 30078;
const ROOM_TAG = "oscraft-room";
const ROOM_ANNOUNCE_S = 15;
const ROOM_TTL_S = 3 * ROOM_ANNOUNCE_S;
const ROOM_PRUNE_MS = 5000;
const announcedRooms = new Map();
let shownRooms = "";

function announceRoom() {
  if (!auth.pubkey) return;
  auth.sign({
    kind: ROOM_KIND,
    created_at: Math.floor(Date.now() / 1000),
    tags: [["d", `oscraft-room:${namespace}:${room}`], ["t", ROOM_TAG], ["ns", namespace], ["room", room], ["players", String(Math.min(MAX_PLAYERS, hosting ? seating.size() + 1 : peers.size + 1))]],
    content: "",
  }).then((signed) => relayPool.publish(signed)).catch(() => {});
}

function showRooms() {
  const names = [...announcedRooms].map(([name, a]) => ({ name, players: a.players })).sort((a, b) => a.name.localeCompare(b.name));
  const key = names.map((n) => `${n.name}:${n.players}`).join(",");
  if (key === shownRooms) return;
  shownRooms = key;
  window.dispatchEvent(new CustomEvent("oscraft:rooms", { detail: { rooms: names } }));
}

function pruneRooms() {
  const nowS = Math.floor(Date.now() / 1000);
  for (const [name, entry] of announcedRooms) {
    if (nowS - entry.createdAt > ROOM_TTL_S) announcedRooms.delete(name);
  }
  showRooms();
}

relayPool.subscribe("oscraft-rooms", [{ kinds: [ROOM_KIND], "#t": [ROOM_TAG] }], (event) => {
  const tagValue = (name) => (event.tags.find((t) => t[0] === name) || [])[1];
  if (tagValue("ns") !== namespace || typeof tagValue("room") !== "string") return;
  if (Math.floor(Date.now() / 1000) - event.created_at > ROOM_TTL_S) return;
  announcedRooms.set(tagValue("room"), { createdAt: event.created_at, players: Number(tagValue("players")) || 1 });
  pruneRooms();
});
announceRoom();
setInterval(announceRoom, ROOM_ANNOUNCE_S * 1000);
setInterval(pruneRooms, ROOM_PRUNE_MS);
// ?ice=<JSON list of stun entries> replaces the default STUN list. Relay test mode (?relay=) keeps no ICE servers unless ?ice is given.
const iceConfig = (() => {
  try {
    return iceServersFrom(params.get("ice"));
  } catch (err) {
    setStatus(`ice config refused: ${err.message}`);
    return iceServersFrom(null);
  }
})();
const ciPeerConnections = [];
const createPeerConnection = (config) => {
  const pc = new RTCPeerConnection(config);
  if (params.get("ci") === "1") ciPeerConnections.push(pc);
  return pc;
};
const session = createDataSession({ fsm, xstate, relayPool, auth, namespace, createPeerConnection, iceServers: relayUrls && !params.has("ice") ? [] : iceConfig.servers });

session.addEventListener("peer-open", (e) => {
  peers.add(e.detail.peerPubkey);
  if (hosting) {
    const key = e.detail.peerPubkey;
    seating.join(key);
    autoSeat(key);
    encoder.keyframe();
    sendRoster(key);
  } else {
    try { sendTo(e.detail.peerPubkey, encodeFrame({ type: FRAME.JOIN, lastTick: lastAppliedTick })); } catch { /* the host asks again on the next join */ }
  }
  reconnectAttempt = 0;
  setStatus(`peer-open ${String(e.detail.peerPubkey).slice(0, 8)}`);
});
session.addEventListener("peer-close", (e) => {
  peers.delete(e.detail.peerPubkey);
  if (hosting) seating.leave(e.detail.peerPubkey);
  setStatus(`peer-close ${String(e.detail.peerPubkey).slice(0, 8)}`);
  if (peers.size === 0) {
    netTick = -1;
    scheduleReconnect();
  }
});
const netUnits = new Map();
let netTick = -1;
session.addEventListener("data", (e) => {
  let frame = null;
  try { frame = decodeFrame(new Uint8Array(e.detail.data)); } catch { return; }
  if (frame.type === FRAME.ORDER || frame.type === FRAME.CAST || frame.type === FRAME.ATTACK || frame.type === FRAME.HOLD || frame.type === FRAME.USE_ITEM || frame.type === FRAME.ATTACK_MOVE) {
    if (hosting) applyPeerCommand(e.detail.peerPubkey, frame);
    return;
  }
  if (frame.type === FRAME.JOIN) {
    if (hosting) {
      seating.join(e.detail.peerPubkey);
      encoder.keyframe();
      sendRoster(e.detail.peerPubkey);
    }
    return;
  }
  if (frame.type === FRAME.START) {
    const sender = e.detail.peerPubkey;
    if (hosting && !frame.seats.some((s) => s.owner === sender)) return;
    if (hosting) {
      applyStart(frame);
      for (const key of peers) if (key !== sender) sendTo(key, encodeFrame(frame));
    } else {
      seatsExplicit = true;
    }
    endLobby();
    return;
  }
  if (frame.type === FRAME.ROSTER) {
    if (hosting) return;
    rosterView = frame;
    if (frame.you !== SPECTATOR) myPlayer = frame.you;
    document.body.dataset.you = String(frame.you);
    document.body.dataset.players = String(frame.players.length);
    return;
  }
  if (hosting) return;
  if (frame.type !== FRAME.STATE && frame.type !== FRAME.DELTA) return;
  if (frame.tick <= netTick) return;
  if (!applyFrame(netUnits, frame)) return;
  netTick = frame.tick;
  lastAppliedTick = frame.tick;
  netState = { tick: frame.tick, units: [...netUnits.values()].map((u) => ({ id: u.id, x: u.x / WIRE_SCALE, z: u.y / WIRE_SCALE, player: u.player, hp: u.hp, type: u.kind })) };
  advanceTickClock(netState.units);
  if (window.__ciRecv) recordTick(window.__ciRecv, frame.tick, [...netUnits.values()]);
});

// CI witness of wireweave agreement: each side keeps the unit positions it sent or applied for the last host ticks.
const CI_TICK_LOG = 300;
function recordTick(log, tick, units) {
  log.set(tick, units.map((u) => `${u.id}:${u.x}:${u.y}`).sort().join(","));
  if (log.size > CI_TICK_LOG) log.delete(log.keys().next().value);
}

// Accepted commands are published when they are counted, not on the next animation frame: a CI frame can take longer than the
// probe's wait for the witness, so a frame-only publish reads an accepted command as dropped.
function publishCommandCounts() {
  document.body.dataset.orders = String(orders);
  document.body.dataset.casts = String(casts);
  document.body.dataset.attacks = String(attacks);
}

// Page state for headless checks. The HUD (gold, winner, health bars, minimap) refreshes from the same frame.
function publishWitness() {
  document.body.dataset.units = String(currentUnits().length);
  document.body.dataset.ticks = String(localGame ? localGame.ticks() : netState ? netState.tick : 0);
  document.body.dataset.mode = "3d";
  document.body.dataset.center = renderer3d.camera.target.map((v) => v.toFixed(2)).join(",");
  document.body.dataset.distance = renderer3d.camera.distance.toFixed(3);
  document.body.dataset.drawTerrain = String(drawCounts.terrain);
  // No quad placeholder exists any more: units with no model are not drawn, so the quad count is always 0.
  document.body.dataset.drawQuad = "0";
  document.body.dataset.drawSplat = String(drawCounts.splat);
  document.body.dataset.splatGround = splat ? "1" : "0";
  document.body.dataset.shadow = SHADOW_ON && renderer3d && renderer3d.setShadows ? "1" : "0";
  document.body.dataset.shadowUnits = String(shadowUnitCount);
  document.body.dataset.texelUploads = String(renderer3d.stats.texelUploads || 0);
  document.body.dataset.terrainMode = "heightfield";
  document.body.dataset.drawDoodad = String(drawCounts.doodad);
  document.body.dataset.drawUnit = String(drawCounts.unit);
  document.body.dataset.drawAnimated = String(drawCounts.animated);
  document.body.dataset.drawCalls = String(renderer3d ? renderer3d.stats.drawCalls : 0);
  document.body.dataset.drawInstances = String(renderer3d ? renderer3d.stats.instances : 0);
  document.body.dataset.drawCulled = String(renderer3d ? renderer3d.stats.culled : 0);
  document.body.dataset.drawCpuMs = renderer3d ? renderer3d.stats.frameCpuMs.toFixed(3) : "0";
  publishCommandCounts();
  if (hud) hud.update();
  if (hud5) hud5.update();
  for (const panel of heroPanels) {
    if (netState) panel.root.hidden = true;
    else panel.update();
  }
}

// The hero panels read and act on the local sim, so they need every hero export of the wasm build and show only
// where this page owns the sim; a guest's page keeps them hidden.
function hasHeroApi(game) {
  return ["buy", "item_catalog_json", "abilities_json", "inventory_json", "character_level", "skill_points"].every((name) => typeof game[name] === "function");
}

// Hero picks for the lobby: see heroKindsFor (lobby.js). Empty when the wasm build predates the hero getters.
function liveHeroRoster(game) {
  heroKinds = new Map();
  if (typeof game.unit_type_names !== "function" || typeof game.unit_hero_kinds !== "function") return [];
  heroKinds = heroKindsFor(game.unit_type_names(), game.unit_hero_kinds(), heroRuleNames);
  return [...heroKinds.keys()];
}

function createBarCanvas() {
  const bars = document.createElement("canvas");
  bars.style.cssText = "position:fixed;left:0;top:0;pointer-events:none";
  document.body.appendChild(bars);
  return bars;
}

// Terrain positions in tile units: the map's cell size is the unit of the sim's coordinates.
function terrainInTiles(terrain) {
  const s = 1 / terrain.cellMilli;
  return { ...terrain, positions: terrain.positions.map((v) => v * s) };
}

// Height of the terrain at a tile position: the nearest grid corner's height.
function groundY(terrain, x, z) {
  const cols = terrain.width + 1;
  const cx = Math.min(terrain.width, Math.max(0, Math.round(x)));
  const cy = Math.min(terrain.height, Math.max(0, Math.round(z)));
  return terrain.positions[(cy * cols + cx) * 3 + 1];
}

const DOODAD_TINT = [0.55, 0.5, 0.42];

// Deterministic per-placement variety: a hash of the placement's own position (and a salt per property) gives a value in
// [0, 1), so the map looks the same on every run and every placement differs from its neighbours.
function unitHash(n) {
  let h = n | 0;
  h = Math.imul(h ^ (h >>> 16), 0x45d9f3b);
  h = Math.imul(h ^ (h >>> 16), 0x45d9f3b);
  return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
}
function placementHash(x, y, salt) {
  return unitHash(Math.imul(x | 0, 0x9e3779b1) ^ Math.imul(y | 0, 0x85ebca77) ^ Math.imul(salt, 0xc2b2ae3d));
}
// Tints multiply a doodad's colour (its texture, or its fallback colour when the model has no texture); index 0 is the plain model.
const DOODAD_TINTS = [[1, 1, 1], [0.88, 0.92, 0.98], [1.08, 1.02, 0.9], [0.94, 1.05, 0.96]];
const doodadVariants = new Map();
function doodadVariant(base, kind, v) {
  if (v === 0) return base;
  const key = kind + ":" + v;
  if (!doodadVariants.has(key)) doodadVariants.set(key, { ...base, tint: DOODAD_TINTS[v] });
  return doodadVariants.get(key);
}

// Builds the 3D world from the staged assets: the terrain mesh from map.bin, doodads at their placements with
// their GLB models, and unit models from the sim's model names. Each unit type keeps a model, or null when it has none.
async function build3d(gl, game) {
  const terrain = terrainInTiles(buildTerrain(mapBytes));
  terrainTiles = terrain;
  terrainMesh = { positions: terrain.positions, normals: terrain.normals, uvs: planarGroundUvs(terrain.positions), indices: terrain.indices, color: MAP_COLOR };
  // The river's surface: flat quads over the water cells, drawn by the splat pass with the procedural water shader.
  const water = waterSurface(mapBytes);
  waterMesh = water.indices.length ? { ...water, color: MAP_COLOR } : null;
  // Splat layers, doodad and unit models are independent downloads: they start together, so the wait is the slowest
  // chain rather than the sum of the stages.
  const placements = readDoodads(mapBytes);
  const usedKinds = new Set(placements.map((p) => p.kind));
  const splatReady = buildSplat(gl);
  const doodadModelsReady = loadDoodadModels(usedKinds);
  const unitFitsReady = loadModels(game);
  // The splat ground is the default floor: a layer that does not build stops the start (buildSplat throws).
  splat = await splatReady;
  const doodadByKind = await Promise.all((await doodadModelsReady).map(async ({ high, low }) => ({
    base: high ? await modelOrRecord(high.glbUrl, toMesh(gl, high, DOODAD_TINT)) : null,
    lowBase: low ? await modelOrRecord(low.glbUrl, toMesh(gl, low, DOODAD_TINT)) : null,
  })));
  const s = 1 / terrain.cellMilli;
  const placed = placements.flatMap((p) => {
    const kind = doodadByKind[p.kind];
    if (!kind?.base) return [];
    const x = p.x * s;
    const z = p.y * s;
    const angle = (p.rotation / 1000) * 2 * Math.PI + placementHash(p.x, p.y, 1) * 2 * Math.PI;
    const k = 0.85 + 0.3 * placementHash(p.x, p.y, 2);
    const variant = Math.floor(placementHash(p.x, p.y, 3) * DOODAD_TINTS.length);
    const mesh = doodadVariant(kind.base, p.kind, variant);
    const lowMesh = kind.lowBase ? doodadVariant(kind.lowBase, `${p.kind}-low`, variant) : null;
    return [{ mesh, lowMesh, transform: multiply(multiply(translation(x, groundY(terrain, x, z), z), rotationY(angle)), scaling(k, k, k)) }];
  });
  doodadBatches = batchByMesh(placed);
  if (renderer3d.setShadows) renderer3d.setShadows({ enabled: SHADOW_ON, size: SHADOW_SIZE, center: [terrain.width / 2, 0, terrain.height / 2], halfExtent: Math.max(terrain.width, terrain.height) / 2 + 4 });
  // A hero's mesh failure stops the start; any other unit's failure is recorded and that type is not drawn.
  const heroFlags = typeof game.unit_hero_kinds === "function" ? game.unit_hero_kinds() : [];
  unitMeshes = await Promise.all((await unitFitsReady).map((f, type) => {
    if (!f) return null;
    const mesh = toMesh(gl, f, [1, 1, 1], { animated: true });
    return heroFlags[type] === 1 ? mesh : modelOrRecord(f.glbUrl, mesh);
  }));
  arrowMesh = await loadArrowMesh(gl);
}

// Splat ground: the control map from the grid and each layer's albedo and height textures, in SPLAT_LAYERS order.
// A layer texture that does not load throws, naming the layer: the splat ground is required.
async function buildSplat(gl) {
  const layers = await Promise.all(SPLAT_LAYERS.map(async (name) => {
    const [albedo, height] = await Promise.all(SPLAT_TEXTURES[name].map(async (uri) => {
      const image = await loadBaseColorImage({ uri }, location.href);
      if (!image) throw new Error(`splat layer ${name} texture ${uri} did not load`);
      return image;
    }));
    return { albedo: uploadTexture(gl, albedo), height: uploadTexture(gl, height) };
  }));
  const control = controlMap(mapBytes);
  return {
    control, layers, width: control.width, height: control.height, uvScale: GROUND_UV_PER_CELL, band: SPLAT_BAND, shade: SPLAT_SHADE, tints: SPLAT_TINTS,
    waterMap: waterMap(mapBytes), waterTint: SPLAT_WATER_TINT, gpu: null,
  };
}

const WORLD_STALL_MS = 15000;

async function main() {
  resize();
  bindInput();
  const gl = canvas.getContext("webgl2", { antialias: true });
  if (!gl) throw new Error("WebGL2 is required: this browser gave no WebGL2 context for the game canvas");
  localGame = await loadSim();
  renderer3d = createRenderer3d(gl);
  renderer3d.setCamera({ target: orbitOrigin, distance: cameraDistance, maxDistance: cameraMaxDistance, far: cameraMaxDistance * CAMERA_FAR_PER_MAX, bounds: cameraBounds, pitch: 0.9 });
  // A download that never settles would otherwise leave the status on its initial "connecting" text.
  setStatus("loading world assets");
  const stalled = setTimeout(() => setStatus(`loading world assets: still pending after ${WORLD_STALL_MS / 1000} s, check the network`), WORLD_STALL_MS);
  try {
    await build3d(gl, localGame);
  } finally {
    clearTimeout(stalled);
  }
  hud = createHud({
    game: localGame,
    hudEl: document.getElementById("hud"),
    winEl: document.getElementById("win"),
    minimapEl: document.getElementById("minimap"),
    barCanvas: createBarCanvas(),
    mapSize: { w: W, h: H },
    grid: mapGrid,
    units: currentUnits,
    selected: () => selectedId,
    select: (unitId) => { selectedId = unitId; },
    localPlayer: localPlayerId,
    commands,
    pick,
    toWorld: screenToWorld,
    toScreen: worldToScreen,
    playerColor: (player) => PLAYER_COLORS[player] || PLAYER_COLORS[0],
  });
  if (renderer3d) fx = createFx({ renderer: renderer3d, game: () => localGame, toScreen: worldToScreen, groundAt, playerColor: (player) => PLAYER_COLORS[player] || PLAYER_COLORS[0], selected: () => selectedId, arrow: () => arrowMesh });
  hud5 = createHud5({ game: localGame, units: currentUnits, selected: () => selectedId, localPlayer: localPlayerId, isHero: (id) => localGame !== null && isHeroUnit(id), heroUnits: simUnits });
  const eventsBase = params.get("events");
  const modUi = createModUi({
    root: document.body,
    renderer: () => renderer3d,
    localPlayer: localPlayerId,
    defaultDistance: () => cameraDistance,
    baseUrl: eventsBase === null ? undefined : eventsBase.replace(/\/$/, ""),
  });
  window.__modUi = modUi;
  const heroUi = { game: localGame, selected: () => selectedId, localPlayer: localPlayerId, input: hud ? hud.input : null };
  if (hasHeroApi(localGame)) heroPanels.push(createShop(heroUi), createSkills(heroUi), createInventory(heroUi));
  const heroNames = liveHeroRoster(localGame);
  createLobby({ playerId: auth.pubkey || "local", roster: heroNames, onStart: onLobbyStart });
  fogview = createFogView({ game: localGame, localPlayer: localPlayerId, toClip: worldClip, groundAt });
  window.__fogView = fogview;
  if (params.get("host") === "1") startHost();
  joinRoom();

  function frame(now) {
    const dtS = lastFrameMs ? Math.min(0.1, (now - lastFrameMs) / 1000) : 0;
    lastFrameMs = now;
    panByKeys(dtS);
    modUi.step(dtS);
    if (pendingStartCentre && renderer3d) {
      const own = simUnits().find((u) => u.player === localPlayerId() && isHeroUnit(u.id));
      if (own) {
        renderer3d.setCamera({ target: [own.x, groundAt(own.x, own.z), own.z], distance: Math.min(CAMERA_START_DISTANCE, cameraMaxDistance) });
        pendingStartCentre = false;
      }
    }
    if (!netState && localGame && !hosting) {
      localTickAcc += dtS;
      while (localTickAcc >= SIM_TICK_S) {
        localGame.tick();
        advanceTickClock(simUnits());
        localTickAcc -= SIM_TICK_S;
      }
    }
    drawWorld3d();
    fogview.draw();
    publishWitness();
    requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);
  setStatus("local preview");
  document.body.dataset.ready = "1";
}

// Live debug API (window.oscraft) on every load, with no per-frame work; see debug_api.js for each call's effect.
// Shadows are tracked here because the boot-time ?shadows switch seeds them and publishWitness keeps the page's shadow mark.
let shadowsEnabled = SHADOW_ON;
installDebugApi({
  renderer: () => renderer3d,
  rendererKind: () => "webgl",
  game: () => localGame,
  localPlayer: () => myPlayer,
  netTick: () => (netState ? netState.tick : 0),
  units: currentUnits,
  isHero: (id) => localGame !== null && isHeroUnit(id),
  selected: () => (hud ? hud.input.selection() : selectedId === -1 ? [] : [selectedId]),
  select: (ids) => { if (hud) hud.input.setSelection(ids); else selectedId = ids[0] ?? -1; },
  pick: (world) => pick(world),
  order: (id, x, z) => commands.order(id, x, z),
  attack: (id, targetId) => commands.attack(id, targetId),
  drawCounts: () => ({ ...drawCounts }),
  terrainMode: () => "heightfield",
  shadowsOn: () => shadowsEnabled,
  setShadows: (on) => {
    if (!renderer3d || !renderer3d.setShadows) return false;
    renderer3d.setShadows({ enabled: on, size: SHADOW_SIZE, center: [W / 2, 0, H / 2], halfExtent: Math.max(W, H) / 2 + 4 });
    shadowsEnabled = on;
    return true;
  },
  fogOn: () => (fogview ? fogview.enabled() : false),
  setFog: (on) => {
    if (!fogview) return false;
    fogview.setEnabled(on);
    return true;
  },
});

if (params.get("ci") === "1") {
  window.__ciPeers = () => peers.size;
  window.__ciFrames = [];
  window.__ciUnits = () => currentUnits().map((u) => ({ id: u.id, x: u.x, z: u.z, player: u.player, type: u.type, modeled: !!unitMeshes[u.type] }));
  // Read-only: every living unit of the local sim, fogged or not, for probes that walk toward a foe until it is in sight.
  window.__ciRoster = () => simUnits().map((u) => ({ id: u.id, x: u.x, z: u.z, player: u.player, type: u.type }));
  window.__ciOrder = (unitId, x, z) => session.broadcast(sendOrder(unitId, x, z));
  // Local order through the sim's own command path (the local match has no peers to broadcast to).
  window.__ciMove = (unitId, x, z) => commands.order(unitId, x, z);
  window.__ciAttack = (unitId, targetId) => commands.attack(unitId, targetId);
  window.__ciCast = (unitId, slot, target, x, z) => session.broadcast(sendCast(unitId, slot, target, x, z));
  window.__ciLearn = (unitId, slot) => (localGame ? localGame.skill_up(unitId, slot) : false);
  // The first ability slot of a unit whose cast waits for a click (target unit or point), or -1: the probe learns and casts that one.
  window.__ciItemIndex = (name) => JSON.parse(localGame.item_catalog_json()).findIndex((e) => e.name === name);
  window.__ciGiveItem = (unitId, index, count) => localGame.give_item(unitId, index, count);
  window.__ciInventory = (unitId) => JSON.parse(localGame.inventory_json(unitId)).slots;
  window.__ciTargetedSlot =(unitId) => { const list = JSON.parse(localGame.abilities_json(unitId)); const t = list.find((a) => (a.target === "unit" || a.target === "point") && a.slot < 4); return t ? t.slot : -1; };
  window.__ciHostile = (a, b) => localGame.hostile(a, b);
  window.__ciIsHero = (unitId) => isHeroUnit(unitId);
  window.__ciAttack = (unitId, targetId) => session.broadcast(sendAttack(unitId, targetId));
  // Local cast for the probe: the sim cast and the effect the page draws for it, without a network session.
  window.__ciLocalCast = (unitId, slot, target, x, z) => { const ok = localGame.cast(unitId, slot, target, x, z); if (ok && fx) fx.cast(unitId, slot, target, x, z); return ok; };
  window.__ciLocalAttack = (unitId, targetId) => localGame.attack(unitId, targetId);
  window.__ciFxCounts = () => (fx ? fx.counts() : null);
  window.__ciSent = new Map();
  window.__ciRecv = new Map();
  window.__ciSentTicks = () => [...window.__ciSent];
  window.__ciRecvTicks = () => [...window.__ciRecv];
  window.__ciSelected = () => selectedId;
  // Candidate types of every local candidate and of each succeeded, selected pair, one entry per peer connection.
  window.__ciCandidates = async () => {
    const localTypes = new Set();
    const pairs = [];
    for (const pc of ciPeerConnections) {
      const stats = await pc.getStats();
      stats.forEach((s) => { if (s.type === "local-candidate") localTypes.add(s.candidateType); });
      stats.forEach((s) => {
        if (s.type !== "candidate-pair" || s.state !== "succeeded" || !(s.nominated || s.selected)) return;
        pairs.push({
          local: stats.get(s.localCandidateId)?.candidateType || "?",
          remote: stats.get(s.remoteCandidateId)?.candidateType || "?",
        });
      });
    }
    return { localTypes: [...localTypes].sort(), pairs };
  };
  // Selects through the input layer, the same path a left click takes, so a right click orders this unit.
  window.__ciSelect = (unitId) => { if (hud) hud.input.setSelection([unitId]); else selectedId = unitId; };
  window.__ciInputSelection = () => (hud ? hud.input.selection() : []);
  window.__ciLocalPlayer = () => myPlayer;
  // Moves the 3D camera target onto a ground point, so a probe can act on units that the default view does not show.
  window.__ciFocus = (x, z) => { if (renderer3d) renderer3d.setCamera({ target: [x, groundAt(x, z), z] }); };
  window.__ciToScreen = (x, z) => worldToScreen(x, z, groundAt(x, z));
  // Every unit of the side, fogged or not: the sim's own roster, so a side is killed whole whatever the local vision.
  window.__ciKillSide = (player) => {
    for (const u of simUnits()) if (u.player === player) localGame.hurt(u.id, 10000);
  };
  // Read-only: per START seat, the picked hero name and the catalog name of the unit the sim actually spawned for it.
  window.__ciHeroNames = () => {
    const names = typeof localGame.unit_type_names === "function" ? localGame.unit_type_names() : [];
    return spawnedSeats.map(({ slot, hero }) => {
      // The seat's hero is the unit of the picked kind; the first hero of the slot may be a map-placed unit of another type.
      const kind = heroKinds.get(hero);
      const u = simUnits().find((v) => v.player === slot && isHeroUnit(v.id) && v.type === kind) || null;
      return { slot, picked: hero, spawned: u ? names[u.type] ?? null : null };
    });
  };
  // Read-only: the first ramp cell's centre in tile units, and the effects' counts (ranged attack projectiles in flight).
  window.__ciRampCell = () => {
    const grid = readGrid(mapBytes);
    const [cell] = rampLowCells(grid);
    return cell === undefined ? null : { x: (cell % grid.width) + 0.5, z: Math.floor(cell / grid.width) + 0.5 };
  };
  window.__ciFx = () => (fx ? fx.counts() : null);
  // Debug API for probes and screenshots: move the 3D camera onto a ground point (cell units), or set its distance.
  window.oscraft = {
    setCamera: (opts = {}) => (renderer3d ? renderer3d.setCamera(opts) : null),
    centreOn: (x, z) => (renderer3d ? renderer3d.setCamera({ target: [x, groundAt(x, z), z] }) : null),
    fx: () => (fx ? fx.counts() : null),
  };
  window.__ciDebug = () => [...session.peers.values()].map((p) => ({ ice: p.pc?.iceConnectionState, conn: p.pc?.connectionState, remoteMdns: (p.pc?.remoteDescription?.sdp || '').split('.local').length - 1 }));
}

main().catch((err) => {
  const message = err && err.message ? err.message : String(err);
  document.body.dataset.error = message;
  failStart(err);
});
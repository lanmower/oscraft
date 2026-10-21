// Map editor: paints cell kinds and heights, places doodads and unit spawns, marks regions, and exports map.osd text.
// The model and exportOsd are pure so the Node witness (editor-check.mjs) shares them with the page.
import { readDoodads, readGrid } from "./terrain.js";
import { cellKind, TILE_IDS, TILE_TINTS } from "./tiles.js";
import { createCamera, createRenderer, eyeOf, multiply, orbitCamera, panCamera, scaling, translation, viewProjection, zoomCamera } from "./render3d.js";

export const DOODAD_KINDS = Object.freeze(["rock", "tree", "crystal", "debris", "base", "shop", "fountain", "lane", "camp", "side_shop", "secret_shop", "aid_cane", "aid_cane_blind", "aid_cane_low_vision", "aid_crutch", "aid_defibrillator_green", "aid_defibrillator_red", "aid_glasses", "aid_mask", "aid_sunglasses", "aid_hearing", "altar_stone", "altar_wood", "arrow", "arrow_basic", "arrow_basic_rounded", "arrow_rounded", "balcony_wall", "balcony_wall_fence", "banner", "banner_green", "banner_red", "barrel", "barrel_open", "bedroll", "bedroll_frame", "bedroll_packed", "bench", "bench_damaged", "blade", "border_pillar", "bottle", "bottle_large", "box", "box_large", "box_large_open", "box_long", "box_open", "box_small", "box_wide", "brick_wall", "brick_wall_curve", "brick_wall_curve_small", "brick_wall_end", "bridge", "bridge_draw", "bridge_pillar", "bridge_pillar_wide", "bridge_straight", "bridge_straight_pillar", "bucket", "building_a", "building_b", "building_c", "building_d", "building_e", "building_f", "building_g", "building_h", "building_i", "building_j", "building_k", "building_l", "building_m", "building_n", "building_o", "building_p", "building_platform", "building_q", "building_r", "building_roof", "building_s", "building_structure", "building_t", "button_floor_round", "button_floor_round_small", "button_floor_square", "button_floor_square_small", "campfire_fishing_stand", "campfire_pit", "campfire_stand", "candle", "candle_multiple", "cart", "cart_high", "catwalk_corner", "catwalk_cross", "catwalk_junction", "catwalk_stairs", "catwalk_stairs_loop", "catwalk_straight", "chair", "character_archer", "character_female_a", "character_female_b", "character_female_c", "character_female_d", "character_female_e", "character_female_f", "character_ghost", "character_human", "character_keeper", "character_male_a", "character_male_b", "character_male_c", "character_male_d", "character_male_e", "character_male_f", "character_orc", "character_skeleton", "character_vampire", "character_zombie", "chest", "chimney", "chimney_base", "chimney_basic", "chimney_large", "chimney_medium", "chimney_small", "chimney_top", "coffin", "coffin_old", "cog_a", "cog_b", "cog_c", "cog_d", "cog_e", "coin", "column", "column_large", "cone", "construction_barrier", "construction_cone", "construction_fence", "construction_light", "conveyor", "conveyor_bars", "conveyor_bars_fence", "conveyor_bars_fence_slope", "conveyor_bars_high", "conveyor_bars_high_slope", "conveyor_bars_sides", "conveyor_bars_stripe", "conveyor_bars_stripe_fence", "conveyor_bars_stripe_fence_slope", "conveyor_bars_stripe_high", "conveyor_bars_stripe_high_slope", "conveyor_bars_stripe_side", "conveyor_corner", "conveyor_cross", "conveyor_junction_t", "conveyor_long", "conveyor_long_part_end", "conveyor_long_part_middle", "conveyor_long_sides", "conveyor_long_sides_part_end", "conveyor_long_sides_part_middle", "conveyor_long_stripe", "conveyor_long_stripe_sides", "conveyor_long_stripe_sides_part_end", "conveyor_long_stripe_sides_part_middle", "conveyor_sides", "conveyor_sides_cross", "conveyor_sides_junction_t", "conveyor_stripe", "conveyor_stripe_corner", "conveyor_stripe_cross", "conveyor_stripe_junction_t", "conveyor_stripe_part_end", "conveyor_stripe_part_middle", "conveyor_stripe_sides", "conveyor_stripe_sides_cross", "conveyor_stripe_sides_junction_t", "corridor", "corridor_corner", "corridor_end", "corridor_intersection", "corridor_junction", "corridor_transition", "corridor_wide", "corridor_wide_corner", "corridor_wide_end", "corridor_wide_intersection", "corridor_wide_junction", "crane", "crane_lift", "crane_magnet", "cross", "cross_column", "cross_wood", "crypt", "crypt_a", "crypt_b", "crypt_door", "crypt_large", "crypt_large_door", "crypt_large_roof", "crypt_small", "crypt_small_roof", "debris_wood", "detail_bowl", "detail_chalice", "detail_plate", "detail_tank", "detail_tank_large", "dirt", "door", "door_wide_closed", "door_wide_half", "door_wide_open", "dumpster", "electricity_pole", "electricity_pole_single", "electricity_pole_wide", "electricity_side", "electricity_side_single", "electricity_side_wide", "electricity_wires", "electricity_wires_wide", "fence", "fence_broken", "fence_curved", "fence_damaged", "fence_doorway", "fence_fortified", "fence_gate", "fire_basket", "fish", "fish_large", "flag", "flag_banner_long", "flag_banner_short", "flag_pennant", "flag_wide", "floor", "floor_detail", "floor_hole", "floor_large", "floor_old", "fountain_center", "fountain_corner", "fountain_corner_inner", "fountain_corner_inner_square", "fountain_curved", "fountain_edge", "fountain_round", "fountain_round_detail", "fountain_square", "fountain_square_detail", "gate", "gate_metal_bars", "gate_overhang", "gate_rock", "grass", "grass_large", "grave", "grave_border", "gravestone_bevel", "gravestone_broken", "gravestone_cross", "gravestone_cross_large", "gravestone_debris", "gravestone_decorative", "gravestone_roof", "gravestone_round", "gravestone_wide", "ground", "ground_hills", "ground_pathTile", "ground_riverTile", "hay_bale", "hay_bale_bundled", "hedge", "hedge_curved", "hedge_gate", "hedge_large", "hedge_large_curved", "hedge_large_gate", "hopper_high_round", "hopper_high_square", "hopper_round", "hopper_square", "indicator_special_area", "indicator_special_arrow", "indicator_special_cross", "indicator_special_lines", "iron_fence", "iron_fence_bar", "iron_fence_border", "iron_fence_border_column", "iron_fence_border_curve", "iron_fence_border_gate", "iron_fence_curve", "iron_fence_damaged", "key", "ladder", "lantern", "lantern_candle", "lantern_glass", "lever_double", "lever_single", "light_curved", "light_curved_cross", "light_curved_double", "light_square", "light_square_cross", "light_square_double", "lightpost_all", "lightpost_double", "lightpost_single", "machine", "machine_bed", "machine_connection_hole", "machine_connection_pipe", "machine_fortified", "machine_window", "machine_window_bar", "metal_gate", "metal_panel", "metal_panel_narrow", "metal_panel_screws", "metal_panel_screws_half", "metal_panel_screws_narrow", "oopi", "overhang", "patch_dirt", "patch_grass", "patch_grass_large", "pillar_large", "pillar_obelisk", "pillar_small", "pillar_square", "pillar_stone", "pillar_wood", "pine", "pine_crooked", "pine_fall", "pine_fall_crooked", "pipe_glass_large", "pipe_glass_large_bend", "pipe_glass_large_bump", "pipe_glass_large_cross", "pipe_glass_large_curve", "pipe_glass_large_junction", "pipe_glass_large_long", "pipe_glass_large_side", "pipe_glass_large_valve", "pipe_large", "pipe_large_bend", "pipe_large_bump", "pipe_large_cross", "pipe_large_curve", "pipe_large_junction", "pipe_large_long", "pipe_large_side", "pipe_large_valve", "piston_round", "piston_square", "piston_thin_round", "piston_thin_square", "planks", "planks_half", "planks_opening", "plant", "platform", "poles", "poles_horizontal", "pot", "potion", "pumpkin", "pumpkin_carved", "pumpkin_tall", "pumpkin_tall_carved", "resource_planks", "resource_stone", "resource_stone_large", "resource_wood", "road", "road_bend", "road_bend_barrier", "road_bend_sidewalk", "road_bend_square", "road_bend_square_barrier", "road_bridge", "road_corner", "road_corner_inner", "road_crossing", "road_crossroad", "road_crossroad_barrier", "road_crossroad_line", "road_crossroad_path", "road_curb", "road_curb_end", "road_curve", "road_curve_barrier", "road_curve_intersection", "road_curve_intersection_barrier", "road_curve_pavement", "road_driveway_double", "road_driveway_double_barrier", "road_driveway_single", "road_driveway_single_barrier", "road_edge", "road_edge_slope", "road_end", "road_end_barrier", "road_end_round", "road_end_round_barrier", "road_intersection", "road_intersection_barrier", "road_intersection_line", "road_intersection_path", "road_roundabout", "road_roundabout_barrier", "road_side", "road_side_barrier", "road_side_entry", "road_side_entry_barrier", "road_side_exit", "road_side_exit_barrier", "road_sign_empty", "road_sign_empty_hanging", "road_sign_object_stop", "road_sign_object_street", "road_sign_object_warning", "road_sign_stop", "road_sign_street", "road_sign_warning", "road_slant", "road_slant_barrier", "road_slant_curve", "road_slant_curve_barrier", "road_slant_flat", "road_slant_flat_curve", "road_slant_flat_high", "road_slant_high", "road_slant_high_barrier", "road_slope", "road_split", "road_split_barrier", "road_square", "road_square_barrier", "road_straight", "road_straight_barrier", "road_straight_barrier_end", "road_straight_barrier_half", "road_straight_half", "robot_arm_a", "robot_arm_b", "rock_a", "rock_b", "rock_c", "rock_flat", "rock_flat_grass", "rock_large", "rock_sand_a", "rock_sand_b", "rock_sand_c", "rock_small", "rock_wide", "rocks", "rocks_high", "rocks_large", "rocks_low", "rocks_ramp", "rocks_small", "rocks_tall", "roof", "roof_corner", "roof_corner_inner", "roof_corner_round", "roof_flat", "roof_gable", "roof_gable_detail", "roof_gable_end", "roof_gable_top", "roof_high", "roof_high_corner", "roof_high_corner_round", "roof_high_cornerinner", "roof_high_flat", "roof_high_gable", "roof_high_gable_detail", "roof_high_gable_end", "roof_high_gable_top", "roof_high_left", "roof_high_point", "roof_high_right", "roof_high_window", "roof_left", "roof_point", "roof_right", "roof_window", "room_corner", "room_large", "room_large_variation", "room_small", "room_small_variation", "room_wide", "room_wide_variation", "scanner_high", "scanner_low", "screen_flat", "screen_hanging_small", "screen_hanging_wide", "screen_panel_flat", "screen_panel_small", "screen_panel_wide", "screen_small", "screen_wide", "shield_rectangle", "shield_round", "shipping_container_a", "shipping_container_b", "shipping_container_c", "shovel", "shovel_dirt", "siege_ballista", "siege_ballista_demolished", "siege_catapult", "siege_catapult_demolished", "siege_ram", "siege_ram_demolished", "siege_tower", "siege_tower_demolished", "siege_trebuchet", "siege_trebuchet_demolished", "sign_highway", "sign_highway_detailed", "sign_highway_wide", "signpost", "signpost_single", "solar_panel_flat", "solar_panel_landscape", "solar_panel_landscape_group", "solar_panel_portrait", "solar_panel_portrait_group", "stairs", "stairs_full", "stairs_full_corner_inner", "stairs_full_corner_outer", "stairs_stone", "stairs_stone_corner", "stairs_stone_handrail", "stairs_stone_round", "stairs_stone_square", "stairs_wide", "stairs_wide_stone", "stairs_wide_stone_handrail", "stairs_wide_wood", "stairs_wide_wood_handrail", "stairs_wood", "stairs_wood_handrail", "stall", "stall_bench", "stall_green", "stall_red", "stall_stool", "stone_wall", "stone_wall_column", "stone_wall_curve", "stone_wall_damaged", "stones", "structure", "structure_canvas", "structure_corner_inner", "structure_corner_outer", "structure_doorway", "structure_doorway_wide", "structure_floor", "structure_high", "structure_medium", "structure_metal", "structure_metal_doorway", "structure_metal_floor", "structure_metal_roof", "structure_metal_wall", "structure_roof", "structure_short", "structure_tall", "structure_wall", "structure_window", "structure_window_wide", "structure_yellow_high", "structure_yellow_medium", "structure_yellow_short", "structure_yellow_tall", "table", "target", "template_corner", "template_detail", "template_floor", "template_floor_big", "template_floor_detail", "template_floor_detail_a", "template_floor_layer", "template_floor_layer_hole", "template_floor_layer_raised", "template_wall", "template_wall_corner", "template_wall_detail_a", "template_wall_half", "template_wall_stairs", "template_wall_top", "tent", "tent_canvas", "tent_canvas_half", "tile_high", "tile_low", "tile_slant", "tile_slantHigh", "tool_axe", "tool_axe_upgraded", "tool_hammer", "tool_hammer_upgraded", "tool_hoe", "tool_hoe_upgraded", "tool_pickaxe", "tool_pickaxe_upgraded", "tool_shovel", "tool_shovel_upgraded", "top", "top_large", "top_large_checkerboard", "tower_base", "tower_hexagon_base", "tower_hexagon_mid", "tower_hexagon_roof", "tower_hexagon_roof_secondary", "tower_hexagon_top", "tower_hexagon_top_wood", "tower_slant_roof", "tower_square", "tower_square_arch", "tower_square_base", "tower_square_base_border", "tower_square_base_color", "tower_square_mid", "tower_square_mid_color", "tower_square_mid_door", "tower_square_mid_open", "tower_square_mid_open_simple", "tower_square_mid_windows", "tower_square_roof", "tower_square_top", "tower_square_top_color", "tower_square_top_roof", "tower_square_top_roof_high", "tower_square_top_roof_high_windows", "tower_square_top_roof_rounded", "tower_top", "traffic_light", "traffic_light_hanging", "traffic_light_object_hanging", "traffic_light_object_horizontal", "traffic_light_object_vertical", "trap", "tree_autumn", "tree_autumn_tall", "tree_autumn_trunk", "tree_crooked", "tree_high", "tree_high_crooked", "tree_high_round", "tree_large", "tree_log", "tree_log_small", "tree_small", "tree_tall", "tree_trunk", "trunk", "trunk_long", "urn_round", "urn_square", "wall", "wall_arch", "wall_arch_top", "wall_arch_top_detail", "wall_block", "wall_block_half", "wall_broken", "wall_corner", "wall_corner_detail", "wall_corner_diagonal", "wall_corner_diagonal_half", "wall_corner_edge", "wall_corner_half", "wall_corner_half_tower", "wall_corner_slant", "wall_curved", "wall_detail_cross", "wall_detail_diagonal", "wall_detail_horizontal", "wall_diagonal", "wall_door", "wall_doorway", "wall_doorway_base", "wall_doorway_round", "wall_doorway_square", "wall_doorway_square_wide", "wall_doorway_square_wide_curved", "wall_half", "wall_half_modular", "wall_narrow", "wall_narrow_corner", "wall_narrow_gate", "wall_narrow_stairs", "wall_narrow_stairs_rail", "wall_narrow_wood", "wall_narrow_wood_fence", "wall_opening", "wall_pillar", "wall_rounded", "wall_side", "wall_slope", "wall_stud", "wall_to_narrow", "wall_window_glass", "wall_window_round", "wall_window_shutters", "wall_window_small", "wall_window_stone", "wall_wood", "wall_wood_arch", "wall_wood_arch_top", "wall_wood_arch_top_detail", "wall_wood_block", "wall_wood_block_half", "wall_wood_broken", "wall_wood_corner", "wall_wood_corner_diagonal", "wall_wood_corner_diagonal_half", "wall_wood_corner_edge", "wall_wood_curved", "wall_wood_detail_cross", "wall_wood_detail_diagonal", "wall_wood_detail_horizontal", "wall_wood_diagonal", "wall_wood_door", "wall_wood_doorway_base", "wall_wood_doorway_round", "wall_wood_doorway_square", "wall_wood_doorway_square_wide", "wall_wood_doorway_square_wide_curved", "wall_wood_half", "wall_wood_rounded", "wall_wood_side", "wall_wood_slope", "wall_wood_window_glass", "wall_wood_window_round", "wall_wood_window_shutters", "wall_wood_window_small", "wall_wood_window_stone", "warning_orange", "warning_traffic", "water_tower", "watermill", "watermill_wide", "weapon_arrow", "weapon_bow", "weapon_spear", "weapon_sword", "wheel", "wheelchair", "wheelchair_deluxe", "wheelchair_power", "wheelchair_power_deluxe", "windmill", "windmill_low", "wood_structure", "wood_support", "workbench", "workbench_anvil", "workbench_grind"]);
export const CELL_KINDS = Object.freeze({
  grass: { cell: 0, tile: 0 },
  dirt: { cell: 0, tile: TILE_IDS.dirt },
  lane: { cell: 0, tile: TILE_IDS.lane },
  water: { cell: 1, tile: TILE_IDS.water },
  cliff: { cell: 2, tile: 0 },
});
const MAX_LEVEL = 255;

export function newModel(width, height, cellMilli = 1000) {
  const n = width * height;
  return { width, height, cellMilli, heightStep: 1000, cell: new Uint8Array(n), tile: new Uint8Array(n), level: new Uint8Array(n), doodads: [], spawns: [], regions: [] };
}

export function modelFromMap(bytes) {
  const g = readGrid(bytes);
  const m = newModel(g.width, g.height, g.cellMilli);
  m.heightStep = g.heightStep;
  m.cell.set(g.flags);
  m.tile.set(g.tiles);
  m.level.set(g.levels);
  m.doodads = readDoodads(bytes).map((d) => ({ id: d.id, kind: DOODAD_KINDS[d.kind] ?? DOODAD_KINDS[0], x: d.x, y: d.y, rotation: d.rotation }));
  return m;
}

export function kindAt(m, x, y) {
  const i = y * m.width + x;
  return cellKind(m.cell[i], m.tile[i]);
}

function inside(m, x, y) {
  return x >= 0 && y >= 0 && x < m.width && y < m.height;
}

// Applies f(i) to the cells within `radius` of (x, y); radius 0 is one cell.
function brush(m, x, y, radius, f) {
  for (let dy = -radius; dy <= radius; dy++) {
    for (let dx = -radius; dx <= radius; dx++) {
      if (dx * dx + dy * dy <= radius * radius + radius && inside(m, x + dx, y + dy)) f((y + dy) * m.width + x + dx);
    }
  }
}

export function paintKind(m, x, y, kind, radius = 0) {
  const def = CELL_KINDS[kind];
  brush(m, x, y, radius, (i) => { m.cell[i] = def.cell; m.tile[i] = def.tile; });
}

export function paintLevel(m, x, y, level, radius = 0) {
  const v = Math.max(0, Math.min(MAX_LEVEL, level | 0));
  brush(m, x, y, radius, (i) => { m.level[i] = v; });
}

function nextDoodadId(m) {
  return m.doodads.reduce((top, d) => Math.max(top, d.id), 0) + 1;
}

export function placeDoodad(m, kind, xMilli, yMilli) {
  const d = { id: nextDoodadId(m), kind, x: Math.round(xMilli), y: Math.round(yMilli), rotation: 0 };
  m.doodads.push(d);
  return d;
}

export function placeSpawn(m, unit, player, xMilli, yMilli) {
  const s = { unit, player: player | 0, x: Math.round(xMilli), y: Math.round(yMilli) };
  m.spawns.push(s);
  return s;
}

export function addRegion(m, name, x0, y0, x1, y1) {
  const r = { name, x0: Math.min(x0, x1), y0: Math.min(y0, y1), x1: Math.max(x0, x1) + 1, y1: Math.max(y0, y1) + 1 };
  m.regions.push(r);
  return r;
}

// Deletes the doodad or spawn nearest the point within `reach` milli-units; returns what was removed or null.
export function deleteNear(m, xMilli, yMilli, reach) {
  let best = null;
  const consider = (list, kind) => list.forEach((item, index) => {
    const d = Math.hypot(item.x - xMilli, item.y - yMilli);
    if (d <= reach && (!best || d < best.d)) best = { d, list, index, kind };
  });
  consider(m.doodads, "doodad");
  consider(m.spawns, "spawn");
  if (!best) return null;
  return { kind: best.kind, item: best.list.splice(best.index, 1)[0] };
}

// Rectangles covering every cell whose value is not `skip`: row runs of equal value, merged down while they line up.
export function rects(values, width, height, skip) {
  const out = [];
  let open = new Map();
  for (let y = 0; y < height; y++) {
    const next = new Map();
    let x = 0;
    while (x < width) {
      const v = values[y * width + x];
      if (v === skip) { x++; continue; }
      let e = x + 1;
      while (e < width && values[y * width + e] === v) e++;
      const key = `${x},${e},${v}`;
      const r = open.get(key);
      if (r) { r.y1 = y + 1; next.set(key, r); } else {
        const fresh = { x0: x, y0: y, x1: e, y1: y + 1, v };
        out.push(fresh);
        next.set(key, fresh);
      }
      x = e;
    }
    open = next;
  }
  return out;
}

export function exportOsd(m) {
  const lines = [`grid(${m.width}, ${m.height}, cell_milli=${m.cellMilli}, fill=0)`];
  const rect = (fn, r) => `${fn}(${r.x0}, ${r.y0}, ${r.x1}, ${r.y1}, ${r.v})`;
  for (const r of rects(m.cell, m.width, m.height, 0)) lines.push(rect("paint", r));
  for (const r of rects(m.tile, m.width, m.height, 0)) lines.push(rect("paint_tile", r));
  lines.push(`height_step(${m.heightStep})`);
  for (const r of rects(m.level, m.width, m.height, 0)) lines.push(rect("paint_height", r));
  for (const d of m.doodads) lines.push(`doodad(${d.id}, "${d.kind}", ${d.x}, ${d.y}${d.rotation ? `, rotation=${d.rotation}` : ""})`);
  for (const s of m.spawns) lines.push(`spawn("${s.unit}", ${s.player}, ${s.x}, ${s.y})`);
  for (const r of m.regions) lines.push(`# region ${r.name} ${r.x0} ${r.y0} ${r.x1} ${r.y1}`);
  return lines.join("\n") + "\n";
}

// Pure world helpers for drawing: cell units on x and z, height in cells.
function levelHeight(m, i) {
  return (m.level[i] * m.heightStep) / m.cellMilli;
}

export function kindMeshes(m) {
  const n = m.width * m.height;
  const kinds = [];
  const kindOf = new Uint8Array(n);
  const counts = [];
  for (let i = 0; i < n; i++) {
    const kind = cellKind(m.cell[i], m.tile[i]);
    let k = kinds.indexOf(kind);
    if (k < 0) {
      k = kinds.push(kind) - 1;
      counts.push(0);
    }
    kindOf[i] = k;
    counts[k]++;
  }
  const bufs = kinds.map((_, k) => ({ positions: new Float32Array(counts[k] * 12), normals: new Float32Array(counts[k] * 12), indices: new Uint32Array(counts[k] * 6), at: 0 }));
  for (let i = 0; i < n; i++) {
    const buf = bufs[kindOf[i]];
    const c = buf.at++;
    const x = i % m.width;
    const y = (i - x) / m.width;
    const h = levelHeight(m, i);
    const p = c * 12;
    const pos = buf.positions;
    pos[p] = x; pos[p + 1] = h; pos[p + 2] = y;
    pos[p + 3] = x + 1; pos[p + 4] = h; pos[p + 5] = y;
    pos[p + 6] = x; pos[p + 7] = h; pos[p + 8] = y + 1;
    pos[p + 9] = x + 1; pos[p + 10] = h; pos[p + 11] = y + 1;
    buf.normals[p + 1] = buf.normals[p + 4] = buf.normals[p + 7] = buf.normals[p + 10] = 1;
    const b = c * 4;
    const ix = c * 6;
    buf.indices[ix] = b; buf.indices[ix + 1] = b + 2; buf.indices[ix + 2] = b + 1;
    buf.indices[ix + 3] = b + 1; buf.indices[ix + 4] = b + 2; buf.indices[ix + 5] = b + 3;
  }
  const meshes = new Map();
  kinds.forEach((kind, k) => meshes.set(kind, { positions: bufs[k].positions, normals: bufs[k].normals, uvs: new Float32Array(counts[k] * 8), indices: bufs[k].indices, color: TILE_TINTS[kind] }));
  return meshes;
}

function cubeMesh(color) {
  const p = [];
  const idx = [];
  const n = [];
  const faces = [[[0, 0, 1], [1, 0, 0], [0, 1, 0]], [[0, 0, -1], [-1, 0, 0], [0, 1, 0]], [[1, 0, 0], [0, 0, -1], [0, 1, 0]], [[-1, 0, 0], [0, 0, 1], [0, 1, 0]], [[0, 1, 0], [1, 0, 0], [0, 0, -1]], [[0, -1, 0], [1, 0, 0], [0, 0, 1]]];
  for (const [nrm, u, v] of faces) {
    const base = p.length / 3;
    for (const [a, b] of [[-1, -1], [1, -1], [-1, 1], [1, 1]]) {
      for (let k = 0; k < 3; k++) p.push(0.5 * nrm[k] + 0.5 * a * u[k] + 0.5 * b * v[k] + (k === 1 ? 0.5 : 0));
      n.push(...nrm);
    }
    idx.push(base, base + 1, base + 2, base + 1, base + 3, base + 2);
  }
  return { positions: Float32Array.from(p), normals: Float32Array.from(n), uvs: new Float32Array(p.length / 3 * 2), indices: Uint32Array.from(idx), color };
}

function invert(m) {
  const a = Float64Array.from(m);
  const inv = Float64Array.from([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]);
  for (let c = 0; c < 4; c++) {
    let p = c;
    for (let r = c + 1; r < 4; r++) if (Math.abs(a[c * 4 + r]) > Math.abs(a[c * 4 + p])) p = r;
    for (let k = 0; k < 4; k++) {
      [a[k * 4 + c], a[k * 4 + p]] = [a[k * 4 + p], a[k * 4 + c]];
      [inv[k * 4 + c], inv[k * 4 + p]] = [inv[k * 4 + p], inv[k * 4 + c]];
    }
    const d = a[c * 4 + c] || 1e-12;
    for (let k = 0; k < 4; k++) { a[k * 4 + c] /= d; inv[k * 4 + c] /= d; }
    for (let r = 0; r < 4; r++) {
      if (r === c) continue;
      const f = a[c * 4 + r];
      for (let k = 0; k < 4; k++) { a[k * 4 + r] -= f * a[k * 4 + c]; inv[k * 4 + r] -= f * inv[k * 4 + c]; }
    }
  }
  return inv;
}

// The ground point (cell units) under a pixel: unprojects the pixel to a ray and meets it with y = 0.
export function pickGround(cam, aspect, px, py, w, h) {
  const inv = invert(viewProjection(cam, aspect));
  const ndc = (z) => {
    const v = [(px / w) * 2 - 1, 1 - (py / h) * 2, z, 1];
    const o = [0, 0, 0, 0];
    for (let r = 0; r < 4; r++) for (let k = 0; k < 4; k++) o[r] += inv[k * 4 + r] * v[k];
    return [o[0] / o[3], o[1] / o[3], o[2] / o[3]];
  };
  const a = eyeOf(cam);
  const b = ndc(1);
  const dir = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
  if (Math.abs(dir[1]) < 1e-9) return null;
  const t = -a[1] / dir[1];
  if (t < 0) return null;
  return { x: a[0] + dir[0] * t, z: a[2] + dir[2] * t };
}

function startPage() {
  const $ = (id) => document.getElementById(id);
  const canvas = $("view");
  const gl = canvas.getContext("webgl2", { antialias: true });
  if (!gl) { $("status").textContent = "WebGL2 is required"; return; }
  const renderer = createRenderer(gl);
  const cam = createCamera({ distance: 50, pitch: 1.0 });
  let model = newModel(32, 32);
  let meshes = new Map();
  let dirty = true;
  let regionStart = null;
  const markers = { doodad: cubeMesh([0.85, 0.7, 0.2]), region: cubeMesh([0.8, 0.2, 0.8]), spawn: [0, 1, 2, 3].map((p) => cubeMesh([[0.9, 0.2, 0.2], [0.2, 0.4, 0.9], [0.2, 0.8, 0.3], [0.9, 0.8, 0.2]][p])), neutral: cubeMesh([0.7, 0.7, 0.7]) };

  const fill = (select, items) => { select.replaceChildren(...items.map((v) => new Option(v, v))); };
  fill($("kind"), Object.keys(CELL_KINDS));
  fill($("doodad-kind"), DOODAD_KINDS);

  const load = (bytes) => {
    model = modelFromMap(bytes);
    cam.target = [model.width / 2, 0, model.height / 2];
    cam.distance = Math.max(model.width, model.height) * 1.1;
    cam.maxDistance = cam.distance * 4;
    cam.far = cam.maxDistance * 2;
    dirty = true;
    $("status").textContent = `${model.width}x${model.height} cells, ${model.doodads.length} doodads`;
  };
  const rebuild = () => { meshes = kindMeshes(model); dirty = false; };
  const status = (text) => { $("status").textContent = text; };

  const apply = (x, z) => {
    const cx = Math.floor(x);
    const cy = Math.floor(z);
    const tool = $("tool").value;
    const radius = Number($("radius").value);
    const xm = x * model.cellMilli;
    const ym = z * model.cellMilli;
    if (tool === "paint" && inside(model, cx, cy)) { paintKind(model, cx, cy, $("kind").value, radius); dirty = true; }
    else if (tool === "height" && inside(model, cx, cy)) { paintLevel(model, cx, cy, Number($("level").value), radius); dirty = true; }
    else if (tool === "doodad") placeDoodad(model, $("doodad-kind").value, xm, ym);
    else if (tool === "spawn") placeSpawn(model, $("unit").value, Number($("player").value), xm, ym);
    else if (tool === "delete") { const gone = deleteNear(model, xm, ym, model.cellMilli); if (gone) status(`deleted ${gone.kind}`); }
    else if (tool === "region" && inside(model, cx, cy)) {
      if (!regionStart) { regionStart = [cx, cy]; status("region: click the opposite corner"); }
      else { const r = addRegion(model, $("region-name").value || "region", regionStart[0], regionStart[1], cx, cy); regionStart = null; status(`region ${r.name} ${r.x0},${r.y0}..${r.x1},${r.y1}`); }
    }
  };

  const aspect = () => canvas.width / Math.max(1, canvas.height);
  const at = (e) => { const r = canvas.getBoundingClientRect(); return pickGround(cam, aspect(), e.clientX - r.left, e.clientY - r.top, r.width, r.height); };
  let drag = null;
  canvas.addEventListener("contextmenu", (e) => e.preventDefault());
  canvas.addEventListener("pointerdown", (e) => {
    canvas.setPointerCapture(e.pointerId);
    drag = { button: e.button, x: e.clientX, y: e.clientY };
    if (e.button === 0) { const p = at(e); if (p) apply(p.x, p.z); }
  });
  canvas.addEventListener("pointermove", (e) => {
    if (!drag) return;
    const dx = e.clientX - drag.x;
    const dy = e.clientY - drag.y;
    drag.x = e.clientX;
    drag.y = e.clientY;
    if (drag.button === 2) orbitCamera(cam, dx, dy);
    else if (drag.button === 1 || e.shiftKey) panCamera(cam, dx, dy, canvas.height);
    else if (drag.button === 0 && ["paint", "height"].includes($("tool").value)) { const p = at(e); if (p) apply(p.x, p.z); }
  });
  canvas.addEventListener("pointerup", () => { drag = null; });
  canvas.addEventListener("wheel", (e) => { e.preventDefault(); zoomCamera(cam, e.deltaY); }, { passive: false });

  $("open").addEventListener("change", async (e) => {
    const f = e.target.files[0];
    if (f) load(new Uint8Array(await f.arrayBuffer()));
  });
  $("export").addEventListener("click", () => {
    const text = exportOsd(model);
    $("osd").value = text;
    const a = document.createElement("a");
    a.href = URL.createObjectURL(new Blob([text], { type: "text/plain" }));
    a.download = "map.osd";
    a.click();
    status(`exported ${text.split("\n").length - 1} lines`);
  });
  $("new").addEventListener("click", () => {
    model = newModel(Number($("new-w").value), Number($("new-h").value));
    cam.target = [model.width / 2, 0, model.height / 2];
    dirty = true;
  });

  const drawBox = (mesh, xCell, zCell, size, height) => {
    const gx = Math.floor(xCell);
    const gy = Math.floor(zCell);
    const base = inside(model, gx, gy) ? levelHeight(model, gy * model.width + gx) : 0;
    renderer.drawMesh(mesh, multiply(translation(xCell - size / 2, base, zCell - size / 2), scaling(size, height, size)));
  };

  const frame = () => {
    const w = Math.floor(canvas.clientWidth * devicePixelRatio);
    const h = Math.floor(canvas.clientHeight * devicePixelRatio);
    if (canvas.width !== w || canvas.height !== h) { canvas.width = w; canvas.height = h; }
    if (dirty) rebuild();
    renderer.setCamera({ ...cam, aspect: aspect() });
    renderer.beginFrame();
    for (const mesh of meshes.values()) renderer.drawMesh(mesh, null);
    for (const d of model.doodads) drawBox(markers.doodad, d.x / model.cellMilli, d.y / model.cellMilli, 0.6, 0.8);
    for (const s of model.spawns) drawBox(s.player >= 0 && s.player < 4 ? markers.spawn[s.player] : markers.neutral, s.x / model.cellMilli, s.y / model.cellMilli, 0.5, 1.4);
    for (const r of model.regions) {
      for (const [x, z] of [[r.x0, r.y0], [r.x1, r.y0], [r.x0, r.y1], [r.x1, r.y1]]) drawBox(markers.region, x, z, 0.3, 2);
    }
    requestAnimationFrame(frame);
  };

  // The map image is the pack's manifest.image, read through pack_load.js as the page reads it. Imported here, not at the top,
  // so the module (and its pack fetch) loads only in the browser.
  import("./pack_load.js")
    .then(({ manifest, packBytes }) => packBytes(manifest.image))
    .then((b) => load(new Uint8Array(b)))
    .catch(() => status("map image not loaded: open a file or create a new map"));
  requestAnimationFrame(frame);
}

if (typeof document !== "undefined") startPage();

use std::path::{Path, PathBuf};

use mod_host::{build_image, ModHost};
use sim::fixed::{self, HALF};

const ORDER_MOVE: u8 = 0;
const ATTACKER: &str = "antimage";
const ENEMY: &str = "axe";
const LANE_CREEP: &str = "creep";
const ATTACKER_PLAYER: u8 = 0;
const ENEMY_PLAYER: u8 = 1;
const LANE_CREEP_PLAYER: u8 = 1;
const FOG_ENEMY_PLAYER: u8 = 3;
const CREEP_TEAM: u8 = 2;
const ORIGIN_X: i32 = 92;
const ORIGIN_Y: i32 = 240;
const ENEMY_GAP_CELLS: i32 = 5;
const VISIBLE_ENEMY_GAP_CELLS: i32 = -12;
const FAR_ENEMY_GAP_CELLS: i32 = 40;
const CREEP_OFFSET_X: i32 = 2;
const CREEP_OFFSET_Y: i32 = 1;
const SECOND_CREEP_OFFSET_Y: i32 = -2;
const HOLD_TICKS: u32 = 2 * 16;

fn pack_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba")
}

fn collect(dir: &Path, rel: &str, out: &mut Vec<(String, String)>) {
    for entry in std::fs::read_dir(dir).expect("read the pack directory") {
        let path = entry.expect("read a pack entry").path();
        let name = path.file_name().expect("a pack file name").to_string_lossy().into_owned();
        if path.is_dir() {
            if name != "fixtures" {
                collect(&path, &format!("{rel}{name}/"), out);
            }
        } else if path.extension().is_some_and(|e| e == "lua") {
            out.push((format!("{rel}{name}"), std::fs::read_to_string(&path).expect("read a lua module")));
        }
    }
}

fn stem(rel: &str) -> &str {
    let file = rel.rsplit('/').next().unwrap_or(rel);
    file.strip_suffix(".lua").unwrap_or(file)
}

fn visit(files: &[(String, String)], module: &str, out: &mut Vec<(String, String)>) {
    let Some((rel, src)) = files.iter().find(|(rel, _)| stem(rel) == module) else { return };
    if out.iter().any(|(name, _)| name == rel) {
        return;
    }
    out.push((rel.clone(), src.clone()));
    for required in src.split("require(\"").skip(1).filter_map(|rest| rest.split('"').next()) {
        visit(files, required, out);
    }
}

fn pack_host() -> ModHost {
    let dir = pack_dir();
    let manifest: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("manifest.json")).expect("the pack manifest"))
        .expect("a valid pack manifest");
    let rules = std::fs::read_to_string(dir.join(manifest["rules"].as_str().expect("the manifest rules file")))
        .expect("the pack rules file");
    let mut files = Vec::new();
    collect(&dir, "", &mut files);
    files.sort();
    let mut mods = Vec::new();
    for root in manifest["mods"].as_array().expect("the manifest mods list") {
        visit(&files, stem(root.as_str().expect("a mod file name")), &mut mods);
    }
    let image = build_image(&dir).expect("build the moba image");
    let mut host = ModHost::new(&image).expect("load the moba image");
    assert!(host.sim_mut().set_attr_rules_text(&rules), "the pack rules are accepted");
    host.load_mods(&mods).expect("load the moba mods");
    host.sim_mut().set_fog(false);
    host
}

fn hero(host: &mut ModHost, kind_name: &str, player: u8, cell_x: i32, cell_y: i32) -> usize {
    let mut sim = host.sim_mut();
    let kind = sim.catalog().unit_index(kind_name).expect("the hero kind is in the moba catalog");
    let x = fixed::from_int(cell_x) + HALF;
    let y = fixed::from_int(cell_y) + HALF;
    sim.spawn_attributed(player, kind as u32, x, y).expect("the hero fits on the map")
}

fn creep(host: &mut ModHost, player: u8, cell_x: i32, cell_y: i32) -> usize {
    let mut sim = host.sim_mut();
    let kind = sim.catalog().unit_index(LANE_CREEP).expect("the lane creep kind is in the moba catalog");
    let x = fixed::from_int(cell_x) + HALF;
    let y = fixed::from_int(cell_y) + HALF;
    sim.spawn_for(player, kind as u32, x, y).expect("the creep fits on the map")
}

struct Field {
    host: ModHost,
    attacker: usize,
    enemy: usize,
    lane_creep: usize,
    bystander: usize,
}

fn field() -> Field {
    let mut host = pack_host();
    let attacker = hero(&mut host, ATTACKER, ATTACKER_PLAYER, ORIGIN_X, ORIGIN_Y);
    let enemy = hero(&mut host, ENEMY, ENEMY_PLAYER, ORIGIN_X + ENEMY_GAP_CELLS, ORIGIN_Y);
    let lane_creep = creep(&mut host, ENEMY_PLAYER, ORIGIN_X + CREEP_OFFSET_X, ORIGIN_Y + CREEP_OFFSET_Y);
    let bystander = creep(&mut host, ENEMY_PLAYER, ORIGIN_X + CREEP_OFFSET_X, ORIGIN_Y + SECOND_CREEP_OFFSET_Y);
    Field { host, attacker, enemy, lane_creep, bystander }
}

fn aggro_of(host: &ModHost, unit: usize) -> i32 {
    host.sim().aggro_target(unit)
}

fn advance(host: &mut ModHost, ticks: u32) {
    for _ in 0..ticks {
        host.advance().expect("advance the match");
    }
}

#[test]
fn a_right_click_on_an_enemy_hero_hands_the_nearby_lane_creep_to_the_attacker() {
    let mut f = field();
    let attacker = f.attacker;
    assert_eq!(aggro_of(&f.host, f.lane_creep), -1, "the lane creep starts without an aggro target");
    f.host.sim_mut().order_attack_or_approach(attacker, f.enemy).expect("the attack order on the enemy hero is accepted");
    f.host.advance().expect("advance the match");
    assert_eq!(aggro_of(&f.host, f.lane_creep), attacker as i32, "the lane creep is redirected to the right-clicking hero");
}

#[test]
fn a_move_order_towards_an_enemy_hero_does_not_redirect_the_lane_creep() {
    let mut f = field();
    let attacker = f.attacker;
    let (x, y) = f.host.sim().position_fixed(f.enemy);
    f.host.sim_mut().issue_order_kind(attacker, ORDER_MOVE, x, y).expect("the move order is accepted");
    advance(&mut f.host, 4);
    assert_eq!(aggro_of(&f.host, f.lane_creep), -1, "a move order does not redirect the lane creep");
}

#[test]
fn a_right_click_on_a_lane_creep_does_not_redirect_the_other_lane_creep() {
    let mut f = field();
    let attacker = f.attacker;
    let target = f.bystander;
    f.host.sim_mut().order_attack_or_approach(attacker, target).expect("the attack order on the creep is accepted");
    f.host.advance().expect("advance the match");
    assert_eq!(aggro_of(&f.host, f.lane_creep), -1, "a right-click on a creep is not a hero hit");
}

fn fogged_arena(enemy_gap_cells: i32, spotter: bool) -> (ModHost, usize, usize) {
    let mut host = pack_host();
    host.sim_mut().set_fog(true);
    let attacker = hero(&mut host, ATTACKER, ATTACKER_PLAYER, ORIGIN_X, ORIGIN_Y);
    let enemy_x = ORIGIN_X + enemy_gap_cells;
    let enemy = hero(&mut host, ENEMY, FOG_ENEMY_PLAYER, enemy_x, ORIGIN_Y);
    if spotter {
        creep(&mut host, ATTACKER_PLAYER, enemy_x - 1, ORIGIN_Y);
    }
    advance(&mut host, 1);
    (host, attacker, enemy)
}

fn lane_creep_beside_attacker(host: &mut ModHost) -> usize {
    creep(host, LANE_CREEP_PLAYER, ORIGIN_X + CREEP_OFFSET_X, ORIGIN_Y + CREEP_OFFSET_Y)
}

#[test]
fn a_right_click_the_enemy_hero_can_see_redirects_the_lane_creep_under_fog() {
    let (mut host, attacker, enemy) = fogged_arena(VISIBLE_ENEMY_GAP_CELLS, false);
    assert!(host.sim().visible(FOG_ENEMY_PLAYER, attacker), "the enemy hero's owner sees the attacker 5 cells away");
    let lane_creep = lane_creep_beside_attacker(&mut host);
    host.sim_mut().order_attack_or_approach(attacker, enemy).expect("the attack order on the enemy hero is accepted");
    advance(&mut host, 2);
    assert_eq!(aggro_of(&host, lane_creep), attacker as i32, "a right-click the enemy can see redirects the lane creep");
}

#[test]
fn a_right_click_the_enemy_hero_cannot_see_does_not_redirect_the_lane_creep() {
    let (mut host, attacker, enemy) = fogged_arena(FAR_ENEMY_GAP_CELLS, true);
    assert!(host.sim().visible(ATTACKER_PLAYER, enemy), "the attacker sees the enemy hero through its spotter");
    assert!(!host.sim().visible(FOG_ENEMY_PLAYER, attacker), "the enemy hero's owner cannot see the attacker");
    let lane_creep = lane_creep_beside_attacker(&mut host);
    assert!(host.sim().hostile(LANE_CREEP_PLAYER, ATTACKER_PLAYER), "the lane creep is hostile to the attacker");
    host.sim_mut().order_attack_or_approach(attacker, enemy).expect("the attack order on the enemy hero is accepted");
    advance(&mut host, 2);
    assert_eq!(aggro_of(&host, lane_creep), -1, "a right-click the enemy cannot see does not redirect the lane creep (yU)");
}

#[test]
fn a_redirected_lane_creep_attacks_the_hero_for_the_hold_then_returns_to_its_order() {
    let mut f = field();
    let attacker = f.attacker;
    let creep = f.lane_creep;
    f.host.sim_mut().order_attack_or_approach(attacker, f.enemy).expect("the attack order on the enemy hero is accepted");
    advance(&mut f.host, 2);
    assert_eq!(aggro_of(&f.host, creep), attacker as i32, "the lane creep is redirected");
    assert_eq!(f.host.sim().attack_order_of(creep), attacker as i32, "the redirected lane creep attacks the hero");
    advance(&mut f.host, HOLD_TICKS);
    assert_eq!(aggro_of(&f.host, creep), -1, "the redirect ends after the hold");
    assert_eq!(f.host.sim().attack_order_of(creep), -1, "the lane creep stops attacking the hero when the hold ends");
}

const NON_LANE_CREEP_PLAYER: u8 = 2;

#[test]
fn a_hostile_creep_that_no_lane_player_owns_is_not_redirected() {
    let mut host = pack_host();
    host.sim_mut()
        .set_team(NON_LANE_CREEP_PLAYER, CREEP_TEAM)
        .expect("the stray creep side has a team of its own");
    let attacker = hero(&mut host, ATTACKER, ATTACKER_PLAYER, ORIGIN_X, ORIGIN_Y);
    let enemy = hero(&mut host, ENEMY, ENEMY_PLAYER, ORIGIN_X + ENEMY_GAP_CELLS, ORIGIN_Y);
    let stray = creep(&mut host, NON_LANE_CREEP_PLAYER, ORIGIN_X + CREEP_OFFSET_X, ORIGIN_Y + CREEP_OFFSET_Y);
    assert!(host.sim().hostile(NON_LANE_CREEP_PLAYER, ATTACKER_PLAYER), "the stray creep is hostile to the attacker");
    host.sim_mut().order_attack_or_approach(attacker, enemy).expect("the attack order on the enemy hero is accepted");
    advance(&mut host, 2);
    assert_eq!(
        aggro_of(&host, stray),
        -1,
        "6.88g SME (war3map.j 31391) redirects only creeps and towers that players 0 and 1 own"
    );
}

const SIEGE_CREEP: &str = "creep_siege";
const IMAGE_LIFE_TICKS: i32 = 60 * 16;
const IMAGE_PERCENT: i32 = 100;

fn spawn_kind(host: &mut ModHost, kind_name: &str, player: u8, cell_x: i32, cell_y: i32) -> usize {
    let mut sim = host.sim_mut();
    let kind = sim.catalog().unit_index(kind_name).expect("the kind is in the moba catalog");
    let x = fixed::from_int(cell_x) + HALF;
    let y = fixed::from_int(cell_y) + HALF;
    sim.spawn_for(player, kind as u32, x, y).expect("the unit fits on the map")
}

fn enemy_image(host: &mut ModHost, caster: usize, cell_x: i32, cell_y: i32) -> usize {
    let mut sim = host.sim_mut();
    let x = fixed::from_int(cell_x) + HALF;
    let y = fixed::from_int(cell_y) + HALF;
    sim.spawn_image(caster, x, y, IMAGE_LIFE_TICKS, IMAGE_PERCENT, IMAGE_PERCENT).expect("the enemy hero makes an image")
}

#[test]
fn a_right_click_on_an_enemy_illusion_does_not_redirect_the_lane_creep() {
    let mut host = pack_host();
    let attacker = hero(&mut host, ATTACKER, ATTACKER_PLAYER, ORIGIN_X, ORIGIN_Y);
    let enemy = hero(&mut host, ENEMY, ENEMY_PLAYER, ORIGIN_X + FAR_ENEMY_GAP_CELLS, ORIGIN_Y);
    let image = enemy_image(&mut host, enemy, ORIGIN_X + ENEMY_GAP_CELLS, ORIGIN_Y);
    let (image_x, image_y) = host.sim().position_fixed(image);
    host.sim_mut().enroll_attributed(image, image_x, image_y);
    let lane_creep = lane_creep_beside_attacker(&mut host);
    host.sim_mut().order_attack_or_approach(attacker, image).expect("the attack order on the illusion is accepted");
    advance(&mut host, 2);
    assert_eq!(host.sim().attack_order_of(attacker), image as i32, "the attacker's order lands on the illusion");
    assert_eq!(
        aggro_of(&host, lane_creep),
        -1,
        "6.88g SUE (war3map.j 31437) redirects only a hero that is not an illusion: IsUnitIllusion(mS)==false"
    );
}

#[test]
fn a_right_click_on_an_enemy_hero_does_not_redirect_a_siege_creep() {
    let mut f = field();
    let attacker = f.attacker;
    let siege = spawn_kind(&mut f.host, SIEGE_CREEP, LANE_CREEP_PLAYER, ORIGIN_X + CREEP_OFFSET_X, ORIGIN_Y + 3);
    f.host.sim_mut().order_attack_or_approach(attacker, f.enemy).expect("the attack order on the enemy hero is accepted");
    advance(&mut f.host, 2);
    assert_eq!(aggro_of(&f.host, siege), -1, "6.88g SME (war3map.j 31391) excludes Aw, the siege units");
}

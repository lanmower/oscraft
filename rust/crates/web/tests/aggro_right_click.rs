use std::path::{Path, PathBuf};

use mod_host::{build_image, ModHost};
use sim::fixed::{self, HALF};

const ORDER_MOVE: u8 = 0;
const ATTACKER: &str = "antimage";
const ENEMY: &str = "axe";
const LANE_CREEP: &str = "creep";
const ATTACKER_PLAYER: u8 = 0;
const ENEMY_PLAYER: u8 = 1;
const ORIGIN_X: i32 = 92;
const ORIGIN_Y: i32 = 240;
const ENEMY_GAP_CELLS: i32 = 5;
const CREEP_OFFSET_X: i32 = 2;
const CREEP_OFFSET_Y: i32 = 1;
const SECOND_CREEP_OFFSET_Y: i32 = -2;

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
    for _ in 0..4 {
        f.host.advance().expect("advance the match");
    }
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

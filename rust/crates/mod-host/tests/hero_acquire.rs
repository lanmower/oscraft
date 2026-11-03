use std::path::{Path, PathBuf};

use mod_host::{build_image, ModHost};
use sim::fixed::{self, HALF};

const ORDER_STOP: u8 = 1;
const ORDER_ATTACK_MOVE: u8 = 2;
const ATTACKER: &str = "antimage";
const ENEMY: &str = "axe";
const ACQUIRE_GAME_UNITS: u32 = 640;
const GAP_CELLS: i32 = 5;
const SEARCH_TICKS: u32 = 400;

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

fn distance_cells(host: &ModHost, a: usize, b: usize) -> f64 {
    let sim = host.sim();
    let (ax, ay) = sim.position_fixed(a);
    let (bx, by) = sim.position_fixed(b);
    let dx = f64::from(ax - bx) / f64::from(fixed::ONE);
    let dy = f64::from(ay - by) / f64::from(fixed::ONE);
    (dx * dx + dy * dy).sqrt()
}

fn engage(order: impl FnOnce(&mut ModHost, usize)) {
    let mut host = pack_host();
    let (ax, ay) = (92, 240);
    let attacker = hero(&mut host, ATTACKER, 0, ax, ay);
    let enemy = hero(&mut host, ENEMY, 1, ax + GAP_CELLS, ay);
    host.sim_mut().set_acquire(ENEMY, 0).expect("the enemy kind takes no acquire range");
    host.sim_mut().set_acquire(ATTACKER, ACQUIRE_GAME_UNITS).expect("the attacker kind takes an acquire range");

    let (_, weapon_range) = host.sim().weapon(attacker);
    let start = distance_cells(&host, attacker, enemy);
    let acquire = f64::from(host.sim().acquire_of(attacker)) / 1000.0;
    assert!(start > f64::from(weapon_range) / f64::from(fixed::ONE), "the enemy starts beyond weapon reach ({start} cells)");
    assert!(start <= acquire, "the enemy starts inside acquire range ({acquire} cells)");

    order(&mut host, attacker);
    let enemy_hp = host.sim().hp(enemy);
    let start_pos = host.sim().position_fixed(attacker);
    let mut struck_at = None;
    for t in 1..=SEARCH_TICKS {
        host.advance().expect("advance the match");
        if host.sim().hp(enemy) < enemy_hp {
            struck_at = Some(t);
            break;
        }
    }

    let t = struck_at.expect("the hero acquired the enemy and attacked it");
    assert_ne!(host.sim().position_fixed(attacker), start_pos, "the hero chased the enemy (struck at tick {t})");
    assert!(distance_cells(&host, attacker, enemy) <= f64::from(weapon_range) / f64::from(fixed::ONE) + 1.0, "the strike came from weapon reach");
}

#[test]
fn an_attack_moving_hero_chases_an_enemy_hero_inside_acquire_range_and_attacks() {
    engage(|host, attacker| {
        let (x, y) = host.sim().position_fixed(attacker);
        host.sim_mut().issue_order_kind(attacker, ORDER_ATTACK_MOVE, x, y).expect("the attack-move order is accepted");
    });
}

#[test]
fn an_idle_hero_chases_an_enemy_hero_inside_acquire_range_and_attacks() {
    engage(|_, _| {});
}

#[test]
fn a_stopped_hero_chases_an_enemy_hero_inside_acquire_range_and_attacks() {
    engage(|host, attacker| {
        let (x, y) = host.sim().position_fixed(attacker);
        host.sim_mut().issue_order_kind(attacker, ORDER_STOP, x, y).expect("the stop order is accepted");
    });
}

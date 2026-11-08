use std::path::{Path, PathBuf};

use mod_host::{build_image, ModHost};
use serde_json::json;
use sim::fixed::{self, ONE};

const OWNER: u8 = 0;
const NEUTRAL: u8 = 255;
const ORDER_MOVE: u8 = 0;
const HERO_X: i32 = 92;
const HERO_Y: i32 = 240;
const CREEP_OFFSET_CELLS: i32 = 2;
const TARGET_CELLS: i32 = 5;
const TICKS: usize = 60;
const BEFORE_SWING_TICKS: usize = 5;

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

fn spawn(host: &mut ModHost, kind_name: &str, player: u8, x: i32, y: i32) -> usize {
    let mut sim = host.sim_mut();
    let kind = sim.catalog().unit_index(kind_name).expect("the kind is in the moba catalog");
    sim.spawn_attributed(player, kind as u32, x, y).expect("the unit fits on the map")
}

fn one_hand_on(host: &mut ModHost) {
    host.dispatch_action("ui", &json!({ "call": "one_hand", "args": { "on": true }, "player": OWNER }))
        .expect("the panels accept the one-hand toggle");
}

fn step_length(from: (i32, i32), to: (i32, i32)) -> i64 {
    let dx = i64::from(to.0) - i64::from(from.0);
    let dy = i64::from(to.1) - i64::from(from.1);
    ((dx * dx + dy * dy) as f64).sqrt() as i64
}

#[test]
fn a_walking_strike_on_move_hero_keeps_walking_through_its_swing() {
    let mut host = pack_host();
    let hero = spawn(&mut host, "pudge", OWNER, fixed::from_int(HERO_X) + ONE / 2, fixed::from_int(HERO_Y) + ONE / 2);
    let creep = spawn(
        &mut host,
        "creep",
        NEUTRAL,
        fixed::from_int(HERO_X) + ONE / 2 + CREEP_OFFSET_CELLS * ONE,
        fixed::from_int(HERO_Y) + ONE / 2,
    );
    one_hand_on(&mut host);
    let target = (fixed::from_int(HERO_X + TARGET_CELLS) + ONE / 2, fixed::from_int(HERO_Y) + ONE / 2);
    let hp0 = host.sim().hp(creep);

    let mut steps = Vec::with_capacity(TICKS);
    let mut in_flight = Vec::with_capacity(TICKS);
    let mut previous = host.sim().position_fixed(hero);
    for _ in 0..TICKS {
        host.sim_mut().issue_order_kind(hero, ORDER_MOVE, target.0, target.1).expect("the move order is accepted");
        host.advance().expect("advance the match");
        let position = host.sim().position_fixed(hero);
        steps.push(step_length(previous, position));
        in_flight.push(host.sim().hit_in_flight(hero));
        previous = position;
    }

    let first = in_flight.iter().position(|&flying| flying).expect("the walking hero starts a swing while one-hand is on");
    let walking_step = steps[first.saturating_sub(BEFORE_SWING_TICKS)..first].iter().copied().max().unwrap_or(0);
    let swing_steps: Vec<i64> = (first..TICKS).take_while(|&t| in_flight[t]).map(|t| steps[t]).collect();
    assert!(walking_step > 0, "the hero is walking before its swing starts");
    assert!(swing_steps.len() >= 3, "the swing lasts several ticks: {} swing ticks", swing_steps.len());
    assert!(
        swing_steps.iter().all(|&step| 2 * step >= walking_step),
        "the hero keeps walking through its swing: steps {swing_steps:?} against a walking step of {walking_step}"
    );
    assert!(host.sim().hp(creep) < hp0, "the swing lands while the hero walks");
}

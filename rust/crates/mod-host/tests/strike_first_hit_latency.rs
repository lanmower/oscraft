use std::path::{Path, PathBuf};

use mod_host::{build_image, ModHost};
use sim::fixed::{self, HALF, ONE};

const OWNER: u8 = 0;
const ENEMY: u8 = 1;
const HERO_X: i32 = 92;
const HERO_Y: i32 = 240;
const TARGET_OFFSET: i32 = ONE;
const REACH: i64 = 2 * ONE as i64;
const TICKS: usize = 20;
const MAX_LATENCY_ADVANCES: usize = 1;

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

fn spawn_hero(host: &mut ModHost, x: i32, y: i32) -> usize {
    let mut sim = host.sim_mut();
    let kind = sim.catalog().unit_index("pudge").expect("the hero kind is in the moba catalog");
    sim.spawn_attributed(OWNER, kind as u32, x, y).expect("the hero fits on the map")
}

fn place_target(host: &mut ModHost, x: i32, y: i32) -> usize {
    let mut sim = host.sim_mut();
    let kind = sim.catalog().unit_index("objective").expect("the target kind is in the moba catalog");
    sim.spawn_for(ENEMY, kind as u32, x, y).expect("the target fits on the map")
}

fn distance_sq(a: (i32, i32), b: (i32, i32)) -> i64 {
    let (dx, dy) = (i64::from(a.0) - i64::from(b.0), i64::from(a.1) - i64::from(b.1));
    dx * dx + dy * dy
}

#[test]
fn a_hero_in_reach_starts_its_first_swing_within_one_advance() {
    let mut host = pack_host();
    let hero = spawn_hero(&mut host, fixed::from_int(HERO_X) + HALF, fixed::from_int(HERO_Y) + HALF);
    let target = place_target(&mut host, fixed::from_int(HERO_X) + HALF + TARGET_OFFSET, fixed::from_int(HERO_Y) + HALF);
    let mut first_reach = None;
    let mut first_flight = None;
    for advance in 0..TICKS {
        host.advance().expect("advance the match");
        let in_reach = distance_sq(host.sim().position_fixed(hero), host.sim().position_fixed(target)) <= REACH * REACH;
        if first_reach.is_none() && in_reach {
            first_reach = Some(advance);
        }
        if first_flight.is_none() && host.sim().hit_in_flight(hero) {
            first_flight = Some(advance);
        }
    }
    let reached = first_reach.expect("the hero comes into reach of the target");
    let launched = first_flight.expect("the hero in reach starts a swing");
    assert!(
        launched <= reached + MAX_LATENCY_ADVANCES,
        "the hero came into reach at advance {reached} and launched its first swing at advance {launched}, more than {MAX_LATENCY_ADVANCES} advance later"
    );
}

#[test]
fn a_strike_stamp_records_the_reach_and_launch_ticks_and_the_attack_point() {
    let mut host = pack_host();
    let hero = spawn_hero(&mut host, fixed::from_int(HERO_X) + HALF, fixed::from_int(HERO_Y) + HALF);
    let target = place_target(&mut host, fixed::from_int(HERO_X) + HALF + TARGET_OFFSET, fixed::from_int(HERO_Y) + HALF);
    let mut stamp = None;
    for _ in 0..TICKS {
        let (x, y) = host.sim().position_fixed(hero);
        host.sim_mut().order(hero, x, y);
        host.advance().expect("advance the match");
        let seen = host.sim().strike_stamp_of(hero);
        if seen[2] >= 0 {
            stamp = Some(seen);
            break;
        }
    }
    let [struck, reach, launch, point, _land] = stamp.expect("the hero's first swing launches within the window");
    assert_eq!(struck, target as i32, "the stamped strike is on the target");
    assert!(reach >= 0 && launch >= reach, "the swing launches on or after the reach tick: reach {reach}, launch {launch}");
    assert!(launch - reach <= MAX_LATENCY_ADVANCES as i32, "the swing launches within {MAX_LATENCY_ADVANCES} tick of reach: reach {reach}, launch {launch}");
    assert!(point > 0, "the attack point is stamped at launch, got {point}");
}

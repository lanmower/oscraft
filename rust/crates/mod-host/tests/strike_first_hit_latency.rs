use std::path::{Path, PathBuf};

use mod_host::{build_image, ModHost};
use sim::fixed::{self, HALF, ONE};

const OWNER: u8 = 0;
const NEUTRAL: u8 = 255;
const HERO_X: i32 = 92;
const HERO_Y: i32 = 240;
const CREEP_OFFSET_CELLS: i32 = 2;
const TICKS: usize = 20;
const MAX_LAUNCH_ADVANCES: usize = 2;

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

#[test]
fn a_stopped_hero_in_reach_starts_its_first_swing_within_two_advances() {
    let mut host = pack_host();
    let hero = spawn(&mut host, "pudge", OWNER, fixed::from_int(HERO_X) + HALF, fixed::from_int(HERO_Y) + HALF);
    spawn(
        &mut host,
        "creep",
        NEUTRAL,
        fixed::from_int(HERO_X) + HALF + CREEP_OFFSET_CELLS * ONE,
        fixed::from_int(HERO_Y) + HALF,
    );
    let mut first_flight = None;
    for advance in 0..TICKS {
        host.advance().expect("advance the match");
        if first_flight.is_none() && host.sim().hit_in_flight(hero) {
            first_flight = Some(advance);
        }
    }
    let launched = first_flight.expect("the stopped hero in reach starts a swing");
    assert!(
        launched < MAX_LAUNCH_ADVANCES,
        "the stopped hero in reach launched its first swing at advance {launched}, after {MAX_LAUNCH_ADVANCES} allowed"
    );
}

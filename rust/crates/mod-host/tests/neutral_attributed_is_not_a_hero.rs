use std::path::{Path, PathBuf};

use mod_host::{build_image, ModHost};
use sim::fixed::{self, HALF};

const OWNER: u8 = 0;
const NEUTRAL: u8 = 255;
const TICKS: usize = 12;

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

fn spawn(host: &mut ModHost, kind_name: &str, player: u8, attributed: bool, x: i32, y: i32) -> usize {
    let mut sim = host.sim_mut();
    let kind = sim.catalog().unit_index(kind_name).expect("the kind is in the moba catalog");
    if attributed {
        sim.spawn_attributed(player, kind as u32, x, y).expect("the unit fits on the map")
    } else {
        sim.spawn_for(player, kind as u32, x, y).expect("the unit fits on the map")
    }
}

#[test]
fn a_neutral_attributed_unit_is_not_a_hero_and_raises_no_player_error() {
    let mut host = pack_host();
    let x = fixed::from_int(92) + HALF;
    let y = fixed::from_int(240) + HALF;
    spawn(&mut host, "antimage", OWNER, true, x, y);
    spawn(&mut host, "creep", NEUTRAL, true, x + fixed::from_int(2), y);
    for _ in 0..TICKS {
        host.advance().expect("advance the match");
    }
    let player_errors: Vec<&String> = host.hook_errors().iter().filter(|e| e.contains("invalid_player")).collect();
    assert!(player_errors.is_empty(), "a neutral attributed unit reached a player call: {player_errors:?}");
}

#[test]
fn an_owned_attributed_hero_raises_no_player_error_either() {
    let mut host = pack_host();
    let x = fixed::from_int(92) + HALF;
    let y = fixed::from_int(240) + HALF;
    spawn(&mut host, "antimage", OWNER, true, x, y);
    for _ in 0..TICKS {
        host.advance().expect("advance the match");
    }
    assert!(host.hook_errors().is_empty(), "the owned hero's ticks raise no hook error: {:?}", host.hook_errors());
}

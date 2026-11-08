use std::path::{Path, PathBuf};

use mod_host::{build_image, ModHost};
use serde_json::json;
use sim::fixed::{self, HALF};

const OWNER: u8 = 0;
const OTHER: u8 = 1;
const HERO_KIND: &str = "antimage";
const ORIGIN_X: i32 = 92;
const ORIGIN_Y: i32 = 240;

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

fn spawn_hero(host: &mut ModHost, player: u8) -> usize {
    let mut sim = host.sim_mut();
    let kind = sim.catalog().unit_index(HERO_KIND).expect("the hero kind is in the moba catalog");
    let x = fixed::from_int(ORIGIN_X) + HALF;
    let y = fixed::from_int(ORIGIN_Y) + HALF;
    sim.spawn_attributed(player, kind as u32, x, y).expect("the hero fits on the map")
}

fn one_hand(host: &mut ModHost, player: u8, on: bool) {
    host.dispatch_action("ui", &json!({ "call": "one_hand", "args": { "on": on }, "player": player }))
        .expect("the panels accept the one-hand toggle");
}

#[test]
fn a_hero_does_not_strike_on_move_until_its_owner_turns_one_hand_on() {
    let mut host = pack_host();
    let hero = spawn_hero(&mut host, OWNER);
    host.advance().expect("advance the match");
    assert!(!host.sim().strikes_on_move(hero), "one-hand is off by default, so a walking hero does not strike");
}

#[test]
fn one_hand_on_lets_the_owners_hero_strike_while_it_walks_and_off_takes_it_back() {
    let mut host = pack_host();
    let hero = spawn_hero(&mut host, OWNER);
    one_hand(&mut host, OWNER, true);
    host.advance().expect("advance the match");
    assert!(host.sim().strikes_on_move(hero), "the lock's one-hand setting lets the hero strike while it walks");
    one_hand(&mut host, OWNER, false);
    host.advance().expect("advance the match");
    assert!(!host.sim().strikes_on_move(hero), "turning one-hand off takes the strike-on-move flag back");
}

#[test]
fn one_hand_is_per_owner_and_leaves_another_players_hero_alone() {
    let mut host = pack_host();
    let owner_hero = spawn_hero(&mut host, OWNER);
    let other_hero = spawn_hero(&mut host, OTHER);
    one_hand(&mut host, OWNER, true);
    host.advance().expect("advance the match");
    assert!(host.sim().strikes_on_move(owner_hero), "the owner's hero takes the setting");
    assert!(!host.sim().strikes_on_move(other_hero), "another player's hero is not changed by the owner's setting");
}

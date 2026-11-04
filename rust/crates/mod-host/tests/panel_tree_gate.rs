use std::path::{Path, PathBuf};

use mod_host::{build_image, ModHost};
use serde_json::json;
use sim::fixed::{self, HALF};

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

fn ui_file() -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(pack_dir().join("moba_ui.json")).expect("the moba ui file"))
        .expect("a valid moba ui file")
}

fn setup_panels(host: &mut ModHost) {
    host.dispatch_action("setup", &json!({ "ui": ui_file(), "player": 0, "pubkey": "owner" }))
        .expect("the panels accept the setup");
}

fn spawn_hero(host: &mut ModHost, kind_name: &str, player: u8, cell_x: i32, cell_y: i32) -> usize {
    let mut sim = host.sim_mut();
    let kind = sim.catalog().unit_index(kind_name).expect("the hero kind is in the moba catalog");
    let x = fixed::from_int(cell_x) + HALF;
    let y = fixed::from_int(cell_y) + HALF;
    sim.spawn_attributed(player, kind as u32, x, y).expect("the hero fits on the map")
}

fn advance(host: &mut ModHost, ticks: u32) {
    for _ in 0..ticks {
        host.advance().expect("advance the match");
    }
}

fn start_quiet_window(host: &mut ModHost) {
    let mut last = host.sim().resource_a(0);
    for _ in 0..256 {
        advance(host, 1);
        let now = host.sim().resource_a(0);
        if now != last {
            advance(host, 2);
            return;
        }
        last = now;
    }
    panic!("the player's income never ticked, so no quiet window can be found");
}

fn key_name(ui: &serde_json::Value, group: &str) -> String {
    ui["keys"][group][0].as_str().expect("a key binding for the panel").to_string()
}

#[test]
fn idle_panel_trees_are_not_rebuilt_while_the_state_counter_holds() {
    let mut host = pack_host();
    setup_panels(&mut host);
    advance(&mut host, 8);
    let settled = host.sim().panel_rebuilds();
    advance(&mut host, 40);
    let rebuilt = host.sim().panel_rebuilds() - settled;
    eprintln!("panel-probe counter=unchanged rebuilds={rebuilt} ticks=40");
    assert_eq!(rebuilt, 0, "the panel trees were rebuilt {rebuilt} times with the counter unchanged");
}

#[test]
fn a_changed_state_counter_rebuilds_only_the_trees_that_read_it() {
    let mut host = pack_host();
    setup_panels(&mut host);
    spawn_hero(&mut host, "antimage", 0, 92, 240);
    advance(&mut host, 240);
    start_quiet_window(&mut host);
    let settled = host.sim().panel_rebuilds();
    advance(&mut host, 8);
    let idle = host.sim().panel_rebuilds() - settled;
    assert_eq!(idle, 0, "the panel trees were rebuilt {idle} times with the counter unchanged");
    let before_change = host.sim().panel_rebuilds();
    host.sim_mut().add_resources(0, 25, 0);
    advance(&mut host, 4);
    let rebuilt = host.sim().panel_rebuilds() - before_change;
    eprintln!("panel-probe counter=unchanged rebuilds={idle} ticks=8; counter=changed rebuilds={rebuilt} ticks=4");
    assert_eq!(rebuilt, 2, "the open sim-reading trees (team, select) rebuild once each, got {rebuilt}");
}

#[test]
fn a_scoreboard_toggle_rebuilds_only_the_scoreboard_tree() {
    let mut host = pack_host();
    setup_panels(&mut host);
    spawn_hero(&mut host, "antimage", 0, 92, 240);
    advance(&mut host, 240);
    start_quiet_window(&mut host);
    let settled = host.sim().panel_rebuilds();
    host.dispatch_action("key", &json!({ "phase": "down", "key": key_name(&ui_file(), "scoreboard"), "player": 0 }))
        .expect("the panels accept the scoreboard key");
    advance(&mut host, 4);
    let rebuilt = host.sim().panel_rebuilds() - settled;
    eprintln!("panel-probe scoreboard-open rebuilds={rebuilt} ticks=4");
    assert_eq!(rebuilt, 1, "the scoreboard toggle rebuilt {rebuilt} trees, expected only the scoreboard");
}

#[test]
fn a_selection_change_rebuilds_only_the_select_tree() {
    let mut host = pack_host();
    setup_panels(&mut host);
    let hero = spawn_hero(&mut host, "antimage", 0, 92, 240);
    advance(&mut host, 240);
    start_quiet_window(&mut host);
    let settled = host.sim().panel_rebuilds();
    host.dispatch_action("key", &json!({ "phase": "down", "key": key_name(&ui_file(), "scoreboard"), "player": 0 }))
        .expect("the panels accept the scoreboard key");
    advance(&mut host, 4);
    let after_scoreboard = host.sim().panel_rebuilds();
    host.dispatch_action("selection", &json!({ "player": 0, "ids": [hero] }))
        .expect("the panels accept the selection");
    advance(&mut host, 4);
    let rebuilt = host.sim().panel_rebuilds() - after_scoreboard;
    eprintln!("panel-probe scoreboard-open rebuilds={} ticks=4; selection rebuilds={rebuilt} ticks=4", after_scoreboard - settled);
    assert_eq!(rebuilt, 2, "the selection rebuilt {rebuilt} trees, expected the select tree and the skills tree it now shows");
}

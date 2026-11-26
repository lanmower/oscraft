use std::path::{Path, PathBuf};

use mod_host::{build_image, ModHost, ModRuntime};
use serde_json::{json, Value};

fn pack_dir(pack: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios").join(pack)
}

fn visit(modules: &serde_json::Map<String, Value>, dir: &Path, module: &str, out: &mut Vec<(String, String)>) {
    let Some(file) = modules.get(module).and_then(Value::as_str) else { return };
    if out.iter().any(|(name, _)| name == module) {
        return;
    }
    let src = std::fs::read_to_string(dir.join(file)).expect("read a lua module");
    out.push((module.to_owned(), src.clone()));
    for required in src.split("require(\"").skip(1).filter_map(|rest| rest.split('"').next()) {
        visit(modules, dir, required, out);
    }
}

fn pack_host(pack: &str) -> impl ModRuntime {
    let dir = pack_dir(pack);
    let manifest: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("manifest.json")).expect("the pack manifest"))
        .expect("a valid pack manifest");
    let rules = std::fs::read_to_string(dir.join(manifest["rules"].as_str().expect("the manifest rules file")))
        .expect("the pack rules file");
    let modules = manifest["modules"].as_object().expect("the manifest modules map");
    let mut mods = Vec::new();
    for root in manifest["mods"].as_array().expect("the manifest mods list") {
        let name = root.as_str().expect("a mod file name").trim_end_matches(".lua");
        visit(modules, &dir, name, &mut mods);
    }
    let image = build_image(&dir).expect("build the pack image");
    let mut host = ModHost::new(&image).expect("load the pack image");
    assert!(host.sim_mut().set_attr_rules_text(&rules), "the pack rules are accepted");
    host.load_mods(&mods).expect("load the pack mods");
    host.sim_mut().set_fog(false);
    host
}

fn ui_file(pack: &str) -> Value {
    let dir = pack_dir(pack);
    let manifest: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("manifest.json")).expect("the pack manifest"))
        .expect("a valid pack manifest");
    let ui = manifest["ui"].as_str().expect("the manifest ui file");
    serde_json::from_str(&std::fs::read_to_string(dir.join(ui)).expect("the pack ui file")).expect("a valid pack ui file")
}

fn lobby_heroes(lines: &[String]) -> Vec<Value> {
    let line = lines
        .iter()
        .find(|l| l.starts_with("\"kind\":\"lobby\""))
        .expect("the lobby panel is sent at setup");
    let value: Value = serde_json::from_str(&format!("{{{line}}}")).expect("the lobby line is JSON");
    value["data"]["heroes"].as_array().expect("the lobby lists heroes").clone()
}

fn check_pick_menu(pack: &str, roster: usize, want: &[(&str, &str)]) {
    let mut host = pack_host(pack);
    host.dispatch_action("setup", &json!({ "ui": ui_file(pack), "player": 0, "pubkey": "owner" }))
        .expect("the panels accept the setup");
    host.advance().expect("advance one tick");
    let heroes = lobby_heroes(&host.take_ui());
    let acronyms: Vec<&str> = heroes.iter().filter_map(|h| h["acronym"].as_str()).collect();
    let question_marks = acronyms.iter().filter(|a| **a == "?").count();
    eprintln!("menu-probe pack={pack} pick-menu heroes={} question_marks={question_marks} sample={:?}", heroes.len(), &acronyms[..acronyms.len().min(8)]);
    assert_eq!(heroes.len(), roster, "{pack}: the pick menu lists {} heroes, expected {roster}", heroes.len());
    assert_eq!(question_marks, 0, "{pack}: the pick menu shows \"?\" for {question_marks} heroes that have an acronym");
    assert!(heroes.iter().all(|h| h["unit"].as_str() != Some("lycan_wolf_form")), "{pack}: a shapeshift form is in the pick menu");
    for (unit, acronym) in want {
        let shown = heroes.iter().find(|h| h["unit"].as_str() == Some(unit)).and_then(|h| h["acronym"].as_str());
        assert_eq!(shown, Some(*acronym), "{pack}: the pick menu shows {shown:?} for {unit}");
    }
}

#[test]
fn default_acronym_pack_pick_menu_shows_every_hero_acronym() {
    check_pick_menu("moba-acronyms", 127, &[("am", "AM"), ("axe", "AXE"), ("be", "BE")]);
}

#[test]
fn full_name_pack_pick_menu_shows_every_hero_acronym() {
    check_pick_menu("moba", 127, &[("antimage", "AM"), ("axe", "AXE"), ("crystal_maiden", "CM")]);
}

#[test]
fn bota_pack_pick_menu_lists_every_active_hero() {
    check_pick_menu("bota", 24, &[("neckbeard", "neckbeard"), ("wojak", "wojak")]);
}

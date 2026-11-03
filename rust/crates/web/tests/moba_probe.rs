use std::path::{Path, PathBuf};

use mod_host::{build_image, ModHost};
use serde_json::json;

fn scenario_dir() -> PathBuf {
    match std::env::var_os("PROBE_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba"),
    }
}

fn image_for(dir: &Path) -> Vec<u8> {
    match std::env::var_os("PROBE_IMAGE") {
        Some(path) => std::fs::read(path).expect("read PROBE_IMAGE"),
        None => build_image(dir).expect("build image"),
    }
}

fn load_pack(host: &mut ModHost, dir: &Path) {
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join("manifest.json")).expect("read manifest")).expect("manifest json");
    let mut files: Vec<String> = manifest["modules"]
        .as_object()
        .expect("modules")
        .values()
        .map(|file| file.as_str().expect("module file").to_owned())
        .collect();
    files.extend(manifest["mods"].as_array().expect("mods").iter().map(|file| file.as_str().expect("mod file").to_owned()));
    let rules = std::fs::read_to_string(dir.join(manifest["rules"].as_str().expect("rules file"))).expect("read rules");
    assert!(host.sim_mut().set_attr_rules_text(&rules), "pack rules text refused");
    files.sort();
    files.dedup();
    let mods: Vec<(String, String)> = files
        .into_iter()
        .map(|file| {
            let src = std::fs::read_to_string(dir.join(&file)).expect("read module");
            (file, src)
        })
        .collect();
    host.load_mods(&mods).expect("load mods");
}

fn ui_call(host: &mut ModHost, call: &str, args: serde_json::Value) {
    host.dispatch_action("ui", &json!({ "call": call, "args": args, "player": 1 })).expect("ui call");
    let lines = drain_ui(host);
    println!("probe call {call}: ui_lines={}", lines.len());
}

fn drain_ui(host: &mut ModHost) -> Vec<String> {
    host.flush_ui();
    host.take_ui()
}

fn print_win(label: &str, lines: &[String]) {
    println!("probe {label}: ui_lines={}", lines.len());
    for line in lines.iter() {
        println!("probe {label}: {}", line.chars().take(900).collect::<String>());
    }
}

#[test]
fn probe_win_screen() {
    let dir = scenario_dir();
    let mut host = ModHost::new(&image_for(&dir)).expect("host");
    host.sim_mut().set_fog(false);
    load_pack(&mut host, &dir);
    let ui: serde_json::Value = serde_json::from_slice(&std::fs::read(dir.join("moba_ui.json")).expect("read moba_ui.json")).expect("moba_ui.json");
    host.dispatch_action("setup", &json!({ "ui": ui, "player": 1 })).expect("setup");
    for _ in 0..60 {
        host.advance().expect("tick");
    }
    ui_call(&mut host, "fill", json!({}));
    ui_call(&mut host, "start", json!({}));
    for _ in 0..8 {
        ui_call(&mut host, "draft_clock", json!({ "ticks": 64 }));
        host.advance().expect("draft tick");
    }
    ui_call(&mut host, "start", json!({}));
    for _ in 0..120 {
        host.advance().expect("tick");
    }
    println!("probe heroes p0={} p1={} units={}", host.units(Some(0)).len(), host.units(Some(1)).len(), host.units(None).len());
    let player0 = host.units(Some(0));
    for unit in &player0 {
        host.sim_mut().remove_unit(*unit);
    }
    for _ in 0..30 {
        host.advance().expect("tick after match end");
    }
    print_win("match_end", &drain_ui(&mut host));
    println!("probe winner {:?}", host.sim().winner());
    host.dispatch_action("ui", &json!({ "call": "win_refresh", "player": 1 })).expect("refresh");
    print_win("refresh", &drain_ui(&mut host));
    host.dispatch_action("ui", &json!({ "call": "win_save_replay", "player": 1 })).expect("save");
    print_win("save_replay", &drain_ui(&mut host));
    host.dispatch_action("ui", &json!({ "call": "replay_saved", "player": 1, "args": { "name": "oscraft-replay-probe.replay" } })).expect("saved");
    print_win("replay_saved", &drain_ui(&mut host));
    host.dispatch_action("ui", &json!({ "call": "win_new_game", "player": 1 })).expect("new game");
    print_win("new_game", &drain_ui(&mut host));
    println!("probe removed_units={} ticks_after_end=5", player0.len());
}

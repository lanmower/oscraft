#![cfg(feature = "build")]
mod common;
use mod_host::{build_image, Death, ModHost};
use serde_json::json;

#[test]
fn smoke() {
    let scenario = common::scenario();
    let mut mods = common::pack_mods(&scenario, &common::pack_roots(&scenario));
    mods.push(("fixtures/moba_siege.lua".to_owned(), std::fs::read_to_string(scenario.join("fixtures/moba_siege.lua")).unwrap()));
    let image = build_image(&scenario).unwrap();
    let mut host = ModHost::new(&image).unwrap();
    assert!(host.sim().set_attr_rules_text(&common::pack_rules(&scenario)));
    host.load_mods(&mods).unwrap();
    let ui: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(scenario.join("moba_ui.json")).unwrap()).unwrap();
    host.dispatch_action("setup", &json!({ "ui": ui, "player": 0 })).unwrap();
    host.sim().enroll_attributed(20, 0, 0);
    for t in 0..6000 {
        host.advance().unwrap_or_else(|e| panic!("tick {t}: {e}"));
        if t % 1500 == 0 {
            host.dispatch_action("key", &json!({ "key": "tab", "phase": "down", "shift": false, "ctrl": false, "alt": false, "meta": false, "player": 0, "world": { "x": 0.0, "z": 0.0 } })).unwrap();
        }
    }
    println!("SMOKE waves_total={:?} waves_siege={:?}", host.state_int("waves_total"), host.state_int("waves_siege"));
    let fed = host.feed_death(&Death { victim: 10, victim_player: 2, killer: Some(20), killer_player: Some(0), bounty: 40 }).unwrap();
    println!("SMOKE death lines {:?}", fed.len());
    let ui = host.take_ui();
    println!("SMOKE ui events {}", ui.len());
    host.dispatch_action("ui", &json!({ "player": 0, "call": "talent", "args": { "unit": 20, "tier": 0, "side": 0 } })).unwrap();
    host.advance().unwrap();
}

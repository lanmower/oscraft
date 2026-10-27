
#![cfg(feature = "build")]

use mod_host::{build_image, ModHost};
use serde_json::json;

mod common;
use common::scenario;

fn unit_of(host: &mut ModHost, player: u8) -> (usize, f64, f64) {
    let sim = host.sim();
    for i in 0..sim.count() {
        if sim.alive(i) && sim.player(i) == player {
            let (x, z) = sim.position_fixed(i);
            return (i, x as f64 / 65536.0, z as f64 / 65536.0);
        }
    }
    panic!("seat {player} has no living unit");
}

#[test]
fn click_selects_and_right_click_orders_the_selection() {
    let image = build_image(&scenario()).expect("moba scenario builds an image");
    let mut host = ModHost::new(&image).expect("host");
    let ui: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(scenario().join("moba_ui.json")).expect("ui")).expect("ui json");
    let rules = std::fs::read_to_string(scenario().join("attr_rules.txt")).expect("rules");
    assert!(host.sim_mut().set_attr_rules_text(&rules), "the pack's rules file loads");
    host.load_mods(&common::pack_mods(&scenario(), &common::pack_roots(&scenario()))).expect("moba mods load");
    host.dispatch_action("setup", &json!({ "ui": ui, "player": 0 })).expect("setup");

    let (id, x, z) = unit_of(&mut host, 0);
    let press = |host: &mut ModHost, kind: &str, x: f64, z: f64, button: i64| {
        host.dispatch_action("pointer", &json!({ "player": 0, "kind": kind, "x": x, "y": z, "button": button })).expect("pointer");
    };

    press(&mut host, "down", x, z, 0);
    press(&mut host, "up", x, z, 0);
    host.advance().expect("tick");
    let ui = host.take_ui();
    assert!(ui.iter().any(|b| b.contains("\"kind\":\"selection\"") && b.contains(&format!("\"ids\":[{id}]"))), "left click selects unit {id}: {ui:?}");

    press(&mut host, "down", x + 2.0, z, 2);
    press(&mut host, "up", x + 2.0, z, 2);
    host.advance().expect("tick");
    let ui = host.take_ui();
    assert!(ui.iter().any(|b| b.contains("\"kind\":\"move\"") && b.contains(&format!("\"ids\":[{id}]"))), "right click orders unit {id}: {ui:?}");

    host.dispatch_action("key", &json!({ "key": "s", "phase": "down", "shift": false, "ctrl": false, "alt": false, "meta": false, "player": 0, "world": { "x": x, "z": z } })).expect("key");
    host.advance().expect("tick");
    let ui = host.take_ui();
    assert!(ui.iter().all(|b| !b.contains("\"refusal\"")), "the stop key is not refused: {ui:?}");
    host.dispatch_action("key", &json!({ "key": "h", "phase": "down", "shift": false, "ctrl": false, "alt": false, "meta": false, "player": 0, "world": { "x": x, "z": z } })).expect("key");
    host.dispatch_action("key", &json!({ "key": "tab", "phase": "down", "shift": false, "ctrl": false, "alt": false, "meta": false, "player": 0, "world": { "x": x, "z": z } })).expect("key");
    host.advance().expect("tick");
    let ui = host.take_ui();
    assert!(ui.iter().any(|b| b.contains("\"action\":\"scoreboard\"")), "tab opens the scoreboard panel action: {ui:?}");
}

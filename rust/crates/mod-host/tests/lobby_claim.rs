
#![cfg(feature = "build")]

use mod_host::{build_image, ModHost};
use serde_json::json;

mod common;
use common::scenario;

fn lobby_after(host: &mut ModHost) -> String {
    let mut last = String::new();
    for _ in 0..8 {
        host.advance().expect("tick");
        for e in host.take_ui() {
            if e.contains("\"lobby\"") {
                last = e;
            }
        }
    }
    last
}

#[test]
fn a_claim_from_the_host_page_takes_the_first_open_seat() {
    let image = build_image(&scenario()).expect("moba scenario builds an image");
    let mut host = ModHost::new(&image).expect("host");
    let rules = std::fs::read_to_string(scenario().join("attr_rules.txt")).expect("rules");
    assert!(host.sim_mut().set_attr_rules_text(&rules), "the rules text loads");
    host.load_mods(&common::pack_mods(&scenario(), &common::pack_roots(&scenario()))).expect("moba mods load");
    let ui: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(scenario().join("moba_ui.json")).expect("ui")).expect("ui json");
    host.dispatch_action("setup", &json!({ "ui": ui, "player": 0, "pubkey": "host-key" })).expect("setup");
    host.dispatch_action("ui", &json!({ "call": "claim", "args": { "slot": 0 } })).expect("claim");
    let lobby = lobby_after(&mut host);
    assert!(lobby.contains("Slot 1: you"), "the host's claim takes slot 1 for the host: {lobby}");
}

#[test]
fn a_claim_from_a_joined_page_takes_the_seat_under_the_senders_key() {
    let image = build_image(&scenario()).expect("moba scenario builds an image");
    let mut host = ModHost::new(&image).expect("host");
    let rules = std::fs::read_to_string(scenario().join("attr_rules.txt")).expect("rules");
    assert!(host.sim_mut().set_attr_rules_text(&rules), "the rules text loads");
    host.load_mods(&common::pack_mods(&scenario(), &common::pack_roots(&scenario()))).expect("moba mods load");
    let ui: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(scenario().join("moba_ui.json")).expect("ui")).expect("ui json");
    host.dispatch_action("setup", &json!({ "ui": ui, "player": 0, "pubkey": "host-key" })).expect("setup");
    host.dispatch_action("ui", &json!({ "call": "claim", "args": { "slot": 1 }, "pubkey": "guest-key", "player": 255 })).expect("guest claim");
    let lobby = lobby_after(&mut host);
    assert!(lobby.contains("player"), "the guest's claim is shown as another player's seat: {lobby}");
}

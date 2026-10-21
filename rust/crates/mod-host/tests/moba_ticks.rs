//! The moba pack runs its per-tick and per-event hooks through a real match without any of them failing. A hook
//! whose Lua errors is switched off by the host, so a switched-off hook after play is the failure this catches.

#![cfg(feature = "build")]

use mod_host::{build_image, ModHost};
use serde_json::json;

mod common;
use common::scenario;

#[test]
fn the_moba_pack_plays_without_a_hook_switched_off() {
    let image = build_image(&scenario()).expect("moba scenario builds an image");
    let mut host = ModHost::new(&image).expect("host");
    let rules = std::fs::read_to_string(scenario().join("attr_rules.txt")).expect("rules");
    assert!(host.sim_mut().set_attr_rules_text(&rules), "the pack's rules file loads");
    let mods = common::pack_mods(&scenario(), &common::pack_roots(&scenario()));
    host.load_mods(&mods).expect("moba mods load");
    let ui: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(scenario().join("moba_ui.json")).expect("ui")).expect("ui json");
    host.dispatch_action("setup", &json!({ "ui": ui, "player": 0 })).expect("setup");
    host.dispatch_action("seat_pick", &json!({ "player": 0, "hero": "antimage" })).expect("seat_pick");
    for _ in 0..1200 {
        host.advance().expect("tick");
    }
    assert_eq!(host.disabled_hooks(), Vec::<String>::new(), "no hook of the pack may fail while it plays");
}

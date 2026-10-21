//! A seat's pick from the lobby's start spawns the picked hero at the seat's start point: the page dispatches the
//! "seat_pick" action (controls.lua routes it to heroes.seat_pick), which reads the start from the map and spawns the unit.

#![cfg(feature = "build")]

use mod_host::{build_image, ModHost};
use serde_json::json;

mod common;
use common::scenario;

#[test]
fn a_seat_pick_spawns_the_picked_hero_as_its_own_unit() {
    let image = build_image(&scenario()).expect("moba scenario builds an image");
    let mut host = ModHost::new(&image).expect("host");
    let rules = std::fs::read_to_string(scenario().join("attr_rules.txt")).expect("rules");
    assert!(host.sim_mut().set_attr_rules_text(&rules), "the pack's rules file loads");
    let mods = common::pack_mods(&scenario(), &common::pack_roots(&scenario()));
    host.load_mods(&mods).expect("moba mods load");
    let ui: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(scenario().join("moba_ui.json")).expect("ui")).expect("ui json");
    host.dispatch_action("setup", &json!({ "ui": ui, "player": 0 })).expect("setup");
    host.dispatch_action("seat_pick", &json!({ "player": 0, "hero": "antimage" })).expect("seat_pick");
    host.advance().expect("tick");
    let kind = host.sim().catalog().unit_index("antimage").expect("antimage is a unit kind") as i32;
    let picked = (0..host.sim().count()).filter(|&i| host.sim().alive(i) && host.sim().unit_type(i) == kind).count();
    assert_eq!(picked, 1, "the picked hero spawns once, as its own unit type");
}

#![cfg(feature = "build")]

use mod_host::{build_image, ModHost};
use serde_json::json;
use sim::fixed;

mod common;
use common::scenario;

fn host_with_hero(player: u64) -> (ModHost, usize) {
    let image = build_image(&scenario()).expect("moba scenario builds an image");
    let mut host = ModHost::new(&image).expect("host");
    let rules = std::fs::read_to_string(scenario().join("attr_rules.txt")).expect("rules");
    assert!(host.sim_mut().set_attr_rules_text(&rules), "the pack's rules file loads");
    let mods = common::pack_mods(&scenario(), &common::pack_roots(&scenario()));
    host.load_mods(&mods).expect("moba mods load");
    let ui: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(scenario().join("moba_ui.json")).expect("ui")).expect("ui json");
    host.dispatch_action("setup", &json!({ "ui": ui, "player": player })).expect("setup");
    host.dispatch_action("seat_pick", &json!({ "player": player, "hero": "antimage" })).expect("seat_pick");
    host.advance().expect("tick");
    let kind = host.sim().catalog().unit_index("antimage").expect("antimage is a unit kind") as i32;
    let hero = (0..host.sim().count()).find(|&i| host.sim().alive(i) && host.sim().unit_type(i) == kind).expect("the picked hero");
    (host, hero)
}

fn milli(v: i64) -> i32 {
    (v * i64::from(fixed::ONE) / 1000) as i32
}

fn buy_back(player: u64, shop: (i64, i64)) {
    let (mut host, h) = host_with_hero(player);
    host.sim_mut().hurt(h, i32::MAX / 2);
    host.advance().expect("the death tick");
    assert!(!host.sim().alive(h), "the hero is dead before the buyback");
    host.dispatch_action("buyback", &json!({ "player": player })).expect("buyback");
    host.advance().expect("tick");
    assert!(host.sim().alive(h), "a buyback returns the hero at once");
    let (x, y) = host.sim().position_fixed(h);
    let want = (milli(shop.0), milli(shop.1));
    assert!((x - want.0).abs() <= fixed::ONE && (y - want.1).abs() <= fixed::ONE, "revived at the Circle of Power {want:?}, not {:?}", (x, y));
}

#[test]
fn a_sentinel_buyback_returns_the_hero_at_its_circle_of_power() {
    buy_back(0, (46000, 275125));
}

#[test]
fn a_scourge_buyback_returns_the_hero_at_its_circle_of_power() {
    buy_back(1, (273000, 63000));
}

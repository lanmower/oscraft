//! The economy's opening gold reaches the seat. economy(...) declares opening_resource_a, and a match
//! has to start with it: without it a hero stands at its fountain with nothing to spend, the shop's
//! opening choices never happen, and buying is impossible until the first creep dies.
//!
//! The value is read the way a player reads it: the gold line of the hud panel panels.lua builds from
//! game.gold(LOCAL), which is the seat's resource A.

#![cfg(feature = "build")]

use mod_host::{build_image, ModHost};
use serde_json::json;

mod common;
use common::scenario;

// economy() in scenarios/moba/economy.osd, marked UNVERIFIED there: the reference starting gold.
const OPENING_GOLD: i64 = 625;

fn host_with_hero() -> ModHost {
    let image = build_image(&scenario()).expect("moba scenario builds an image");
    let mut host = ModHost::new(&image).expect("host");
    let read = |name: &str| std::fs::read_to_string(scenario().join(name)).expect(name);
    assert!(host.sim_mut().set_attr_rules_text(&read("attr_rules.txt")), "the pack's rules file loads");
    let mods = common::pack_mods(&scenario(), &common::pack_roots(&scenario()));
    host.load_mods(&mods).expect("moba mods load");
    let ui: serde_json::Value = serde_str(&read("moba_ui.json"));
    host.dispatch_action("setup", &json!({ "ui": ui, "player": 0 })).expect("setup");
    host.dispatch_action("seat_pick", &json!({ "player": 0, "hero": "antimage" })).expect("seat_pick");
    host
}

fn serde_str(text: &str) -> serde_json::Value {
    serde_json::from_str(text).expect("ui json")
}

// The panels refresh every few ticks, so any window of five holds a refresh.
fn ticks(host: &mut ModHost, n: usize) {
    for _ in 0..n {
        host.advance().expect("tick");
    }
}

// The gold the hud panel reports. panels.lua builds it as "<label> <n>" from game.gold(LOCAL).
fn hud_gold(ui: &[String]) -> i64 {
    let hud = ui
        .iter()
        .find(|b| b.contains("\"id\":\"hud\""))
        .unwrap_or_else(|| panic!("the hud panel is sent: {ui:?}"));
    let at = hud.find("gold ").expect("the hud reports gold") + "gold ".len();
    let digits: String = hud[at..].chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().unwrap_or_else(|_| panic!("gold is a number: {}", &hud[at..]))
}

#[test]
fn a_seat_opens_on_the_economy_s_opening_gold() {
    let mut host = host_with_hero();
    ticks(&mut host, 5);
    assert_eq!(hud_gold(&host.take_ui()), OPENING_GOLD, "the seat holds the opening gold before it has earned any");
}

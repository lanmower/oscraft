
#![cfg(feature = "build")]

use mod_host::{build_image, ModHost};
use serde_json::json;

mod common;
use common::scenario;

const OPENING_GOLD: i64 = 875;

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

fn ticks(host: &mut ModHost, n: usize) {
    for _ in 0..n {
        host.advance().expect("tick");
    }
}

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

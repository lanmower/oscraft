//! The bot seats of the moba pack: a seat whose pick says it is a bot has its hero driven by the bot library
//! (bots.lua), every decision, with no hook disabled.

#![cfg(feature = "build")]

use mod_host::{build_image, ModHost};
use serde_json::json;

mod common;
use common::scenario;

#[test]
fn bot_seats_drive_their_heroes_and_no_hook_is_disabled() {
    let image = build_image(&scenario()).expect("moba scenario builds an image");
    let mut host = ModHost::new(&image).expect("host");
    let rules = std::fs::read_to_string(scenario().join("attr_rules.txt")).expect("rules");
    assert!(host.sim_mut().set_attr_rules_text(&rules), "the pack's rules file loads");
    let mods = common::pack_mods(&scenario(), &common::pack_roots(&scenario()));
    host.load_mods(&mods).expect("moba mods load");
    let ui: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(scenario().join("moba_ui.json")).expect("ui")).expect("ui json");
    host.dispatch_action("setup", &json!({ "ui": ui, "player": 0 })).expect("setup");
    for player in 1..=3u8 {
        host.dispatch_action("seat_pick", &json!({ "player": player, "hero": "antimage", "bot": true })).expect("bot seat_pick");
    }
    for _ in 0..200 {
        host.advance().expect("tick");
    }
    assert!(host.disabled_hooks().is_empty(), "no hook is disabled: {:?}", host.disabled_hooks());
    assert_eq!(host.state_int("bot_seats"), Some(3), "three seats are bots");
    let acted = host.state_int("bot_walks").unwrap_or(0) + host.state_int("bot_attacks").unwrap_or(0);
    assert!(acted > 0, "the bots acted: walks {:?} attacks {:?}", host.state_int("bot_walks"), host.state_int("bot_attacks"));
}

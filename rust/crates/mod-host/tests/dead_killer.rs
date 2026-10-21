//! A death whose killer died in the same step: the moba hooks must not fail on the dead unit (the host refuses gold and
//! xp for a dead unit), so no hook is switched off.

#![cfg(feature = "build")]

use mod_host::{build_image, Death, ModHost};
use serde_json::json;

mod common;
use common::scenario;

#[test]
fn a_death_whose_killer_is_already_dead_switches_no_hook_off() {
    let image = build_image(&scenario()).expect("moba scenario builds an image");
    let mut host = ModHost::new(&image).expect("host");
    let rules = std::fs::read_to_string(scenario().join("attr_rules.txt")).expect("rules");
    assert!(host.sim_mut().set_attr_rules_text(&rules));
    let mods = common::pack_mods(&scenario(), &common::pack_roots(&scenario()));
    host.load_mods(&mods).expect("moba mods load");
    let ui: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(scenario().join("moba_ui.json")).expect("ui")).expect("ui json");
    host.dispatch_action("setup", &json!({ "ui": ui, "player": 0 })).expect("setup");
    host.dispatch_action("seat_pick", &json!({ "player": 0, "hero": "antimage" })).expect("seat 0");
    host.dispatch_action("seat_pick", &json!({ "player": 1, "hero": "antimage" })).expect("seat 1");
    host.advance().expect("tick");
    let hero_of = |host: &ModHost, p: u8| -> usize {
        let sim = host.sim();
        (0..sim.count()).find(|&i| sim.alive(i) && sim.player(i) == p && sim.is_attributed(i)).expect("the seat has a hero")
    };
    let killer = hero_of(&host, 0);
    let victim = hero_of(&host, 1);
    // The killer dies first.
    host.sim_mut().hurt(killer, 2_000_000_000);
    host.advance().expect("tick");
    assert!(!host.sim().alive(killer), "the killer is dead before the death is fed");
    let death = Death {
        victim: victim as u32,
        victim_player: 1,
        killer: Some(killer as u32),
        killer_player: Some(0),
        bounty: 40,
    };
    host.feed_death(&death).expect("the death is fed");
    assert!(host.disabled_hooks().is_empty(), "no hook is switched off: {:?}", host.disabled_hooks());
}


#![cfg(feature = "build")]

use mod_host::{build_image, ModHost};
use serde_json::json;
use sim::{fixed, TICKS_PER_SECOND};

mod common;
use common::scenario;

fn host_with_hero() -> (ModHost, usize) {
    let image = build_image(&scenario()).expect("moba scenario builds an image");
    let mut host = ModHost::new(&image).expect("host");
    let rules = std::fs::read_to_string(scenario().join("attr_rules.txt")).expect("rules");
    assert!(host.sim_mut().set_attr_rules_text(&rules), "the pack's rules file loads");
    let mods = common::pack_mods(&scenario(), &common::pack_roots(&scenario()));
    host.load_mods(&mods).expect("moba mods load");
    let ui: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(scenario().join("moba_ui.json")).expect("ui")).expect("ui json");
    host.dispatch_action("setup", &json!({ "ui": ui, "player": 0 })).expect("setup");
    host.dispatch_action("seat_pick", &json!({ "player": 0, "hero": "skeleton_king" })).expect("seat_pick");
    host.advance().expect("tick");
    let kind = host.sim().catalog().unit_index("skeleton_king").expect("skeleton_king is a unit kind") as i32;
    let hero = (0..host.sim().count()).find(|&i| host.sim().alive(i) && host.sim().unit_type(i) == kind).expect("the picked hero");
    (host, hero)
}

fn kill(host: &ModHost, h: usize) {
    host.sim_mut().hurt(h, i32::MAX / 2);
    assert!(!host.sim().alive(h), "the hero dies to lethal damage");
}

#[test]
fn leoric_reincarnation_returns_the_hero_in_three_seconds_for_mana_and_a_cooldown() {
    let (mut host, h) = host_with_hero();
    let slot = host.sim().ability_slot(h, "skeleton_king_reincarnation_leoric").expect("the hero holds Reincarnation");
    assert!(host.sim_mut().set_skill_rank(h, slot, 1).is_some(), "rank one is set");
    let mana = host.sim().mana_of(h);
    assert!(mana >= 160, "the hero holds the 160 mana the skill costs");
    let spot = host.sim().position_fixed(h);
    kill(&host, h);
    host.advance().expect("the death tick");
    for _ in 0..2 * TICKS_PER_SECOND {
        host.advance().expect("tick");
    }
    assert!(!host.sim().alive(h), "still dead before the 3 s delay");
    for _ in 0..2 * TICKS_PER_SECOND {
        if host.sim().alive(h) {
            break;
        }
        host.advance().expect("tick");
    }
    assert!(host.sim().alive(h), "alive again within 4 s of the death");
    let (x, y) = host.sim().position_fixed(h);
    assert!((x - spot.0).abs() <= fixed::ONE && (y - spot.1).abs() <= fixed::ONE, "back where it fell");
    assert!(host.sim().mana_of(h) <= mana - 160, "the skill spent 160 mana");
    assert!(host.sim().ability_cooldown(h, slot) > 3000, "the 260 s cooldown runs");
}

#[test]
fn a_hero_with_a_cooling_leoric_reincarnation_waits_for_the_respawn_timer() {
    let (mut host, h) = host_with_hero();
    let slot = host.sim().ability_slot(h, "skeleton_king_reincarnation_leoric").expect("the hero holds Reincarnation");
    assert!(host.sim_mut().set_skill_rank(h, slot, 1).is_some());
    host.sim_mut().set_ability_cooldown(h, slot, 1000);
    kill(&host, h);
    host.advance().expect("the death tick");
    for _ in 0..5 * TICKS_PER_SECOND {
        host.advance().expect("tick");
    }
    assert!(!host.sim().alive(h), "no reincarnation while the skill cools");
}

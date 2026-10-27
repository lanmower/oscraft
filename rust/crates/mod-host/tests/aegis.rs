
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
    host.dispatch_action("seat_pick", &json!({ "player": 0, "hero": "antimage" })).expect("seat_pick");
    host.advance().expect("tick");
    let kind = host.sim().catalog().unit_index("antimage").expect("antimage is a unit kind") as i32;
    let hero = (0..host.sim().count()).find(|&i| host.sim().alive(i) && host.sim().unit_type(i) == kind).expect("the picked hero");
    (host, hero)
}

fn kill(host: &ModHost, h: usize) {
    host.sim_mut().hurt(h, i32::MAX / 2);
    assert!(!host.sim().alive(h), "the hero dies to lethal damage");
}

#[test]
fn a_hero_dying_with_the_aegis_returns_where_it_fell_at_full_health_and_the_charge_is_spent() {
    let (mut host, h) = host_with_hero();
    let aegis = host.sim().catalog().item_index("aegis").expect("the pack has an aegis") as u32 + 1;
    assert!(host.sim_mut().inventory_add(h, aegis, 1), "the aegis fits the hero");
    let spot = host.sim().position_fixed(h);
    let hp_max = host.sim().hp_max(h);
    kill(&host, h);
    host.advance().expect("the death tick");
    assert_eq!(host.sim().inventory_count(h, aegis), 0, "one charge of the aegis is spent at the death");

    for _ in 0..4 * TICKS_PER_SECOND {
        host.advance().expect("tick");
    }
    assert!(!host.sim().alive(h), "the hero is still dead before the aegis's seconds run out");
    for _ in 0..3 * TICKS_PER_SECOND {
        if host.sim().alive(h) {
            break;
        }
        host.advance().expect("tick");
    }
    assert!(host.sim().alive(h), "the hero is alive again within 7 seconds of its death");
    let (x, y) = host.sim().position_fixed(h);
    assert!((x - spot.0).abs() <= fixed::ONE && (y - spot.1).abs() <= fixed::ONE, "back where it fell: {:?} vs {spot:?}", (x, y));
    assert_eq!(host.sim().hp(h), hp_max, "full health");
    assert_eq!(host.sim().mana_of(h), host.sim().unit_max_mana(h), "full mana");
}

#[test]
fn a_hero_without_the_aegis_goes_to_the_respawn_timer_and_returns_home() {
    let (mut host, h) = host_with_hero();
    let home = host.sim().position_fixed(h);
    kill(&host, h);
    host.advance().expect("the death tick");
    for _ in 0..6 * TICKS_PER_SECOND {
        host.advance().expect("tick");
    }
    assert!(!host.sim().alive(h), "a hero with no aegis waits for the respawn timer");
    let mut waited = 0;
    while !host.sim().alive(h) && waited < 70 * TICKS_PER_SECOND {
        host.advance().expect("tick");
        waited += 1;
    }
    assert!(host.sim().alive(h), "the hero returns after the respawn timer");
    let (x, y) = host.sim().position_fixed(h);
    assert!((x - home.0).abs() <= fixed::ONE && (y - home.1).abs() <= fixed::ONE, "the hero returns home");
}

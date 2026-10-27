#![cfg(feature = "build")]

use mod_host::{build_image, Cmd, Death, ModHost};
use serde_json::json;

mod common;
use common::scenario;

const NEUTRAL: u8 = 255;
const GREED: &str = "alchemist_goblin_s_greed";

fn host_with_greedy_hero() -> (ModHost, usize) {
    let image = build_image(&scenario()).expect("moba scenario builds an image");
    let mut host = ModHost::new(&image).expect("host");
    let rules = std::fs::read_to_string(scenario().join("attr_rules.txt")).expect("rules");
    assert!(host.sim_mut().set_attr_rules_text(&rules), "the pack's rules file loads");
    let mods = common::pack_mods(&scenario(), &common::pack_roots(&scenario()));
    host.load_mods(&mods).expect("moba mods load");
    let ui: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(scenario().join("moba_ui.json")).expect("ui")).expect("ui json");
    host.dispatch_action("setup", &json!({ "ui": ui, "player": 1 })).expect("setup");
    host.dispatch_action("seat_pick", &json!({ "player": 1, "hero": "alchemist" })).expect("seat_pick");
    host.advance().expect("tick");
    let kind = host.sim().catalog().unit_index("alchemist").expect("alchemist is a unit kind") as i32;
    let hero = (0..host.sim().count()).find(|&i| host.sim().alive(i) && host.sim().unit_type(i) == kind).expect("the picked hero");
    let slot = host.sim().ability_slot(hero, GREED).expect("alchemist carries Goblin's Greed");
    host.sim_mut().set_skill_rank(hero, slot, 1).expect("rank one");
    (host, hero)
}

fn paid(host: &mut ModHost, d: &Death) -> Vec<i32> {
    host.feed_death(d)
        .expect("fed")
        .iter()
        .filter_map(|c| match c {
            Cmd::AddGold { player: 1, amount } if *amount > 0 => Some(*amount),
            _ => None,
        })
        .collect()
}

#[test]
fn each_hostile_creep_kill_raises_the_greed_stack_by_one() {
    let (mut host, hero) = host_with_greedy_hero();
    let creeps: Vec<usize> = (0..host.sim().count()).filter(|&u| host.sim().player(u) == NEUTRAL).take(4).collect();
    assert_eq!(creeps.len(), 4, "the map has neutral creeps");
    let mut got = Vec::new();
    for c in creeps {
        let d = Death { victim: c as u32, victim_player: NEUTRAL, killer: Some(hero as u32), killer_player: Some(1), bounty: 0 };
        got.extend(paid(&mut host, &d));
    }
    assert_eq!(got, [6, 9, 12, 12], "4 + 2 per rank plus 3 per kill in 30 s, capped at 4 + 8 per rank");
}

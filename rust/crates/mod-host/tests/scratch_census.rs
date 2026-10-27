#![cfg(feature = "build")]
use mod_host::{build_image, ModHost};
use serde_json::json;
mod common;
use common::scenario;

#[test]
fn census() {
    let image = build_image(&scenario()).expect("image");
    let mut host = ModHost::new(&image).expect("host");
    let rules = std::fs::read_to_string(scenario().join("attr_rules.txt")).expect("rules");
    host.sim_mut().set_attr_rules_text(&rules);
    let mods = common::pack_mods(&scenario(), &common::pack_roots(&scenario()));
    host.load_mods(&mods).expect("mods");
    let ui: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(scenario().join("moba_ui.json")).unwrap()).unwrap();
    host.dispatch_action("setup", &json!({ "ui": ui, "player": 0 })).unwrap();
    host.dispatch_action("seat_pick", &json!({ "player": 0, "hero": "antimage" })).unwrap();
    let ticks: usize = std::env::var("CENSUS_TICKS").ok().and_then(|v| v.parse().ok()).unwrap_or(500);
    for _ in 0..ticks {
        host.advance().unwrap();
    }
    let sim = host.sim();
    let mut counts = std::collections::BTreeMap::new();
    for i in 0..sim.count() {
        if sim.alive(i) {
            let name = sim.kind_name(i).unwrap_or("?").to_string();
            *counts.entry((name, sim.player(i))).or_insert(0) += 1;
        }
    }
    for ((n, p), c) in counts {
        if n.contains("creep") || n.contains("lane") {
            println!("CENSUS {n} player {p}: {c}");
        }
    }
    for k in ["waves_total","waves_creep","waves_creep_ranged","waves_siege"] { println!("CENSUS state {k} {:?}", host.state_int(k)); }
    println!("CENSUS disabled {:?}", host.disabled_hooks());
}

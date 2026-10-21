//! The moba pack loads as the page loads it: the manifest names its root (init.lua), the modules the root reaches load in
//! that order with no error, and the pack runs 300 ticks. Every .lua file of the pack is one of those modules.

#![cfg(feature = "build")]

mod common;

use mod_host::{build_image, ModHost};

#[test]
fn every_module_loads_and_runs_300_ticks() {
    let scenario = common::scenario();
    let roots = common::pack_roots(&scenario);
    assert_eq!(roots, vec!["init.lua"], "the manifest names the root module only");
    let mods = common::pack_mods(&scenario, &roots);
    assert_eq!(mods.len(), common::pack_files(&scenario).len(), "every .lua file of the pack is a module the root reaches");

    let image = build_image(&scenario).expect("moba scenario builds an image");
    let mut host = ModHost::new(&image).expect("host");
    assert!(host.sim_mut().set_attr_rules_text(&common::pack_rules(&scenario)), "the pack's rules file loads");
    host.load_mods(&mods).unwrap_or_else(|e| panic!("moba mods failed to load: {e}"));
    for tick in 0..300 {
        host.advance().unwrap_or_else(|e| panic!("tick {tick} failed: {e}"));
    }
}

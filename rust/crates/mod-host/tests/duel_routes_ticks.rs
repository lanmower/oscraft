//! The duel-routes pack plays through a run without any of its hooks failing: the economy start, harvest, production
//! and transport are its mods. A hook whose Lua errors is switched off by the host, so a switched-off hook is the failure.

#![cfg(feature = "build")]

use mod_host::{build_image, ModHost};

mod common;
use std::path::PathBuf;

fn duel_routes() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/duel-routes")
}

#[test]
fn the_duel_routes_pack_plays_without_a_hook_switched_off() {
    let dir = duel_routes();
    let image = build_image(&dir).expect("duel-routes builds an image");
    let mut host = ModHost::new(&image).expect("host");
    assert!(host.sim_mut().set_attr_rules_text(&common::pack_rules(&dir)), "the pack's rules file loads");
    host.load_mods(&common::pack_mods(&dir, &common::pack_roots(&dir))).expect("duel-routes mods load");
    for _ in 0..1200 {
        host.advance().expect("tick");
    }
    assert_eq!(host.disabled_hooks(), Vec::<String>::new(), "no hook of the pack may fail while it plays");
}

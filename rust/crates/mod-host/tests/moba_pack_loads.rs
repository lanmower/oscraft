//! The moba pack's mod chunks, as scenarios/moba/manifest.json lists them, load into the piccolo host
//! in manifest order and run 300 ticks with no error. No chunk needs an on_tick of its own.

#![cfg(feature = "build")]

use std::path::Path;

use mod_host::{build_image, ModHost};

#[test]
fn every_manifest_mod_loads_and_runs_300_ticks() {
    let scenario = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba");
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(scenario.join("manifest.json")).expect("manifest")).expect("manifest json");
    let mods: Vec<(String, String)> = manifest["mods"]
        .as_array()
        .expect("manifest mods list")
        .iter()
        .map(|m| {
            let file = m.as_str().expect("mod file name");
            let src = std::fs::read_to_string(scenario.join(file)).unwrap_or_else(|e| panic!("read {file}: {e}"));
            (file.to_owned(), src)
        })
        .collect();
    assert_eq!(mods.len(), 15, "the manifest lists 15 mod chunks");

    let image = build_image(&scenario).expect("moba scenario builds an image");
    let mut host = ModHost::new(&image).expect("host");
    host.load_mods(&mods).unwrap_or_else(|e| panic!("moba mods failed to load: {e}"));
    for tick in 0..300 {
        host.advance().unwrap_or_else(|e| panic!("tick {tick} failed: {e}"));
    }
}

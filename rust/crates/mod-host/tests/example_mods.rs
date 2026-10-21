//! Every example mod under mods/ loads on the piccolo host. A mod that names a game binding the host lacks fails
//! here with the binding in its error, so the docs' list of unbound names stays checked.

#![cfg(feature = "build")]

use std::path::Path;

use mod_host::ModHost;

#[test]
fn every_example_mod_loads_on_the_host() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../");
    let scenario = root.join("scenarios/moba");
    let image = mod_host::build_image(&scenario).expect("moba scenario builds an image");
    let mut failures = Vec::new();
    let mut names: Vec<_> = std::fs::read_dir(root.join("mods"))
        .expect("mods dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "lua"))
        .collect();
    names.sort();
    for path in names {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let mut text = std::fs::read_to_string(&path).expect("read mod");
        // A mod that names another file's API loads joined to it, as its header says (cat bot_lib.lua first).
        if name == "bot_unit_example.lua" {
            text = std::fs::read_to_string(root.join("mods/bot_lib.lua")).expect("bot_lib") + "\n" + &text;
        }
        let mut host = ModHost::new(&image).expect("host");
        if let Err(e) = host.load_mods(&[(name.clone(), text)]) {
            failures.push(format!("{name}: {e}"));
        }
    }
    assert!(failures.is_empty(), "example mods that do not load:\n{}", failures.join("\n"));
}

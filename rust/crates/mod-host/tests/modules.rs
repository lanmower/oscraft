
#![cfg(feature = "build")]

use std::path::Path;

use mod_host::ModHost;

fn host() -> ModHost {
    let scenario = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba");
    let image = mod_host::build_image(&scenario).expect("moba scenario builds an image");
    ModHost::new(&image).expect("host")
}

fn mods(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter().map(|(f, s)| ((*f).to_owned(), (*s).to_owned())).collect()
}

#[test]
fn a_module_requires_another_and_calls_its_function() {
    let mut host = host();
    let a = r#"
        local b = require("b")
        return { on_tick = function(state) state.v = b.double(21) end }
    "#;
    let b = "return { double = function(n) return n * 2 end }";
    host.load_mods(&mods(&[("a.lua", a), ("b.lua", b)])).expect("a and b load");
    host.advance().expect("tick");
    assert_eq!(host.state_int("v"), Some(42));
}

#[test]
fn a_require_cycle_is_reported_by_name() {
    let mut host = host();
    let a = r#"local b = require("b") return { on_tick = function(state) state.a = 1 end }"#;
    let b = r#"local a = require("a") return { on_tick = function(state) state.b = 1 end }"#;
    let c = r#"return { on_tick = function(state) state.c = 1 end }"#;
    let err = host.load_mods(&mods(&[("a.lua", a), ("b.lua", b), ("c.lua", c)])).expect_err("the cycle is an error");
    assert!(err.contains("require_cycle: a -> b -> a"), "error names the cycle: {err}");
    assert!(!err.contains("c:"), "c is not in the cycle: {err}");
    host.advance().expect("tick");
    assert_eq!(host.state_int("c"), Some(1), "the module outside the cycle runs");
    assert_eq!(host.state_int("a"), None, "a cycle member does not load");
}

#[test]
fn an_erroring_hook_does_not_stop_the_same_hook_of_another_module() {
    let mut host = host();
    let bad = r#"return { on_tick = function(state) error("boom") end }"#;
    let good = r#"return { on_tick = function(state) state.ran = (state.ran or 0) + 1 end }"#;
    host.load_mods(&mods(&[("bad.lua", bad), ("good.lua", good)])).expect("both load");
    host.advance().expect("first tick");
    host.advance().expect("second tick");
    assert_eq!(host.state_int("ran"), Some(2), "good.lua's on_tick ran on both ticks");
}

#[test]
fn a_failed_load_keeps_the_last_known_good_module() {
    let mut host = host();
    let v1 = r#"return { on_tick = function(state) state.v = 1 end }"#;
    host.load_mods(&mods(&[("m.lua", v1)])).expect("v1 loads");
    let v2 = r#"error("broken top level") return { on_tick = function(state) state.v = 2 end }"#;
    let err = host.load_mods(&mods(&[("m.lua", v2)])).expect_err("v2 fails to load");
    assert!(err.contains("broken top level"), "the error text is reported: {err}");
    host.advance().expect("tick");
    assert_eq!(host.state_int("v"), Some(1), "the last-known-good hook still runs");
}

#[test]
fn an_old_style_global_hook_file_still_works() {
    let mut host = host();
    let old = "function on_tick(state) state.old = 7 end";
    host.load_mods(&mods(&[("old.lua", old)])).expect("old-style file loads");
    host.advance().expect("tick");
    assert_eq!(host.state_int("old"), Some(7));
}

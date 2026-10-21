//! The text helpers packs use (game.str_lower, game.str_find_ci), and the old names of the renamed bindings.

#![cfg(feature = "build")]

use std::path::Path;

use mod_host::ModHost;

fn host() -> ModHost {
    let scenario = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba");
    let image = mod_host::build_image(&scenario).expect("moba scenario builds an image");
    ModHost::new(&image).expect("host")
}

// Loads one module as the pack, runs one tick, and returns the host to read the state it left.
fn run(src: &str) -> ModHost {
    let mut host = host();
    host.load_mods(&[("pack.lua".to_owned(), src.to_owned())]).expect("module loads");
    host.advance().expect("tick");
    host
}

#[test]
fn str_lower_lowers_ascii_letters_and_leaves_the_rest() {
    let host = run(r#"return { on_tick = function(state)
        state.mixed = game.str_lower("Shop: Ab-Z 9") == "shop: ab-z 9"
        state.empty = game.str_lower("") == ""
        state.accent = game.str_lower("\u{c9}A") == "\u{c9}a"
    end }"#);
    assert_eq!(host.state_bool("mixed"), Some(true));
    assert_eq!(host.state_bool("empty"), Some(true));
    assert_eq!(host.state_bool("accent"), Some(true), "only ASCII letters change case");
}

#[test]
fn str_find_ci_returns_the_one_based_index_ignoring_case_or_nil() {
    let host = run(r#"return { on_tick = function(state)
        state.at = game.str_find_ci("Open the SHOP", "shop")
        state.first = game.str_find_ci("abcabc", "BC")
        state.quote = game.str_find_ci('say "hi"', '"')
        state.empty = game.str_find_ci("abc", "")
        state.none = game.str_find_ci("abc", "z") == nil
        state.longer = game.str_find_ci("ab", "abc") == nil
    end }"#);
    assert_eq!(host.state_int("at"), Some(10), "SHOP starts at byte 10");
    assert_eq!(host.state_int("first"), Some(2), "the first occurrence wins");
    assert_eq!(host.state_int("quote"), Some(5));
    assert_eq!(host.state_int("empty"), Some(1), "an empty needle is found at 1, as string.find finds it");
    assert_eq!(host.state_bool("none"), Some(true), "a missing needle is nil");
    assert_eq!(host.state_bool("longer"), Some(true), "a needle longer than the haystack is nil");
}

#[test]
fn the_old_binding_names_are_the_renamed_bindings() {
    let host = run(r#"return { on_tick = function(state)
        state.old_xp = game.xp_bounty == game.reward_xp
        state.old_unit = game.unit_bounty == game.reward_unit
        state.old_radius = game.assist_radius_milli == game.credit_radius_milli
        state.new_names = type(game.reward_xp) == "function" and type(game.reward_unit) == "function"
            and type(game.credit_radius_milli) == "function"
    end }"#);
    assert_eq!(host.state_bool("old_xp"), Some(true));
    assert_eq!(host.state_bool("old_unit"), Some(true));
    assert_eq!(host.state_bool("old_radius"), Some(true));
    assert_eq!(host.state_bool("new_names"), Some(true));
}

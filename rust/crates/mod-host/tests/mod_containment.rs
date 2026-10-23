//! Containment: a hook that never returns is bounded and switched off while the host keeps advancing; a module that
//! holds too much Lua memory is retired while the others run; and a native's bad argument is a Lua error the mod sees,
//! not a silent value. A native that panics is covered by the unit test in src/piccolo_host.rs, since a test-only native
//! exists only in the library's own test build.

#![cfg(feature = "build")]

use std::path::Path;

use mod_host::ModHost;
use serde_json::json;

fn host() -> ModHost {
    let scenario = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba");
    let image = mod_host::build_image(&scenario).expect("moba scenario builds an image");
    ModHost::new(&image).expect("host")
}

fn mods(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter().map(|(f, s)| ((*f).to_owned(), (*s).to_owned())).collect()
}

fn is_disabled(host: &ModHost, key: &str) -> bool {
    host.disabled_hooks().iter().any(|h| h == key)
}

// A living unit with an ability in slot 0: one the scenario has, or else one spawned for player 0 at its start.
fn ability_unit(host: &mut ModHost) -> usize {
    let found = (0..host.sim().count()).find(|&u| host.sim().alive(u) && host.sim().ability_in_slot(u, 0).is_some());
    if let Some(u) = found {
        return u;
    }
    let start = host.sim().catalog().starts.iter().find(|s| s.player == 0).map(|s| (s.x, s.y)).expect("player 0 has a start");
    for kind in 0..host.sim().catalog().units.len() {
        let spawned = host.sim_mut().spawn_for(0, kind as u32, start.0, start.1);
        if let Some(u) = spawned {
            if host.sim().ability_in_slot(u, 0).is_some() {
                return u;
            }
        }
    }
    panic!("no unit kind of the scenario has an ability in slot 0");
}

// (a) an on_tick that never returns: the call is bounded, the hook is off, and the other module runs on every tick.
#[test]
fn an_infinite_loop_in_on_tick_is_bounded() {
    let mut host = host();
    let spin = "return { on_tick = function(state) while true do end end }";
    let steady = "return { on_tick = function(state) state.steady = (state.steady or 0) + 1 end }";
    host.load_mods(&mods(&[("spin.lua", spin), ("steady.lua", steady)])).expect("both load");
    for _ in 0..300 {
        host.advance().expect("the host keeps advancing");
    }
    assert!(is_disabled(&host, "spin:on_tick"), "the looping hook is off: {:?}", host.disabled_hooks());
    assert_eq!(host.state_int("steady"), Some(300), "the other module ran on every tick");
}

// (b) on_hit, on_behaviour and on_cast_request that never return: each is bounded and switched off.
#[test]
fn an_infinite_loop_in_each_other_hook_is_bounded() {
    let mut host = host();
    let spin = r#"return {
        on_hit = function(attacker, victim, dealt, state) while true do end end,
        on_behaviour = function() while true do end end,
        on_cast_request = function() while true do end end,
    }"#;
    host.load_mods(&mods(&[("spin_all.lua", spin)])).expect("loads");
    let caster = ability_unit(&mut host);
    let kind = host.sim().unit_type(caster) as u32;
    let player = host.sim().player(caster);
    // The victim has to be an enemy, not merely another player: the map assigns teams, and two players of the same
    // team are allies, so a hit on them never reaches on_hit. Falls back to a start whose player is hostile.
    let victim = {
        let found = (0..host.sim().count())
            .find(|&u| u != caster && host.sim().alive(u) && host.sim().hostile(player, host.sim().player(u)));
        match found {
            Some(u) => u,
            None => {
                let start = host
                    .sim()
                    .catalog()
                    .starts
                    .iter()
                    .find(|s| host.sim().hostile(player, s.player))
                    .map(|s| (s.player, s.x, s.y))
                    .expect("a start belongs to a player hostile to the caster");
                let spawned = host.sim_mut().spawn_for(start.0, kind, start.1, start.2);
                spawned.expect("a victim spawns")
            }
        }
    };
    host.sim_mut().damage_target(caster as i32, player, 100, victim as i32);
    host.advance().expect("the hit tick completes");
    assert!(is_disabled(&host, "spin_all:on_hit"), "on_hit is off: {:?}", host.disabled_hooks());

    host.emit_unit("on_behaviour", caster).expect("the behaviour event completes");
    assert!(is_disabled(&host, "spin_all:on_behaviour"), "on_behaviour is off: {:?}", host.disabled_hooks());

    let allowed = host.cast_allowed(caster, 0, -1, 0, 0, false);
    assert!(!allowed, "a cast_request whose hook fails is refused");
    assert!(is_disabled(&host, "spin_all:on_cast_request"), "on_cast_request is off: {:?}", host.disabled_hooks());

    host.advance().expect("the host keeps advancing after all three are off");
}

// (c) a module that keeps Lua memory past the cap is retired (all its hooks off, its tables emptied); the other module
// keeps running every tick.
#[test]
fn a_module_over_the_memory_cap_is_retired_and_the_rest_run() {
    let mut host = host();
    let hog = r#"return { on_tick = function(state) big = big or "0123456789abcdef"; big = big .. big end }"#;
    let small = "return { on_tick = function(state) state.small = (state.small or 0) + 1 end }";
    host.load_mods(&mods(&[("hog.lua", hog), ("small.lua", small)])).expect("both load");
    let mut ticks = 0;
    while !is_disabled(&host, "hog:on_tick") {
        assert!(ticks < 200, "the hog must reach the memory cap within 200 ticks");
        host.advance().expect("the host keeps advancing");
        ticks += 1;
    }
    for _ in 0..5 {
        host.advance().expect("the host keeps advancing after the retire");
        ticks += 1;
    }
    assert!(is_disabled(&host, "hog:on_hit"), "a retired module has every hook off, not only the one that failed");
    assert!(!host.disabled_hooks().iter().any(|h| h.starts_with("small:")), "the other module is not retired");
    assert_eq!(host.state_int("small"), Some(ticks), "the other module ran on every tick");
}

// (d) a native's non-finite or negative argument, and an over-long counter name, are Lua errors the mod can catch. The
// hook does not fail: the mod sees the refusal, as it would any bad argument.
#[test]
fn a_bad_native_argument_is_a_lua_error_the_mod_sees() {
    let mut host = host();
    let probe = r#"return { on_action = function(name, args, state)
        local refused = 0
        if not pcall(game.effect, "pulse", 0 / 0, 0) then refused = refused + 1 end
        if not pcall(game.effect, "pulse", 0, 0, { duration = -1 }) then refused = refused + 1 end
        local long = "counter_name_that_is_well_past_the_sixty_four_byte_limit_for_mod_counters_xxxxxxxxxxxxxxxx"
        if not pcall(game.set_counter, args.unit, long, 1) then refused = refused + 1 end
        state.refused = refused
    end }"#;
    host.load_mods(&mods(&[("probe.lua", probe)])).expect("loads");
    let unit = ability_unit(&mut host);
    host.dispatch_action("probe", &json!({ "unit": unit })).expect("the action completes");
    assert_eq!(host.state_int("refused"), Some(3), "each bad argument was refused");
    assert!(host.disabled_hooks().is_empty(), "a refused argument is not a hook failure: {:?}", host.disabled_hooks());
}

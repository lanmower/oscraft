//! The per-unit rules the pack sets each tick (scenarios/moba/unit_rules.lua): the damage scale of a unit's armour, the
//! speed bounds of its movement, and the miss chance of its shots. Each is checked against the formula it implements.

#![cfg(feature = "build")]

use mod_host::{build_image, ModHost};
use serde_json::json;

mod common;
use common::scenario;

const ONE: i64 = 65536;

/// The damage scale of an armour value, as the armour curve states it: 1 - k*a / (1 + k*|a|), in Q16.16.
fn armour_scale(armour: i64, k: i64) -> i64 {
    let ka = (k * armour) >> 16;
    let reduction = (ka << 16) / (ONE + ka.abs());
    ONE - reduction
}

/// A milli-unit speed in Q16.16 per tick.
fn from_milli(m: i64) -> i64 {
    m * ONE / 1000
}

fn seated_host() -> ModHost {
    let image = build_image(&scenario()).expect("moba scenario builds an image");
    let mut host = ModHost::new(&image).expect("host");
    let rules = std::fs::read_to_string(scenario().join("attr_rules.txt")).expect("rules");
    assert!(host.sim_mut().set_attr_rules_text(&rules), "the pack's rules file loads");
    let mods = common::pack_mods(&scenario(), &common::pack_roots(&scenario()));
    host.load_mods(&mods).expect("moba mods load");
    let ui: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(scenario().join("moba_ui.json")).expect("ui")).expect("ui json");
    host.dispatch_action("setup", &json!({ "ui": ui, "player": 0 })).expect("setup");
    host.dispatch_action("seat_pick", &json!({ "player": 0, "hero": "antimage" })).expect("seat_pick");
    for _ in 0..3 {
        host.advance().expect("tick");
    }
    host
}

fn seat_hero(host: &ModHost) -> usize {
    let sim = host.sim();
    (0..sim.count()).find(|&i| sim.alive(i) && sim.player(i) == 0 && sim.is_attributed(i)).expect("seat 0 has a hero")
}

#[test]
fn the_pack_sets_each_units_armour_scale_from_its_armour() {
    let host = seated_host();
    let hero = seat_hero(&host);
    let k = host.sim().rule("armour_per_point_q16").expect("the rules text sets the armour rate") as i64;
    let armour = host.sim().eff_armour_of(hero) as i64;
    assert_eq!(host.sim().armour_scale_of(hero) as i64, armour_scale(armour, k), "armour {armour} at rate {k}");
}

#[test]
fn the_pack_holds_each_units_speed_between_the_reference_limits() {
    let host = seated_host();
    let hero = seat_hero(&host);
    let sim = host.sim();
    let max_points = sim.rule("move_speed_max").expect("the rules text sets the speed maximum") as i64;
    let min_points = sim.rule("move_speed_min").expect("the rules text sets the speed minimum") as i64;
    let point = sim.rule("speed_point_q16").expect("the rules text sets the speed point") as i64;
    let own = sim.base_speed_of(hero) as i64;
    let type_base = from_milli(sim.type_speed_milli(hero) as i64);
    let cap = (max_points * point).max(type_base);
    let floor = (min_points * point).min(own);
    assert_eq!(sim.speed_bounds_of(hero), (floor as i32, cap as i32), "bounds of a unit with base speed {own}");
}

#[test]
fn the_pack_sets_a_shots_miss_chance_to_its_data_chance_when_it_is_ranged() {
    let host = seated_host();
    let hero = seat_hero(&host);
    let sim = host.sim();
    let (_, range) = sim.weapon(hero);
    let melee = sim.rule("melee_range_q16").expect("the rules text sets the melee range") as i32;
    assert!(range > melee, "the hero's weapon is ranged");
    assert_eq!(sim.miss_chance_of(hero), sim.data_miss_chance(hero), "a ranged shot keeps its data chance");
}

#[test]
fn the_pack_sets_each_units_magic_keep_from_its_magic_resistance() {
    let host = seated_host();
    let hero = seat_hero(&host);
    let sim = host.sim();
    let base = sim.magic_base_of(hero) as i64;
    let bonus = sim.stat(hero, 11) as i64; // the magic resistance stat code (spell.rs S_MAGIC_RESIST)
    let want = (100 - base) * (100 - bonus) / 100;
    assert_eq!(sim.magic_keep_of(hero) as i64, want, "base {base}, bonus {bonus}");
}

#[test]
fn the_pack_sets_each_units_damage_keep_and_response_keep() {
    let host = seated_host();
    let hero = seat_hero(&host);
    let sim = host.sim();
    let reduction = (sim.stat(hero, 30) as i64).clamp(-100, 100); // the damage reduction stat code (spell.rs S_DAMAGE_REDUCTION)
    assert_eq!(sim.damage_keep_of(hero) as i64, 100 - reduction, "damage reduction {reduction}");
    let want = match sim.response_of(hero) {
        2 => 0, // immune
        1 => sim.rule("reduced_response_pct").expect("the rules text sets the reduced response") as i64,
        _ => 100,
    };
    assert_eq!(sim.response_keep_of(hero) as i64, want, "response kind {}", sim.response_of(hero));
}

mod common;

use std::path::Path;

use common::*;
use sim::{fixed, Sim};

const HERO: &str = "omniknight";
const BLINK: &str = "blink";
const EAGLE: &str = "eagle";
const PURIFICATION: &str = "omniknight_purification";
const HOME_CELL: i32 = 4;
const ROW: i32 = 5;
const OVERKILL: i32 = 30_000;
const CAST_TICKS: usize = 60;
const SAVED_GOLD: i64 = 975;
const DAMAGE_COUNTERS: [&str; 2] = ["sim_damage_dealt", "sim_damage_taken"];

fn moba_field() -> Sim {
    let mut sim = open_field();
    let rules = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba/attr_rules.txt"))
        .expect("the moba attribute rules");
    assert!(sim.set_attr_rules_text(&rules), "the moba attribute rules load");
    sim
}

fn spawn_hero(sim: &mut Sim, player: u8, cx: i32, cy: i32) -> usize {
    let kind = sim.catalog().unit_index(HERO).expect("omniknight in the moba catalog") as u32;
    sim.spawn_attributed(player, kind, centre(cx), centre(cy)).expect("hero spawns")
}

fn kill(sim: &mut Sim, victim: usize, attacker: usize) {
    sim.hit_by(victim, fixed::from_int(OVERKILL), attacker as i32);
    assert!(!sim.alive(victim), "the lethal hit kills the hero");
}

fn revive(sim: &mut Sim, hero: usize) {
    assert!(sim.revive_at_point(hero, centre(HOME_CELL), centre(ROW + 1)), "the dead hero revives");
    assert!(sim.alive(hero), "the revived hero is alive");
}

fn occupied_item_slots(sim: &Sim, unit: usize) -> usize {
    (0..sim.inventory_slots()).filter(|&s| sim.inventory_slot(unit, s).1 > 0).count()
}

#[test]
fn assists_experience_kills_and_damage_survive_a_revival() {
    let mut sim = moba_field();
    let hero = spawn_hero(&mut sim, PLAYER, HOME_CELL, ROW);
    let foe = spawn_hero(&mut sim, FOE, 8, ROW);
    let max = sim.hp_max(hero);

    sim.hit_by(hero, fixed::from_int(40), foe as i32);
    sim.add_credit(hero, 1);
    sim.add_credit(hero, 1);
    sim.add_xp(hero, 250);
    sim.add_kill(hero, 1);
    kill(&mut sim, hero, foe);
    revive(&mut sim, hero);

    assert_eq!(sim.credits_of(hero), 2, "assists survive the revival");
    assert_eq!(sim.xp(hero), 250, "experience survives the revival");
    assert_eq!(sim.kills_of(hero), 1, "kills survive the revival");
    assert_eq!(
        sim.counter(hero, "sim_damage_taken"),
        i64::from(max),
        "damage taken covers the hp the hero had across both hits"
    );
}

#[test]
fn damage_dealt_and_taken_equal_the_hp_a_hero_loses() {
    let mut sim = moba_field();
    let attacker = spawn_hero(&mut sim, PLAYER, HOME_CELL, ROW);
    let victim = spawn_hero(&mut sim, FOE, 6, ROW);
    let before = sim.hp(victim);

    sim.hit_by(victim, fixed::from_int(60), attacker as i32);
    let lost = i64::from(before - sim.hp(victim));

    assert!(lost > 0, "the hit removes hp");
    assert_eq!(sim.counter(attacker, "sim_damage_dealt"), lost, "damage dealt is the hp removed");
    assert_eq!(sim.counter(victim, "sim_damage_taken"), lost, "damage taken is the hp removed");
    assert_eq!(sim.counter(victim, "sim_damage_dealt"), 0, "a hero that only takes damage deals none");
}

#[test]
fn an_overkill_hit_counts_only_the_hp_the_hero_had_and_a_plain_unit_counts_nothing() {
    let mut sim = moba_field();
    let victim = spawn_hero(&mut sim, PLAYER, HOME_CELL, ROW);
    let creep = spawn(&mut sim, FOE, 6, ROW);
    let before = sim.hp(victim);

    sim.hit_by(victim, fixed::from_int(OVERKILL), creep as i32);

    assert_eq!(
        sim.counter(victim, "sim_damage_taken"),
        i64::from(before),
        "damage taken stops at the hp the hero had"
    );
    assert_eq!(sim.counter(creep, "sim_damage_dealt"), 0, "a unit without attributes keeps no damage counter");
}

#[test]
fn healing_done_counts_only_the_hp_the_ally_gains() {
    let mut sim = moba_field();
    let healer = spawn_hero(&mut sim, PLAYER, HOME_CELL, ROW);
    assert!(sim.grant_energy(healer, 400).is_ok());
    let ally = spawn_hero(&mut sim, PLAYER, 6, ROW);
    let gap = fixed::from_int(30);
    assert!(sim.set_hp_within(ally, sim.hp_max(ally) - gap));

    assert!(sim.give_ability(healer, 0, PURIFICATION));
    assert!(sim.rank_up(healer, 0).is_ok(), "the heal ability takes a rank");
    assert!(sim.cast_ability(healer, 0, ally as i32, 0, 0), "the heal ability casts on the ally");
    for _ in 0..CAST_TICKS {
        sim.tick();
    }

    assert_eq!(sim.hp(ally), sim.hp_max(ally), "the ally is topped up");
    assert_eq!(
        sim.counter(healer, "sim_healing"),
        i64::from(gap),
        "healing done is the hp the ally gained, not the heal amount"
    );
}

#[test]
fn held_items_stay_counted_through_a_revival() {
    let mut sim = moba_field();
    let hero = spawn_hero(&mut sim, PLAYER, HOME_CELL, ROW);
    let foe = spawn_hero(&mut sim, FOE, 8, ROW);
    let blink = sim.catalog().item_index(BLINK).expect("blink in the moba catalog");
    let eagle = sim.catalog().item_index(EAGLE).expect("eagle in the moba catalog");
    assert!(sim.grant_item(hero, blink, 1));
    assert!(sim.grant_item(hero, eagle, 1));
    assert_eq!(occupied_item_slots(&sim, hero), 2, "two items are held before the death");

    kill(&mut sim, hero, foe);
    revive(&mut sim, hero);

    assert_eq!(occupied_item_slots(&sim, hero), 2, "both items are held after the revival");
}

#[test]
fn the_win_counters_survive_a_save_and_load_with_the_same_state_hash() {
    let mut sim = moba_field();
    let hero = spawn_hero(&mut sim, PLAYER, HOME_CELL, ROW);
    let foe = spawn_hero(&mut sim, FOE, 8, ROW);
    sim.add_credit(hero, 3);
    sim.add_xp(hero, 120);
    sim.add_kill(hero, 2);
    sim.set_counter(hero, "st_gold_earned", SAVED_GOLD);
    sim.hit_by(foe, fixed::from_int(40), hero as i32);
    sim.hit_by(hero, fixed::from_int(25), foe as i32);
    let bytes = sim.save_state();

    let mut restored = moba_field();
    restored.load_state(&bytes).expect("the saved state loads");

    assert_eq!(restored.credits_of(hero), 3, "assists are saved");
    assert_eq!(restored.xp(hero), 120, "experience is saved");
    assert_eq!(restored.kills_of(hero), 2, "kills are saved");
    assert_eq!(restored.counter(hero, "st_gold_earned"), SAVED_GOLD, "earned gold is saved");
    for name in DAMAGE_COUNTERS {
        assert_eq!(restored.counter(hero, name), sim.counter(hero, name), "{name} of the hero is saved");
        assert_eq!(restored.counter(foe, name), sim.counter(foe, name), "{name} of the foe is saved");
    }
    assert_eq!(restored.state_hash(), sim.state_hash(), "the restored state hashes the same");
}

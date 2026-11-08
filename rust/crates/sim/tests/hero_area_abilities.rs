mod common;

use common::*;
use sim::Sim;

const CENTAUR: &str = "centaur";
const MAGNATAUR: &str = "magnataur";
const HOOF_STOMP: &str = "centaur_hoof_stomp";
const REVERSE_POLARITY: &str = "magnataur_reverse_polarity";
const STUN: usize = 0;
const CAST_TICKS: usize = 120;
const PULL_MIN_MILLI: f64 = 1000.0;

fn moba_field() -> Sim {
    let mut sim = open_field();
    let rules = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba/attr_rules.txt"))
        .expect("the moba attribute rules");
    assert!(sim.set_attr_rules_text(&rules), "the moba attribute rules load");
    sim
}

fn hero(sim: &mut Sim, kind: &str, cx: i32, cy: i32) -> usize {
    let index = sim.catalog().unit_index(kind).expect("hero kind in the moba catalog") as u32;
    let unit = sim.spawn_attributed(PLAYER, index, centre(cx), centre(cy)).expect("hero spawns");
    assert!(sim.grant_energy(unit, 400).is_ok(), "the hero has energy to cast");
    unit
}

fn learn(sim: &mut Sim, unit: usize, ability: &str) {
    assert!(sim.give_ability(unit, 0, ability), "the ability is granted");
    assert!(sim.rank_up(unit, 0).is_ok(), "the ability takes a rank");
}

fn run_watching_stun(sim: &mut Sim, ticks: usize, unit: usize) -> bool {
    let mut stunned = false;
    for _ in 0..ticks {
        sim.tick();
        stunned |= sim.status_left(unit, STUN) > 0;
    }
    stunned
}

#[test]
fn hoof_stomp_hurts_and_stuns_only_enemies_in_its_radius() {
    let mut sim = moba_field();
    let caster = hero(&mut sim, CENTAUR, 4, 5);
    let near = spawn(&mut sim, FOE, 6, 5);
    let far = spawn(&mut sim, FOE, 14, 5);
    learn(&mut sim, caster, HOOF_STOMP);
    let (near_hp, far_hp) = (sim.hp(near), sim.hp(far));

    assert!(sim.cast_ability(caster, 0, -1, centre(4), centre(5)), "Hoof Stomp casts at the caster's feet");
    let near_stunned = run_watching_stun(&mut sim, CAST_TICKS, near);
    let far_stunned = run_watching_stun(&mut sim, 0, far);

    assert!(sim.hp(near) < near_hp, "the enemy 2 cells away takes Hoof Stomp damage");
    assert_eq!(sim.hp(far), far_hp, "the enemy 10 cells away is outside the radius and takes no damage");
    assert!(near_stunned, "the enemy in the radius is stunned");
    assert!(!far_stunned, "the enemy outside the radius is not stunned");
}

#[test]
fn reverse_polarity_pulls_enemies_in_its_radius_to_the_caster_and_stuns_them() {
    let mut sim = moba_field();
    let caster = hero(&mut sim, MAGNATAUR, 4, 10);
    let near = spawn(&mut sim, FOE, 9, 10);
    let far = spawn(&mut sim, FOE, 16, 10);
    learn(&mut sim, caster, REVERSE_POLARITY);
    let (cx, cy) = sim.position_fixed(caster);
    let near_before = distance_milli(&sim, near, cx, cy);
    let far_hp = sim.hp(far);

    assert!(sim.cast_ability(caster, 0, -1, cx, cy), "Reverse Polarity casts from the caster");
    let near_stunned = run_watching_stun(&mut sim, CAST_TICKS, near);
    let far_stunned = run_watching_stun(&mut sim, 0, far);

    let near_after = distance_milli(&sim, near, cx, cy);
    assert!(
        near_before - near_after >= PULL_MIN_MILLI,
        "the enemy 5 cells away is pulled toward the caster ({near_before:.0} -> {near_after:.0} milli)"
    );
    assert!(near_stunned, "the pulled enemy is stunned");
    assert_eq!(sim.hp(far), far_hp, "the enemy 12 cells away is outside the radius and takes no damage");
    assert!(!far_stunned, "the enemy outside the radius is not stunned");
}

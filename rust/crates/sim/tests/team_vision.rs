mod common;

use common::*;

const ALLY: u8 = 2;
const SETTLE_TICKS: u32 = 8;

fn fogged_field() -> sim::Sim {
    let mut sim = open_field();
    sim.set_fog(true);
    sim
}

fn settle(sim: &mut sim::Sim) {
    for _ in 0..SETTLE_TICKS {
        sim.tick();
    }
}

#[test]
fn a_teammate_sees_an_enemy_inside_an_allied_unit_sight() {
    let mut sim = fogged_field();
    spawn(&mut sim, PLAYER, 10, 15);
    let ally_unit = spawn(&mut sim, ALLY, 2, 15);
    let enemy = spawn(&mut sim, FOE, 21, 15);
    settle(&mut sim);
    assert!(!sim.visible(ALLY, enemy), "a lone ally on its own team does not see the enemy");

    assert!(sim.set_team(ALLY, 0).is_ok());
    settle(&mut sim);
    assert!(sim.alive(ally_unit));
    assert!(sim.visible(ALLY, enemy), "the teammate's sight reveals the enemy to the ally");
}

#[test]
fn an_alliance_relation_does_not_cancel_teammate_sight() {
    let mut sim = fogged_field();
    assert!(sim.set_team(ALLY, 0).is_ok());
    spawn(&mut sim, PLAYER, 10, 15);
    spawn(&mut sim, ALLY, 2, 15);
    let enemy = spawn(&mut sim, FOE, 21, 15);
    assert!(sim.set_relation(PLAYER, ALLY, 1).is_ok());
    settle(&mut sim);
    assert!(sim.visible(ALLY, enemy), "teammates keep shared sight when an ally relation is set");
}

#[test]
fn an_enemy_outside_every_teammate_sight_stays_hidden() {
    let mut sim = fogged_field();
    assert!(sim.set_team(ALLY, 0).is_ok());
    spawn(&mut sim, PLAYER, 10, 15);
    spawn(&mut sim, ALLY, 2, 15);
    let far = spawn(&mut sim, FOE, 28, 28);
    settle(&mut sim);
    assert!(!sim.visible(ALLY, far), "no teammate sight reaches the far enemy");
    assert!(!sim.visible(PLAYER, far), "the seer's own sight does not reach the far enemy");
}

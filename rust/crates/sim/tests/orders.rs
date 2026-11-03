mod common;

use common::*;

#[test]
fn move_order_cancels_an_attack_chase() {
    let mut sim = open_field();
    let attacker = spawn(&mut sim, PLAYER, 4, 5);
    let target = spawn(&mut sim, FOE, 16, 5);
    assert_eq!(sim.order_attack_or_approach(attacker, target), Ok(false));
    for _ in 0..10 {
        sim.tick();
    }
    assert_eq!(sim.attack_order_of(attacker), target as i32, "chase is under way");

    sim.order(attacker, centre(4), centre(20));
    for _ in 0..400 {
        sim.tick();
    }

    assert_eq!(sim.attack_order_of(attacker), -1, "a move order clears the attack order");
    assert!(
        distance_milli(&sim, attacker, centre(4), centre(20)) <= REACH_MILLI,
        "the moved unit reaches its move destination"
    );
}

#[test]
fn move_order_routes_around_a_wall_through_the_gap() {
    let mut sim = open_field();
    wall(&mut sim);
    let unit = spawn(&mut sim, PLAYER, 4, 5);
    let (dest_x, dest_y) = (centre(20), centre(5));
    sim.order(unit, dest_x, dest_y);

    let mut through_gap = false;
    for t in 0..MAX_TICKS {
        sim.tick();
        assert_never_on_blocked_cell(&sim, unit, t);
        let (_, y) = sim.position_fixed(unit);
        through_gap |= cell(y) >= GAP_FROM_Y;
        if distance_milli(&sim, unit, dest_x, dest_y) <= REACH_MILLI {
            assert!(through_gap, "the unit reached the far side without using the gap");
            return;
        }
    }
    panic!("the unit did not reach its destination within {MAX_TICKS} ticks");
}

#[test]
fn chase_closes_to_strike_range_around_a_wall() {
    let mut sim = open_field();
    wall(&mut sim);
    let attacker = spawn(&mut sim, PLAYER, 4, 5);
    let target = spawn(&mut sim, FOE, 20, 5);
    assert_eq!(sim.order_attack_or_approach(attacker, target), Ok(false));

    for t in 0..MAX_TICKS {
        sim.tick();
        assert_never_on_blocked_cell(&sim, attacker, t);
        if sim.can_strike(attacker, target) {
            return;
        }
    }
    panic!("the chaser did not reach strike range within {MAX_TICKS} ticks");
}

#[test]
fn chase_keeps_up_with_a_moving_target_around_a_wall() {
    let mut sim = open_field();
    wall(&mut sim);
    let attacker = spawn(&mut sim, PLAYER, 4, 5);
    let target = spawn(&mut sim, FOE, 20, 5);
    assert_eq!(sim.order_attack_or_approach(attacker, target), Ok(false));

    let mut legs = 0u32;
    for t in 0..MAX_TICKS {
        if t % 200 == 0 && sim.alive(target) {
            let row = if legs % 2 == 0 { 25 } else { 5 };
            sim.order(target, centre(20), centre(row));
            legs += 1;
        }
        sim.tick();
        assert_never_on_blocked_cell(&sim, attacker, t);
        if sim.can_strike(attacker, target) {
            return;
        }
    }
    panic!("the chaser did not reach strike range of the moving target within {MAX_TICKS} ticks");
}

#[test]
fn attack_move_routes_around_a_wall_through_the_gap() {
    let mut sim = open_field();
    wall(&mut sim);
    let unit = spawn(&mut sim, PLAYER, 4, 5);
    let (dest_x, dest_y) = (centre(20), centre(5));
    assert!(sim
        .issue_order_kind(unit, sim::ORDER_ATTACK_MOVE, dest_x, dest_y)
        .is_ok());

    let mut through_gap = false;
    for t in 0..MAX_TICKS {
        drive_targeting(&mut sim, unit);
        sim.tick();
        assert_never_on_blocked_cell(&sim, unit, t);
        let (_, y) = sim.position_fixed(unit);
        through_gap |= cell(y) >= GAP_FROM_Y;
        if distance_milli(&sim, unit, dest_x, dest_y) <= REACH_MILLI {
            assert!(through_gap, "the attack-move unit reached the far side without using the gap");
            return;
        }
    }
    panic!("the attack-move unit did not reach its destination within {MAX_TICKS} ticks");
}

#[test]
fn attack_move_engages_an_enemy_on_its_path_then_resumes() {
    let mut sim = open_field();
    let unit = spawn(&mut sim, PLAYER, 4, 5);
    let foe = spawn(&mut sim, FOE, 9, 5);
    let (dest_x, dest_y) = (centre(20), centre(5));
    assert!(sim
        .issue_order_kind(unit, sim::ORDER_ATTACK_MOVE, dest_x, dest_y)
        .is_ok());

    let mut killed_at: Option<u32> = None;
    for t in 0..MAX_TICKS {
        drive_targeting(&mut sim, unit);
        sim.tick();
        if killed_at.is_none() && !sim.alive(foe) {
            killed_at = Some(t);
            assert!(
                distance_milli(&sim, unit, centre(9), centre(5)) <= 2000.0,
                "the attacker stopped within reach of the enemy it engaged"
            );
        }
        if killed_at.is_some() && distance_milli(&sim, unit, dest_x, dest_y) <= REACH_MILLI {
            return;
        }
    }
    panic!("attack-move did not engage the enemy on its path (killed_at={killed_at:?})");
}

#[test]
fn move_order_during_the_attack_point_cancels_the_strike() {
    let mut sim = open_field();
    let attacker = spawn(&mut sim, PLAYER, 4, 5);
    let target = spawn(&mut sim, FOE, 5, 5);
    let hp_before = sim.hp(target);
    assert_eq!(sim.order_attack_or_approach(attacker, target), Ok(true));
    assert!(sim.hit_in_flight(attacker), "the strike is in its attack point");

    sim.order(attacker, centre(4), centre(20));
    for _ in 0..40 {
        sim.tick();
    }

    assert_eq!(sim.hp(target), hp_before, "a move during the attack point cancels the strike");
    assert!(!sim.hit_in_flight(attacker));
}

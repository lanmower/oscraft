mod common;

use common::*;
use sim::{angle_diff, FACE_TOLERANCE, FULL_TURN};

const WEST: i32 = FULL_TURN / 2;
const TURN_STEP_90_DEG_PER_S: i32 = FULL_TURN / 4 / 16;
const ORDER_PATROL: u8 = 3;

#[test]
fn heading_turns_no_faster_than_the_turn_rate() {
    let mut sim = open_field();
    let unit = spawn(&mut sim, PLAYER, 25, 5);
    assert!(sim.set_turn_rate(unit, 90));
    assert!(sim.set_heading(unit, 0));
    sim.order(unit, centre(2), centre(5));

    let mut previous = sim.heading_of(unit);
    let mut turned = false;
    for _ in 0..40 {
        sim.tick();
        let now = sim.heading_of(unit);
        let step = angle_diff(now, previous).abs();
        assert!(step <= TURN_STEP_90_DEG_PER_S, "heading moved {step} in one tick");
        turned |= step > 0;
        previous = now;
    }
    assert!(turned, "the unit turned toward its direction of travel");
    assert!(
        angle_diff(sim.heading_of(unit), WEST).abs() <= FACE_TOLERANCE,
        "after 40 ticks the unit faces west"
    );
}

#[test]
fn an_ordered_strike_waits_until_the_unit_faces_its_target() {
    let mut sim = open_field();
    let attacker = spawn(&mut sim, PLAYER, 10, 5);
    let target = spawn(&mut sim, FOE, 9, 5);
    assert!(sim.set_turn_rate(attacker, 90));
    assert!(sim.set_heading(attacker, 0));
    let hp_before = sim.hp(target);

    assert_eq!(sim.order_attack_or_approach(attacker, target), Ok(true));
    assert!(!sim.hit_in_flight(attacker), "no strike launches while the unit still faces east");

    let mut struck_at = None;
    for t in 0..MAX_TICKS {
        drive_targeting(&mut sim, attacker);
        sim.tick();
        if sim.hp(target) < hp_before {
            struck_at = Some(t);
            break;
        }
    }
    let t = struck_at.expect("the ordered strike lands once the unit faces its target");
    assert!(t >= 20, "the strike waited for the half turn (landed at tick {t})");
    assert!(angle_diff(sim.heading_of(attacker), WEST).abs() <= FACE_TOLERANCE);
}

#[test]
fn a_move_order_during_the_backswing_moves_the_unit_at_once() {
    let mut sim = open_field();
    let attacker = spawn(&mut sim, PLAYER, 4, 5);
    let target = spawn(&mut sim, FOE, 5, 5);
    let hp_before = sim.hp(target);
    assert_eq!(sim.order_attack_or_approach(attacker, target), Ok(true));
    for _ in 0..40 {
        sim.tick();
        if sim.hp(target) < hp_before {
            break;
        }
    }
    assert!(sim.hp(target) < hp_before, "the strike landed");
    assert!(sim.backswing_left(attacker) > 0, "the unit is in its backswing");

    let before = sim.position_fixed(attacker);
    sim.order(attacker, centre(4), centre(20));
    sim.tick();
    sim.tick();
    assert_ne!(sim.position_fixed(attacker), before, "a move order during the backswing moves the unit at once");
}

#[test]
fn a_patrol_walks_between_its_two_points() {
    let mut sim = open_field();
    let unit = spawn(&mut sim, PLAYER, 4, 5);
    assert!(sim.issue_order_kind(unit, ORDER_PATROL, centre(12), centre(5)).is_ok());

    let (mut far, mut back) = (false, false);
    for _ in 0..MAX_TICKS {
        sim.tick();
        far |= distance_milli(&sim, unit, centre(12), centre(5)) <= 1000.0;
        if far && distance_milli(&sim, unit, centre(4), centre(5)) <= 1000.0 {
            back = true;
            break;
        }
    }
    assert!(far && back, "the patrol reached its far point and came back to its start");
}

#[test]
fn a_hold_position_order_stays_put_and_strikes_only_what_is_in_reach() {
    let mut sim = open_field();
    let holder = spawn(&mut sim, PLAYER, 4, 5);
    let near = spawn(&mut sim, FOE, 5, 5);
    let far = spawn(&mut sim, FOE, 12, 5);
    assert!(sim.issue_order_kind(holder, sim::ORDER_HOLD, 0, 0).is_ok());
    let (near_hp, far_hp) = (sim.hp(near), sim.hp(far));
    let start = sim.position_fixed(holder);

    for _ in 0..60 {
        drive_targeting(&mut sim, holder);
        sim.tick();
    }

    assert!(sim.hp(near) < near_hp, "the holder strikes the foe in reach");
    assert_eq!(sim.hp(far), far_hp, "the holder does not strike out of reach");
    assert_eq!(sim.position_fixed(holder), start, "the holder does not move");
}

#[test]
fn hold_fire_strikes_nothing_even_when_attacked_explicitly() {
    let mut sim = open_field();
    let attacker = spawn(&mut sim, PLAYER, 4, 5);
    let target = spawn(&mut sim, FOE, 5, 5);
    let hp = sim.hp(target);
    assert!(sim.set_hold_fire(attacker, true));

    let _ = sim.order_attack_or_approach(attacker, target);
    for _ in 0..60 {
        drive_targeting(&mut sim, attacker);
        sim.tick();
    }

    assert_eq!(sim.hp(target), hp, "a unit on hold fire strikes nothing");
}

#[test]
fn a_homing_missile_follows_its_target_and_a_plain_one_misses_a_target_that_moved() {
    for (kind, homing) in [("sentinel_creep_ranged", false), ("sentinel_creep_siege", true)] {
        let mut sim = open_field();
        let attacker = spawn_kind(&mut sim, PLAYER, kind, 4, 5);
        let target = spawn(&mut sim, FOE, 11, 5);
        let hp = sim.hp(target);

        assert_eq!(sim.order_attack_or_approach(attacker, target), Ok(true), "{kind} is in reach");
        sim.order(target, centre(11), centre(14));
        for _ in 0..60 {
            sim.tick();
        }

        assert_eq!(sim.hp(target) < hp, homing, "{kind}: the shot hits a target that moved away only when homing");
    }
}

#[test]
fn a_unit_on_a_move_order_strikes_its_target_in_reach_while_it_moves() {
    let mut sim = open_field();
    let mover = spawn(&mut sim, PLAYER, 4, 5);
    let foe = spawn(&mut sim, FOE, 5, 5);
    let foe_hp = sim.hp(foe);
    let start = sim.position_fixed(mover);
    assert!(sim.set_strike_on_move(mover, true));
    sim.order(mover, centre(20), centre(5));

    for _ in 0..60 {
        drive_targeting(&mut sim, mover);
        sim.tick();
    }

    assert!(sim.hp(foe) < foe_hp, "the mover strikes the foe in reach while it walks");
    assert_ne!(sim.position_fixed(mover), start, "the mover keeps walking");
}

#[test]
fn a_unit_on_a_move_order_does_not_strike_without_the_strike_on_move_flag() {
    let mut sim = open_field();
    let mover = spawn(&mut sim, PLAYER, 4, 5);
    let foe = spawn(&mut sim, FOE, 5, 5);
    let foe_hp = sim.hp(foe);
    sim.order(mover, centre(20), centre(5));

    for _ in 0..60 {
        drive_targeting(&mut sim, mover);
        sim.tick();
    }

    assert_eq!(sim.hp(foe), foe_hp, "a plain move order does not strike");
}

const ORDER_MOVE: u8 = 0;

fn windup_survives_a_move_order(strike_on_move: bool) -> bool {
    let mut sim = open_field();
    let attacker = spawn_kind(&mut sim, PLAYER, "antimage", 10, 5);
    let foe = spawn(&mut sim, FOE, 11, 5);
    assert!(sim.set_strike_on_move(attacker, strike_on_move));
    assert_eq!(sim.order_attack_or_approach(attacker, foe), Ok(true));
    let mut in_windup = false;
    for _ in 0..30 {
        drive_targeting(&mut sim, attacker);
        sim.tick();
        if sim.swing_point(attacker) > 0 {
            in_windup = true;
            break;
        }
    }
    assert!(in_windup, "the attacker is in its windup before the move");
    assert!(sim.issue_order_kind(attacker, ORDER_MOVE, centre(9), centre(5)).is_ok());
    sim.swing_point(attacker) > 0
}

#[test]
fn a_move_order_cancels_a_windup_only_when_strike_on_move_is_off() {
    assert!(windup_survives_a_move_order(true), "strike on move keeps the windup through a move order");
    assert!(!windup_survives_a_move_order(false), "a move order cancels the windup when strike on move is off");
}

const ONE_CELL_SQ: i64 = (1 << 16) * (1 << 16);

fn starts_while_walking(sim: &mut sim::Sim, attacker: usize, ticks: u32) -> Vec<u32> {
    let mut previous = sim.swing_point(attacker);
    let mut starts = Vec::new();
    for t in 0..ticks {
        drive_targeting(sim, attacker);
        sim.tick();
        let (kind, tx, ty) = sim.order_of(attacker);
        let (px, py) = sim.position_fixed(attacker);
        let (dx, dy) = (i64::from(px - tx), i64::from(py - ty));
        let walking = kind == ORDER_MOVE && dx * dx + dy * dy > ONE_CELL_SQ;
        let now = sim.swing_point(attacker);
        if walking && previous == 0 && now > 0 {
            starts.push(t);
        }
        previous = now;
    }
    starts
}

#[test]
fn a_walking_unit_with_strike_on_move_off_never_starts_a_swing() {
    let mut sim = open_field();
    let attacker = spawn_kind(&mut sim, PLAYER, "antimage", 10, 5);
    spawn(&mut sim, FOE, 11, 5);
    assert!(sim.set_strike_on_move(attacker, false));
    assert!(sim.set_heading(attacker, 0));
    assert!(sim.set_turn_rate(attacker, 90));
    assert!(sim.issue_order_kind(attacker, ORDER_MOVE, centre(7), centre(5)).is_ok());
    let starts = starts_while_walking(&mut sim, attacker, 120);
    assert!(starts.is_empty(), "a walking unit started a swing at ticks {starts:?} with strike on move off");
}

#[test]
fn a_unit_turning_past_a_foe_on_a_move_order_does_not_start_a_swing_with_strike_on_move_off() {
    let mut sim = open_field();
    let attacker = spawn_kind(&mut sim, PLAYER, "antimage", 10, 5);
    spawn(&mut sim, FOE, 10, 7);
    assert!(sim.set_strike_on_move(attacker, false));
    assert!(sim.set_heading(attacker, 0));
    assert!(sim.set_turn_rate(attacker, 90));
    assert!(sim.issue_order_kind(attacker, ORDER_MOVE, centre(7), centre(5)).is_ok());
    let starts = starts_while_walking(&mut sim, attacker, 120);
    assert!(starts.is_empty(), "a unit turning on a move order started a swing at ticks {starts:?} with strike on move off");
}

#[test]
fn a_walking_unit_turns_to_a_foe_behind_it_and_launches_a_strike_with_strike_on_move_on() {
    let mut sim = open_field();
    let attacker = spawn_kind(&mut sim, PLAYER, "antimage", 10, 5);
    let foe = spawn(&mut sim, FOE, 9, 5);
    assert!(sim.set_strike_on_move(attacker, true));
    assert!(sim.set_heading(attacker, 0));
    assert!(sim.set_turn_rate(attacker, 1337));
    let start = sim.position_fixed(attacker);
    sim.order(attacker, centre(20), centre(5));

    for _ in 0..8 {
        drive_targeting(&mut sim, attacker);
        sim.tick();
    }

    assert!(sim.walk_probe_of(attacker)[0] > 0, "the walking unit turned to the foe behind it and launched a strike");
    assert!(sim.position_fixed(attacker).0 > start.0, "the unit kept walking while it turned");
}

#[test]
fn a_moving_attack_move_unit_with_strike_on_move_on_is_walking_when_it_launches_a_strike() {
    let mut sim = open_field();
    let attacker = spawn_kind(&mut sim, PLAYER, "antimage", 10, 5);
    spawn(&mut sim, FOE, 9, 5);
    assert!(sim.set_strike_on_move(attacker, true));
    assert!(sim.set_heading(attacker, 0));
    assert!(sim.set_turn_rate(attacker, 1337));
    let start = sim.position_fixed(attacker);
    assert!(sim.issue_order_kind(attacker, sim::ORDER_ATTACK_MOVE, centre(20), centre(5)).is_ok());

    for _ in 0..8 {
        drive_targeting(&mut sim, attacker);
        sim.tick();
    }

    assert!(sim.walk_probe_of(attacker)[0] > 0, "the moving attack-move unit launched a strike counted as walking");
    assert!(sim.position_fixed(attacker).0 > start.0, "the unit kept moving under its attack-move order");
}

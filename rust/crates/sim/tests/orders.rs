use std::path::Path;

use sim::fixed::{self, HALF};
use sim::{Grid, Sim};

const PLAYER: u8 = 0;
const FOE: u8 = 1;
const UNIT: &str = "sentinel_creep";
const FIELD_W: usize = 30;
const FIELD_H: usize = 30;
const WALL_X: i32 = 12;
const GAP_FROM_Y: i32 = 21;
const REACH_MILLI: i32 = 500;
const MAX_TICKS: u32 = 3000;

fn scenario() -> Sim {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba");
    let image = osdt_data::generate(&dir).expect("compile the moba scenario");
    let catalog = sim::load(&image, 4096).expect("load the moba scenario").catalog().clone();
    let mut sim = Sim::with_catalog(64, Grid::open(FIELD_W, FIELD_H), catalog);
    sim.set_fog(false);
    assert!(sim.set_team(PLAYER, 0).is_ok());
    assert!(sim.set_team(FOE, 1).is_ok());
    sim
}

fn centre(cell: i32) -> i32 {
    fixed::from_int(cell) + HALF
}

fn spawn(sim: &mut Sim, player: u8, cx: i32, cy: i32) -> usize {
    let kind = sim.catalog().unit_index(UNIT).expect("unit kind in the moba catalog");
    sim.spawn_for(player, kind as u32, centre(cx), centre(cy)).expect("spawn fits the open field")
}

fn wall(sim: &mut Sim) {
    for y in 0..GAP_FROM_Y {
        assert!(sim.set_cell_blocked(WALL_X, y, true).is_ok());
    }
}

fn cell(v: i32) -> i32 {
    fixed::to_int(v)
}

fn distance_milli(sim: &Sim, u: usize, x: i32, y: i32) -> f64 {
    let (ux, uy) = sim.position_fixed(u);
    let dx = fixed::to_float(ux - x);
    let dy = fixed::to_float(uy - y);
    (dx * dx + dy * dy).sqrt() * 1000.0
}

fn assert_never_on_blocked_cell(sim: &Sim, u: usize, tick: u32) {
    let (x, y) = sim.position_fixed(u);
    assert!(
        sim.grid().walkable(cell(x), cell(y)),
        "unit {u} stood on a blocked cell ({}, {}) at tick {tick}",
        cell(x),
        cell(y)
    );
}

#[test]
fn move_order_cancels_an_attack_chase() {
    let mut sim = scenario();
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
        distance_milli(&sim, attacker, centre(4), centre(20)) <= f64::from(REACH_MILLI),
        "the moved unit reaches its move destination"
    );
}

#[test]
fn move_order_routes_around_a_wall_through_the_gap() {
    let mut sim = scenario();
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
        if distance_milli(&sim, unit, dest_x, dest_y) <= f64::from(REACH_MILLI) {
            assert!(through_gap, "the unit reached the far side without using the gap");
            return;
        }
    }
    panic!("the unit did not reach its destination within {MAX_TICKS} ticks");
}

#[test]
fn chase_closes_to_strike_range_around_a_wall() {
    let mut sim = scenario();
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

fn in_reach_foe(sim: &Sim, u: usize) -> i32 {
    let ordered = sim.attack_order_of(u);
    if ordered >= 0 && sim.can_strike(u, ordered as usize) {
        return ordered;
    }
    (0..sim.count())
        .find(|&j| sim.can_strike(u, j))
        .map_or(-1, |j| j as i32)
}

fn drive_targeting(sim: &mut Sim, u: usize) {
    if !sim.alive(u) {
        return;
    }
    let strike = in_reach_foe(sim, u);
    assert!(sim.set_strike_target(u, strike).is_ok());
    if sim.order_of(u).0 == sim::ORDER_ATTACK_MOVE {
        assert!(sim.set_hold_override(u, Some(strike >= 0)).is_ok());
    }
}

#[test]
fn attack_move_routes_around_a_wall_through_the_gap() {
    let mut sim = scenario();
    wall(&mut sim);
    let unit = spawn(&mut sim, PLAYER, 4, 5);
    let (dest_x, dest_y) = (centre(20), centre(5));
    assert!(sim.issue_order_kind(unit, sim::ORDER_ATTACK_MOVE, dest_x, dest_y).is_ok());

    let mut through_gap = false;
    for t in 0..MAX_TICKS {
        drive_targeting(&mut sim, unit);
        sim.tick();
        assert_never_on_blocked_cell(&sim, unit, t);
        let (_, y) = sim.position_fixed(unit);
        through_gap |= cell(y) >= GAP_FROM_Y;
        if distance_milli(&sim, unit, dest_x, dest_y) <= f64::from(REACH_MILLI) {
            assert!(through_gap, "the attack-move unit reached the far side without using the gap");
            return;
        }
    }
    panic!("the attack-move unit did not reach its destination within {MAX_TICKS} ticks");
}

#[test]
fn attack_move_engages_an_enemy_on_its_path_then_resumes() {
    let mut sim = scenario();
    let unit = spawn(&mut sim, PLAYER, 4, 5);
    let foe = spawn(&mut sim, FOE, 9, 5);
    let (dest_x, dest_y) = (centre(20), centre(5));
    assert!(sim.issue_order_kind(unit, sim::ORDER_ATTACK_MOVE, dest_x, dest_y).is_ok());

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
        if killed_at.is_some() && distance_milli(&sim, unit, dest_x, dest_y) <= f64::from(REACH_MILLI) {
            return;
        }
    }
    panic!("attack-move did not engage the enemy on its path (killed_at={killed_at:?})");
}

#[test]
fn chase_keeps_up_with_a_moving_target_around_a_wall() {
    let mut sim = scenario();
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

mod common;

use common::*;

const ORDER_MOVE: u8 = 0;
const ORDER_STOP: u8 = 1;
const ORDER_FOLLOW: u8 = 5;
const STATION_MILLI: f64 = 2000.0;
const STATION_SLACK_MILLI: f64 = 500.0;

fn gap_milli(sim: &sim::Sim, follower: usize, leader: usize) -> f64 {
    let (lx, ly) = sim.position_fixed(leader);
    distance_milli(sim, follower, lx, ly)
}

#[test]
fn a_follower_keeps_station_on_a_leader_that_moves_away_and_back() {
    let mut sim = open_field();
    let follower = spawn(&mut sim, PLAYER, 4, 5);
    let leader = spawn(&mut sim, PLAYER, 8, 5);
    assert!(sim.issue_follow(follower, leader).is_ok());
    assert!(sim.order_of(follower).0 == ORDER_FOLLOW, "the follow order is recorded");
    sim.order(leader, centre(20), centre(5));

    let mut worst_settled = 0.0_f64;
    for t in 0..600 {
        if t == 300 {
            sim.order(leader, centre(6), centre(5));
        }
        sim.tick();
        if t >= 480 {
            worst_settled = worst_settled.max(gap_milli(&sim, follower, leader));
        }
    }

    assert!(
        worst_settled <= STATION_MILLI + STATION_SLACK_MILLI,
        "the follower stays within station of the leader (worst gap {worst_settled} milli)"
    );
    assert_eq!(sim.order_of(follower).0, ORDER_FOLLOW, "the follow order persists");
}

#[test]
fn a_follower_routes_around_a_wall_to_reach_its_leader() {
    let mut sim = open_field();
    wall(&mut sim);
    let follower = spawn(&mut sim, PLAYER, 4, 5);
    let leader = spawn(&mut sim, PLAYER, 20, 5);
    assert!(sim.issue_follow(follower, leader).is_ok());

    let mut reached = None;
    for t in 0..MAX_TICKS {
        sim.tick();
        assert_never_on_blocked_cell(&sim, follower, t);
        if gap_milli(&sim, follower, leader) <= STATION_MILLI {
            reached = Some(t);
            break;
        }
    }
    assert!(reached.is_some(), "the follower walks through the gap to within station of its leader");
}

#[test]
fn a_follower_stops_when_its_leader_dies() {
    let mut sim = open_field();
    let follower = spawn(&mut sim, PLAYER, 4, 5);
    let leader = spawn(&mut sim, PLAYER, 8, 5);
    assert!(sim.issue_follow(follower, leader).is_ok());
    for _ in 0..10 {
        sim.tick();
    }
    assert!(sim.remove_unit(leader));
    sim.tick();

    assert_eq!(sim.order_of(follower).0, ORDER_STOP, "a follower whose leader died stops");
}

#[test]
fn a_move_order_replaces_a_follow_order() {
    let mut sim = open_field();
    let follower = spawn(&mut sim, PLAYER, 4, 5);
    let leader = spawn(&mut sim, PLAYER, 8, 5);
    assert!(sim.issue_follow(follower, leader).is_ok());

    sim.order(follower, centre(2), centre(2));

    assert_eq!(sim.order_of(follower).0, ORDER_MOVE, "a move order ends the follow");
}

#[test]
fn a_unit_cannot_follow_itself_or_a_dead_leader() {
    let mut sim = open_field();
    let follower = spawn(&mut sim, PLAYER, 4, 5);
    let leader = spawn(&mut sim, PLAYER, 8, 5);

    assert!(sim.issue_follow(follower, follower).is_err(), "a unit refuses to follow itself");
    assert!(sim.remove_unit(leader));
    assert!(sim.issue_follow(follower, leader).is_err(), "a dead leader is refused");
}

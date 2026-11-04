mod common;

use common::*;
use sim::{Sim, ORDER_ATTACK_MOVE};

const ORDER_MOVE: u8 = 0;
const QUEUED_ATTACK: u8 = 0xFF;
const RESYNC_CHECK_TICKS: u32 = 2000;

fn issued_orders(sim: &mut Sim, unit: usize, ticks: u32) -> Vec<(u8, i32, i32)> {
    let mut issued: Vec<(u8, i32, i32)> = Vec::new();
    for _ in 0..ticks {
        drive_targeting(sim, unit);
        sim.tick();
        let order = sim.order_of(unit);
        if issued.last() != Some(&order) {
            issued.push(order);
        }
    }
    issued
}

#[test]
fn queued_orders_survive_a_keyframe_resync() {
    let mut original = open_field();
    let unit = spawn(&mut original, PLAYER, 4, 5);
    let foe = spawn(&mut original, FOE, 24, 5);
    assert!(original.queue_order(unit, ORDER_MOVE, centre(10), centre(5), -1).is_ok());
    assert!(original.queue_order(unit, ORDER_ATTACK_MOVE, centre(16), centre(5), -1).is_ok());
    assert!(original.queue_order(unit, QUEUED_ATTACK, 0, 0, foe as i32).is_ok());
    original.tick();
    let queued_at_save = original.queued_orders(unit);
    assert_eq!(queued_at_save.len(), 2, "the first queued move is issued on the first tick");

    let keyframe = original.save_state();
    let mut resynced = open_field();
    resynced.load_state(&keyframe).expect("the keyframe loads into a fresh sim");

    let queued_after_load = resynced.queued_orders(unit);
    assert_eq!(queued_after_load, queued_at_save, "the resynced sim keeps every queued order");
    assert_eq!(resynced.state_hash(), original.state_hash(), "the resynced sim has the same state hash");

    let from_original = issued_orders(&mut original, unit, RESYNC_CHECK_TICKS);
    let from_resynced = issued_orders(&mut resynced, unit, RESYNC_CHECK_TICKS);
    assert_eq!(from_resynced, from_original, "the resynced sim issues the same order sequence");
    assert_eq!(original.queued_orders(unit).len(), 0, "the original drained its queue");
    assert_eq!(resynced.queued_orders(unit).len(), 0, "the resynced sim drained its queue");
    assert_eq!(resynced.hp(foe), original.hp(foe), "the queued attack lands the same on both sims");
    assert!(
        !original.alive(foe) || original.hp(foe) < original.hp_max(foe),
        "the queued attack reached the foe"
    );

    eprintln!(
        "order queue resync: queued {:?} at save, {:?} after load, state hash equal, original issued {:?}, resynced issued {:?}, foe hp {} on both",
        queued_at_save,
        queued_after_load,
        from_original,
        from_resynced,
        original.hp(foe)
    );
}

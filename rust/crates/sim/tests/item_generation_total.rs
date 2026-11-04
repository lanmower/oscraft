mod common;

use common::*;

const BLINK: &str = "blink";

#[test]
fn an_item_edit_advances_the_total_and_the_unit_generation() {
    let mut sim = open_field();
    let unit = spawn(&mut sim, PLAYER, 5, 5);
    let blink = sim.catalog().item_index(BLINK).expect("blink in the moba catalog");
    let total_before = sim.item_generation_total();
    let unit_before = sim.item_generation(unit);
    assert!(sim.grant_item(unit, blink, 1), "the unit takes the item");
    assert!(sim.item_generation_total() > total_before, "the total moves with an item edit");
    assert!(sim.item_generation(unit) > unit_before, "the unit generation moves with its item edit");
}

#[test]
fn a_tick_with_no_item_edit_leaves_the_total_unchanged() {
    let mut sim = open_field();
    spawn(&mut sim, PLAYER, 5, 5);
    spawn(&mut sim, FOE, 8, 5);
    let total_before = sim.item_generation_total();
    for _ in 0..30 {
        sim.tick();
    }
    assert_eq!(sim.item_generation_total(), total_before, "no item edit, no total change");
}

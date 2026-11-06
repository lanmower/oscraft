mod common;
use common::*;

const BLINK: &str = "blink";

#[test]
fn a_full_plane_save_load_round_trip_preserves_the_state_hash() {
    let mut sim = open_field();
    let unit = spawn(&mut sim, PLAYER, 5, 5);
    spawn(&mut sim, FOE, 8, 5);
    let blink = sim.catalog().item_index(BLINK).expect("blink in the moba catalog");
    assert!(sim.grant_item(unit, blink, 1), "the unit takes an item");
    wall(&mut sim);
    for _ in 0..24 {
        sim.tick();
    }
    let bytes = sim.save_state();
    assert_eq!(&bytes[..4], b"OSSV", "state magic");
    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    let mut restored = open_field();
    restored.load_state(&bytes).expect("a saved state loads into a fresh sim");
    assert_eq!(restored.state_hash(), sim.state_hash(), "every saved plane is restored");
    assert_eq!(restored.ticks(), sim.ticks(), "the tick counter is restored");
    let mut foreign = bytes.clone();
    foreign[4] = foreign[4].wrapping_add(1);
    assert!(
        matches!(restored.load_state(&foreign), Err(sim::StateError::BadVersion(_))),
        "a foreign STATE_VERSION is refused"
    );
    assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), version, "the version bytes are stable");
}

use std::path::{Path, PathBuf};

use replay::duel;

fn pack() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/duel-routes")
}

fn committed(name: &str) -> String {
    let path = pack().join("replays").join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn image() -> Vec<u8> {
    osdt_data::generate(&pack()).expect("compile the duel-routes pack")
}

fn recorded() -> (Vec<u8>, duel::Duel) {
    let image = image();
    let duel = duel::record(&image).expect("record the duel-routes duel");
    (image, duel)
}

#[test]
fn the_recorded_duel_replays_with_every_tick_hash() {
    let (image, duel) = recorded();
    duel::verify(&image, &duel.replay, &duel.hashes).expect("the replay reproduces every per-tick hash");
}

#[test]
fn the_committed_replay_and_hashes_are_the_generator_output() {
    let (_, duel) = recorded();
    assert_eq!(committed("duel-routes.replay"), duel.replay, "the committed replay matches the generator");
    assert_eq!(committed("duel-routes.hashes"), duel.hash_table(), "the committed hash table matches the generator");
}

#[test]
fn a_wrong_expected_hash_is_refused() {
    let (image, duel) = recorded();
    let mut hashes = duel.hashes.clone();
    let middle = hashes.len() / 2;
    hashes[middle].1 ^= 1;
    assert!(duel::verify(&image, &duel.replay, &hashes).is_err(), "a changed per-tick hash fails the replay");
}

//! Determinism gate checks on the scenario scenario: a scripted ten-hero run hashes the same twice,
//! a mid-run save and load rejoins the uninterrupted run, and a fixed 1000-tick run prints its
//! hash so CI can compare it across operating systems.

use crate::checks::{fresh, MAX_UNITS};
use sim::{fixed, Sim};

const SCRIPT_TICKS: u32 = 600;
const SAVE_TICK: u32 = 300;
const PARITY_TICKS: u32 = 1000;
const SCRIPT_SEED: u64 = 11;

fn scenario_image() -> Result<Vec<u8>, String> {
    let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("scenarios").join("moba");
    osdt_data::generate(&scenario).map_err(|e| format!("scenario scenario image: {e:#}"))
}

fn attributed_units(s: &Sim) -> Vec<usize> {
    (0..MAX_UNITS).filter(|&i| s.alive(i) && s.kind_name(i) == Some("hero")).collect()
}

/// The fixed command list: every hero gets a move at tick 0 and 200, and a cast on tick 100 and 400
/// at the first hero of the other side. Commands depend only on the tick and the hero order.
fn apply_commands(s: &mut Sim, tick: u32, heroes: &[usize]) {
    for (n, &h) in heroes.iter().enumerate() {
        let n = n as i32;
        match tick {
            0 => s.order(h, fixed::from_int(8 + n), fixed::from_int(8 + 2 * n)),
            200 => s.order(h, fixed::from_int(20 - n), fixed::from_int(12 + n)),
            100 | 400 => {
                let foe = heroes.iter().copied().find(|&o| s.player(o) != s.player(h));
                if let Some(t) = foe {
                    s.cast_ability(h, (tick as usize / 100) % 2, t as i32, 0, 0);
                }
            }
            _ => {}
        }
    }
}

fn scripted(image: &[u8]) -> Result<Sim, String> {
    let mut s = fresh(image)?;
    s.seed(SCRIPT_SEED);
    Ok(s)
}

fn drive(s: &mut Sim, from: u32, to: u32, hs: &[usize]) {
    for t in from..to {
        apply_commands(s, t, hs);
        s.tick();
    }
}

fn run_hash(image: &[u8], ticks: u32) -> Result<u64, String> {
    let mut s = scripted(image)?;
    let hs = attributed_units(&s);
    if hs.len() < 10 {
        return Err(format!("scenario scenario has {} heroes, want at least 10", hs.len()));
    }
    drive(&mut s, 0, ticks, &hs);
    Ok(s.state_hash())
}

/// Two independent runs of the same ten-hero command list end with one state hash.
pub fn scenario_replay_hash(_image: &[u8]) -> Result<(), String> {
    let image = scenario_image()?;
    let (a, b) = (run_hash(&image, SCRIPT_TICKS)?, run_hash(&image, SCRIPT_TICKS)?);
    if a != b {
        return Err(format!("first={a:016x} second={b:016x} after {SCRIPT_TICKS} ticks"));
    }
    println!("scenario replay hash ok ticks={SCRIPT_TICKS} hash={a:016x}");
    Ok(())
}

/// Saving at SAVE_TICK, loading into a fresh sim and continuing matches the uninterrupted run.
pub fn scenario_save_load_hash(_image: &[u8]) -> Result<(), String> {
    let image = scenario_image()?;
    let mut straight = scripted(&image)?;
    let hs = attributed_units(&straight);
    if hs.len() < 10 {
        return Err(format!("scenario scenario has {} heroes, want at least 10", hs.len()));
    }
    drive(&mut straight, 0, SAVE_TICK, &hs);
    let bytes = straight.save_state();
    let mut resumed = fresh(&image)?;
    resumed.load_state(&bytes).map_err(|e| format!("load_state: {e:?}"))?;
    if resumed.state_hash() != straight.state_hash() {
        return Err(format!("hash {:016x} after load, saved at {:016x}", resumed.state_hash(), straight.state_hash()));
    }
    drive(&mut straight, SAVE_TICK, SCRIPT_TICKS, &hs);
    drive(&mut resumed, SAVE_TICK, SCRIPT_TICKS, &hs);
    let (a, b) = (straight.state_hash(), resumed.state_hash());
    if a != b {
        return Err(format!("uninterrupted={a:016x} resumed={b:016x} at tick {SCRIPT_TICKS}"));
    }
    println!("scenario save-load hash ok save_tick={SAVE_TICK} ticks={SCRIPT_TICKS} hash={a:016x}");
    Ok(())
}

/// Prints the state hash of the fixed 1000-tick run as `determinism-parity hash=...` for the CI
/// job that compares gate output across operating systems.
pub fn scenario_parity_hash(_image: &[u8]) -> Result<(), String> {
    let hash = run_hash(&scenario_image()?, PARITY_TICKS)?;
    println!("determinism-parity ticks={PARITY_TICKS} hash={hash:016x}");
    Ok(())
}

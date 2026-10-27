use crate::checks::{fresh, kind, role};
use sim::{fixed, Sim};

const SCRIPT_TICKS: u32 = 600;
const SAVE_TICK: u32 = 300;
const PARITY_TICKS: u32 = 1000;
const SCRIPT_SEED: u64 = 11;

fn scenario_image() -> Result<Vec<u8>, String> {
    let scenario = crate::checks::pack_dir();
    osdt_data::generate(&scenario).map_err(|e| format!("moba scenario image: {e:#}"))
}

fn seat_heroes(s: &mut Sim) -> Result<Vec<usize>, String> {
    let seats: Vec<(u8, i32, i32)> = s.catalog().starts.iter().map(|st| (st.player, st.x, st.y)).collect();
    let mut heroes = Vec::new();
    for (n, (player, x, y)) in seats.into_iter().enumerate() {
        let name = role(&format!("hero.{}", char::from(b'a' + n as u8)))?;
        let k = kind(s, name)?;
        let hero = s.spawn_attributed(player, k, x, y).ok_or_else(|| format!("spawn_attributed refused {name} for player {player} at its start"))?;
        heroes.push(hero);
    }
    Ok(heroes)
}

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
    let hs = seat_heroes(&mut s)?;
    if hs.len() < 10 {
        return Err(format!("moba scenario starts {} seats, want at least 10 heroes", hs.len()));
    }
    drive(&mut s, 0, ticks, &hs);
    Ok(s.state_hash())
}

pub fn scenario_replay_hash(_image: &[u8]) -> Result<(), String> {
    let image = scenario_image()?;
    let (a, b) = (run_hash(&image, SCRIPT_TICKS)?, run_hash(&image, SCRIPT_TICKS)?);
    if a != b {
        return Err(format!("first={a:016x} second={b:016x} after {SCRIPT_TICKS} ticks"));
    }
    println!("moba replay hash ok ticks={SCRIPT_TICKS} hash={a:016x}");
    Ok(())
}

pub fn scenario_save_load_hash(_image: &[u8]) -> Result<(), String> {
    let image = scenario_image()?;
    let mut straight = scripted(&image)?;
    let hs = seat_heroes(&mut straight)?;
    if hs.len() < 10 {
        return Err(format!("moba scenario starts {} seats, want at least 10 heroes", hs.len()));
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
    println!("moba save-load hash ok save_tick={SAVE_TICK} ticks={SCRIPT_TICKS} hash={a:016x}");
    Ok(())
}

pub fn scenario_parity_hash(_image: &[u8]) -> Result<(), String> {
    let hash = run_hash(&scenario_image()?, PARITY_TICKS)?;
    println!("determinism-parity ticks={PARITY_TICKS} hash={hash:016x}");
    Ok(())
}

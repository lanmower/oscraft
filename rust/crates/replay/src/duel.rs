use std::path::Path;

use sim::{fixed, Sim};

use crate::{Command, Recording, Replayer, CHECK_EVERY};

pub const IMAGE: &str = "duel-routes.osdt";
pub const MAX_UNITS: usize = 4096;
pub const TICKS: u32 = 1280;
const _: () = assert!(TICKS % CHECK_EVERY == 0);
const MOVES: [(u8, i32, i32); 2] = [(0, 36000, 12000), (1, 4000, 12000)];

pub struct Duel {
    pub replay: String,
    pub hashes: Vec<(u32, u64)>,
}

impl Duel {
    pub fn hash_table(&self) -> String {
        self.hashes.iter().map(|(tick, hash)| format!("{tick} {hash:016x}\n")).collect()
    }
}

fn load(pack: &Path) -> Result<Sim, String> {
    let image = osdt_data::generate(pack).map_err(|e| format!("compile {}: {e:?}", pack.display()))?;
    sim::load(&image, MAX_UNITS).map_err(|e| format!("load {}: {e:?}", pack.display()))
}

pub fn record(pack: &Path) -> Result<Duel, String> {
    let mut sim = load(pack)?;
    let mut rec = Recording::new(IMAGE, MAX_UNITS, &sim);
    for (player, x, y) in MOVES {
        rec.apply(&mut sim, Command::OrderAll { player, x: fixed::from_milli(x), y: fixed::from_milli(y) });
    }
    let mut hashes = vec![(sim.ticks(), sim.state_hash())];
    for _ in 0..TICKS {
        rec.tick(&mut sim, true, true);
        hashes.push((sim.ticks(), sim.state_hash()));
    }
    Ok(Duel { replay: rec.render(&sim), hashes })
}

pub fn verify(pack: &Path, replay: &str, hashes: &[(u32, u64)]) -> Result<(), String> {
    let mut sim = load(pack)?;
    let rec = Recording::parse(replay).map_err(|e| format!("parse: {e:?}"))?;
    let mut player = Replayer::new(rec, &sim).map_err(|e| format!("start: {e:?}"))?;
    for &(tick, expected) in hashes.iter().skip(1) {
        player.step(&mut sim).map_err(|e| format!("tick {tick}: {e:?}"))?;
        let actual = sim.state_hash();
        if sim.ticks() != tick || actual != expected {
            return Err(format!("tick {tick}: expected {expected:016x}, replay gave {actual:016x} at tick {}", sim.ticks()));
        }
    }
    Ok(())
}

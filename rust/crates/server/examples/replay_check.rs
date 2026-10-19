// Plays a replay file back from a fresh load of its initial data and checks every recorded state
// hash. Then seeks to the middle, restarts from a fresh load and plays to the end, and requires the
// same final hash both ways. A data override ending in .bin is a map image loaded with sim::load. Prints `replay ok hash=<h> ticks=<n>` and exits 0 on success, 1 on divergence, 2 on a
// bad argument.
//
// Usage: cargo run -p server --example replay_check -- <replay file> [data override]

use std::path::Path;

use server::replay::{Recording, Replayer};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(file) = args.get(1) else {
        eprintln!("usage: replay_check <replay file> [data override]");
        std::process::exit(2);
    };
    let text = std::fs::read_to_string(file).unwrap_or_else(|e| {
        eprintln!("{file}: {e}");
        std::process::exit(2);
    });
    let rec = Recording::parse(&text).unwrap_or_else(|e| {
        eprintln!("{file}: {e:?}");
        std::process::exit(2);
    });
    let image = args.get(2).cloned().unwrap_or_else(|| rec.image.clone());
    let max_units = rec.max_units as usize;
    let gold = rec.gold_by_mod;
    let load = || -> sim::Sim {
        let loaded = if image.ends_with(".bin") {
            std::fs::read(&image).map_err(|e| e.to_string()).and_then(|b| sim::load(&b, max_units).map_err(|e| format!("{e:?}")))
        } else {
            server::mods::load_data(Path::new(&image), max_units)
        };
        loaded
            .map(|mut s| {
                s.set_gold_by_mod(gold);
                s
            })
            .unwrap_or_else(|e| {
                eprintln!("load {image}: {e}");
                std::process::exit(2);
            })
    };
    let mut sim = load();
    if let Some(t) = rec.incomplete() {
        eprintln!("warning: the recording is incomplete from tick {t}; playback may diverge");
    }
    let mut play = Replayer::new(rec, &sim).unwrap_or_else(|e| {
        eprintln!("{e:?}");
        std::process::exit(1);
    });
    let end = play.end();
    let run = |play: &mut Replayer, sim: &mut sim::Sim| -> Result<u64, server::replay::ReplayError> {
        *sim = load();
        play.reset(sim)?;
        play.seek(sim, end / 2)?;
        *sim = load();
        play.reset(sim)?;
        play.run_to_end(sim)
    };
    match play.run_to_end(&mut sim) {
        Ok(h) => match run(&mut play, &mut sim) {
            Ok(again) if again == h && play.end_hash().is_none_or(|e| e == h) => println!("replay ok hash={h:016x} ticks={end}"),
            other => {
                eprintln!("seek replay diverged: {other:?} first={h:016x}");
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("replay diverged: {e:?}");
            std::process::exit(1);
        }
    }
}

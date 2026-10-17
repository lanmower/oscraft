//! Offline mod check. Builds a scenario image, loads one mod into it, runs ticks, and prints one
//! witness line per fact. Usage: `mod_check <scenario-dir> <mod.lua> [ticks]`. Exit 0 when the mod
//! loads and every tick runs; exit 1 with a `mod-check` line naming the failure otherwise.

use std::path::Path;
use std::process::ExitCode;

use mod_host::{build_image, ModHost};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if !(3..=4).contains(&args.len()) {
        eprintln!("usage: mod_check <scenario-dir> <mod.lua> [ticks]");
        return ExitCode::from(2);
    }
    let ticks = match args.get(3).map(|t| t.parse::<u32>()) {
        None => 100,
        Some(Ok(n)) => n,
        Some(Err(_)) => {
            eprintln!("ticks must be a whole number");
            return ExitCode::from(2);
        }
    };
    let image = match build_image(Path::new(&args[1])) {
        Ok(image) => image,
        Err(e) => {
            println!("mod-check image-error {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut host = match ModHost::new(&image) {
        Ok(host) => host,
        Err(e) => {
            println!("mod-check host-error {e:?}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(e) = host.load(Path::new(&args[2])) {
        println!("mod-check load-error {e}");
        return ExitCode::FAILURE;
    }
    let state = host.state().clone();
    for tick in 1..=ticks {
        if let Err(e) = host.advance(&state) {
            println!("mod-check tick-error tick={tick} {e}");
            return ExitCode::FAILURE;
        }
    }
    let units = host.unit_count();
    let sim = host.sim();
    println!(
        "mod-check ok ticks={ticks} units={units} deaths={} casts={} casts_refused={} last_refusal={:?}",
        sim.stats.deaths,
        sim.stats.casts,
        sim.stats.casts_refused,
        sim.stats.last_refusal
    );
    ExitCode::SUCCESS
}

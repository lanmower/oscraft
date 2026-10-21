//! Offline mod check. Builds a scenario image, loads one mod into it, runs ticks, and prints one
//! witness line per fact. Usage: `mod_check <scenario-dir> <mod.lua> [ticks]`. Exit 0 when the mod
//! loads and every tick runs; exit 1 with a `mod-check` line naming the failure otherwise.

use std::path::Path;
use std::process::ExitCode;

use mod_host::{build_image, ModHost};
use sim::ALL_REFUSALS;

fn main() -> ExitCode {
    // `--fog off` turns fog of war off for the run, so a command aimed at a unit the sides cannot see is not refused.
    let mut fog = true;
    let mut args: Vec<String> = Vec::new();
    let mut raw = std::env::args();
    while let Some(arg) = raw.next() {
        if arg == "--fog" {
            match raw.next().as_deref() {
                Some("on") => fog = true,
                Some("off") => fog = false,
                _ => {
                    eprintln!("--fog takes on or off");
                    return ExitCode::from(2);
                }
            }
        } else {
            args.push(arg);
        }
    }
    if !(3..=4).contains(&args.len()) {
        eprintln!("usage: mod_check [--fog on|off] <scenario-dir> <mod.lua> [ticks]");
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
    host.sim_mut().set_fog(fog);
    if let Err(e) = host.load_file(Path::new(&args[2])) {
        println!("mod-check load-error {e}");
        return ExitCode::FAILURE;
    }
    for tick in 1..=ticks {
        if let Err(e) = host.advance() {
            println!("mod-check tick-error tick={tick} {e}");
            return ExitCode::FAILURE;
        }
    }
    let units = host.units(None).len();
    // The scalar fields a mod left in `state`, so a CI step can assert what the mod did.
    let state_line = host.state_line();
    let sim = host.sim();
    // Refusals by reason, most frequent first, so a mod that is refused shows which rule refuses it.
    let mut by_reason: Vec<(&str, usize)> = ALL_REFUSALS
        .iter()
        .zip(sim.stats.refused_by.iter())
        .filter(|(_, &n)| n > 0)
        .map(|(r, &n)| (r.name(), n))
        .collect();
    by_reason.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    let reasons = if by_reason.is_empty() {
        "none".to_string()
    } else {
        by_reason.iter().map(|(name, n)| format!("{name}:{n}")).collect::<Vec<_>>().join(",")
    };
    println!(
        "mod-check ok ticks={ticks} units={units} deaths={} casts={} casts_refused={} last_refusal={:?} refusals={reasons}",
        sim.stats.deaths,
        sim.stats.casts,
        sim.stats.casts_refused,
        sim.stats.last_refusal
    );
    println!("{state_line}");
    ExitCode::SUCCESS
}

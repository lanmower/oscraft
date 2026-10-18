//! The sim as a WASIp3 guest. Stdin: a u32 little-endian image length, the OSDT image bytes, then
//! one command per line. Stdout: one reply line per command.
//! Command names are the WIT exports of rust/crates/wasip3/wit/sim-component.wit. COMMANDS lists
//! them once; ci.yml's host-api job fails when this list and the WIT exports differ.

use std::io::{self, BufRead, Write};

use mod_host::ModHost;
use sim::Sim;

pub const MAX_UNITS: usize = 1024;

// ahash seeds piccolo's hashers through getrandom 0.3, which has no wasip3 backend (the cfg in
// .cargo/config.toml selects this custom one). Hash seeds need no entropy: the guest is one
// deterministic run, so a fixed-seed xorshift stream is enough.
#[cfg(getrandom_backend = "custom")]
#[no_mangle]
unsafe extern "Rust" fn __getrandom_v03_custom(dest: *mut u8, len: usize) -> Result<(), getrandom::Error> {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    for i in 0..len {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        dest.add(i).write(state as u8);
    }
    Ok(())
}

const TEST_MOD_NAME: &str = "wasip3-test";
const TEST_MOD: &str = "-- a test mod loaded inside the wasip3 guest\nlocal loaded = true\nfunction on_tick(state) end\n";

pub const COMMANDS: &[&str] = &[
    "load",
    "mod-load",
    "mod-version",
    "tick",
    "ticks",
    "unit-count",
    "alive-count",
    "state-hash",
    "unit-position",
    "unit-player",
    "unit-kind",
    "unit-hp",
    "order",
    "order-all",
    "fire-trigger",
    "catalog-unit-name",
    "mod-source",
    "mod-kill",
    "mod-step",
    "mod-state",
];

pub fn run<R: BufRead, W: Write>(mut input: R, mut out: W) -> io::Result<()> {
    let mut len = [0u8; 4];
    input.read_exact(&mut len)?;
    let mut image = vec![0u8; u32::from_le_bytes(len) as usize];
    input.read_exact(&mut image)?;
    let mut host = match ModHost::new(&image) {
        Ok(host) => host,
        Err(e) => {
            writeln!(out, "err load {e:?}")?;
            return Ok(());
        }
    };
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let reply = reply(&mut host, line.trim()).unwrap_or_else(|e| format!("err {e}"));
        writeln!(out, "{reply}")?;
    }
    Ok(())
}

fn reply(host: &mut ModHost, line: &str) -> Result<String, String> {
    let mut words = line.split_whitespace();
    let name = words.next().ok_or("empty command")?;
    if !COMMANDS.contains(&name) {
        return Err(format!("unknown command {name}"));
    }
    match name {
        "mod-load" => match host.load_source(TEST_MOD_NAME, TEST_MOD) {
            Ok(()) => Ok(format!("ok mod-load {TEST_MOD_NAME}")),
            Err(e) => Err(format!("mod-load {e}")),
        },
        "mod-version" => Ok(format!("ok mod-version {}", host.version())),
        "mod-source" => {
            let which = words.next().ok_or("mod name missing")?;
            let src = unhex(words.next().ok_or("mod source missing")?)?;
            host.load_source(which, &src).map_err(|e| format!("mod-source {which}: {e}"))?;
            Ok(format!("ok mod-source {which}"))
        }
        "mod-kill" => {
            let kind = words.next().ok_or("unit kind missing")?;
            let unit = kill_first(host, kind)?;
            Ok(format!("ok mod-kill {unit}"))
        }
        "mod-step" => {
            let n: u32 = match words.next() {
                Some(word) => parse(Some(word), "step count")?,
                None => 1,
            };
            for _ in 0..n {
                host.advance()?;
            }
            let (fired, applied) = host.counters();
            Ok(format!("ok mod-step fired={fired} applied={applied}"))
        }
        "mod-state" => Ok(format!("ok {}", host.state_line())),
        _ => sim_reply(&mut host.sim(), name, words),
    }
}

// Kills the first living unit of `kind` with a hit from a living unit of another player, so the
// death reaches the mod at the next mod-step like any in-game kill.
fn kill_first(host: &mut ModHost, kind: &str) -> Result<usize, String> {
    let mut sim = host.sim();
    let target = (0..sim.count()).find(|&i| sim.alive(i) && sim.kind_name(i) == Some(kind)).ok_or_else(|| format!("no living {kind}"))?;
    let owner = sim.player(target);
    let caster = (0..sim.count()).find(|&i| sim.alive(i) && sim.player(i) != owner).ok_or("no opposing unit to strike with")?;
    let player = sim.player(caster);
    sim.damage_target(caster as i32, player, i32::MAX / 2, target as i32);
    Ok(target)
}

fn sim_reply<'a>(sim: &mut Sim, name: &str, mut words: impl Iterator<Item = &'a str>) -> Result<String, String> {
    match name {
        "load" => Err("load is stdin-only: the image arrives before the first command".into()),
        "mod-version" => Ok("ok mod-version 0".into()),
        "tick" => {
            let n: u32 = match words.next() {
                Some(word) => parse(Some(word), "tick count")?,
                None => 1,
            };
            for _ in 0..n {
                sim.tick();
            }
            Ok(format!("ok tick {}", sim.ticks()))
        }
        "ticks" => Ok(format!("ok ticks {}", sim.ticks())),
        "unit-count" => Ok(format!("ok unit-count {}", sim.count())),
        "alive-count" => Ok(format!("ok alive-count {}", sim.alive_count())),
        "state-hash" => Ok(format!("ok state-hash {:016x}", sim.state_hash())),
        "unit-position" => {
            let unit = unit_of(sim, &mut words)?;
            let (x, y) = sim.position_fixed(unit);
            Ok(format!("ok unit-position {x} {y}"))
        }
        "unit-kind" => {
            let unit = unit_of(sim, &mut words)?;
            Ok(format!("ok unit-kind {}", sim.unit_type(unit)))
        }
        "unit-player" => {
            let unit = unit_of(sim, &mut words)?;
            Ok(format!("ok unit-player {}", sim.player(unit)))
        }
        "unit-hp" => {
            let unit = unit_of(sim, &mut words)?;
            Ok(format!("ok unit-hp {}", sim.hp(unit)))
        }
        "order" => {
            let unit = unit_of(sim, &mut words)?;
            let x: i32 = parse(words.next(), "x")?;
            let y: i32 = parse(words.next(), "y")?;
            sim.order(unit, x, y);
            Ok("ok order".into())
        }
        "order-all" => {
            let player: u8 = parse(words.next(), "player")?;
            let x: i32 = parse(words.next(), "x")?;
            let y: i32 = parse(words.next(), "y")?;
            sim.order_all(player, x, y);
            Ok("ok order-all".into())
        }
        "fire-trigger" => {
            let unit = unit_of(sim, &mut words)?;
            let trigger = words.next().ok_or("trigger name missing")?;
            Ok(format!("ok fire-trigger {}", sim.fire_trigger(unit, trigger)))
        }
        "catalog-unit-name" => {
            let kind: u32 = parse(words.next(), "kind")?;
            let catalog = sim.catalog();
            match catalog.units.get(kind as usize) {
                Some(unit) => Ok(format!("ok catalog-unit-name {}", catalog.name(unit.name))),
                None => Ok("ok catalog-unit-name none".into()),
            }
        }
        other => Err(format!("command {other} has no handler")),
    }
}

fn unit_of<'a>(sim: &Sim, words: &mut impl Iterator<Item = &'a str>) -> Result<usize, String> {
    let unit: usize = parse(words.next(), "unit")?;
    if unit >= sim.count() {
        return Err(format!("unit {unit} out of range"));
    }
    Ok(unit)
}

// A mod source travels as lowercase hex so it fits one whitespace-free command line.
fn unhex(hex: &str) -> Result<String, String> {
    if hex.len() % 2 != 0 {
        return Err("mod source hex has odd length".into());
    }
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| format!("mod source hex: {e}")))
        .collect::<Result<Vec<u8>, String>>()?;
    String::from_utf8(bytes).map_err(|e| format!("mod source utf8: {e}"))
}

fn parse<T: std::str::FromStr>(word: Option<&str>, what: &str) -> Result<T, String>
where
    T::Err: std::fmt::Display,
{
    word.ok_or_else(|| format!("{what} missing"))?
        .parse()
        .map_err(|e| format!("{what}: {e}"))
}

//! The sim as a WASIp3 guest. Stdin: a u32 little-endian image length, the OSDT image bytes, then
//! one command per line. Stdout: one reply line per command.
//! Command names are the WIT exports of rust/crates/wasip3/wit/sim-component.wit. COMMANDS lists
//! them once; ci.yml's host-api job fails when this list and the WIT exports differ.

use std::io::{self, BufRead, Read, Write};

use mod_host::ModHost;
use sim::Sim;

pub const MAX_UNITS: usize = 1024;

const TEST_MOD_NAME: &str = "wasip3-test";
const TEST_MOD: &str = "-- a test mod loaded inside the wasip3 guest\nlocal loaded = true\n";

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
];

pub fn run<R: BufRead, W: Write>(mut input: R, mut out: W) -> io::Result<()> {
    let mut len = [0u8; 4];
    input.read_exact(&mut len)?;
    let mut image = vec![0u8; u32::from_le_bytes(len) as usize];
    input.read_exact(&mut image)?;
    let mut sim = match sim::load(&image, MAX_UNITS) {
        Ok(sim) => sim,
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
        let reply = match line.trim() {
            "mod-load" => mod_load(&image),
            command => reply(&mut sim, command).unwrap_or_else(|e| format!("err {e}")),
        };
        writeln!(out, "{reply}")?;
    }
    Ok(())
}

fn mod_load(image: &[u8]) -> String {
    let mut host = match ModHost::new(image) {
        Ok(host) => host,
        Err(e) => return format!("err mod-load host {e:?}"),
    };
    match host.load_source(TEST_MOD_NAME, TEST_MOD) {
        Ok(()) => format!("ok mod-load {TEST_MOD_NAME}"),
        Err(e) => format!("err mod-load {e}"),
    }
}

fn reply(sim: &mut Sim, line: &str) -> Result<String, String> {
    let mut words = line.split_whitespace();
    let name = match words.next() {
        Some(name) => name,
        None => return Err("empty command".into()),
    };
    if !COMMANDS.contains(&name) {
        return Err(format!("unknown command {name}"));
    }
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

fn parse<T: std::str::FromStr>(word: Option<&str>, what: &str) -> Result<T, String>
where
    T::Err: std::fmt::Display,
{
    word.ok_or_else(|| format!("{what} missing"))?
        .parse()
        .map_err(|e| format!("{what}: {e}"))
}

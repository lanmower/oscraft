//! The sim as a wasip1 guest. Stdin: a u32 little-endian image length, the OSDT image bytes, then
//! one command per line. Stdout: one reply line per command.
//! Commands: `tick <n>`, `state`, `order <unit> move <x> <y>`.

use std::io::{self, BufRead, Read, Write};

use sim::Sim;

pub const MAX_UNITS: usize = 1024;

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
        let reply = reply(&mut sim, line.trim()).unwrap_or_else(|e| format!("err {e}"));
        writeln!(out, "{reply}")?;
    }
    Ok(())
}

fn reply(sim: &mut Sim, line: &str) -> Result<String, String> {
    let mut words = line.split_whitespace();
    match words.next() {
        Some("tick") => {
            let n: u32 = parse(words.next(), "tick count")?;
            for _ in 0..n {
                sim.tick();
            }
            Ok(format!("ok tick {}", sim.ticks()))
        }
        Some("state") => Ok(format!(
            "state ticks={} units={} alive={} hash={:016x}",
            sim.ticks(),
            sim.count(),
            sim.alive_count(),
            sim.state_hash()
        )),
        Some("order") => {
            let unit: usize = parse(words.next(), "unit")?;
            if words.next() != Some("move") {
                return Err("order kind must be move".into());
            }
            let x: i32 = parse(words.next(), "x")?;
            let y: i32 = parse(words.next(), "y")?;
            if unit >= sim.count() {
                return Err(format!("unit {unit} out of range"));
            }
            sim.order(unit, x, y);
            Ok("ok order".into())
        }
        Some(other) => Err(format!("unknown command {other}")),
        None => Err("empty command".into()),
    }
}

fn parse<T: std::str::FromStr>(word: Option<&str>, what: &str) -> Result<T, String>
where
    T::Err: std::fmt::Display,
{
    word.ok_or_else(|| format!("{what} missing"))?
        .parse()
        .map_err(|e| format!("{what}: {e}"))
}

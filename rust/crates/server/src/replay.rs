use std::io::Write;
use std::str::{FromStr, SplitWhitespace};
use std::time::Duration;

use sim::{Refusal, Sim, StateError};

const TICKS_PER_SECOND: u32 = 16;

pub const SNAP_EVERY: u32 = 64;
pub const MAX_SPEED: u32 = 64;

#[derive(Debug, PartialEq)]
pub enum ReplayError {
    LogIncomplete { dropped: u64 },
    MissingCheckpoint { tick: u32 },
    Ahead { tick: u32, at: u32 },
    OrderRefused { tick: u32, unit: i32, refusal: Refusal },
    HashMismatch { tick: u32, expected: u64, actual: u64 },
    TraceGap { tick: u32 },
    TraceLength { want: u32, got: u32 },
    RecordBeyondTrace { tick: u32 },
    BadSpeed { speed: u32 },
    OutOfTrace { tick: u32, base: u32, end: u32 },
    Restore { tick: u32, cause: StateError },
    Parse { line: usize, why: String },
    Init { expected: u64, actual: u64 },
    NotAtStart { tick: u32 },
}

type Record = (u32, i32, u8, i32, i32);

fn order_records(source: &Sim) -> Result<Vec<Record>, ReplayError> {
    let dropped = source.order_log_dropped();
    if dropped > 0 {
        return Err(ReplayError::LogIncomplete { dropped });
    }
    Ok((0..source.order_log_len())
        .filter_map(|k| source.order_record(k))
        .collect())
}

fn apply(sim: &mut Sim, record: Record) -> Result<(), ReplayError> {
    let (tick, unit, kind, x, y) = record;
    sim.issue_order_kind(unit as usize, kind, x, y)
        .map_err(|refusal| ReplayError::OrderRefused { tick, unit, refusal })
}

/// Applies the orders recorded at the sim's current tick, takes the state hash, then steps the
/// sim one tick. The hash is taken after the orders and before the step.
fn step_tick(sim: &mut Sim, records: &[Record]) -> Result<u64, ReplayError> {
    let t = sim.ticks();
    let mut k = records.partition_point(|r| r.0 < t);
    while k < records.len() && records[k].0 == t {
        apply(sim, records[k])?;
        k += 1;
    }
    let hash = sim.state_hash();
    sim.tick();
    Ok(hash)
}

/// Replays the accepted-order log of `source` into `fresh`, which must start at or before the
/// first recorded tick and hold the same map, catalog and units as the live run.
///
/// `checkpoints` holds `(tick, state_hash)` for every tick that has a recorded order, sorted by
/// tick. Each hash is taken after that tick's orders are applied and before the tick steps.
/// Returns the tick `fresh` reached.
pub fn play(source: &Sim, fresh: &mut Sim, checkpoints: &[(u32, u64)]) -> Result<u32, ReplayError> {
    let records = order_records(source)?;
    let mut k = 0;
    while k < records.len() {
        let tick = records[k].0;
        let expected = checkpoints
            .binary_search_by_key(&tick, |c| c.0)
            .map(|i| checkpoints[i].1)
            .map_err(|_| ReplayError::MissingCheckpoint { tick })?;
        if fresh.ticks() > tick {
            return Err(ReplayError::Ahead {
                tick,
                at: fresh.ticks(),
            });
        }
        while fresh.ticks() < tick {
            fresh.tick();
        }
        while k < records.len() && records[k].0 == tick {
            apply(fresh, records[k])?;
            k += 1;
        }
        let actual = fresh.state_hash();
        if actual != expected {
            return Err(ReplayError::HashMismatch {
                tick,
                expected,
                actual,
            });
        }
    }
    Ok(fresh.ticks())
}

/// Plays the order log back at a chosen speed, paused or running, and seeks to any tick up to
/// the source's end. Seeking restores the nearest snapshot at or before the target and re-runs
/// the orders from there; every stepped tick is checked against its state hash.
pub struct Playback {
    sim: Sim,
    base: u32,
    hashes: Vec<u64>,
    records: Vec<Record>,
    snapshots: Vec<(u32, Vec<u8>)>,
    speed: u32,
    paused: bool,
    carry: f64,
}

impl Playback {
    /// Covers every tick from `fresh` up to the source's current tick. With `trace` empty, the
    /// reference hashes come from one replay pass from `fresh` (a determinism check of the
    /// replay itself); otherwise `trace` holds `(tick, state_hash)` for each of those ticks,
    /// contiguous and sorted, as the live run recorded them.
    pub fn new(source: &Sim, mut fresh: Sim, trace: &[(u32, u64)]) -> Result<Playback, ReplayError> {
        let base = fresh.ticks();
        let end = source.ticks();
        if base > end {
            return Err(ReplayError::Ahead { tick: end, at: base });
        }
        let records = order_records(source)?;
        if let Some(first) = records.first() {
            if first.0 < base {
                return Err(ReplayError::Ahead {
                    tick: first.0,
                    at: base,
                });
            }
        }
        if let Some(last) = records.last() {
            if last.0 >= end {
                return Err(ReplayError::RecordBeyondTrace { tick: last.0 });
            }
        }
        let start = fresh.save_state();
        let hashes = if trace.is_empty() {
            let mut hashes = Vec::with_capacity((end - base) as usize);
            while fresh.ticks() < end {
                hashes.push(step_tick(&mut fresh, &records)?);
            }
            fresh
                .load_state(&start)
                .map_err(|cause| ReplayError::Restore { tick: base, cause })?;
            hashes
        } else {
            for (i, &(tick, _)) in trace.iter().enumerate() {
                let want = base + i as u32;
                if tick != want {
                    return Err(ReplayError::TraceGap { tick: want });
                }
            }
            if base + trace.len() as u32 != end {
                return Err(ReplayError::TraceLength {
                    want: end - base,
                    got: trace.len() as u32,
                });
            }
            trace.iter().map(|&(_, h)| h).collect()
        };
        Ok(Playback {
            sim: fresh,
            base,
            hashes,
            records,
            snapshots: vec![(base, start)],
            speed: 1,
            paused: false,
            carry: 0.0,
        })
    }

    pub fn tick(&self) -> u32 {
        self.sim.ticks()
    }

    pub fn end(&self) -> u32 {
        self.base + self.hashes.len() as u32
    }

    pub fn at_end(&self) -> bool {
        self.tick() >= self.end()
    }

    pub fn hash(&self) -> u64 {
        self.sim.state_hash()
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Speed is a multiple of real time: 1 runs at the room's tick rate.
    pub fn set_speed(&mut self, speed: u32) -> Result<(), ReplayError> {
        if speed == 0 || speed > MAX_SPEED {
            return Err(ReplayError::BadSpeed { speed });
        }
        self.speed = speed;
        Ok(())
    }

    /// Steps by the ticks `elapsed` real time is worth at the current speed. Does nothing
    /// while paused or at the end of the trace.
    pub fn advance(&mut self, elapsed: Duration) -> Result<u32, ReplayError> {
        if self.paused || self.at_end() {
            return Ok(self.tick());
        }
        self.carry +=
            elapsed.as_secs_f64() * f64::from(TICKS_PER_SECOND) * f64::from(self.speed);
        let due = self.carry.floor() as u64;
        self.carry -= due as f64;
        let mut stepped = 0;
        while stepped < due && !self.at_end() {
            self.step()?;
            stepped += 1;
        }
        Ok(self.tick())
    }

    pub fn seek(&mut self, tick: u32) -> Result<u32, ReplayError> {
        if tick < self.base || tick > self.end() {
            return Err(ReplayError::OutOfTrace {
                tick,
                base: self.base,
                end: self.end(),
            });
        }
        if tick < self.sim.ticks() {
            let i = self.snapshots.partition_point(|s| s.0 <= tick) - 1;
            let at = self.snapshots[i].0;
            self.sim
                .load_state(&self.snapshots[i].1)
                .map_err(|cause| ReplayError::Restore { tick: at, cause })?;
            self.snapshots.truncate(i + 1);
        }
        while self.sim.ticks() < tick {
            self.step()?;
        }
        Ok(self.tick())
    }

    fn step(&mut self) -> Result<(), ReplayError> {
        let t = self.sim.ticks();
        let last_snapshot = self.snapshots[self.snapshots.len() - 1].0;
        if t - last_snapshot >= SNAP_EVERY {
            self.snapshots.push((t, self.sim.save_state()));
        }
        let expected = self.hashes[(t - self.base) as usize];
        let actual = step_tick(&mut self.sim, &self.records)?;
        if actual != expected {
            return Err(ReplayError::HashMismatch {
                tick: t,
                expected,
                actual,
            });
        }
        Ok(())
    }
}

pub const FORMAT_HEADER: &str = "oscraft-replay 1";
pub const CHECK_EVERY: u32 = 64;
pub const MAX_KEPT: usize = 4_000_000;

/// One input the sim accepts between ticks. A replay is the initial data image plus these, each
/// tagged with the sim tick it was applied at, in order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    Order { unit: usize, x: i32, y: i32 },
    OrderAll { player: u8, x: i32, y: i32 },
    Attack { unit: usize, target: usize },
    Cast { unit: usize, slot: usize, target: i32, x: i32, y: i32 },
    SkillUp { unit: usize, slot: usize },
    UseItem { unit: usize, slot: usize, target: i32, x: i32, y: i32 },
    Buy { unit: usize, item: usize },
    Sell { unit: usize, slot: usize },
    Unstash { unit: usize, slot: usize },
    Fog { on: bool },
    Cadence { paths: bool, vision: bool },
}

fn num<T: FromStr>(it: &mut SplitWhitespace) -> Option<T> {
    it.next()?.parse().ok()
}

impl Command {
    /// Applies the command; false when the sim refused it. A refusal leaves unit state alone but
    /// bumps the refusal counters, which the state hash covers.
    pub fn apply(&self, sim: &mut Sim) -> bool {
        match *self {
            Command::Order { unit, x, y } => {
                sim.order(unit, x, y);
                true
            }
            Command::OrderAll { player, x, y } => {
                sim.order_all(player, x, y);
                true
            }
            Command::Attack { unit, target } => sim.order_attack_or_approach(unit, target).is_ok(),
            Command::Cast { unit, slot, target, x, y } => sim.cast_ability(unit, slot, target, x, y),
            Command::SkillUp { unit, slot } => sim.skill_up(unit, slot).is_ok(),
            Command::UseItem { unit, slot, target, x, y } => sim.use_item(unit, slot, target, x, y),
            Command::Buy { unit, item } => sim.buy_item(unit, item),
            Command::Sell { unit, slot } => sim.sell_item(unit, slot),
            Command::Unstash { unit, slot } => sim.unstash(unit, slot),
            Command::Fog { on } => {
                sim.set_fog(on);
                true
            }
            Command::Cadence { .. } => true,
        }
    }

    fn encode(&self) -> String {
        match *self {
            Command::Order { unit, x, y } => format!("o {unit} {x} {y}"),
            Command::OrderAll { player, x, y } => format!("oa {player} {x} {y}"),
            Command::Attack { unit, target } => format!("a {unit} {target}"),
            Command::Cast { unit, slot, target, x, y } => format!("c {unit} {slot} {target} {x} {y}"),
            Command::SkillUp { unit, slot } => format!("s {unit} {slot}"),
            Command::UseItem { unit, slot, target, x, y } => format!("u {unit} {slot} {target} {x} {y}"),
            Command::Buy { unit, item } => format!("b {unit} {item}"),
            Command::Sell { unit, slot } => format!("l {unit} {slot}"),
            Command::Unstash { unit, slot } => format!("t {unit} {slot}"),
            Command::Fog { on } => format!("f {}", u8::from(on)),
            Command::Cadence { paths, vision } => format!("k {} {}", u8::from(paths), u8::from(vision)),
        }
    }

    fn decode(code: &str, it: &mut SplitWhitespace) -> Option<Command> {
        Some(match code {
            "o" => Command::Order { unit: num(it)?, x: num(it)?, y: num(it)? },
            "oa" => Command::OrderAll { player: num(it)?, x: num(it)?, y: num(it)? },
            "a" => Command::Attack { unit: num(it)?, target: num(it)? },
            "c" => Command::Cast { unit: num(it)?, slot: num(it)?, target: num(it)?, x: num(it)?, y: num(it)? },
            "s" => Command::SkillUp { unit: num(it)?, slot: num(it)? },
            "u" => Command::UseItem { unit: num(it)?, slot: num(it)?, target: num(it)?, x: num(it)?, y: num(it)? },
            "b" => Command::Buy { unit: num(it)?, item: num(it)? },
            "l" => Command::Sell { unit: num(it)?, slot: num(it)? },
            "t" => Command::Unstash { unit: num(it)?, slot: num(it)? },
            "f" => Command::Fog { on: num::<u8>(it)? != 0 },
            "k" => Command::Cadence { paths: num::<u8>(it)? != 0, vision: num::<u8>(it)? != 0 },
            _ => return None,
        })
    }
}

/// The commands a sim run received, per tick, with the state hash every `CHECK_EVERY` ticks.
/// Text format: a header (`oscraft-replay 1`, `image`, `max_units`, `init`), then one line per
/// command `<tick> <code> <args>` and `chk <ticks> <hash>` lines. A `chk` line follows the tick
/// that brought the sim to `<ticks>` and precedes the commands applied at that tick.
pub struct Recording {
    pub image: String,
    pub max_units: u32,
    pub init: u64,
    cmds: Vec<(u32, Command)>,
    checks: Vec<(u32, u64)>,
    incomplete: Option<u32>,
    sink: Option<Box<dyn Write + Send>>,
}

impl Recording {
    /// `image` names the initial data (a scenario directory, a catalog image or a map file) the
    /// playback loads; `sim` is that data freshly loaded, at tick 0.
    pub fn new(image: &str, max_units: usize, sim: &Sim) -> Recording {
        Recording {
            image: image.to_string(),
            max_units: max_units as u32,
            init: sim.state_hash(),
            cmds: Vec::new(),
            checks: Vec::new(),
            incomplete: None,
            sink: None,
        }
    }

    /// Streams the recording to `sink` from now on, after writing what it holds so far.
    pub fn set_sink(&mut self, mut sink: Box<dyn Write + Send>) {
        let held = self.render_to(None);
        if sink.write_all(held.as_bytes()).and_then(|_| sink.flush()).is_ok() {
            self.sink = Some(sink);
        }
    }

    fn write_line(&mut self, line: &str, flush: bool) {
        if let Some(w) = self.sink.as_mut() {
            let ok = w.write_all(line.as_bytes()).and_then(|_| w.write_all(b"\n")).and_then(|_| if flush { w.flush() } else { Ok(()) });
            if ok.is_err() {
                self.sink = None;
            }
        }
    }

    fn note(&mut self, tick: u32, cmd: Command) {
        if self.cmds.len() >= MAX_KEPT {
            self.mark_incomplete(tick);
        } else {
            self.cmds.push((tick, cmd));
        }
        self.write_line(&format!("{tick} {}", cmd.encode()), false);
    }

    /// Applies `cmd` to `sim` and records it at the sim's tick, refused or not: a refusal bumps the
    /// refusal counters the state hash covers, so playback must attempt it too.
    pub fn apply(&mut self, sim: &mut Sim, cmd: Command) -> bool {
        let accepted = cmd.apply(sim);
        self.note(sim.ticks(), cmd);
        accepted
    }

    /// Records `cmd` at the sim's tick for a caller that already applied it to `sim` itself.
    pub fn record_applied(&mut self, sim: &Sim, cmd: Command) {
        self.note(sim.ticks(), cmd);
    }

    /// Steps `sim` one tick at the given cadence, recording a non-default cadence, and records a
    /// state hash every `CHECK_EVERY` ticks.
    pub fn tick(&mut self, sim: &mut Sim, paths: bool, vision: bool) {
        if !(paths && vision) {
            self.note(sim.ticks(), Command::Cadence { paths, vision });
        }
        sim.tick_with_cadence(paths, vision);
        if sim.ticks() % CHECK_EVERY == 0 {
            self.check(sim);
        }
    }

    /// Records the state hash at the sim's current tick unless one is already held for it.
    pub fn check(&mut self, sim: &Sim) {
        let t = sim.ticks();
        if self.checks.last().is_some_and(|c| c.0 == t) {
            return;
        }
        let h = sim.state_hash();
        self.checks.push((t, h));
        self.write_line(&format!("chk {t} {h:016x}"), true);
    }

    /// Notes that something outside the command set changed the sim at `tick` (a data reload or a
    /// mod command with no replay form), so a playback of this recording may diverge.
    pub fn mark_incomplete(&mut self, tick: u32) {
        if self.incomplete.is_none() {
            self.incomplete = Some(tick);
            self.write_line(&format!("incomplete {tick}"), true);
        }
    }

    pub fn incomplete(&self) -> Option<u32> {
        self.incomplete
    }

    pub fn command_count(&self) -> usize {
        self.cmds.len()
    }

    /// The recording as text, ending with a check at the sim's current tick.
    pub fn render(&self, sim: &Sim) -> String {
        self.render_to(Some((sim.ticks(), sim.state_hash())))
    }

    fn render_to(&self, last: Option<(u32, u64)>) -> String {
        let mut out = format!("{FORMAT_HEADER}\nimage {}\nmax_units {}\ninit {:016x}\n", self.image, self.max_units, self.init);
        if let Some(t) = self.incomplete {
            out.push_str(&format!("incomplete {t}\n"));
        }
        let mut checks = self.checks.clone();
        if let Some((t, h)) = last {
            if checks.last().map(|c| c.0) != Some(t) {
                checks.push((t, h));
            }
        }
        let mut k = 0;
        for (ct, h) in checks {
            while k < self.cmds.len() && self.cmds[k].0 < ct {
                out.push_str(&format!("{} {}\n", self.cmds[k].0, self.cmds[k].1.encode()));
                k += 1;
            }
            out.push_str(&format!("chk {ct} {h:016x}\n"));
        }
        for (t, c) in &self.cmds[k..] {
            out.push_str(&format!("{t} {}\n", c.encode()));
        }
        out
    }

    pub fn parse(text: &str) -> Result<Recording, ReplayError> {
        let bad = |line: usize, why: &str| ReplayError::Parse { line, why: why.to_string() };
        let mut lines = text.lines().enumerate().map(|(i, l)| (i + 1, l.trim_end())).filter(|(_, l)| !l.is_empty());
        match lines.next() {
            Some((_, FORMAT_HEADER)) => {}
            Some((n, _)) => return Err(bad(n, "not an oscraft replay (first line must be `oscraft-replay 1`)")),
            None => return Err(bad(0, "empty replay")),
        }
        let mut rec = Recording {
            image: String::new(),
            max_units: 0,
            init: 0,
            cmds: Vec::new(),
            checks: Vec::new(),
            incomplete: None,
            sink: None,
        };
        let mut have_init = false;
        for (n, line) in lines {
            if let Some(rest) = line.strip_prefix("image ") {
                rec.image = rest.to_string();
                continue;
            }
            let mut it = line.split_whitespace();
            let first = it.next().unwrap_or_default();
            match first {
                "max_units" => rec.max_units = num(&mut it).ok_or_else(|| bad(n, "max_units needs a number"))?,
                "init" => {
                    rec.init = it.next().and_then(|h| u64::from_str_radix(h, 16).ok()).ok_or_else(|| bad(n, "init needs a hex hash"))?;
                    have_init = true;
                }
                "incomplete" => rec.incomplete = Some(num(&mut it).ok_or_else(|| bad(n, "incomplete needs a tick"))?),
                "chk" => {
                    let t: u32 = num(&mut it).ok_or_else(|| bad(n, "chk needs a tick"))?;
                    let h = it.next().and_then(|h| u64::from_str_radix(h, 16).ok()).ok_or_else(|| bad(n, "chk needs a hex hash"))?;
                    if rec.checks.last().is_some_and(|c| c.0 >= t) {
                        return Err(bad(n, "chk ticks must increase"));
                    }
                    if rec.cmds.last().is_some_and(|c| c.0 >= t) {
                        return Err(bad(n, "a chk must come before the commands of its tick"));
                    }
                    rec.checks.push((t, h));
                }
                tick => {
                    let t: u32 = tick.parse().map_err(|_| bad(n, "unknown line"))?;
                    let code = it.next().ok_or_else(|| bad(n, "command code missing"))?;
                    let cmd = Command::decode(code, &mut it).ok_or_else(|| bad(n, "bad command"))?;
                    if rec.cmds.last().is_some_and(|c| c.0 > t) || rec.checks.last().is_some_and(|c| c.0 > t) {
                        return Err(bad(n, "commands must not go back in time"));
                    }
                    rec.cmds.push((t, cmd));
                }
            }
        }
        if !have_init || rec.image.is_empty() {
            return Err(bad(0, "header lacks image or init"));
        }
        Ok(rec)
    }
}

/// Drives a parsed recording against a freshly loaded sim, one tick at a time, checking every
/// recorded hash. The sim only moves forward: to go back, load a fresh sim, call `reset`, and
/// seek forward again (a saved state does not carry every transient field, so it is not used).
pub struct Replayer {
    rec: Recording,
    next: usize,
    nchk: usize,
    cad: (bool, bool),
}

impl Replayer {
    pub fn new(rec: Recording, sim: &Sim) -> Result<Replayer, ReplayError> {
        let p = Replayer { rec, next: 0, nchk: 0, cad: (true, true) };
        p.check_start(sim)?;
        Ok(p)
    }

    fn check_start(&self, sim: &Sim) -> Result<(), ReplayError> {
        if sim.ticks() != 0 {
            return Err(ReplayError::NotAtStart { tick: sim.ticks() });
        }
        let actual = sim.state_hash();
        if actual != self.rec.init {
            return Err(ReplayError::Init { expected: self.rec.init, actual });
        }
        Ok(())
    }

    /// Starts over against `fresh`, a newly loaded sim of the same data.
    pub fn reset(&mut self, fresh: &Sim) -> Result<(), ReplayError> {
        self.check_start(fresh)?;
        self.next = 0;
        self.nchk = 0;
        self.cad = (true, true);
        Ok(())
    }

    pub fn recording(&self) -> &Recording {
        &self.rec
    }

    /// The last tick the recording holds a hash for.
    pub fn end(&self) -> u32 {
        self.rec.checks.last().map_or(0, |c| c.0)
    }

    /// Hash recorded for the end tick, if any.
    pub fn end_hash(&self) -> Option<u64> {
        self.rec.checks.last().map(|c| c.1)
    }

    pub fn step(&mut self, sim: &mut Sim) -> Result<(), ReplayError> {
        let t = sim.ticks();
        if t >= self.end() {
            return Err(ReplayError::OutOfTrace { tick: t + 1, base: 0, end: self.end() });
        }
        while self.next < self.rec.cmds.len() && self.rec.cmds[self.next].0 <= t {
            let cmd = self.rec.cmds[self.next].1;
            self.next += 1;
            match cmd {
                Command::Cadence { paths, vision } => self.cad = (paths, vision),
                _ => {
                    cmd.apply(sim);
                }
            }
        }
        let (paths, vision) = std::mem::replace(&mut self.cad, (true, true));
        sim.tick_with_cadence(paths, vision);
        let now = sim.ticks();
        while self.nchk < self.rec.checks.len() && self.rec.checks[self.nchk].0 <= now {
            let (ct, expected) = self.rec.checks[self.nchk];
            self.nchk += 1;
            if ct == now {
                let actual = sim.state_hash();
                if actual != expected {
                    return Err(ReplayError::HashMismatch { tick: ct, expected, actual });
                }
            }
        }
        Ok(())
    }

    /// Steps forward to `tick`; a tick behind the sim is refused (see `reset`).
    pub fn seek(&mut self, sim: &mut Sim, tick: u32) -> Result<u32, ReplayError> {
        if tick > self.end() || tick < sim.ticks() {
            return Err(ReplayError::OutOfTrace { tick, base: sim.ticks(), end: self.end() });
        }
        while sim.ticks() < tick {
            self.step(sim)?;
        }
        Ok(sim.ticks())
    }

    /// Plays from the current tick to the end of the recording and returns the final hash.
    pub fn run_to_end(&mut self, sim: &mut Sim) -> Result<u64, ReplayError> {
        while sim.ticks() < self.end() {
            self.step(sim)?;
        }
        Ok(sim.state_hash())
    }
}

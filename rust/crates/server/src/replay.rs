use std::time::Duration;

use sim::{Refusal, Sim, StateError};

use crate::room::TICKS_PER_SECOND;

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

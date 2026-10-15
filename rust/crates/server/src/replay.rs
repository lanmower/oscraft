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

/// Plays the order log back at a chosen speed, paused or running, and seeks to any tick in the
/// trace. Seeking restores the nearest snapshot at or before the target and re-runs the orders
/// from there; every tick stepped is checked against its traced state hash.
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
    /// `fresh` must sit at the first traced tick. `trace` holds `(tick, state_hash)` for every
    /// tick from there to the end, contiguous and sorted; each hash is taken after that tick's
    /// orders are applied and before the tick steps.
    pub fn new(source: &Sim, fresh: Sim, trace: &[(u32, u64)]) -> Result<Playback, ReplayError> {
        let base = fresh.ticks();
        let mut hashes = Vec::with_capacity(trace.len());
        for (i, &(tick, hash)) in trace.iter().enumerate() {
            let want = base + i as u32;
            if tick != want {
                return Err(ReplayError::TraceGap { tick: want });
            }
            hashes.push(hash);
        }
        let end = base + hashes.len() as u32;
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
        let snapshots = vec![(base, fresh.save_state())];
        Ok(Playback {
            sim: fresh,
            base,
            hashes,
            records,
            snapshots,
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
        let mut k = self.records.partition_point(|r| r.0 < t);
        while k < self.records.len() && self.records[k].0 == t {
            apply(&mut self.sim, self.records[k])?;
            k += 1;
        }
        let expected = self.hashes[(t - self.base) as usize];
        let actual = self.sim.state_hash();
        if actual != expected {
            return Err(ReplayError::HashMismatch {
                tick: t,
                expected,
                actual,
            });
        }
        self.sim.tick();
        Ok(())
    }
}

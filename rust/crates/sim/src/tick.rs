//! One simulation tick. Order: buckets, fog cadence, flow fields, buffs, projectiles,
//! then per unit upkeep, economy, routing, steering, integration and attack.
//!
//! Each top-level system's wall-clock cost is recorded into a fixed histogram of
//! atomics, one row per `System`. Recording is observational only: it never feeds
//! back into sim state, and it allocates nothing.

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

use crate::arrays::Sim;
use crate::*;

/// Systems timed inside `Sim::tick`, in call order. Names say what each one does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum System {
    Rebucket,
    Vision,
    Paths,
    Buffs,
    Healers,
    Projectiles,
    Units,
}

pub const SYSTEM_COUNT: usize = 7;
/// Width of one cost bucket in microseconds. The last bucket also holds every larger cost.
pub const COST_BUCKET_US: u64 = 2;
pub const COST_BUCKETS: usize = 2048;

const ZERO: AtomicU32 = AtomicU32::new(0);
const ROW: [AtomicU32; COST_BUCKETS] = [ZERO; COST_BUCKETS];
static COSTS: [[AtomicU32; COST_BUCKETS]; SYSTEM_COUNT] = [ROW; SYSTEM_COUNT];

fn record(system: System, cost_us: u64) {
    let bucket = usize::try_from(cost_us / COST_BUCKET_US)
        .unwrap_or(COST_BUCKETS - 1)
        .min(COST_BUCKETS - 1);
    COSTS[system as usize][bucket].fetch_add(1, Ordering::Relaxed);
}

/// Records the time since `since` against `system` and returns the instant it was recorded at.
fn lap(system: System, since: Instant) -> Instant {
    let now = Instant::now();
    record(system, u64::try_from(now.duration_since(since).as_micros()).unwrap_or(u64::MAX));
    now
}

impl Sim {
    pub const TICK_SYSTEMS: [&'static str; SYSTEM_COUNT] = [
        "rebucket",
        "vision",
        "paths",
        "buffs",
        "healers",
        "projectiles",
        "units",
    ];
    pub const TICK_COST_BUCKET_US: u64 = COST_BUCKET_US;
    pub const TICK_COST_BUCKETS: usize = COST_BUCKETS;

    /// Samples recorded for `system` (index into `TICK_SYSTEMS`) in cost bucket `bucket`,
    /// counted over every tick this process has run. Returns 0 for out-of-range indices.
    pub fn tick_cost_bucket(system: usize, bucket: usize) -> u32 {
        match COSTS.get(system).and_then(|row| row.get(bucket)) {
            Some(n) => n.load(Ordering::Relaxed),
            None => 0,
        }
    }

    pub fn tick(&mut self) {
        let mut mark = Instant::now();
        self.rebucket();
        mark = lap(System::Rebucket, mark);
        self.scan_left = RETARGET_BUDGET;
        if self.fog && self.ticks % VIS_PERIOD == 0 {
            self.refresh_vision();
        }
        mark = lap(System::Vision, mark);
        self.paths.tick(&self.grid);
        mark = lap(System::Paths, mark);
        self.tick_buffs();
        mark = lap(System::Buffs, mark);
        self.tick_healers();
        mark = lap(System::Healers, mark);
        self.tick_projectiles();
        mark = lap(System::Projectiles, mark);
        for i in 0..self.n {
            if self.alive[i] == 0 {
                continue;
            }
            let before = (self.pos_x[i], self.pos_y[i], self.tgt_x[i], self.tgt_y[i]);
            if self.abil_on {
                self.tick_cooldowns(i);
            }
            self.fire_triggers(i);
            if self.alive[i] == 0 {
                continue;
            }
            self.regen_shield(i);
            self.regen_health(i);
            if self.channel_left[i] > 0 {
                self.tick_channel(i);
                continue;
            }
            self.research_step(i);
            self.burrow_step(i);
            if self.harvest && self.kind[i] == KIND_WORKER {
                self.harvest_step(i);
            }
            if self.produce && self.kind[i] == KIND_PRODUCER {
                self.produce_step(i);
            }
            if self.order_kind[i] == ORDER_HOLD {
                self.vel_x[i] = 0;
                self.vel_y[i] = 0;
            } else {
                self.route_for(i);
                self.steer(i);
                self.integrate(i);
                self.advance_patrol(i);
            }
            if (self.pos_x[i], self.pos_y[i], self.tgt_x[i], self.tgt_y[i]) != before {
                self.touch(i);
            }
            self.note_idle(i);
            self.attack(i);
        }
        lap(System::Units, mark);
        self.ticks = self.ticks.wrapping_add(1);
    }

    /// Counts down every ability cooldown on unit `i`.
    pub(crate) fn tick_cooldowns(&mut self, i: usize) {
        for slot in 0..ABILITY_SLOTS {
            let idx = i * ABILITY_SLOTS + slot;
            if self.abil_cd[idx] > 0 {
                self.abil_cd[idx] -= 1;
                if self.abil_cd[idx] == 0 {
                    self.trig_ready[idx] = 1;
                }
            }
        }
    }
}

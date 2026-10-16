use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::log::{line, Ev};
use crate::room::Room;

const BUCKET_US: u64 = 1000;
const BUCKETS: usize = 64;
const REPORT_EVERY_TICKS: u64 = 80;
const POLL: Duration = Duration::from_millis(250);

/// Consecutive overrunning ticks after which the room sheds deferrable work on the next tick.
pub const SHED_AFTER_OVERRUNS: u64 = 3;
/// Queued submissions drained per tick while shedding; the rest wait until the room recovers.
pub const SHED_ORDER_CAP: usize = 0;
/// Queued submissions drained per tick when the room is not shedding.
pub const NORMAL_ORDER_CAP: usize = usize::MAX;
/// While shedding, a shed line is printed on the first shedding tick and then every this many ticks.
pub const SHED_LOG_EVERY_TICKS: u64 = 80;

#[derive(Clone)]
pub struct TickHistogram {
    counts: [u64; BUCKETS],
    total: u64,
    max_us: u64,
}

pub struct Timing {
    pub tick: u64,
    pub hist: TickHistogram,
    pub queued: u64,
    pub overruns: u64,
}

impl TickHistogram {
    pub fn new() -> Self {
        TickHistogram {
            counts: [0; BUCKETS],
            total: 0,
            max_us: 0,
        }
    }

    pub fn record(&mut self, cost: Duration) {
        let us = u64::try_from(cost.as_micros()).unwrap_or(u64::MAX);
        let bucket = usize::try_from(us / BUCKET_US).unwrap_or(BUCKETS - 1);
        self.counts[bucket.min(BUCKETS - 1)] += 1;
        self.total += 1;
        self.max_us = self.max_us.max(us);
    }

    /// Upper edge of the bucket holding the pct-th percentile, in microseconds.
    pub fn percentile_us(&self, pct: u64) -> u64 {
        if self.total == 0 {
            return 0;
        }
        let rank = (self.total * pct).div_ceil(100).max(1);
        let mut seen = 0;
        for (i, n) in self.counts.iter().enumerate() {
            seen += n;
            if seen >= rank {
                return (i as u64 + 1) * BUCKET_US;
            }
        }
        self.max_us
    }
}

pub fn report_line(t: &Timing) -> String {
    line(
        Ev::TickTime,
        &[
            ("ticks", t.tick.to_string()),
            ("p50_us", t.hist.percentile_us(50).to_string()),
            ("p90_us", t.hist.percentile_us(90).to_string()),
            ("p99_us", t.hist.percentile_us(99).to_string()),
            ("max_us", t.hist.max_us.to_string()),
            ("queued", t.queued.to_string()),
            ("overruns", t.overruns.to_string()),
        ],
    )
}

/// Prints a tick-time line every REPORT_EVERY_TICKS ticks. When `stop_after` is set, prints a
/// final line once the room reaches that tick and exits the process with status 0.
pub fn spawn_reporter(room: Arc<Room>, stop_after: Option<u64>) {
    thread::spawn(move || {
        let mut last = 0u64;
        loop {
            thread::sleep(POLL);
            let t = room.timing();
            let done = stop_after.is_some_and(|n| t.tick >= n);
            if done || t.tick >= last + REPORT_EVERY_TICKS {
                println!("{}", report_line(&t));
                last = t.tick;
            }
            if done {
                std::process::exit(0);
            }
        }
    });
}

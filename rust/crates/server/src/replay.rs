use sim::{Refusal, Sim};

#[derive(Debug, PartialEq)]
pub enum ReplayError {
    LogIncomplete { dropped: u64 },
    MissingCheckpoint { tick: u32 },
    Ahead { tick: u32, at: u32 },
    OrderRefused { tick: u32, unit: i32, refusal: Refusal },
    HashMismatch { tick: u32, expected: u64, actual: u64 },
}

/// Replays the accepted-order log of `source` into `fresh`, which must start at or before the
/// first recorded tick and hold the same map, catalog and units as the live run.
///
/// `checkpoints` holds `(tick, state_hash)` for every tick that has a recorded order, sorted by
/// tick. Each hash is taken after that tick's orders are applied and before the tick steps.
/// Returns the tick `fresh` reached.
pub fn play(source: &Sim, fresh: &mut Sim, checkpoints: &[(u32, u64)]) -> Result<u32, ReplayError> {
    let dropped = source.order_log_dropped();
    if dropped > 0 {
        return Err(ReplayError::LogIncomplete { dropped });
    }
    let records: Vec<(u32, i32, u8, i32, i32)> = (0..source.order_log_len())
        .filter_map(|k| source.order_record(k))
        .collect();

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
            let (_, unit, kind, x, y) = records[k];
            fresh
                .issue_order_kind(unit as usize, kind, x, y)
                .map_err(|refusal| ReplayError::OrderRefused { tick, unit, refusal })?;
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

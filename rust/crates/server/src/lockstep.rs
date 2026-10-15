use std::collections::BTreeMap;
use std::fmt;
use std::time::{Duration, Instant};

use crate::order::Order;

pub const MAX_AHEAD: u32 = 64;

#[derive(Debug, PartialEq)]
pub enum LockstepError {
    UnknownPlayer { player: u32 },
    Late { tick: u32, next: u32 },
    TooFarAhead { tick: u32, next: u32 },
    Duplicate { tick: u32, player: u32 },
    Timeout { tick: u32, missing: Vec<u32>, waited: Duration },
    HashMismatch { tick: u32, player: u32, expected: u64, got: u64 },
}

impl fmt::Display for LockstepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPlayer { player } => {
                write!(f, "player {player} is not in this lockstep session")
            }
            Self::Late { tick, next } => {
                write!(f, "tick {tick} is already released; the next tick is {next}")
            }
            Self::TooFarAhead { tick, next } => {
                write!(f, "tick {tick} is more than {MAX_AHEAD} ticks past {next}")
            }
            Self::Duplicate { tick, player } => {
                write!(f, "player {player} already submitted tick {tick}")
            }
            Self::Timeout {
                tick,
                missing,
                waited,
            } => write!(
                f,
                "lockstep timeout on tick {tick} after {waited:?}: still waiting for players {missing:?}"
            ),
            Self::HashMismatch {
                tick,
                player,
                expected,
                got,
            } => write!(
                f,
                "state hash mismatch on tick {tick}: player {player} reports {got:016x}, expected {expected:016x}"
            ),
        }
    }
}

pub struct Released {
    pub tick: u32,
    pub orders: Vec<Order>,
}

/// Collects each player's orders for a tick and releases the tick in order once every player
/// has submitted. Released orders are sorted by player, then by their position in that
/// player's submission (their sequence).
pub struct Lockstep {
    players: Vec<u32>,
    timeout: Duration,
    next: u32,
    inputs: BTreeMap<u32, BTreeMap<u32, Vec<Order>>>,
    hashes: BTreeMap<u32, BTreeMap<u32, u64>>,
    waiting_since: Option<Instant>,
}

impl Lockstep {
    pub fn new(players: &[u32], first_tick: u32, timeout: Duration) -> Lockstep {
        let mut players = players.to_vec();
        players.sort_unstable();
        players.dedup();
        Lockstep {
            players,
            timeout,
            next: first_tick,
            inputs: BTreeMap::new(),
            hashes: BTreeMap::new(),
            waiting_since: None,
        }
    }

    pub fn next_tick(&self) -> u32 {
        self.next
    }

    pub fn submit(&mut self, tick: u32, player: u32, orders: Vec<Order>) -> Result<(), LockstepError> {
        self.check_player(player)?;
        if tick < self.next {
            return Err(LockstepError::Late {
                tick,
                next: self.next,
            });
        }
        if tick - self.next > MAX_AHEAD {
            return Err(LockstepError::TooFarAhead {
                tick,
                next: self.next,
            });
        }
        let slot = self.inputs.entry(tick).or_default();
        if slot.contains_key(&player) {
            return Err(LockstepError::Duplicate { tick, player });
        }
        slot.insert(player, orders);
        Ok(())
    }

    /// Releases the next tick once every player's input for it is in. Before that, reports
    /// `Ok(None)` until `timeout` has passed since the tick first came up short, then
    /// `Timeout` naming the players still missing.
    pub fn poll(&mut self, now: Instant) -> Result<Option<Released>, LockstepError> {
        let tick = self.next;
        let slot = self.inputs.get(&tick);
        let missing: Vec<u32> = self
            .players
            .iter()
            .copied()
            .filter(|p| slot.map_or(true, |s| !s.contains_key(p)))
            .collect();
        if !missing.is_empty() {
            let since = *self.waiting_since.get_or_insert(now);
            let waited = now.saturating_duration_since(since);
            if waited >= self.timeout {
                return Err(LockstepError::Timeout {
                    tick,
                    missing,
                    waited,
                });
            }
            return Ok(None);
        }
        self.waiting_since = None;
        self.hashes.remove(&tick);
        self.next += 1;
        let orders = self
            .inputs
            .remove(&tick)
            .unwrap_or_default()
            .into_values()
            .flatten()
            .collect();
        Ok(Some(Released { tick, orders }))
    }

    /// Records a player's state hash for a tick. The first hash reported for a tick is the
    /// reference; any later hash that differs is refused as a mismatch.
    pub fn report_hash(&mut self, tick: u32, player: u32, hash: u64) -> Result<(), LockstepError> {
        self.check_player(player)?;
        if tick < self.next {
            return Err(LockstepError::Late {
                tick,
                next: self.next,
            });
        }
        let seen = self.hashes.entry(tick).or_default();
        if let Some((_, &expected)) = seen.iter().next() {
            if expected != hash {
                return Err(LockstepError::HashMismatch {
                    tick,
                    player,
                    expected,
                    got: hash,
                });
            }
        }
        seen.insert(player, hash);
        Ok(())
    }

    fn check_player(&self, player: u32) -> Result<(), LockstepError> {
        if self.players.binary_search(&player).is_ok() {
            Ok(())
        } else {
            Err(LockstepError::UnknownPlayer { player })
        }
    }
}

//! Neutral camps: a neutral unit the map places keeps its spawn point as its camp home. A camp
//! unit that strays past its leash walks back home. A dead camp unit respawns at home once its
//! timer runs out. Home, leash and timer are sim state: hashed and saved with the rest. Camp units
//! respawn only on a whole-minute clock mark and only when no living unit stands within
//! CAMP_CLEAR_MILLI of their home (Dota stacking rule). Roshan waits a random 8 to 11 minutes and
//! bounty runes reappear on a 3 minute clock.

use crate::arrays::{dist2, Sim};
use crate::fixed;
use crate::{ORDER_MOVE, TICKS_PER_SECOND};

/// Distance a camp unit may roam from home, milli-units (about 700 Dota units at 15.625 milli per
/// unit). UNVERIFIED: the leash radius is not in the dump.
pub const CAMP_LEASH_MILLI: i32 = 10938;
/// Radius around a camp unit's home that must hold no living unit for it to respawn, milli-units
/// (about 300 Dota units). UNVERIFIED: the camp box size is not in the dump.
pub const CAMP_CLEAR_MILLI: i32 = 4688;
/// Seconds into the game at which the first regular neutral camps appear (Dota: the first camp set
/// spawns at 1:00, dotasense.com/guides/jungle citing Valve's Dueling Fates update). Roshan and
/// runes are present from the start.
pub const CAMP_FIRST_SPAWN_S: u32 = 60;
/// Seconds between camp respawn marks: neutral camps refill on every whole minute of the game clock
/// (Dota wiki, Neutral Creeps).
pub const CAMP_RESPAWN_S: u32 = 60;
/// Roshan respawns a random 8 to 11 minutes after death (Dota wiki, Roshan): minimum and spread.
pub const ROSHAN_RESPAWN_MIN_S: u32 = 480;
pub const ROSHAN_RESPAWN_SPREAD_S: u32 = 180;
/// Unit name of the large neutral boss, and item name of its reward. Both are catalog names.
pub const ROSHAN_NAME: &str = "roshan";
pub const AEGIS_NAME: &str = "aegis";
/// Bounty rune: a 1-hp neutral pickup at a river point. A hero within the pickup range takes its
/// gold and the rune dies, then respawns at its point on the next RUNE_RESPAWN_S clock mark
/// (every 3 minutes, Dota wiki, Runes). UNVERIFIED: pickup range and gold.
pub const RUNE_NAME: &str = "bounty_rune";
pub const RUNE_RESPAWN_S: u32 = 180;
pub const RUNE_PICKUP_MILLI: i32 = 1500;
pub const RUNE_GOLD: i32 = 50;

/// Per-unit camp state, sized to the unit cap. `boss_kind` and `reward_kind` come from the
/// catalog at load, not from the save, so they are neither hashed nor saved.
#[derive(Clone, Debug)]
pub(crate) struct CampState {
    pub(crate) is_camp: Vec<u8>,
    pub(crate) home_x: Vec<i32>,
    pub(crate) home_y: Vec<i32>,
    pub(crate) waiting: Vec<u8>,
    pub(crate) respawn_at: Vec<u32>,
    pub(crate) boss_kind: Option<u32>,
    pub(crate) reward_kind: u32,
    pub(crate) rune_kind: Option<u32>,
}

impl CampState {
    pub(crate) fn new(cap: usize) -> CampState {
        CampState {
            is_camp: vec![0; cap],
            home_x: vec![0; cap],
            home_y: vec![0; cap],
            waiting: vec![0; cap],
            respawn_at: vec![0; cap],
            boss_kind: None,
            reward_kind: 0,
            rune_kind: None,
        }
    }

    /// Forgets any camp role of slot `i`; a slot reused by another spawn is not a camp unit.
    pub(crate) fn clear(&mut self, i: usize) {
        if let Some(v) = self.is_camp.get_mut(i) {
            *v = 0;
            self.waiting[i] = 0;
        }
    }
}

impl Sim {
    /// Marks live unit `i` as a camp unit whose home is its current point.
    pub(crate) fn mark_camp(&mut self, i: usize) {
        let c = &mut self.camps;
        c.is_camp[i] = 1;
        c.home_x[i] = self.pos_x[i];
        c.home_y[i] = self.pos_y[i];
    }

    /// Takes every placed regular camp unit off the map until the first camp mark, as Dota's camps
    /// only appear at 1:00. Roshan and runes stay. Call once at load, after the boss and rune kinds are set.
    pub(crate) fn hold_first_camp_spawn(&mut self) {
        let first = CAMP_FIRST_SPAWN_S * TICKS_PER_SECOND as u32;
        for i in 0..self.n {
            if self.camps.is_camp[i] == 0 || self.respawns_free(i) || !self.remove_unit(i) {
                continue;
            }
            self.camps.waiting[i] = 1;
            self.camps.respawn_at[i] = first;
        }
    }

    /// Books the respawn timer of camp unit `j` when it dies. Roshan waits longer. A Roshan killed
    /// by an attacker unit grants that unit the Aegis item.
    pub(crate) fn camp_died(&mut self, j: usize, attacker: i32) {
        if self.camps.is_camp[j] == 0 {
            return;
        }
        let kind = self.type_of[j] as u32;
        let boss = self.camps.boss_kind == Some(kind);
        let tps = TICKS_PER_SECOND as u32;
        self.camps.waiting[j] = 1;
        self.camps.respawn_at[j] = if boss {
            self.rng = self.rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let extra = ((self.rng >> 33) as u32) % (ROSHAN_RESPAWN_SPREAD_S * tps + 1);
            self.ticks.wrapping_add(ROSHAN_RESPAWN_MIN_S * tps + extra)
        } else if self.camps.rune_kind == Some(kind) {
            next_mark(self.ticks, RUNE_RESPAWN_S * tps)
        } else {
            next_mark(self.ticks, CAMP_RESPAWN_S * tps)
        };
        if boss && self.camps.reward_kind != 0 && attacker >= 0 && (attacker as usize) < self.n {
            let _ = self.inventory_add(attacker as usize, self.camps.reward_kind, 1);
        }
    }

    /// Per tick: each live camp unit past its leash is ordered home, and each dead camp unit whose
    /// timer has run out respawns at home. A respawn blocked by a full home cell retries next tick.
    pub(crate) fn tick_camps(&mut self) {
        let leash = fixed::from_milli(CAMP_LEASH_MILLI) as i64;
        self.tick_rune_pickups();
        let mut due: Vec<usize> = Vec::new();
        for i in 0..self.n {
            if self.camps.is_camp[i] == 0 {
                continue;
            }
            if self.camps.waiting[i] != 0 {
                if self.ticks >= self.camps.respawn_at[i] {
                    due.push(i);
                }
                continue;
            }
            if self.alive[i] == 0 {
                continue;
            }
            let (hx, hy) = (self.camps.home_x[i], self.camps.home_y[i]);
            if dist2(self.pos_x[i], self.pos_y[i], hx, hy) > leash * leash {
                let _ = self.issue_order_kind(i, ORDER_MOVE, hx, hy);
            }
        }
        let clear: Vec<bool> = due.iter().map(|&i| self.respawns_free(i) || self.camp_home_clear(i)).collect();
        for (&i, ok) in due.iter().zip(clear) {
            if ok {
                self.respawn_camp(i);
            } else {
                self.camps.respawn_at[i] = next_mark(self.ticks, CAMP_RESPAWN_S * TICKS_PER_SECOND as u32);
            }
        }
    }

    /// Each live bounty rune within pickup range of a living hero pays that hero RUNE_GOLD, then
    /// dies and goes on its respawn timer. The first hero in range takes it.
    fn tick_rune_pickups(&mut self) {
        let Some(rune) = self.camps.rune_kind else {
            return;
        };
        let reach = fixed::from_milli(RUNE_PICKUP_MILLI) as i64;
        for r in 0..self.n {
            if self.camps.is_camp[r] == 0 || self.alive[r] == 0 || self.type_of[r] as u32 != rune {
                continue;
            }
            let taker = (0..self.n).find(|&h| {
                self.is_hero(h)
                    && self.alive[h] != 0
                    && dist2(self.pos_x[h], self.pos_y[h], self.pos_x[r], self.pos_y[r]) <= reach * reach
            });
            if let Some(h) = taker {
                self.heroes.gold[h] = self.heroes.gold[h].saturating_add(RUNE_GOLD);
                self.hurt(r, fixed::from_int(10000));
            }
        }
    }

    /// Whether camp unit `i` is a boss or a rune, which respawn without a clear-home check.
    fn respawns_free(&self, i: usize) -> bool {
        let kind = self.type_of[i] as u32;
        self.camps.boss_kind == Some(kind) || self.camps.rune_kind == Some(kind)
    }

    /// True when no living unit stands within CAMP_CLEAR_MILLI of camp unit `i`'s home.
    fn camp_home_clear(&self, i: usize) -> bool {
        let reach = fixed::from_milli(CAMP_CLEAR_MILLI) as i64;
        let (hx, hy) = (self.camps.home_x[i], self.camps.home_y[i]);
        !(0..self.n).any(|k| k != i && self.alive[k] != 0 && dist2(self.pos_x[k], self.pos_y[k], hx, hy) <= reach * reach)
    }

    fn respawn_camp(&mut self, i: usize) {
        let kind = self.type_of[i] as u32;
        let player = self.player[i];
        let (x, y) = (self.camps.home_x[i], self.camps.home_y[i]);
        if !self.spawn_at(i, kind, player, x, y) {
            return;
        }
        self.camps.is_camp[i] = 1;
        self.camps.waiting[i] = 0;
    }
}

/// The first multiple of `interval` ticks strictly after `now`.
fn next_mark(now: u32, interval: u32) -> u32 {
    (now / interval + 1).wrapping_mul(interval)
}

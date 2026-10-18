//! Neutral camps: a neutral unit the map places keeps its spawn point as its camp home. A camp
//! unit that strays past its leash walks back home. A dead camp unit respawns at home once its
//! timer runs out. Home, leash and timer are sim state: hashed and saved with the rest. UNVERIFIED:
//! both numbers below are placeholders, not sourced values.

use crate::arrays::{dist2, Sim};
use crate::fixed;
use crate::{ORDER_MOVE, TICKS_PER_SECOND};

/// Distance a camp unit may roam from home, milli-units. UNVERIFIED: placeholder.
pub const CAMP_LEASH_MILLI: i32 = 6000;
/// Seconds a dead camp unit waits before it respawns at home. UNVERIFIED: placeholder.
pub const CAMP_RESPAWN_S: u32 = 60;

/// Per-unit camp state, sized to the unit cap.
#[derive(Clone, Debug)]
pub(crate) struct CampState {
    pub(crate) is_camp: Vec<u8>,
    pub(crate) home_x: Vec<i32>,
    pub(crate) home_y: Vec<i32>,
    pub(crate) waiting: Vec<u8>,
    pub(crate) respawn_at: Vec<u32>,
}

impl CampState {
    pub(crate) fn new(cap: usize) -> CampState {
        CampState {
            is_camp: vec![0; cap],
            home_x: vec![0; cap],
            home_y: vec![0; cap],
            waiting: vec![0; cap],
            respawn_at: vec![0; cap],
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

    /// Books the respawn timer of camp unit `j` when it dies.
    pub(crate) fn camp_died(&mut self, j: usize) {
        if self.camps.is_camp[j] == 0 {
            return;
        }
        self.camps.waiting[j] = 1;
        self.camps.respawn_at[j] = self.ticks.wrapping_add(CAMP_RESPAWN_S * TICKS_PER_SECOND as u32);
    }

    /// Per tick: each live camp unit past its leash is ordered home, and each dead camp unit whose
    /// timer has run out respawns at home. A respawn blocked by a full home cell retries next tick.
    pub(crate) fn tick_camps(&mut self) {
        let leash = fixed::from_milli(CAMP_LEASH_MILLI) as i64;
        for i in 0..self.n {
            if self.camps.is_camp[i] == 0 {
                continue;
            }
            if self.camps.waiting[i] != 0 {
                if self.ticks >= self.camps.respawn_at[i] {
                    self.respawn_camp(i);
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

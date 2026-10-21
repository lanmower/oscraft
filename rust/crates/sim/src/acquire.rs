//! Acquire range for wave creeps. A creep with no hold order that has a hostile, visible, targetable
//! enemy inside its acquire range chases it, strikes it from weapon reach, and keeps that target
//! until the target dies or leaves acquire range. Then it resumes the order it had before the chase.
//!
//! Each unit record carries its acquire range (the `acquire=` key of `unit_extra` in the scenario data,
//! read when the catalog is built). Values are in game units as the 6.88g UnitWeapons acquire
//! column gives them; `acquire_milli` converts them at 125/8 milli-units per game unit (the same
//! scale units.osd uses). Role mapping, coordinator decision: creep = Ghoul (500), creep_ranged =
//! Necromancer (600), creep_siege = Meat Wagon (800); each super and mega tier takes the value of
//! its role here: creep_super Treant (500), creep_ranged_super Druid of the Talon (600),
//! creep_siege_super Glaive Thrower (800). The event spawns n003, n00E and n00D keep their own
//! values (500, 300, 500); no unit of those names is in the catalog yet, so they match nothing.
//! Towers and barracks have no entry and keep only their weapon reach.
//!
//! Chase and return state is sim state: target and its spawn generation, plus the order to restore,
//! are hashed and saved with the rest.

use crate::arrays::{cell_of, dist2, Sim};
use crate::fixed;
use crate::loader::Catalog;
use crate::{ORDER_ATTACK_MOVE, ORDER_HOLD};

/// Game units to milli-units: 125 / 8 milli-units per game unit.
const MILLI_PER_GAME_NUM: i64 = 125;
const MILLI_PER_GAME_DEN: i64 = 8;

/// Ticks between acquire scans of one unit. Each unit scans on its own phase, so the scans spread
/// across ticks instead of bunching on one.
pub const ACQUIRE_PERIOD: u32 = 4;

/// Milli-units for an acquire value in game units.
pub fn acquire_milli(game: i32) -> i32 {
    (game as i64 * MILLI_PER_GAME_NUM / MILLI_PER_GAME_DEN) as i32
}

/// One acquire range per unit kind, milli-units; 0 for kinds with no acquire value in their unit record.
pub(crate) fn per_kind(cat: &Catalog) -> Vec<i32> {
    cat.unit_ext.iter().map(|ext| acquire_milli(ext.acquire as i32)).collect()
}

/// Chase and return state per unit: target slot (-1 none), the spawn generation of that target,
/// and the order to restore when the chase ends.
#[derive(Clone, Debug)]
pub(crate) struct ChaseState {
    pub(crate) target: Vec<i32>,
    pub(crate) target_gen: Vec<u32>,
    pub(crate) ret_kind: Vec<u8>,
    pub(crate) ret_x: Vec<i32>,
    pub(crate) ret_y: Vec<i32>,
}

impl ChaseState {
    pub(crate) fn new(cap: usize) -> ChaseState {
        ChaseState {
            target: vec![-1; cap],
            target_gen: vec![0; cap],
            ret_kind: vec![0; cap],
            ret_x: vec![0; cap],
            ret_y: vec![0; cap],
        }
    }
}

impl Sim {
    pub(crate) fn chase_clear(&mut self, i: usize) {
        self.chase.target[i] = -1;
    }

    pub(crate) fn chasing(&self, i: usize) -> bool {
        self.chase.target[i] >= 0
    }

    /// Acquire range of unit `i` in milli-units, 0 for units with no entry.
    fn acquire_range(&self, i: usize) -> i32 {
        self.acquire.get(self.type_of[i] as usize).copied().unwrap_or(0)
    }

    /// Whether the chase target of unit `i` is still a live enemy inside `range` of it.
    fn chase_holds(&self, i: usize, range: i32) -> bool {
        let t = self.chase.target[i] as usize;
        if t >= self.n || self.alive[t] == 0 || self.spawn_gen_of(t) != self.chase.target_gen[i] {
            return false;
        }
        if !self.hostile(self.player[i], self.player[t]) {
            return false;
        }
        let r = fixed::from_milli(range) as i64;
        dist2(self.pos_x[i], self.pos_y[i], self.pos_x[t], self.pos_y[t]) <= r * r
    }

    fn spawn_gen_of(&self, j: usize) -> u32 {
        self.aggro.spawn_gen[j]
    }

    /// Nearest hostile, visible, targetable enemy in `range` of unit `i`, lowest slot on a tie.
    fn acquire_target(&self, i: usize, range: i32) -> Option<usize> {
        let r = fixed::from_milli(range);
        let (x0, x1) = (cell_of(self.pos_x[i].saturating_sub(r)), cell_of(self.pos_x[i].saturating_add(r)));
        let (y0, y1) = (cell_of(self.pos_y[i].saturating_sub(r)), cell_of(self.pos_y[i].saturating_add(r)));
        let (w, h) = (self.grid.w as i32, self.grid.h as i32);
        let limit = (r as i64) * (r as i64);
        let mut best: Option<(usize, i64)> = None;
        for cy in y0.max(0)..=y1.min(h - 1) {
            for cx in x0.max(0)..=x1.min(w - 1) {
                for j in self.bucket_members((cy * w + cx) as usize) {
                    if j == i || self.alive[j] == 0 || self.is_structure(j) || !self.hostile(self.player[i], self.player[j]) {
                        continue;
                    }
                    if (self.targets[i] & self.layer_of(j)) == 0 || !self.visible(self.player[i], j) || !self.targetable(self.player[i], j) {
                        continue;
                    }
                    let d2 = dist2(self.pos_x[i], self.pos_y[i], self.pos_x[j], self.pos_y[j]);
                    if d2 > limit {
                        continue;
                    }
                    let better = match best {
                        None => true,
                        Some((b, bd2)) => d2 < bd2 || (d2 == bd2 && j < b),
                    };
                    if better {
                        best = Some((j, d2));
                    }
                }
            }
        }
        best.map(|(j, _)| j)
    }

    fn chase_end(&mut self, i: usize) {
        self.chase.target[i] = -1;
        let (kind, x, y) = (self.chase.ret_kind[i], self.chase.ret_x[i], self.chase.ret_y[i]);
        let _ = self.issue_order_kind(i, kind, x, y);
    }

    /// Chase and return for unit `i`, run once per tick before its movement. A chasing unit keeps
    /// its target while it lives inside acquire range, and steers to it; it returns to its saved
    /// order when that ends. An idle or walking creep with no chase scans for an enemy every
    /// ACQUIRE_PERIOD ticks.
    pub(crate) fn acquire_step(&mut self, i: usize) {
        let range = self.acquire_range(i);
        if range <= 0 || self.alive[i] == 0 {
            return;
        }
        if self.chasing(i) {
            if self.order_kind[i] == ORDER_HOLD {
                self.chase.target[i] = -1;
                return;
            }
            if !self.chase_holds(i, range) {
                self.chase_end(i);
                return;
            }
            let t = self.chase.target[i] as usize;
            let (tx, ty) = (self.pos_x[t], self.pos_y[t]);
            if self.tgt_x[i] != tx || self.tgt_y[i] != ty {
                let _ = self.issue_order_kind(i, ORDER_ATTACK_MOVE, tx, ty);
            }
            return;
        }
        if self.order_kind[i] == ORDER_HOLD || (self.ticks.wrapping_add(i as u32)) % ACQUIRE_PERIOD != 0 {
            return;
        }
        let Some(t) = self.acquire_target(i, range) else {
            return;
        };
        self.chase.ret_kind[i] = self.order_kind[i];
        self.chase.ret_x[i] = self.tgt_x[i];
        self.chase.ret_y[i] = self.tgt_y[i];
        self.chase.target[i] = t as i32;
        self.chase.target_gen[i] = self.spawn_gen_of(t);
        let (tx, ty) = (self.pos_x[t], self.pos_y[t]);
        if self.issue_order_kind(i, ORDER_ATTACK_MOVE, tx, ty).is_err() {
            self.chase.target[i] = -1;
        }
    }
}

impl crate::arrays::Sim {
    /// Sets the acquire range of unit `name` in game units. A load-time mod call: the unit record, the
    /// per-kind value and the catalog digest all change, so the witnesses see the new range.
    pub fn set_acquire(&mut self, name: &str, game: u32) -> Result<(), &'static str> {
        let k = self.cat.unit_index(name).ok_or("unknown unit")?;
        self.cat.unit_ext[k].acquire = game;
        self.acquire[k] = acquire_milli(game as i32);
        self.cat_digest = crate::hash::catalog_digest(&self.cat, &self.grid);
        Ok(())
    }
}

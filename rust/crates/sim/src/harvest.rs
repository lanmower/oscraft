//! Worker gather-return-deposit loop. The OSDT unit record carries no kind, so no
//! unit is a worker or a resource yet and this runs only once a kind source exists.

use crate::arrays::{dist2, Sim};
use crate::loader::FILTER_STRUCTURE;
use crate::*;

impl Sim {
    pub(crate) fn retarget_scan(&mut self) -> bool {
        if self.scan_left <= 0 {
            self.stats.retargets_deferred += 1;
            return false;
        }
        self.scan_left -= 1;
        self.stats.retargets += 1;
        true
    }

    pub(crate) fn arrived(&self, i: usize, j: usize) -> bool {
        let reach = self.radius[i].wrapping_add(self.radius[j]).wrapping_add(ARRIVE_SLACK) as i64;
        dist2(self.pos_x[i], self.pos_y[i], self.pos_x[j], self.pos_y[j]) <= reach * reach
    }

    fn nearest_resource(&self, i: usize) -> i32 {
        let mut best = -1i32;
        let mut best_d2 = 0i64;
        for j in 0..self.n {
            if self.alive[j] == 0 || self.amount[j] <= 0 {
                continue;
            }
            if self.kind[j] != KIND_MINERAL && self.kind[j] != KIND_GEYSER {
                continue;
            }
            if self.kind[j] == KIND_GEYSER && !self.has_extractor(i, j) {
                continue;
            }
            let d2 = dist2(self.pos_x[i], self.pos_y[i], self.pos_x[j], self.pos_y[j]);
            if best < 0 || d2 < best_d2 {
                best = j as i32;
                best_d2 = d2;
            }
        }
        best
    }

    fn nearest_base(&self, i: usize) -> i32 {
        let mut best = -1i32;
        let mut best_d2 = 0i64;
        for j in 0..self.n {
            if self.alive[j] == 0 || self.kind[j] != KIND_BASE || self.player[j] != self.player[i] {
                continue;
            }
            let d2 = dist2(self.pos_x[i], self.pos_y[i], self.pos_x[j], self.pos_y[j]);
            if best < 0 || d2 < best_d2 {
                best = j as i32;
                best_d2 = d2;
            }
        }
        best
    }

    /// Whether a structure of worker `i`'s player stands on geyser `j`: the extractor. A geyser
    /// yields gas only under one, and a structure stands on the geyser when their radii touch.
    fn has_extractor(&self, i: usize, j: usize) -> bool {
        let p = self.player[i];
        (0..self.n).any(|k| {
            self.alive[k] != 0
                && self.player[k] == p
                && self.filter[k] & FILTER_STRUCTURE != 0
                && {
                    let reach = self.radius[k].wrapping_add(self.radius[j]) as i64;
                    dist2(self.pos_x[k], self.pos_y[k], self.pos_x[j], self.pos_y[j]) <= reach * reach
                }
        })
    }

    /// Gather time for a worker arriving at resource `t` now. Up to `sat` workers gather at the
    /// full rate; past that the time stretches with the crowd, so the patch's income stops rising.
    /// The arriving worker counts as one; the others are the workers already gathering at `t`.
    fn gather_ticks_at(&self, t: usize, sat: i32) -> i32 {
        let sat = sat.max(1);
        let mut crowd = 1i32;
        for j in 0..self.n {
            if self.alive[j] != 0 && self.wstate[j] == W_GATHER && self.wtarget[j] == t as i32 {
                crowd += 1;
            }
        }
        GATHER_TICKS * crowd.max(sat) / sat
    }

    /// Ticks a worker arriving at resource `t` now would spend gathering: the full rate up to the
    /// saturation count, stretched beyond it. None for a unit that is not a mineral or geyser.
    pub fn gather_time_at(&self, t: usize) -> Option<i32> {
        if t >= self.n || self.alive[t] == 0 {
            return None;
        }
        match self.kind[t] {
            KIND_MINERAL => Some(self.gather_ticks_at(t, self.cat.econ.workers_per_mineral as i32)),
            KIND_GEYSER => Some(self.gather_ticks_at(t, self.cat.econ.workers_per_extractor as i32)),
            _ => None,
        }
    }

    /// Whether player `p` has an extractor on geyser `j`: a living structure of `p` whose radius
    /// touches the geyser. False for a unit that is not a living geyser.
    pub fn extractor_on(&self, p: u8, j: usize) -> bool {
        if j >= self.n || self.alive[j] == 0 || self.kind[j] != KIND_GEYSER {
            return false;
        }
        (0..self.n).any(|k| {
            self.alive[k] != 0
                && self.player[k] == p
                && self.filter[k] & FILTER_STRUCTURE != 0
                && {
                    let reach = self.radius[k].wrapping_add(self.radius[j]) as i64;
                    dist2(self.pos_x[k], self.pos_y[k], self.pos_x[j], self.pos_y[j]) <= reach * reach
                }
        })
    }

    pub(crate) fn harvest_step(&mut self, i: usize) {
        let p = self.player[i] as usize;
        match self.wstate[i] {
            W_SEEK => {
                let mut t = self.wtarget[i];
                if !self.live_at(t)
                    || self.amount[t as usize] <= 0
                    || (self.kind[t as usize] == KIND_GEYSER && !self.has_extractor(i, t as usize))
                {
                    if !self.retarget_scan() {
                        return;
                    }
                    t = self.nearest_resource(i);
                    if t < 0 {
                        return;
                    }
                    self.wtarget[i] = t;
                }
                let tu = t as usize;
                self.tgt_x[i] = self.pos_x[tu];
                self.tgt_y[i] = self.pos_y[tu];
                if self.arrived(i, tu) {
                    self.wstate[i] = W_GATHER;
                    self.wtimer[i] = match self.kind[tu] {
                        KIND_MINERAL => self.gather_ticks_at(tu, self.cat.econ.workers_per_mineral as i32),
                        KIND_GEYSER => self.gather_ticks_at(tu, self.cat.econ.workers_per_extractor as i32),
                        _ => GATHER_TICKS,
                    };
                    self.tgt_x[i] = self.pos_x[i];
                    self.tgt_y[i] = self.pos_y[i];
                }
            }
            W_GATHER => {
                self.tgt_x[i] = self.pos_x[i];
                self.tgt_y[i] = self.pos_y[i];
                self.wtimer[i] -= 1;
                if self.wtimer[i] > 0 {
                    return;
                }
                let t = self.wtarget[i];
                if self.live_at(t) && self.amount[t as usize] > 0 {
                    let tu = t as usize;
                    let per_trip = if self.kind[tu] == KIND_GEYSER {
                        self.cat.econ.gas_per_trip
                    } else {
                        self.harvest_amt[tu]
                    };
                    self.carry[i] = per_trip.min(self.amount[tu]);
                    self.carry_kind[i] = self.kind[tu];
                    self.amount[tu] -= self.carry[i];
                    self.touch(tu);
                    if self.amount[tu] <= 0 {
                        self.amount[tu] = 0;
                        self.alive[tu] = 0;
                    }
                } else {
                    self.carry[i] = 0;
                }
                self.wstate[i] = W_RETURN;
                self.wtarget[i] = -1;
            }
            W_RETURN => {
                let mut b = self.wtarget[i];
                if !self.live_at(b) {
                    if !self.retarget_scan() {
                        return;
                    }
                    b = self.nearest_base(i);
                    if b < 0 {
                        return;
                    }
                    self.wtarget[i] = b;
                }
                let bu = b as usize;
                self.tgt_x[i] = self.pos_x[bu];
                self.tgt_y[i] = self.pos_y[bu];
                if self.arrived(i, bu) {
                    self.wstate[i] = W_DEPOSIT;
                    self.wtimer[i] = DEPOSIT_TICKS;
                    self.tgt_x[i] = self.pos_x[i];
                    self.tgt_y[i] = self.pos_y[i];
                }
            }
            W_DEPOSIT => {
                self.tgt_x[i] = self.pos_x[i];
                self.tgt_y[i] = self.pos_y[i];
                self.wtimer[i] -= 1;
                if self.wtimer[i] > 0 {
                    return;
                }
                if p < MAX_PLAYERS {
                    if self.carry_kind[i] == KIND_GEYSER {
                        self.resource_b[p] += self.carry[i];
                    } else {
                        self.resource_a[p] += self.carry[i];
                    }
                    self.stats.harvested += 1;
                }
                self.carry[i] = 0;
                self.wstate[i] = W_SEEK;
                self.wtarget[i] = -1;
            }
            _ => {}
        }
    }
}

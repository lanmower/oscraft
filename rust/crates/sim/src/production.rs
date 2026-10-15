//! Production: a building trains its product. The train cost and build time are
//! the product's own (its minerals, gas and build ticks), paid up front when the
//! train starts, and the finished unit appears beside the building. A train the
//! player cannot pay for does not start and counts as CannotAfford in the stats.

use crate::arrays::Sim;
use crate::fixed;
use crate::loader::FILTER_WORKER;
use crate::rules::Refusal;
use crate::*;

impl Sim {
    /// Gives player `p` the opening resources and workers of its start at (x, y), from ECON.
    /// The workers are the first catalog unit with the worker filter; with none, only the
    /// resources are given. A worker the slots cannot hold is skipped.
    pub(crate) fn open_player(&mut self, p: u8, x: i32, y: i32) {
        let pi = p as usize;
        if pi >= MAX_PLAYERS {
            return;
        }
        let econ = self.cat.econ;
        self.minerals[pi] += econ.open_minerals;
        self.vespene[pi] += econ.open_gas;
        if let Some(kind) = self.cat.units.iter().position(|u| u.filter & FILTER_WORKER != 0) {
            for _ in 0..econ.open_workers {
                if self.spawn_for(p, kind as u32, x, y).is_none() {
                    break;
                }
            }
        }
    }

    pub(crate) fn produce_step(&mut self, i: usize) {
        let p = self.player[i] as usize;
        if self.trainee[i] < 0 || p >= MAX_PLAYERS || self.train_hold[i] != 0 {
            return;
        }
        let tr = self.trainee[i] as usize;
        let def = match self.cat.units.get(tr) {
            Some(d) => *d,
            None => return,
        };
        if self.train_left[i] > 0 {
            self.train_left[i] -= 1;
            if self.train_left[i] == 0 {
                self.finish_train(i, p, tr);
            }
            return;
        }
        if self.minerals[p] < def.minerals || self.vespene[p] < def.gas {
            self.stats.trains_refused += 1;
            self.stats.last_train_refusal = Refusal::CannotAfford;
            return;
        }
        self.minerals[p] -= def.minerals;
        self.vespene[p] -= def.gas;
        self.train_left[i] = def.build_ticks.max(1);
    }

    fn finish_train(&mut self, i: usize, p: usize, tr: usize) {
        let max_x = fixed::from_int(self.grid.w as i32 - 1);
        let beside = fixed::from_milli(self.cat.units[tr].radius);
        let x = self.pos_x[i].wrapping_add(beside).wrapping_add(self.radius[i]).min(max_x);
        let y = self.pos_y[i];
        if self.spawn_for(p as u8, tr as u32, x, y).is_some() {
            self.stats.trained += 1;
        }
    }
}

impl Sim {
    /// Cancels the train in progress at building `i` and refunds its full cost. The building
    /// stays idle until `resume_train`, so the automatic restart does not pay again at once.
    /// Refusals: an unknown or dead building is NoTarget, and a building with no train in
    /// progress is NoTrain. A refused cancel changes nothing.
    pub fn cancel_train(&mut self, i: usize) -> Result<(), Refusal> {
        if i >= self.n || self.alive[i] == 0 {
            return Err(self.reject(Refusal::NoTarget));
        }
        if self.train_left[i] <= 0 || self.trainee[i] < 0 {
            return Err(self.reject(Refusal::NoTrain));
        }
        let p = self.player[i] as usize;
        if let Some(def) = self.cat.units.get(self.trainee[i] as usize).copied() {
            if p < MAX_PLAYERS {
                self.minerals[p] += def.minerals;
                self.vespene[p] += def.gas;
            }
        }
        self.train_left[i] = 0;
        self.train_hold[i] = 1;
        Ok(())
    }

    /// Lets building `i` start its next train again after a cancel. Returns false for an
    /// index past the live units.
    pub fn resume_train(&mut self, i: usize) -> bool {
        if i >= self.n {
            return false;
        }
        self.train_hold[i] = 0;
        true
    }
}


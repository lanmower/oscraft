//! Production: a building trains its product. The train cost and build time are
//! the product's own (its minerals, gas and build ticks), paid up front when the
//! train starts, and the finished unit appears beside the building. A train the
//! player cannot pay for does not start and counts as CannotAfford in the stats.

use crate::arrays::Sim;
use crate::fixed;
use crate::rules::Refusal;
use crate::*;

impl Sim {
    pub(crate) fn produce_step(&mut self, i: usize) {
        let p = self.player[i] as usize;
        if self.trainee[i] < 0 || p >= MAX_PLAYERS {
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

//! Per-unit buff table: BUFF_SLOTS behaviours per unit, each dealing damage once
//! per period until its applications run out. A behaviour already held is refreshed.

use crate::arrays::Sim;
use crate::*;

impl Sim {
    /// Gives unit `j` behaviour `kind`. Returns false (and counts a drop) with no free slot.
    pub(crate) fn apply_buff(&mut self, j: usize, kind: i32, dmg: i32, period: i32, left: i32) -> bool {
        let base = j * BUFF_SLOTS;
        let mut free = None;
        for k in 0..BUFF_SLOTS {
            let slot = base + k;
            if self.buff_kind[slot] == kind {
                self.buff_left[slot] = left;
                self.buff_timer[slot] = period;
                self.buff_period[slot] = period;
                self.buff_dmg[slot] = dmg;
                return true;
            }
            if self.buff_kind[slot] < 0 && free.is_none() {
                free = Some(slot);
            }
        }
        match free {
            Some(slot) => {
                self.buff_kind[slot] = kind;
                self.buff_left[slot] = left;
                self.buff_timer[slot] = period;
                self.buff_period[slot] = period;
                self.buff_dmg[slot] = dmg;
                true
            }
            None => {
                self.stats.buffs_dropped += 1;
                false
            }
        }
    }

    pub(crate) fn clear_buff(&mut self, j: usize, kind: i32) {
        for k in 0..BUFF_SLOTS {
            if self.buff_kind[j * BUFF_SLOTS + k] == kind {
                self.buff_kind[j * BUFF_SLOTS + k] = -1;
            }
        }
    }

    /// Applies or removes a behaviour on one unit, from an ability's buff effect.
    pub(crate) fn buff_unit(&mut self, caster: i32, p: u8, j: usize, kind: i32, dmg: i32, period: i32, remove: bool) {
        if j >= self.n || self.alive[j] == 0 || kind < 0 {
            return;
        }
        if remove {
            self.clear_buff(j, kind);
            return;
        }
        if j as i32 == caster || self.player[j] == p {
            return;
        }
        self.apply_buff(j, kind, dmg, period.max(1), 1);
    }

    /// Ticks every held behaviour: damage on each period, slot freed after the last application.
    pub(crate) fn tick_buffs(&mut self) {
        for i in 0..self.n {
            for k in 0..BUFF_SLOTS {
                let slot = i * BUFF_SLOTS + k;
                if self.buff_kind[slot] < 0 {
                    continue;
                }
                if self.alive[i] == 0 {
                    self.buff_kind[slot] = -1;
                    continue;
                }
                self.buff_timer[slot] -= 1;
                if self.buff_timer[slot] > 0 {
                    continue;
                }
                let d = self.buff_dmg[slot];
                self.hurt(i, d);
                self.buff_left[slot] -= 1;
                if self.buff_left[slot] <= 0 {
                    self.buff_kind[slot] = -1;
                    continue;
                }
                self.buff_timer[slot] = self.buff_period[slot];
            }
        }
    }
}

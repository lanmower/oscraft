//! Healing: an area pulse restores life to living units of one player within a radius,
//! capped at each unit's maximum. A healer is a standing area that pulses every period
//! for its lifetime, from the slots in `HEALER_SLOTS`; a full table refuses new healers.

use crate::arrays::Sim;
use crate::*;

impl Sim {
    pub(crate) fn heal_area(&mut self, p: u8, amt: i32, rad: i32, x: i32, y: i32) {
        if amt <= 0 {
            return;
        }
        let mut buf = std::mem::take(&mut self.scratch);
        let n = self.gather(x, y, rad, &mut buf);
        for k in 0..n {
            let j = buf[k] as usize;
            if self.alive[j] == 0 || self.player[j] != p {
                continue;
            }
            self.hp[j] = self.hp[j].wrapping_add(amt);
            if self.hp[j] > self.hp_max[j] {
                self.hp[j] = self.hp_max[j];
            }
        }
        self.scratch = buf;
    }

    /// Heals the cast target unit alone, for a heal effect with radius 0. Only a living unit
    /// of the caster's player takes it, capped at its maximum.
    pub(crate) fn heal_target(&mut self, p: u8, amt: i32, target: i32) {
        if amt <= 0 || target < 0 || target as usize >= self.n {
            return;
        }
        let j = target as usize;
        if self.alive[j] == 0 || self.player[j] != p {
            return;
        }
        self.hp[j] = self.hp[j].wrapping_add(amt).min(self.hp_max[j]);
    }

    /// The first free healer slot, or None when all are running.
    pub(crate) fn free_healer(&self) -> Option<usize> {
        (0..HEALER_SLOTS).find(|&k| self.healer_left[k] <= 0)
    }

    /// Starts a healer for player `p` at a Q16.16 point: it pulses `amt` in `rad` every
    /// `period` ticks, the first pulse after one period, and ends after `life` ticks.
    /// Returns false when no slot is free.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create_healer(&mut self, p: u8, amt: i32, rad: i32, period: i32, life: i32, x: i32, y: i32) -> bool {
        let Some(k) = self.free_healer() else {
            return false;
        };
        self.healer_player[k] = p;
        self.healer_amt[k] = amt;
        self.healer_rad[k] = rad;
        self.healer_period[k] = period;
        self.healer_timer[k] = period;
        self.healer_left[k] = life;
        self.healer_x[k] = x;
        self.healer_y[k] = y;
        true
    }

    /// Pulses every running healer whose period has elapsed, then counts its lifetime down.
    pub(crate) fn tick_healers(&mut self) {
        for k in 0..HEALER_SLOTS {
            if self.healer_left[k] <= 0 {
                continue;
            }
            self.healer_timer[k] -= 1;
            if self.healer_timer[k] <= 0 {
                let (p, amt, rad) = (self.healer_player[k], self.healer_amt[k], self.healer_rad[k]);
                let (x, y) = (self.healer_x[k], self.healer_y[k]);
                self.heal_area(p, amt, rad, x, y);
                self.healer_timer[k] = self.healer_period[k];
            }
            self.healer_left[k] -= 1;
        }
    }
}

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
            if self.alive[j] == 0 || self.hostile(p, self.player[j]) {
                continue;
            }
            self.hp[j] = self.hp[j].wrapping_add(amt);
            if self.hp[j] > self.hp_max[j] {
                self.hp[j] = self.hp_max[j];
            }
            self.touch(j);
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
        if self.alive[j] == 0 || self.hostile(p, self.player[j]) {
            return;
        }
        self.hp[j] = self.hp[j].wrapping_add(amt).min(self.hp_max[j]);
        self.touch(j);
    }

    /// The first free healer slot, or None when all are running.
    pub(crate) fn free_healer(&self) -> Option<usize> {
        (0..HEALER_SLOTS).find(|&k| self.aura_left[k] <= 0)
    }

    /// Starts a healer for player `p` at a Q16.16 point: it pulses `amt` in `rad` every
    /// `period` ticks, the first pulse after one period, and ends after `life` ticks.
    /// Returns false when no slot is free.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create_healer(
        &mut self,
        p: u8,
        amt: i32,
        rad: i32,
        period: i32,
        life: i32,
        x: i32,
        y: i32,
        receiver: i32,
    ) -> bool {
        let Some(k) = self.free_healer() else {
            return false;
        };
        self.aura_target[k] = receiver;
        self.aura_player[k] = p;
        self.aura_amt[k] = amt;
        self.aura_rad[k] = rad;
        self.aura_period[k] = period;
        self.aura_timer[k] = period;
        self.aura_left[k] = life;
        self.aura_x[k] = x;
        self.aura_y[k] = y;
        true
    }

    /// Pulses every running healer whose period has elapsed, then counts its lifetime down.
    pub(crate) fn tick_healers(&mut self) {
        for k in 0..HEALER_SLOTS {
            if self.aura_left[k] <= 0 {
                continue;
            }
            self.aura_timer[k] -= 1;
            if self.aura_timer[k] <= 0 {
                let (p, amt, rad) = (self.aura_player[k], self.aura_amt[k], self.aura_rad[k]);
                let (x, y) = (self.aura_x[k], self.aura_y[k]);
                if rad == 0 {
                    let receiver = self.aura_target[k];
                    self.heal_target(p, amt, receiver);
                } else {
                    self.heal_area(p, amt, rad, x, y);
                }
                self.aura_timer[k] = self.aura_period[k];
            }
            self.aura_left[k] -= 1;
        }
    }
}

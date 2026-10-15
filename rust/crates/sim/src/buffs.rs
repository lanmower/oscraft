//! Per-unit buff table: BUFF_SLOTS behaviour kinds per unit. Each held kind counts its
//! remaining ticks down and is freed when they run out. Damage per application and its
//! period have no field in format v4, so a buff carries only its kind and duration.
//! A behaviour already held is refreshed to the new duration.

use crate::arrays::Sim;
use crate::*;

impl Sim {
    /// Gives unit `j` behaviour `kind` for `ticks` ticks. Returns false (and counts a drop)
    /// with no free slot.
    pub fn apply_buff(&mut self, j: usize, kind: i32, ticks: i32) -> bool {
        self.touch(j);
        let base = j * BUFF_SLOTS;
        let mut free = None;
        for k in 0..BUFF_SLOTS {
            let slot = base + k;
            if self.buff_kind[slot] == kind {
                self.buff_left[slot] = ticks;
                return true;
            }
            if self.buff_kind[slot] < 0 && free.is_none() {
                free = Some(slot);
            }
        }
        match free {
            Some(slot) => {
                self.buff_kind[slot] = kind;
                self.buff_left[slot] = ticks;
                true
            }
            None => {
                self.stats.buffs_dropped += 1;
                false
            }
        }
    }

    pub(crate) fn clear_buff(&mut self, j: usize, kind: i32) {
        self.touch(j);
        for k in 0..BUFF_SLOTS {
            if self.buff_kind[j * BUFF_SLOTS + k] == kind {
                self.buff_kind[j * BUFF_SLOTS + k] = -1;
            }
        }
    }

    /// Removes behaviour `kind` from every living unit within `rad` of a point, whichever
    /// player owns it. Uses the shared scratch buffer, so nothing is allocated.
    pub(crate) fn remove_behaviour(&mut self, kind: i32, rad: i32, x: i32, y: i32) {
        let mut buf = std::mem::take(&mut self.scratch);
        let n = self.gather(x, y, rad, &mut buf);
        for k in 0..n {
            let j = buf[k] as usize;
            if self.alive[j] != 0 {
                self.clear_buff(j, kind);
            }
        }
        self.scratch = buf;
    }

    /// Applies or removes a behaviour on one unit, from an ability's buff effect. `ticks`
    /// is the duration (format v4 buff p1).
    pub(crate) fn buff_unit(&mut self, caster: i32, p: u8, j: usize, kind: i32, ticks: i32, remove: bool) {
        if j >= self.n || self.alive[j] == 0 || kind < 0 {
            return;
        }
        if remove {
            self.clear_buff(j, kind);
            return;
        }
        if j as i32 == caster || !self.hostile(p, self.player[j]) {
            return;
        }
        self.apply_buff(j, kind, ticks);
    }

    /// Counts every held behaviour down one tick and frees the slot when its duration ends.
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
                self.buff_left[slot] -= 1;
                if self.buff_left[slot] <= 0 {
                    self.buff_kind[slot] = -1;
                    self.touch(i);
                }
            }
        }
    }
}

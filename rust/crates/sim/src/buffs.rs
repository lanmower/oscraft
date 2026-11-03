
use crate::arrays::Sim;
use crate::*;

impl Sim {
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

    pub fn buffs(&self, j: usize) -> impl Iterator<Item = (i32, i32)> + '_ {
        let base = j * BUFF_SLOTS;
        let slots = if j < self.n { BUFF_SLOTS } else { 0 };
        (0..slots).filter_map(move |k| {
            let kind = self.buff_kind[base + k];
            (kind >= 0).then(|| (kind, self.buff_left[base + k]))
        })
    }

    pub fn clear_buff(&mut self, j: usize, kind: i32) {
        if j >= self.n {
            return;
        }
        self.touch(j);
        for k in 0..BUFF_SLOTS {
            if self.buff_kind[j * BUFF_SLOTS + k] == kind {
                self.buff_kind[j * BUFF_SLOTS + k] = -1;
            }
        }
    }

    pub fn remove_behaviour(&mut self, caster: usize, kind: i32, rad: i32, x: i32, y: i32, cost: i32) -> Result<(), Refusal> {
        if caster >= self.n || self.alive[caster] == 0 {
            return Err(self.reject(Refusal::NoTarget));
        }
        if cost > 0 {
            self.spend_energy(caster, cost)?;
        }
        let mut buf = std::mem::take(&mut self.scratch);
        let n = self.gather(x, y, rad, &mut buf);
        for k in 0..n {
            let j = buf[k] as usize;
            if self.alive[j] != 0 {
                self.clear_buff(j, kind);
            }
        }
        self.scratch = buf;
        Ok(())
    }

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

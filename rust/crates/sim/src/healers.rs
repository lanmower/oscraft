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

    pub(crate) fn free_healer(&self) -> Option<usize> {
        (0..HEALER_SLOTS).find(|&k| self.aura_left[k] <= 0)
    }

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

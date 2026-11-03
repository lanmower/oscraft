use crate::arrays::Sim;
use crate::fixed;

impl Sim {
    pub(crate) fn absorb_shield(&mut self, j: usize, dmg: i32) -> i32 {
        if self.shield[j] <= 0 {
            return dmg;
        }
        let taken = dmg.min(self.shield[j]);
        self.shield[j] -= taken;
        self.shield_timed[j] = (self.shield_timed[j] - taken).max(0);
        dmg - taken
    }

    pub(crate) fn expire_shields(&mut self) {
        for i in 0..self.n {
            let until = self.shield_until[i];
            if until != 0 && self.ticks >= until {
                self.shield[i] = (self.shield[i] - self.shield_timed[i]).max(0);
                self.shield_timed[i] = 0;
                self.shield_until[i] = 0;
                self.touch(i);
            }
        }
    }

    pub fn shield_bonus(&mut self, caster: usize, target: i32, amount: i32) {
        self.shield_bonus_timed(caster, target, amount, 0);
    }

    pub fn shield_bonus_timed(&mut self, caster: usize, target: i32, amount: i32, duration: i32) {
        self.shield_bonus_locked(caster, target, amount, duration, 0);
    }

    pub fn shield_bonus_locked(&mut self, caster: usize, target: i32, amount: i32, duration: i32, lock: i32) {
        if caster >= self.n {
            self.refuse(crate::rules::Refusal::NoTarget);
            return;
        }
        let r = if target >= 0 && (target as usize) < self.n { target as usize } else { caster };
        if self.alive[r] == 0 {
            self.refuse(crate::rules::Refusal::NoTarget);
            return;
        }
        if lock > 0 && self.is_attributed(r) {
            if self.ticks < self.shield_lock[r] {
                return;
            }
            self.shield_lock[r] = self.ticks.saturating_add(lock as u32);
        }
        if amount > 0 {
            let added = fixed::from_int(amount);
            self.shield[r] = self.shield[r].saturating_add(added);
            if duration > 0 {
                self.shield_timed[r] = self.shield_timed[r].saturating_add(added);
                self.shield_until[r] = self.ticks.saturating_add(duration as u32);
            }
            self.touch(r);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn vital_fraction(&mut self, caster: usize, target: i32, vital: i32, frac: i32, mode: i32, of_max: i32) {
        if caster >= self.n {
            self.refuse(crate::rules::Refusal::NoTarget);
            return;
        }
        let r = if target >= 0 && (target as usize) < self.n { target as usize } else { caster };
        if self.alive[r] == 0 {
            self.refuse(crate::rules::Refusal::NoTarget);
            return;
        }
        let life = vital == 0;
        let base = match (life, of_max == 0) {
            (true, true) => self.hp[r],
            (true, false) => self.hp_max[r],
            (false, true) => self.shield[r],
            (false, false) => self.shield_max[r],
        };
        let amount = ((base as i64 * frac as i64) >> 16) as i32;
        let current = if life { self.hp[r] } else { self.shield[r] };
        let next = if mode == 0 { current.wrapping_sub(amount) } else { amount };
        if life {
            self.set_life(r, next, caster as i32);
        } else {
            self.shield[r] = next.max(0);
            self.touch(r);
        }
    }

    pub(crate) fn regen_shield(&mut self, i: usize) {
        let k = self.type_of[i];
        if k < 0 || self.alive[i] == 0 {
            return;
        }
        let r = match self.cat.regen.get(k as usize) {
            Some(r) => *r,
            None => return,
        };
        if r.shield_per_tick <= 0 || self.shield[i] >= self.shield_max[i] {
            return;
        }
        if self.ticks.wrapping_sub(self.last_hit[i]) < r.delay as u32 {
            return;
        }
        self.shield[i] = self.shield[i].saturating_add(fixed::from_int(r.shield_per_tick)).min(self.shield_max[i]);
        self.touch(i);
    }

    pub(crate) fn regen_health(&mut self, i: usize) {
        let k = self.type_of[i];
        if k < 0 || self.alive[i] == 0 {
            return;
        }
        let (base, delay) = match self.cat.regen.get(k as usize) {
            Some(r) => (((r.hp_per_tick as i64 * fixed::ONE as i64) / 1000) as i32, r.delay as u32),
            None => (0, 0),
        };
        let per_tick = base.saturating_add(self.unit_regen_per_tick(i));
        if per_tick <= 0 || self.hp[i] >= self.hp_max[i] {
            return;
        }
        if self.ticks.wrapping_sub(self.last_hit[i]) < delay {
            return;
        }
        self.hp[i] = self.hp[i].saturating_add(per_tick).min(self.hp_max[i]);
        self.touch(i);
    }

    fn set_life(&mut self, j: usize, value: i32, attacker: i32) {
        self.hp[j] = value.clamp(0, self.hp_max[j]);
        self.touch(j);
        if self.hp[j] == 0 && self.alive[j] != 0 {
            self.alive[j] = 0;
            self.stats.deaths += 1;
            self.record_death(j, attacker);
        }
    }
}

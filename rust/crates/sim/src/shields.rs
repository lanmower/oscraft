//! Shield pools: a unit's shield takes a hit before its life does. The pool starts
//! at the unit's shield amount and is not regenerated here.

use crate::arrays::Sim;
use crate::fixed;

impl Sim {
    /// Takes damage off unit `j`'s shield and returns what passes through to life.
    pub(crate) fn absorb_shield(&mut self, j: usize, dmg: i32) -> i32 {
        if self.shield[j] <= 0 {
            return dmg;
        }
        if self.shield[j] >= dmg {
            self.shield[j] -= dmg;
            return 0;
        }
        let rest = dmg - self.shield[j];
        self.shield[j] = 0;
        rest
    }

    /// Adds a flat bonus to a unit's shield pool. The receiver is the cast target unit, or
    /// the caster when the cast names no unit. A dead receiver refuses as no-target.
    pub fn shield_bonus(&mut self, caster: usize, target: i32, amount: i32) {
        if caster >= self.n {
            self.refuse(crate::rules::Refusal::NoTarget);
            return;
        }
        let r = if target >= 0 && (target as usize) < self.n { target as usize } else { caster };
        if self.alive[r] == 0 {
            self.refuse(crate::rules::Refusal::NoTarget);
            return;
        }
        if amount > 0 {
            self.shield[r] = self.shield[r].saturating_add(amount);
            self.touch(r);
        }
    }

    /// Drains or sets a vital of the receiver (the cast target unit, or the caster when the
    /// cast names none). `vital` 0 is life, 1 is shield. The amount is `frac` (Q16, 65536 =
    /// 1.0) of the current vital (`of_max` 0) or of its maximum (`of_max` 1). `mode` 0 drains:
    /// the amount is subtracted; 1 sets: the vital becomes the amount. Life reaching zero ends
    /// the unit; a shield is never negative.
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
            self.set_life(r, next);
        } else {
            self.shield[r] = next.max(0);
            self.touch(r);
        }
    }

    /// Regenerates shield on unit `i` once the regeneration delay since its last hit has passed.
    /// The shield never exceeds its maximum. A unit kind with no shield regeneration is left alone.
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

    /// Regenerates life on unit `i` once the regeneration delay since its last hit has passed.
    /// Life never exceeds its maximum, and a dead unit regenerates nothing.
    pub(crate) fn regen_health(&mut self, i: usize) {
        let k = self.type_of[i];
        if k < 0 || self.alive[i] == 0 {
            return;
        }
        let r = match self.cat.regen.get(k as usize) {
            Some(r) => *r,
            None => return,
        };
        if r.hp_per_tick <= 0 || self.hp[i] >= self.hp_max[i] {
            return;
        }
        if self.ticks.wrapping_sub(self.last_hit[i]) < r.delay as u32 {
            return;
        }
        self.hp[i] = self.hp[i].saturating_add(fixed::from_int(r.hp_per_tick)).min(self.hp_max[i]);
        self.touch(i);
    }

    /// Writes life clamped to [0, max]; a unit whose life reaches zero dies.
    fn set_life(&mut self, j: usize, value: i32) {
        self.hp[j] = value.clamp(0, self.hp_max[j]);
        self.touch(j);
        if self.hp[j] == 0 && self.alive[j] != 0 {
            self.alive[j] = 0;
            self.stats.deaths += 1;
        }
    }
}

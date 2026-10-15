//! Shield pools: a unit's shield takes a hit before its life does. The pool starts
//! at the unit's shield amount and is not regenerated here.

use crate::arrays::Sim;

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
    pub(crate) fn shield_bonus(&mut self, caster: usize, target: i32, amount: i32) {
        let r = if target >= 0 && (target as usize) < self.n { target as usize } else { caster };
        if self.alive[r] == 0 {
            self.refuse(crate::rules::Refusal::NoTarget);
            return;
        }
        if amount > 0 {
            self.shield[r] = self.shield[r].saturating_add(amount);
        }
    }
}

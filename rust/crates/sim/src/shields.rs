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
}

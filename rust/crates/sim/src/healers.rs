//! Healing area effect: restores life to living units of one player within a radius,
//! capped at each unit's maximum. The caster is included when it stands in the area.

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
}

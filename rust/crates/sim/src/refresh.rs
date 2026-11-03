use crate::arrays::Sim;
use crate::inventory::INV_SLOTS;
use crate::loader::Effect;
use crate::ABILITY_SLOTS;

pub(crate) const K_REFRESH: u16 = 32;
pub(crate) const REFRESH_ABILITIES: i32 = 1;
pub(crate) const REFRESH_ITEMS: i32 = 2;

impl Sim {
    pub(crate) fn refresh_effect(&mut self, caster: usize, eff: Effect) {
        if eff.p0 & REFRESH_ABILITIES != 0 {
            let base = caster * ABILITY_SLOTS;
            self.abil_cd[base..base + ABILITY_SLOTS].fill(0);
        }
        if eff.p0 & REFRESH_ITEMS != 0 {
            let base = caster * INV_SLOTS;
            let using = self.expiry.casting_slot;
            for slot in 0..INV_SLOTS {
                if slot as i32 != using {
                    self.item_cd[base + slot] = 0;
                }
            }
        }
        self.touch(caster);
    }
}

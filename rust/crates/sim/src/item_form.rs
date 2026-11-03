use crate::arrays::Sim;
use crate::inventory::{item_kind, INV_SLOTS};
use crate::loader::Effect;

pub(crate) const K_ITEM_FORM: u16 = 36;
pub(crate) const FORM_SWAP: i32 = 0;
pub(crate) const FORM_SPEND_CHARGE: i32 = 1;

pub(crate) fn params_valid(p1: i32, p2: i32, p3: i32) -> bool {
    (FORM_SWAP..=FORM_SPEND_CHARGE).contains(&p1) && p2 >= 0 && p3 >= 0
}

impl Sim {
    pub(crate) fn item_form_effect(&mut self, caster: usize, eff: Effect) {
        let using = self.expiry.casting_slot;
        if using < 0 || using as usize >= INV_SLOTS {
            return;
        }
        let key = caster * INV_SLOTS + using as usize;
        let Some(form) = usize::try_from(eff.p0).ok().filter(|&f| f < self.cat.items.len()) else {
            return;
        };
        if self.inv_count[key] == 0 {
            return;
        }
        if eff.p1 == FORM_SPEND_CHARGE && self.inv_count[key] > 1 {
            self.change_items(caster, |sim| sim.inv_count[key] -= 1);
            return;
        }
        let count = if eff.p2 > 0 { eff.p2 as u32 } else { u32::from(self.cat.items[form].charges).max(1) };
        let lock = self.ticks.saturating_add(eff.p3 as u32);
        if self.replace_item(caster, using as usize, item_kind(form), count).is_err() {
            return;
        }
        if eff.p3 > 0 {
            self.item_lock_until[key] = lock;
            self.item_lock_kind[key] = item_kind(form);
        }
    }
}


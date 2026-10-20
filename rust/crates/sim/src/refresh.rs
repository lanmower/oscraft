//! The `refresh` effect belongs to the mod. The sim queues an `EffectEvent` when a refresh is applied;
//! the host feeds it to the mod's `on_effect`, which resets cooldowns through `reset_cooldowns`.
//! p0 is a bit set: 1 resets ability cooldowns, 2 resets item cooldowns (the item being used keeps its own,
//! as a refresher orb does not refresh itself).

use crate::arrays::Sim;
use crate::inventory::INV_SLOTS;
use crate::loader::Effect;
use crate::ABILITY_SLOTS;

pub(crate) const K_REFRESH: u16 = 32;

/// A mod-owned effect the sim applied: its kind name, the caster's unit index, the effect's p0 and the
/// inventory slot being used (-1 for a spell).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectEvent {
    pub kind: &'static str,
    pub caster: u32,
    pub p0: i32,
    pub slot: i32,
}

impl Sim {
    pub(crate) fn refresh_effect(&mut self, caster: usize, eff: Effect) {
        self.effect_events.push(EffectEvent { kind: "refresh", caster: caster as u32, p0: eff.p0, slot: self.wards.casting_slot });
    }

    /// Takes the effect events queued since the last take, in the order they were applied.
    pub fn drain_effect_events(&mut self) -> Vec<EffectEvent> {
        self.effect_events.drain(..).collect()
    }

    /// Resets the cooldowns of unit `unit`: its abilities when `abilities`, its items when `items`, but
    /// never the item in inventory slot `keep_slot` (-1 keeps none). False for an out-of-range unit.
    pub fn reset_cooldowns(&mut self, unit: usize, abilities: bool, items: bool, keep_slot: i32) -> bool {
        if unit >= self.cap {
            return false;
        }
        if abilities {
            let base = unit * ABILITY_SLOTS;
            self.abil_cd[base..base + ABILITY_SLOTS].fill(0);
        }
        if items {
            let base = unit * INV_SLOTS;
            for slot in 0..INV_SLOTS {
                if slot as i32 != keep_slot {
                    self.item_cd[base + slot] = 0;
                }
            }
        }
        self.touch(unit);
        true
    }
}

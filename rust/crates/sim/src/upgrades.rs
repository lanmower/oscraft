//! Upgrades. A completed upgrade record applies its effect to units of its target kind, once
//! per unit: units of the player that exist at completion receive it when the record applies to
//! existing units, and every unit built afterwards receives every completed record for its kind.
//! Records of the same upgrade name and target form a stacking group. Rule 1 adds every level,
//! rule 2 refuses a level while a sibling is complete. Rule 0 (replace) is refused, because the
//! effect of a replaced level cannot be taken back out of the units it already changed.

use crate::arrays::Sim;
use crate::loader::Effect;
use crate::rules::Refusal;
use crate::*;

const EFFECT_SPAWN_CODE: u16 = 3;
const EFFECT_USE_CALLDOWN_CODE: u16 = 8;

impl Sim {
    /// Completes upgrade record `rec` for `player`. Refusals: an unknown record or player, a
    /// record already complete for that player, a stacking rule that refuses, an unsupported
    /// target kind, and an effect that would spawn or call down units.
    pub fn complete_upgrade(&mut self, rec: usize, player: u8) -> Result<(), Refusal> {
        if rec >= self.cat.upgrades.len() {
            return Err(self.reject(Refusal::NoTarget));
        }
        if player as usize >= MAX_PLAYERS {
            return Err(self.reject(Refusal::InvalidPlayer));
        }
        let up = self.cat.upgrades[rec];
        let slot = rec * MAX_PLAYERS + player as usize;
        if self.upg_done[slot] != 0 {
            return Err(self.reject(Refusal::UpgradeRefused));
        }
        if up.target_kind != 0 {
            return Err(self.reject(Refusal::EffectUnsupported));
        }
        let effect = match self.cat.effects.get(up.effect as usize).copied() {
            Some(e) => e,
            None => return Err(self.reject(Refusal::NoTarget)),
        };
        if effect.kind == EFFECT_SPAWN_CODE || effect.kind == EFFECT_USE_CALLDOWN_CODE {
            return Err(self.reject(Refusal::EffectUnsupported));
        }
        match up.stacking {
            1 => {}
            2 => {
                if self.sibling_complete(rec, player) {
                    return Err(self.reject(Refusal::UpgradeRefused));
                }
            }
            _ => return Err(self.reject(Refusal::UpgradeRefused)),
        }
        self.upg_done[slot] = 1;
        if up.applies == 1 {
            for i in 0..self.n {
                if self.alive[i] != 0 && self.player[i] == player && self.type_of[i] == up.target as i32 {
                    self.apply_upgrade_effect(i, effect);
                }
            }
        }
        Ok(())
    }

    /// Gives unit `i`, just built for `player` as unit kind `kind`, every completed record
    /// that targets its kind.
    pub(crate) fn apply_upgrades_to_new(&mut self, i: usize, player: u8, kind: i32) {
        if player as usize >= MAX_PLAYERS || kind < 0 {
            return;
        }
        for rec in 0..self.cat.upgrades.len() {
            let up = self.cat.upgrades[rec];
            if up.target_kind != 0 || up.target as i32 != kind || self.upg_done[rec * MAX_PLAYERS + player as usize] == 0 {
                continue;
            }
            if let Some(effect) = self.cat.effects.get(up.effect as usize).copied() {
                self.apply_upgrade_effect(i, effect);
            }
        }
    }

    /// Applies one upgrade effect to unit `unit`, which is its own receiver.
    fn apply_upgrade_effect(&mut self, unit: usize, effect: Effect) {
        let (x, y) = (self.pos_x[unit], self.pos_y[unit]);
        self.apply_effect(unit, 0, effect, x, y, unit as i32, true);
    }

    /// Whether another record of `rec`'s stacking group, for the same target, is complete for `player`.
    fn sibling_complete(&self, rec: usize, player: u8) -> bool {
        let up = self.cat.upgrades[rec];
        (0..self.cat.upgrades.len()).any(|j| {
            let sib = self.cat.upgrades[j];
            j != rec
                && sib.group == up.group
                && sib.target == up.target
                && sib.target_kind == up.target_kind
                && self.upg_done[j * MAX_PLAYERS + player as usize] != 0
        })
    }
}

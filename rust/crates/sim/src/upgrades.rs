//! Upgrades. Completing an upgrade record starts the next level of its stacking group for a
//! player: the first completion is level 1, the next level 2, and so on. A level costs its
//! number times the record's per-level minerals and gas, and applies the record's effect to the
//! player's units of its target kind, once per level: units existing at completion when the
//! record applies to existing units, and every unit built afterwards.
//! Records of the same upgrade name and target form a stacking group. Rule 1 adds levels,
//! rule 2 refuses a level once any level of the group is complete. Rule 0 (replace) is refused,
//! because the effect of a replaced level cannot be taken back out of the units it already changed.

use crate::arrays::Sim;
use crate::loader::Effect;
use crate::rules::Refusal;
use crate::*;

const EFFECT_SPAWN_CODE: u16 = 3;
const EFFECT_USE_CALLDOWN_CODE: u16 = 8;

impl Sim {
    /// Completes the next level of upgrade record `rec`'s group for `player`, charging that level's
    /// cost first. Refusals: an unknown record or player, a stacking rule that refuses, an unsupported
    /// target kind, an effect that would spawn or call down units, and a player who cannot pay
    /// (CannotAfford, nothing charged).
    pub fn complete_upgrade(&mut self, rec: usize, player: u8) -> Result<(), Refusal> {
        if rec >= self.cat.upgrades.len() {
            return Err(self.reject(Refusal::NoTarget));
        }
        if player as usize >= MAX_PLAYERS {
            return Err(self.reject(Refusal::InvalidPlayer));
        }
        let up = self.cat.upgrades[rec];
        let p = player as usize;
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
        let done = self.group_level(rec, p);
        match up.stacking {
            1 => {}
            2 => {
                if done > 0 {
                    return Err(self.reject(Refusal::UpgradeRefused));
                }
            }
            _ => return Err(self.reject(Refusal::UpgradeRefused)),
        }
        let level = done.saturating_add(1);
        let mineral_cost = level.saturating_mul(up.minerals_per_level);
        let gas_cost = level.saturating_mul(up.gas_per_level);
        if self.minerals[p] < mineral_cost || self.vespene[p] < gas_cost {
            return Err(self.reject(Refusal::CannotAfford));
        }
        self.minerals[p] -= mineral_cost;
        self.vespene[p] -= gas_cost;
        let slot = rec * MAX_PLAYERS + p;
        self.upg_done[slot] = self.upg_done[slot].saturating_add(1);
        if up.applies == 1 {
            for i in 0..self.n {
                if self.alive[i] != 0 && self.player[i] == player && self.type_of[i] == up.target as i32 {
                    self.apply_upgrade_effect(i, effect);
                }
            }
        }
        Ok(())
    }

    /// Gives unit `i`, just built for `player` as unit kind `kind`, the effect of every completed
    /// level of each record that targets its kind.
    pub(crate) fn apply_upgrades_to_new(&mut self, i: usize, player: u8, kind: i32) {
        if player as usize >= MAX_PLAYERS || kind < 0 {
            return;
        }
        for rec in 0..self.cat.upgrades.len() {
            let up = self.cat.upgrades[rec];
            let levels = self.upg_done[rec * MAX_PLAYERS + player as usize];
            if up.target_kind != 0 || up.target as i32 != kind || levels == 0 {
                continue;
            }
            if let Some(effect) = self.cat.effects.get(up.effect as usize).copied() {
                for _ in 0..levels {
                    self.apply_upgrade_effect(i, effect);
                }
            }
        }
    }

    /// Applies one upgrade effect to unit `unit`, which is its own receiver.
    fn apply_upgrade_effect(&mut self, unit: usize, effect: Effect) {
        let (x, y) = (self.pos_x[unit], self.pos_y[unit]);
        self.apply_effect(unit, 0, effect, x, y, unit as i32, true);
    }

    /// Completed levels of `rec`'s stacking group, for the same target, for `player`.
    fn group_level(&self, rec: usize, player: usize) -> i32 {
        let up = self.cat.upgrades[rec];
        (0..self.cat.upgrades.len())
            .filter(|&j| {
                let sib = self.cat.upgrades[j];
                sib.group == up.group && sib.target == up.target && sib.target_kind == up.target_kind
            })
            .map(|j| self.upg_done[j * MAX_PLAYERS + player] as i32)
            .sum()
    }
}

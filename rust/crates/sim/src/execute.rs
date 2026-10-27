use crate::arrays::Sim;
use crate::loader::Effect;
use crate::ABILITY_SLOTS;

pub(crate) const K_EXECUTE: u16 = 37;
pub(crate) const RESET_ON_HERO_KILL: i32 = 1;

impl Sim {
    pub(crate) fn execute_effect(&mut self, caster: usize, eff: Effect, target: i32) {
        let Some(j) = (target >= 0 && (target as usize) < self.n).then_some(target as usize) else {
            self.refuse(crate::rules::Refusal::NoTarget);
            return;
        };
        if j == caster || self.alive[j] == 0 || !self.hostile(self.player[caster], self.player[j]) || self.turned_away(caster, j) {
            return;
        }
        let hero = self.is_attributed(j);
        let kill = crate::fixed::to_int(self.hp[j]) <= eff.p0;
        if kill {
            let all = self.hp[j];
            self.hurt_typed(j, all, caster as i32);
            self.spell.kill_flag = self.alive[j] == 0;
        } else {
            self.spell_damage(caster as i32, j, eff.p1, eff.p2);
        }
        if kill && hero && self.alive[j] == 0 && eff.p3 & RESET_ON_HERO_KILL != 0 {
            let base = caster * ABILITY_SLOTS;
            if let Some(k) = (base..base + ABILITY_SLOTS).find(|&k| self.abil_idx[k] == self.spell.cur_ai) {
                self.abil_cd[k] = 0;
                self.touch(caster);
            }
        }
    }
}

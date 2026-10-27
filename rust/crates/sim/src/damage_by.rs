use crate::arrays::Sim;
use crate::fixed;
use crate::loader::Effect;

pub(crate) const K_DAMAGE_BY: u16 = 35;

pub(crate) const BY_STRENGTH: i32 = 1;
pub(crate) const BY_AGILITY: i32 = 2;
pub(crate) const BY_INTELLIGENCE: i32 = 3;
pub(crate) const BY_TARGET_LIFE: i32 = 4;
pub(crate) const BY_TARGET_MAX_LIFE: i32 = 5;
pub(crate) const BY_TARGET_MISSING_LIFE: i32 = 6;
pub(crate) const BY_TARGET_MANA: i32 = 7;
pub(crate) const BY_INTELLIGENCE_LEAD: i32 = 8;
pub(crate) const BY_DISTANCE: i32 = 9;

pub(crate) fn params_valid(p0: i32, p1: i32, p2: i32, p3: i32) -> bool {
    p0 >= 0 && (BY_STRENGTH..=BY_DISTANCE).contains(&p1) && p2 & 3 != 0 && crate::spell::spell_type_valid(p2) && p3 >= 0
}

impl Sim {
    fn attribute_points(&self, i: usize, p1: i32) -> i64 {
        let name = match p1 {
            BY_STRENGTH => "attr_str",
            BY_AGILITY => "attr_agi",
            _ => "attr_int",
        };
        self.counter(i, name)
    }

    pub(crate) fn damage_by_amount(&self, caster: usize, j: usize, eff: Effect) -> i32 {
        if eff.p1 == BY_DISTANCE {
            let reach = crate::spell::isqrt64(crate::arrays::dist2(self.pos_x[caster], self.pos_y[caster], self.pos_x[j], self.pos_y[j]) as u64);
            let milli = reach as i64 * 1000 / fixed::ONE as i64;
            return (eff.p0 as i64 + milli * eff.p3 as i64 / 100_000).clamp(0, i32::MAX as i64) as i32;
        }
        let scaled = match eff.p1 {
            BY_STRENGTH | BY_AGILITY | BY_INTELLIGENCE => self.attribute_points(caster, eff.p1),
            BY_TARGET_LIFE => fixed::to_int(self.hp[j]) as i64,
            BY_TARGET_MAX_LIFE => fixed::to_int(self.hp_max[j]) as i64,
            BY_TARGET_MISSING_LIFE => fixed::to_int(self.hp_max[j] - self.hp[j]) as i64,
            BY_TARGET_MANA => self.energy[j] as i64,
            _ => (self.attribute_points(caster, BY_INTELLIGENCE) - self.attribute_points(j, BY_INTELLIGENCE)).max(0),
        };
        (eff.p0 as i64 + scaled * eff.p3 as i64 / 100).clamp(0, i32::MAX as i64) as i32
    }

    pub(crate) fn damage_by_effect(&mut self, caster: usize, eff: Effect, target: i32) {
        let Some(j) = (target >= 0 && (target as usize) < self.n).then_some(target as usize) else {
            self.refuse(crate::rules::Refusal::NoTarget);
            return;
        };
        if j == caster || self.alive[j] == 0 || !self.hostile(self.player[caster], self.player[j]) || self.turned_away(caster, j) {
            return;
        }
        let amount = self.damage_by_amount(caster, j, eff);
        self.spell_damage(caster as i32, j, amount, eff.p2);
    }
}

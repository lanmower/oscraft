use crate::arrays::Sim;
use crate::inventory::STAT_SPELL_BLOCK;
use crate::rules::{TARGET_UNIT, TARGET_UNIT_OR_POINT};
use crate::spell::FLAG_PIERCE;

pub(crate) const BLOCK_LOG_CAP: usize = 256;

impl Sim {
    pub(crate) fn absorbs_cast(&mut self, caster: usize, ai: usize, target: i32) -> bool {
        let ability = self.cat.abilities[ai];
        let unit_target = ability.target == TARGET_UNIT || ability.target == TARGET_UNIT_OR_POINT;
        let Some(j) = usize::try_from(target).ok().filter(|&j| unit_target && j < self.n && j != caster && self.alive[j] != 0) else {
            return false;
        };
        if self.ext_of(ai).flags & FLAG_PIERCE != 0
            || !self.hostile(self.player[caster], self.player[j])
            || (self.item_stat(j, STAT_SPELL_BLOCK) <= 0 && self.block_until[j] <= self.ticks)
        {
            return false;
        }
        if self.item_stat(j, STAT_SPELL_BLOCK) <= 0 {
            self.block_until[j] = 0;
        }
        if self.block_log.len() < BLOCK_LOG_CAP {
            self.block_log.push((j as u32, caster as u32, ai as u16));
        }
        true
    }

    pub fn set_spell_block(&mut self, unit: usize, ticks: u32) {
        if unit < self.n && self.alive[unit] != 0 {
            self.block_until[unit] = self.ticks.saturating_add(ticks);
        }
    }

    pub fn spell_block_left(&self, unit: usize) -> u32 {
        self.block_until.get(unit).map_or(0, |&t| t.saturating_sub(self.ticks))
    }

    pub fn take_blocks(&mut self) -> Vec<(usize, usize, usize)> {
        std::mem::take(&mut self.block_log).into_iter().map(|(t, c, a)| (t as usize, c as usize, a as usize)).collect()
    }
}

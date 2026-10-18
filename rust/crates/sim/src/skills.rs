//! Skill trees. A unit's character level is its xp level plus one, so a new unit is level 1.
//! It has one skill point per character level, less the ranks it has spent. Spending a point
//! raises one ability's rank, and only when that ability is below its max rank and the
//! character level meets the level gate of the next rank. A learned rank sets the ability's
//! energy cost and cooldown, and its damage when the ability has a damage or heal effect.

use crate::arrays::Sim;
use crate::loader::Skill;
use crate::rules::Refusal;
use crate::ABILITY_SLOTS;

impl Sim {
    /// Character level of a unit: its xp level plus one.
    pub fn character_level(&self, unit: usize) -> u32 {
        self.level(unit) + 1
    }

    /// Skill points a unit has not spent: one per character level, less the ranks it holds.
    pub fn skill_points(&self, unit: usize) -> u32 {
        if unit >= self.cap {
            return 0;
        }
        let base = unit * ABILITY_SLOTS;
        let spent: u32 = self.abil_rank[base..base + ABILITY_SLOTS].iter().map(|&r| r as u32).sum();
        self.character_level(unit).saturating_sub(spent)
    }

    /// Rank of ability slot `slot` of unit `unit`. Zero when not learned or out of range.
    pub fn skill_rank(&self, unit: usize, slot: usize) -> u8 {
        if unit >= self.cap || slot >= ABILITY_SLOTS {
            return 0;
        }
        self.abil_rank[unit * ABILITY_SLOTS + slot]
    }

    /// Spends one skill point to raise ability slot `slot` of unit `unit` by a rank. Refusals,
    /// each changing nothing: a dead or unknown unit or an empty slot is NoSlot; an ability with
    /// no skill tree is NoSkillTree; a unit with no free point is NoSkillPoint; an ability at its
    /// max rank is SkillMaxed; a character level below the next rank's gate is SkillLevelGate.
    pub fn skill_up(&mut self, unit: usize, slot: usize) -> Result<(), Refusal> {
        if unit >= self.n || self.alive[unit] == 0 || slot >= ABILITY_SLOTS {
            return Err(self.reject(Refusal::NoSlot));
        }
        let idx = unit * ABILITY_SLOTS + slot;
        let ai = self.abil_idx[idx];
        if ai < 0 || ai as usize >= self.cat.abilities.len() {
            return Err(self.reject(Refusal::NoSlot));
        }
        let skill = match self.cat.abilities[ai as usize].skill {
            Some(skill) => skill,
            None => return Err(self.reject(Refusal::NoSkillTree)),
        };
        let rank = self.abil_rank[idx] as usize;
        if rank >= skill.max_rank as usize {
            return Err(self.reject(Refusal::SkillMaxed));
        }
        if self.skill_points(unit) == 0 {
            return Err(self.reject(Refusal::NoSkillPoint));
        }
        if self.character_level(unit) < skill.gates[rank] as u32 {
            return Err(self.reject(Refusal::SkillLevelGate));
        }
        self.abil_rank[idx] = (rank + 1) as u8;
        self.refresh_passives(unit);
        Ok(())
    }

    /// Damage of a learned rank of `skill` for the ability `ai` that unit `unit` holds, or None
    /// when no slot of the unit holds it at a rank above zero.
    pub(crate) fn skill_amount(&self, unit: usize, ai: usize, skill: Skill) -> Option<i32> {
        let base = unit * ABILITY_SLOTS;
        let k = (base..base + ABILITY_SLOTS).find(|&k| self.abil_idx[k] == ai as i32)?;
        let rank = self.abil_rank[k] as usize;
        (rank > 0).then(|| skill.damage[rank - 1])
    }
}

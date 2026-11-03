use crate::arrays::Sim;
use crate::loader::Skill;
use crate::rules::Refusal;
use crate::ABILITY_SLOTS;

impl Sim {
    pub fn ability_rank(&self, unit: usize, slot: usize) -> u8 {
        if unit >= self.cap || slot >= ABILITY_SLOTS {
            return 0;
        }
        self.abil_rank[unit * ABILITY_SLOTS + slot]
    }

    pub fn rank_tree(&self, unit: usize, slot: usize) -> Option<Skill> {
        if unit >= self.n || self.alive[unit] == 0 || slot >= ABILITY_SLOTS {
            return None;
        }
        let ai = self.abil_idx[unit * ABILITY_SLOTS + slot];
        if ai < 0 || ai as usize >= self.cat.abilities.len() {
            return None;
        }
        self.cat.abilities[ai as usize].skill
    }

    pub fn next_rank_gate(&self, unit: usize, slot: usize) -> Option<u32> {
        let tree = self.rank_tree(unit, slot)?;
        let rank = self.ability_rank(unit, slot) as usize;
        (rank < tree.max_rank as usize).then(|| tree.gates[rank] as u32)
    }

    pub fn rank_up(&mut self, unit: usize, slot: usize) -> Result<(), Refusal> {
        if unit >= self.n || self.alive[unit] == 0 || slot >= ABILITY_SLOTS {
            return Err(self.reject(Refusal::NoSlot));
        }
        let idx = unit * ABILITY_SLOTS + slot;
        let ai = self.abil_idx[idx];
        if ai < 0 || ai as usize >= self.cat.abilities.len() {
            return Err(self.reject(Refusal::NoSlot));
        }
        let tree = match self.cat.abilities[ai as usize].skill {
            Some(tree) => tree,
            None => return Err(self.reject(Refusal::NoSkillTree)),
        };
        let rank = self.abil_rank[idx] as usize;
        if rank >= tree.max_rank as usize {
            return Err(self.reject(Refusal::SkillMaxed));
        }
        self.abil_rank[idx] = (rank + 1) as u8;
        self.refresh_passives(unit);
        Ok(())
    }

    pub(crate) fn rank_amount(&self, unit: usize, ai: usize, tree: Skill) -> Option<i32> {
        let base = unit * ABILITY_SLOTS;
        let k = (base..base + ABILITY_SLOTS).find(|&k| self.abil_idx[k] == ai as i32)?;
        let rank = self.abil_rank[k] as usize;
        (rank > 0).then(|| tree.damage[rank - 1]).filter(|&amount| amount != 0)
    }
}

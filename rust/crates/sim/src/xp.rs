use crate::arrays::{Sim, DEFAULT_PERCENT, MAX_PERCENT};
use crate::MAX_PLAYERS;

impl Sim {
    pub fn xp_curve(&self) -> &[u32] {
        &self.xp_curve
    }

    pub fn xp(&self, unit: usize) -> u32 {
        if unit < self.cap {
            self.xp_total[unit]
        } else {
            0
        }
    }

    pub fn add_xp(&mut self, unit: usize, amount: u32) {
        if unit < self.cap {
            let rate = u64::from(self.xp_rate[usize::from(self.player[unit]).min(MAX_PLAYERS - 1)]);
            let scaled = u32::try_from(u64::from(amount) * rate / u64::from(DEFAULT_PERCENT)).unwrap_or(u32::MAX);
            self.xp_total[unit] = self.xp_total[unit].saturating_add(scaled);
        }
    }

    pub fn set_xp_rate(&mut self, player: usize, percent: u16) -> bool {
        if player >= MAX_PLAYERS || percent > MAX_PERCENT {
            return false;
        }
        self.xp_rate[player] = percent;
        true
    }

    pub fn xp_rate(&self, player: usize) -> u16 {
        self.xp_rate.get(player).copied().unwrap_or(DEFAULT_PERCENT)
    }

    pub fn set_unit_scale(&mut self, unit: usize, percent: u16) -> bool {
        if unit >= self.cap || percent > MAX_PERCENT {
            return false;
        }
        self.unit_scale[unit] = percent;
        true
    }

    pub fn unit_scale(&self, unit: usize) -> u16 {
        self.unit_scale.get(unit).copied().unwrap_or(DEFAULT_PERCENT)
    }

    pub fn unit_scales(&self) -> &[u16] {
        &self.unit_scale
    }
}


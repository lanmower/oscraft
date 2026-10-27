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

#[cfg(test)]
mod tests {
    use crate::{load, Sim};

    #[test]
    fn xp_curve_loads_from_data_and_matches_old_constant() {
        let scenario = crate::test_pack_dir();
        let image = osdt_data::generate(&scenario).expect("pack image");
        let s: Sim = load(&image, 1024).expect("pack sim");
        let expected: [u32; 24] = [
            200, 500, 900, 1400, 2000, 2600, 3200, 4400, 5400, 6000, 8200, 9000, 10400, 11900, 13500, 15200, 17000, 18900,
            20900, 23000, 25200, 27500, 29900, 32400,
        ];
        assert_eq!(s.xp_curve(), &expected[..]);
        assert_eq!(s.catalog().tune.xp_curve, expected.to_vec());
    }
}

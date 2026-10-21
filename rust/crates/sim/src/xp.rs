//! Experience: a per-unit xp counter read against a level curve. A unit's level is how many
//! curve thresholds its xp has reached, so a level-up is a rise in that count. The curve is
//! data the sim carries: the scenario's TUNE section supplies it at load, and a preset replaces
//! it through `set_xp_curve`.

use crate::arrays::Sim;

impl Sim {
    /// The level curve in use: cumulative xp thresholds, nondecreasing.
    pub fn xp_curve(&self) -> &[u32] {
        &self.xp_curve
    }

    /// Replaces the level curve. False, with the curve unchanged, when it is not nondecreasing.
    pub fn set_xp_curve(&mut self, curve: &[u32]) -> bool {
        if !curve.windows(2).all(|w| w[0] <= w[1]) {
            return false;
        }
        self.xp_curve = curve.to_vec();
        for i in 0..self.n {
            self.apply_hero_stats(i);
        }
        true
    }

    /// Xp held by a unit. Zero for an out-of-range unit.
    pub fn xp(&self, unit: usize) -> u32 {
        if unit < self.cap {
            self.xp_total[unit]
        } else {
            0
        }
    }

    /// Level of a unit: the number of curve thresholds its xp has reached.
    pub fn level(&self, unit: usize) -> u32 {
        let xp = self.xp(unit);
        self.xp_curve.iter().take_while(|&&t| xp >= t).count() as u32
    }

    /// Adds xp to a unit (saturating) and returns the levels it gained. Zero for an
    /// out-of-range unit.
    pub fn add_xp(&mut self, unit: usize, amount: u32) -> u32 {
        if unit >= self.cap {
            return 0;
        }
        let before = self.level(unit);
        self.xp_total[unit] = self.xp_total[unit].saturating_add(amount);
        let gained = self.level(unit) - before;
        if gained > 0 {
            self.apply_hero_stats(unit);
        }
        gained
    }
}

#[cfg(test)]
mod tests {
    use crate::{load, Sim};

    /// The level curve the moba scenario's TUNE section carries must match the thresholds the sim
    /// used before the curve moved into data.
    #[test]
    fn xp_curve_loads_from_data_and_matches_old_constant() {
        let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..").join("scenarios").join("moba");
        let image = osdt_data::generate(&scenario).expect("moba image");
        let s: Sim = load(&image, 1024).expect("moba sim");
        let expected: [u32; 29] = [
            230, 600, 1080, 1660, 2260, 2980, 3730, 4620, 5550, 6520, 7530, 8580, 9805, 11055, 12330, 13630, 14955, 16455,
            18045, 19645, 21495, 23595, 25945, 28545, 31395, 34495, 37895, 41595, 45595,
        ];
        assert_eq!(s.xp_curve(), &expected[..]);
        assert_eq!(s.catalog().tune.xp_curve, expected.to_vec());
    }
}

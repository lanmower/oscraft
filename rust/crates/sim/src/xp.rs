//! Experience: a per-unit xp counter, and the level curve it is read against. The curve is data the sim
//! carries: the scenario's TUNE section supplies it at load. The level a unit's xp reaches on that curve is
//! the mod's rule (scenarios/moba/levelup.lua reads `xp` and `xp_curve` through natives), so the engine
//! stores xp and adds to it, and says nothing about levels.

use crate::arrays::Sim;

impl Sim {
    /// The level curve in use: cumulative xp thresholds, nondecreasing (checked at load).
    pub fn xp_curve(&self) -> &[u32] {
        &self.xp_curve
    }

    /// Xp held by a unit. Zero for an out-of-range unit.
    pub fn xp(&self, unit: usize) -> u32 {
        if unit < self.cap {
            self.xp_total[unit]
        } else {
            0
        }
    }

    /// Adds xp to a unit (saturating). Nothing happens for an out-of-range unit. The levels the gain reaches
    /// are the mod's rule: it reads `xp` against the curve each tick.
    pub fn add_xp(&mut self, unit: usize, amount: u32) {
        if unit < self.cap {
            self.xp_total[unit] = self.xp_total[unit].saturating_add(amount);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{load, Sim};

    /// The level curve the pack's TUNE section carries must match the thresholds the sim
    /// used before the curve moved into data.
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

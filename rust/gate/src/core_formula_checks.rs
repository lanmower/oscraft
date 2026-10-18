//! Gate checks for the Dota core formulas the sim carries. Each asserts one postcondition through
//! the public sim API.

use crate::checks::fresh;
use sim::fixed;

fn near(got: i32, want: i32) -> bool {
    (got - want).abs() <= 2
}

/// Physical damage is scaled by 1 - 0.06a / (1 + 0.06|a|): armour 10 keeps 62.5 percent, armour -10
/// takes 137.5 percent, armour 20 keeps 45.45 percent, armour 0 changes nothing.
pub fn armour_formula(_image: &[u8]) -> Result<(), String> {
    for (armour, want) in [(0, 1000), (10, 625), (-10, 1375), (20, 454), (23, 420)] {
        let got = sim::armour_reduced(1000, fixed::from_int(armour));
        if !near(got, want) {
            return Err(format!("armour {armour}: 1000 damage became {got}, want about {want}"));
        }
    }
    Ok(())
}

/// The level curve holds Dota's 29 thresholds (levels 2 to 30), first at 230 xp, nondecreasing.
pub fn xp_table(image: &[u8]) -> Result<(), String> {
    let s = fresh(image)?;
    let curve = s.xp_curve();
    if curve.len() != 29 || curve[0] != 230 || !curve.windows(2).all(|w| w[0] < w[1]) {
        return Err(format!("xp curve has {} thresholds, first {:?}", curve.len(), curve.first()));
    }
    Ok(())
}

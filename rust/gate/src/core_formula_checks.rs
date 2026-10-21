//! Gate checks for the reference core formulas the sim carries. Each asserts one postcondition through
//! the public sim API.

use crate::checks::fresh;

/// The level curve holds reference's 29 thresholds (levels 2 to 30), first at 230 xp, nondecreasing.
pub fn xp_table(image: &[u8]) -> Result<(), String> {
    let s = fresh(image)?;
    let curve = s.xp_curve();
    if curve.len() != 29 || curve[0] != 230 || !curve.windows(2).all(|w| w[0] < w[1]) {
        return Err(format!("xp curve has {} thresholds, first {:?}", curve.len(), curve.first()));
    }
    Ok(())
}

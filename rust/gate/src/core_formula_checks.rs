use crate::checks::fresh;

pub fn xp_table(image: &[u8]) -> Result<(), String> {
    let s = fresh(image)?;
    let curve = s.xp_curve();
    if curve.len() != 24 || curve[0] != 200 || curve[23] != 32400 || !curve.windows(2).all(|w| w[0] < w[1]) {
        return Err(format!("xp curve has {} thresholds, first {:?}, last {:?}", curve.len(), curve.first(), curve.last()));
    }
    Ok(())
}

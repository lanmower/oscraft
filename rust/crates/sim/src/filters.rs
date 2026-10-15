//! Target filters. The filter mask on each unit is checked through `Sim::meets_filter`
//! in rules.rs; this module holds the spell-target filter decision. Abilities carry
//! no filter field in format v2, so a cast passes a zero mask and is always met.

use crate::arrays::Sim;

impl Sim {
    /// Whether unit `j` may be the target of an effect that requires `required`.
    pub(crate) fn target_allowed(&self, j: usize, required: u32) -> bool {
        self.meets_filter(j, required)
    }
}

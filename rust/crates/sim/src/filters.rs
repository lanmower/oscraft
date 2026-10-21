//! Target filters. The filter mask on each unit is checked through `Sim::meets_filter`
//! in rules.rs; this module holds the spell-target filter decision. Abilities carry
//! no filter field in format v2, so a cast passes a zero mask and is always met.

use crate::arrays::Sim;

impl Sim {
}

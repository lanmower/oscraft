//! Carries the live units of a sim in service into a freshly loaded image of new data. The server
//! and the browser swap call this one function, so both sides place and drop the same units.

use crate::Sim;

/// Carries the live state of `old` into `new`: the clock first, so `new` continues the match's tick count, then the
/// live units without new ids. A live unit keeps its index and moves
/// to its old point when `new` places a unit of the same player and the same kind name there; any
/// other unit `new` placed is removed, so no unit is duplicated. A carried unit keeps its old health
/// and maximum. Returns `(carried, refused)`: refused counts the old live units that found no such slot.
pub fn carry_from(old: &Sim, new: &mut Sim) -> (usize, usize) {
    new.ticks = old.ticks;
    let mut carried = 0;
    for i in 0..new.count() {
        if !new.alive(i) {
            continue;
        }
        let same = i < old.count()
            && old.alive(i)
            && old.player(i) == new.player(i)
            && old.kind_name(i).is_some()
            && old.kind_name(i) == new.kind_name(i);
        if same {
            let (x, y) = old.position_fixed(i);
            new.set_position(i, x, y);
            new.set_health(i, old.hp(i), old.hp_max(i));
            carried += 1;
        } else {
            new.remove_unit(i);
        }
    }
    (carried, old.alive_count().saturating_sub(carried))
}

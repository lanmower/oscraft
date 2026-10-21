//! The `grant_upgrade` effect: the receiver (the caster when no unit is targeted) gains a unit upgrade for
//! good, p0 0 for Aghanim's Scepter and 1 for Aghanim's Shard. An item holding either upgrade while carried
//! gets the same result from its `scepter` and `shard` stats (see `change_items`).

use crate::arrays::Sim;
use crate::loader::Effect;

pub(crate) const K_GRANT: u16 = 33;

impl Sim {
    pub(crate) fn grant_effect(&mut self, caster: usize, eff: Effect, receiver: Option<usize>) {
        let j = receiver.unwrap_or(caster);
        self.grant_upgrade(j, eff.p0 == 1, true);
    }
}

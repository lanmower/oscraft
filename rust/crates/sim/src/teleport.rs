//! The `teleport` effect: the caster moves to the cast point if the point is walkable. The engine knows
//! nothing about landing rings, receivers or town portals: a mod that wants a landing ring (the pack's
//! spells.lua) tries points around its receiver with `Sim::teleport` through the host, and the first that
//! takes hold is the landing. This effect is the generic move-to-point primitive.

use crate::arrays::Sim;
use crate::rules::Refusal;

pub(crate) const K_TELEPORT: u16 = 31;

impl Sim {
    /// Moves `caster` to (tx, ty). Refused, changing nothing, when the caster is rooted or the point is not
    /// walkable for it.
    pub(crate) fn teleport_effect(&mut self, caster: usize, tx: i32, ty: i32) {
        if self.rooted(caster) {
            self.refuse(Refusal::Rooted);
            return;
        }
        if !self.teleport(caster, tx, ty) {
            self.refuse(Refusal::NoTeleport);
        }
    }
}

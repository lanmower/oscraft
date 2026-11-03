use crate::arrays::Sim;
use crate::rules::Refusal;

pub(crate) const K_POINT_MOVE: u16 = 31;

impl Sim {
    pub(crate) fn point_move_effect(&mut self, caster: usize, tx: i32, ty: i32) {
        if self.rooted(caster) {
            self.refuse(Refusal::Rooted);
            return;
        }
        if !self.teleport(caster, tx, ty) {
            self.refuse(Refusal::NoTeleport);
        }
    }
}

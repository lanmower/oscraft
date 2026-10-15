//! The per-cell unit limit from the LIMT section (format v10): a cell may hold at most that many
//! living units at once. A placement into a full cell is refused as CellFull. Movement into a
//! full cell is not limited by this module.

use crate::arrays::{cell_of, Sim};

impl Sim {
    /// Whether one more living unit may occupy the cell under the Q16 point (x, y). Always true
    /// when the map sets no limit (LIMT 0).
    pub(crate) fn cell_has_room(&self, x: i32, y: i32) -> bool {
        let limit = self.cat.limit as usize;
        if limit == 0 {
            return true;
        }
        let (cx, cy) = (cell_of(x), cell_of(y));
        let held = (0..self.n)
            .filter(|&j| self.alive[j] != 0 && cell_of(self.pos_x[j]) == cx && cell_of(self.pos_y[j]) == cy)
            .count();
        held < limit
    }
}

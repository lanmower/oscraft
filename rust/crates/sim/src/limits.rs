use crate::arrays::{cell_of, Sim};

impl Sim {
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

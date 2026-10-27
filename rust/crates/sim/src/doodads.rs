//! Doodads on the map. The set starts as the map's DODA records and is state from then on: it is
//! hashed and saved, and the mod host places and removes doodads through these methods. Doodads
//! never block movement; walkability is in the grid.

use crate::arrays::Sim;
use crate::loader::{Doodad, CELL_MILLI};
use crate::rules::Refusal;

impl Sim {
    /// Places a doodad. Refused OffGrid when its position lies outside the map, its kind is at or above
    /// the scenario's DKND list, or its rotation is outside 0..1000. Refused DuplicateDoodad when its id is already placed.
    pub fn place_doodad(&mut self, d: Doodad) -> Result<(), Refusal> {
        let w = self.grid.w as i64 * CELL_MILLI as i64;
        let h = self.grid.h as i64 * CELL_MILLI as i64;
        if d.kind as usize >= self.cat.names.doodad_kinds.len() ||!(0..1000).contains(&d.rotation) || d.x < 0 || d.y < 0 || d.x as i64 >= w || d.y as i64 >= h {
            return Err(self.reject(Refusal::OffGrid));
        }
        if self.doodads.iter().any(|e| e.id == d.id) {
            return Err(self.reject(Refusal::DuplicateDoodad));
        }
        self.doodads.push(d);
        Ok(())
    }

    /// Removes the doodad with id `id` and returns it. Refused NoDoodad when no placed doodad has that id.
    pub fn remove_doodad(&mut self, id: u32) -> Result<Doodad, Refusal> {
        match self.doodads.iter().position(|e| e.id == id) {
            Some(k) => Ok(self.doodads.remove(k)),
            None => Err(self.reject(Refusal::NoDoodad)),
        }
    }

    /// The placed doodads, in placement order.
    pub fn doodad_list(&self) -> &[Doodad] {
        &self.doodads
    }
}

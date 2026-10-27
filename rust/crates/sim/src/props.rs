use crate::arrays::Sim;
use crate::loader::{Prop, CELL_MILLI};
use crate::rules::Refusal;

impl Sim {
    pub fn place_prop(&mut self, d: Prop) -> Result<(), Refusal> {
        let w = self.grid.w as i64 * CELL_MILLI as i64;
        let h = self.grid.h as i64 * CELL_MILLI as i64;
        if d.kind as usize >= self.cat.names.prop_kinds.len() ||!(0..1000).contains(&d.rotation) || d.x < 0 || d.y < 0 || d.x as i64 >= w || d.y as i64 >= h {
            return Err(self.reject(Refusal::OffGrid));
        }
        if self.props.iter().any(|e| e.id == d.id) {
            return Err(self.reject(Refusal::DuplicateProp));
        }
        self.props.push(d);
        Ok(())
    }

    pub fn remove_prop(&mut self, id: u32) -> Result<Prop, Refusal> {
        match self.props.iter().position(|e| e.id == id) {
            Some(k) => Ok(self.props.remove(k)),
            None => Err(self.reject(Refusal::NoProp)),
        }
    }

    pub fn prop_list(&self) -> &[Prop] {
        &self.props
    }
}

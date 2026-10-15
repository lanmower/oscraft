//! Transports: a ground unit boards a transport up to its capacity in size units and leaves it
//! at a point. A boarded unit is held out of play (not alive) and records its transport.

use crate::arrays::{cell_of, Sim};
use crate::rules::Refusal;
use crate::*;

/// Capacity a unit takes in a transport, by its UEXT size category: small 1, medium 2, large 4.
fn size_units(size: u8) -> i32 {
    match size {
        0 => 1,
        1 => 2,
        _ => 4,
    }
}

impl Sim {
    /// Capacity unit `j` takes in a transport.
    pub(crate) fn unit_size(&self, j: usize) -> i32 {
        self.cat.unit_ext.get(self.type_of[j] as usize).map_or(1, |e| size_units(e.size))
    }

    /// Capacity of unit `t`'s type (UEXT transport, size units); 0 when it is not a transport.
    fn transport_capacity(&self, t: usize) -> i32 {
        self.cat.unit_ext.get(self.type_of[t] as usize).map_or(0, |e| e.transport as i32)
    }

    /// Boards ground unit `i` into transport `t`. Refusals, each changing nothing: an unknown
    /// or dead unit, or a unit that is its own transport, is NoTarget; a flying unit, a unit of
    /// another player, a unit whose transport has no capacity, or a unit not beside the transport
    /// is InvalidOrderTarget; a boarding past the capacity is TransportFull.
    pub fn load_unit(&mut self, t: usize, i: usize) -> Result<(), Refusal> {
        if t >= self.n || i >= self.n || t == i || self.alive[t] == 0 || self.alive[i] == 0 {
            return Err(self.reject(Refusal::NoTarget));
        }
        if self.transport_capacity(t) <= 0
            || self.flying[i] != 0
            || self.player[i] != self.player[t]
            || !self.arrived(i, t)
        {
            return Err(self.reject(Refusal::InvalidOrderTarget));
        }
        let used: i32 = (0..self.n)
            .filter(|&j| self.loaded_into[j] == t as i32)
            .map(|j| self.unit_size(j))
            .sum();
        if used + self.unit_size(i) > self.transport_capacity(t) {
            return Err(self.reject(Refusal::TransportFull));
        }
        self.loaded_into[i] = t as i32;
        self.alive[i] = 0;
        self.touch(i);
        Ok(())
    }

    /// Unloads every unit boarded in transport `t` at the transport's own position. Refusals,
    /// changing nothing: an unknown or dead transport is NoTarget. Returns the units unloaded.
    pub fn unload_transport(&mut self, t: usize) -> Result<usize, Refusal> {
        if t >= self.n || self.alive[t] == 0 {
            return Err(self.reject(Refusal::NoTarget));
        }
        let (x, y) = (self.pos_x[t], self.pos_y[t]);
        Ok(self.unload_at(t, x, y))
    }

    /// Puts every unit boarded in transport `t` back at (x, y) and alive again.
    fn unload_at(&mut self, t: usize, x: i32, y: i32) -> usize {
        let mut count = 0usize;
        for j in 0..self.n {
            if self.loaded_into[j] != t as i32 {
                continue;
            }
            self.loaded_into[j] = -1;
            self.alive[j] = 1;
            self.pos_x[j] = x;
            self.pos_y[j] = y;
            self.tgt_x[j] = x;
            self.tgt_y[j] = y;
            self.vel_x[j] = 0;
            self.vel_y[j] = 0;
            self.routed[j] = 0;
            self.blocked[j] = 0;
            self.touch(j);
            count += 1;
        }
        count
    }

}

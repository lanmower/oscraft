use crate::arrays::{dist2, Sim};
use crate::fixed;
use crate::store::ItemRequest;

#[derive(Clone, Debug)]
pub(crate) struct OrderMemory {
    pub(crate) set: Vec<u8>,
    pub(crate) x: Vec<i32>,
    pub(crate) y: Vec<i32>,
}

impl OrderMemory {
    pub(crate) fn new(cap: usize) -> OrderMemory {
        OrderMemory { set: vec![0; cap], x: vec![0; cap], y: vec![0; cap] }
    }

    pub(crate) fn clear(&mut self, i: usize) {
        self.set[i] = 0;
    }
}

impl Sim {
    pub fn save_order(&mut self, i: usize, x: i32, y: i32) -> bool {
        if i >= self.n {
            return false;
        }
        let m = &mut self.order_memory;
        m.set[i] = 1;
        m.x[i] = x;
        m.y[i] = y;
        true
    }

    pub fn saved_order(&self, i: usize) -> Option<(i32, i32)> {
        (self.order_memory.set.get(i).copied() == Some(1)).then(|| (self.order_memory.x[i], self.order_memory.y[i]))
    }

    pub fn restore_order(&mut self, i: usize) -> bool {
        match self.saved_order(i) {
            Some((x, y)) if self.alive(i) => {
                self.order(i, x, y);
                true
            }
            _ => false,
        }
    }

    pub fn within_reach(&self, i: usize, x: i32, y: i32, reach_milli: i32) -> bool {
        if i >= self.n {
            return false;
        }
        let reach = fixed::from_milli(reach_milli) as i64;
        dist2(self.pos_x[i], self.pos_y[i], x, y) <= reach * reach
    }

    pub(crate) fn friendly_request_effect(&mut self, caster: usize, target: i32) {
        if target < 0 || target as usize >= self.n {
            return;
        }
        self.item_requests.requests.push(ItemRequest::Carry { caster, target: target as usize });
    }
}

use crate::arrays::Sim;
use crate::rules::Refusal;
use crate::{ORDER_ATTACK_MOVE, ORDER_MOVE, ORDER_STOP};

pub const QUEUE_MAX: usize = 8;
pub const QUEUE_ATTACK: u8 = 0xFF;
const ARRIVE_REACH_MILLI: i32 = 32_768;

#[derive(Clone, Debug)]
pub(crate) struct OrderQueue {
    pub(crate) len: Vec<u8>,
    pub(crate) kind: Vec<u8>,
    pub(crate) x: Vec<i32>,
    pub(crate) y: Vec<i32>,
    pub(crate) target: Vec<i32>,
}

impl OrderQueue {
    pub(crate) fn new(cap: usize) -> OrderQueue {
        let slots = cap * QUEUE_MAX;
        OrderQueue {
            len: vec![0; cap],
            kind: vec![0; slots],
            x: vec![0; slots],
            y: vec![0; slots],
            target: vec![-1; slots],
        }
    }

    fn reset_slot(&mut self, s: usize) {
        self.kind[s] = 0;
        self.x[s] = 0;
        self.y[s] = 0;
        self.target[s] = -1;
    }

    pub(crate) fn clear(&mut self, i: usize) {
        let base = i * QUEUE_MAX;
        for s in base..base + usize::from(self.len[i]) {
            self.reset_slot(s);
        }
        self.len[i] = 0;
    }

    fn push(&mut self, i: usize, kind: u8, x: i32, y: i32, target: i32) {
        let s = i * QUEUE_MAX + usize::from(self.len[i]);
        self.kind[s] = kind;
        self.x[s] = x;
        self.y[s] = y;
        self.target[s] = target;
        self.len[i] += 1;
    }

    fn pop(&mut self, i: usize) -> (u8, i32, i32, i32) {
        let base = i * QUEUE_MAX;
        let end = base + usize::from(self.len[i]);
        let head = (self.kind[base], self.x[base], self.y[base], self.target[base]);
        self.kind.copy_within(base + 1..end, base);
        self.x.copy_within(base + 1..end, base);
        self.y.copy_within(base + 1..end, base);
        self.target.copy_within(base + 1..end, base);
        self.reset_slot(end - 1);
        self.len[i] -= 1;
        head
    }
}

impl Sim {
    pub fn queue_order(&mut self, i: usize, kind: u8, x: i32, y: i32, target: i32) -> Result<(), Refusal> {
        if i >= self.n || self.alive[i] == 0 {
            return Err(self.reject(Refusal::NoTarget));
        }
        match kind {
            ORDER_MOVE | ORDER_ATTACK_MOVE => {}
            QUEUE_ATTACK if target < 0 || target as usize >= self.n => {
                return Err(self.reject(Refusal::InvalidOrderTarget));
            }
            QUEUE_ATTACK => {}
            _ => return Err(self.reject(Refusal::InvalidOrderKind)),
        }
        if usize::from(self.order_queue.len[i]) >= QUEUE_MAX {
            return Err(self.reject(Refusal::NoSlot));
        }
        if kind == QUEUE_ATTACK {
            self.order_queue.push(i, kind, 0, 0, target);
        } else {
            self.order_queue.push(i, kind, x, y, -1);
        }
        Ok(())
    }

    pub fn queue_clear(&mut self, i: usize) -> bool {
        let known = i < self.n;
        if known {
            self.order_queue.clear(i);
        }
        known
    }

    pub fn queue_len(&self, i: usize) -> usize {
        self.order_queue.len.get(i).map_or(0, |&n| usize::from(n))
    }

    pub(crate) fn advance_order_queue(&mut self) {
        for i in 0..self.n {
            if self.order_queue.len[i] == 0 {
                continue;
            }
            if self.alive[i] == 0 {
                self.order_queue.clear(i);
                continue;
            }
            let idle = self.order_kind[i] == ORDER_STOP
                || self.within_reach(i, self.tgt_x[i], self.tgt_y[i], ARRIVE_REACH_MILLI);
            if !idle {
                continue;
            }
            let (kind, x, y, target) = self.order_queue.pop(i);
            let _ = if kind == QUEUE_ATTACK {
                self.order_attack_or_approach(i, target as usize).map(|_| ())
            } else {
                self.issue_order_kind(i, kind, x, y)
            };
        }
    }
}

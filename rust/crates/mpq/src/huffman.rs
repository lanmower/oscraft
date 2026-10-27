use crate::huffman_tables::HUFFMAN_WEIGHTS;
use crate::{Error, Result};

const ITEM_COUNT: usize = 515;
const NODE_COUNT: usize = ITEM_COUNT + 1;
const HEAD: i32 = ITEM_COUNT as i32;
const BYTE_ITEMS: usize = 258;
const ESCAPE: u32 = 0x101;
const TERMINATOR: u32 = 0x100;
const DECODE_ERR: u32 = 0x1ff;

#[derive(Clone, Copy)]
struct Node {
    value: u32,
    weight: u32,
    parent: i32,
    child_lo: i32,
    next: i32,
    prev: i32,
}

struct Tree {
    nodes: [Node; NODE_COUNT],
    items_used: usize,
    items_by_byte: [i32; BYTE_ITEMS],
}

impl Tree {
    fn new() -> Tree {
        let blank = Node {
            value: 0,
            weight: 0,
            parent: -1,
            child_lo: -1,
            next: -1,
            prev: -1,
        };
        let mut t = Tree {
            nodes: [blank; NODE_COUNT],
            items_used: 0,
            items_by_byte: [-1; BYTE_ITEMS],
        };
        t.nodes[HEAD as usize].next = HEAD;
        t.nodes[HEAD as usize].prev = HEAD;
        t
    }

    fn nd(&self, i: i32) -> &Node {
        &self.nodes[i as usize]
    }

    fn nd_mut(&mut self, i: i32) -> &mut Node {
        &mut self.nodes[i as usize]
    }

    fn remove_item(&mut self, i: i32) {
        if self.nd(i).next < 0 {
            return;
        }
        let prev = self.nd(i).prev;
        let next = self.nd(i).next;
        self.nd_mut(prev).next = next;
        self.nd_mut(next).prev = prev;
        self.nd_mut(i).next = -1;
        self.nd_mut(i).prev = -1;
    }

    fn link_two_items(&mut self, i1: i32, i2: i32) {
        let after = self.nd(i1).next;
        self.nd_mut(i2).next = after;
        let after_prev = self.nd(after).prev;
        self.nd_mut(i2).prev = after_prev;
        self.nd_mut(after).prev = i2;
        self.nd_mut(i1).next = i2;
    }

    fn insert_item(&mut self, i: i32, after: bool) {
        self.remove_item(i);
        if after {
            self.link_two_items(HEAD, i);
        } else {
            let p = self.nd(HEAD).prev;
            self.link_two_items(p, i);
        }
    }

    fn find_higher_or_equal(&self, mut i: i32, weight: u32) -> i32 {
        while i != HEAD {
            if self.nd(i).weight >= weight {
                return i;
            }
            i = self.nd(i).prev;
        }
        HEAD
    }

    fn create_new_item(&mut self, value: u32, weight: u32, after: bool) -> i32 {
        if self.items_used >= ITEM_COUNT {
            return -1;
        }
        let i = self.items_used as i32;
        self.items_used += 1;
        self.insert_item(i, after);
        let n = self.nd_mut(i);
        n.value = value;
        n.weight = weight;
        i
    }

    fn fixup_item_pos_by_weight(&mut self, i: i32, max_weight: u32) -> u32 {
        let w = self.nd(i).weight;
        if w >= max_weight {
            return w;
        }
        let tail = self.nd(HEAD).prev;
        let higher = self.find_higher_or_equal(tail, w);
        self.remove_item(i);
        self.link_two_items(higher, i);
        max_weight
    }

    fn build_tree(&mut self, data_type: u32) -> bool {
        let dt = (data_type & 0x0f) as usize;
        if dt >= HUFFMAN_WEIGHTS.len() {
            return false;
        }
        let table = &HUFFMAN_WEIGHTS[dt];
        let mut max_weight = 0u32;
        for sym in 0..256usize {
            if table[sym] == 0 {
                continue;
            }
            let i = self.create_new_item(sym as u32, table[sym] as u32, true);
            if i < 0 {
                return false;
            }
            self.items_by_byte[sym] = i;
            max_weight = self.fixup_item_pos_by_weight(i, max_weight);
        }
        for term in [TERMINATOR, ESCAPE] {
            let i = self.create_new_item(term, 1, false);
            if i < 0 {
                return false;
            }
            self.items_by_byte[term as usize] = i;
        }
        let mut child_lo = self.nd(HEAD).prev;
        while child_lo != HEAD {
            let child_hi = self.nd(child_lo).prev;
            if child_hi == HEAD {
                break;
            }
            let w = self
                .nd(child_hi)
                .weight
                .wrapping_add(self.nd(child_lo).weight);
            let i = self.create_new_item(0, w, true);
            if i < 0 {
                return false;
            }
            self.nd_mut(child_lo).parent = i;
            self.nd_mut(child_hi).parent = i;
            self.nd_mut(i).child_lo = child_lo;
            max_weight = self.fixup_item_pos_by_weight(i, max_weight);
            child_lo = self.nd(child_hi).prev;
        }
        true
    }

    fn inc_weights_and_rebalance(&mut self, start: i32) -> bool {
        let mut i = start;
        let mut steps = 0usize;
        while i >= 0 {
            if steps > NODE_COUNT {
                return false;
            }
            steps += 1;
            let w = self.nd(i).weight.wrapping_add(1);
            self.nd_mut(i).weight = w;
            let prev = self.nd(i).prev;
            let higher = self.find_higher_or_equal(prev, w);
            let child_hi = self.nd(higher).next;
            if child_hi != i {
                self.remove_item(child_hi);
                self.link_two_items(i, child_hi);
                self.remove_item(i);
                self.link_two_items(higher, i);

                let hp = self.nd(child_hi).parent;
                if hp < 0 {
                    return false;
                }
                let child_lo = self.nd(hp).child_lo;
                let parent = self.nd(i).parent;
                if parent >= 0 && self.nd(parent).child_lo == i {
                    self.nd_mut(parent).child_lo = child_hi;
                }
                if child_lo == child_hi {
                    let p = self.nd(child_hi).parent;
                    if p >= 0 {
                        self.nd_mut(p).child_lo = i;
                    }
                }
                let parent2 = self.nd(i).parent;
                self.nd_mut(i).parent = self.nd(child_hi).parent;
                self.nd_mut(child_hi).parent = parent2;
            }
            i = self.nd(i).parent;
        }
        true
    }

    fn insert_new_branch_and_rebalance(&mut self, value1: u32, value2: u32) -> bool {
        let last = self.nd(HEAD).prev;
        let w = self.nd(last).weight;
        let hi = self.create_new_item(value1, w, false);
        if hi < 0 {
            return false;
        }
        self.nd_mut(hi).parent = last;
        self.items_by_byte[value1 as usize] = hi;
        let lo = self.create_new_item(value2, 0, false);
        if lo < 0 {
            return false;
        }
        self.nd_mut(lo).parent = last;
        self.nd_mut(last).child_lo = lo;
        self.items_by_byte[value2 as usize] = lo;
        self.inc_weights_and_rebalance(lo)
    }

    fn decode_one_byte(&self, r: &mut Bits) -> u32 {
        if self.nd(HEAD).next == HEAD {
            return DECODE_ERR;
        }
        let mut item = self.nd(HEAD).next;
        let mut steps = 0usize;
        while self.nd(item).child_lo >= 0 {
            if steps > NODE_COUNT {
                return DECODE_ERR;
            }
            steps += 1;
            let bit = match r.get_1bit() {
                Some(b) => b,
                None => return DECODE_ERR,
            };
            let child = self.nd(item).child_lo;
            item = if bit != 0 {
                self.nd(child).prev
            } else {
                child
            };
        }
        self.nd(item).value
    }
}

struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
    buf: u32,
    cnt: u32,
}

impl Bits<'_> {
    fn refill(&mut self) -> bool {
        if self.pos >= self.data.len() {
            return false;
        }
        self.buf |= (self.data[self.pos] as u32) << self.cnt;
        self.pos += 1;
        self.cnt += 8;
        true
    }

    fn get_1bit(&mut self) -> Option<u32> {
        if self.cnt == 0 && !self.refill() {
            return None;
        }
        let v = self.buf & 1;
        self.buf >>= 1;
        self.cnt -= 1;
        Some(v)
    }

    fn get_8bits(&mut self) -> Option<u32> {
        if self.cnt < 8 && !self.refill() {
            return None;
        }
        let v = self.buf & 0xff;
        self.buf >>= 8;
        self.cnt -= 8;
        Some(v)
    }
}

pub(crate) fn inflate(body: &[u8], want: u32) -> Result<Vec<u8>> {
    if want == 0 {
        return Ok(Vec::new());
    }
    let mut t = Tree::new();
    let mut r = Bits {
        data: body,
        pos: 0,
        buf: 0,
        cnt: 0,
    };
    let data_type = r
        .get_8bits()
        .ok_or_else(|| Error::Corrupt("mpq: huffman: stream has no data-type byte".into()))?;
    if !t.build_tree(data_type) {
        return Err(Error::Corrupt(format!(
            "mpq: huffman: data type {}",
            data_type & 0x0f
        )));
    }
    let sparse = data_type == 0;

    let mut out: Vec<u8> = Vec::with_capacity(want as usize);
    loop {
        let mut v = t.decode_one_byte(&mut r);
        match v {
            TERMINATOR => return Ok(out),
            DECODE_ERR => return Err(Error::Corrupt("mpq: huffman: invalid code".into())),
            _ => {}
        }
        if v == ESCAPE {
            v = r
                .get_8bits()
                .ok_or_else(|| Error::Corrupt("mpq: huffman: escape with no byte after it".into()))?;
            let last = t.nd(HEAD).prev;
            let last_value = t.nd(last).value;
            if !t.insert_new_branch_and_rebalance(last_value, v) {
                return Err(Error::Corrupt("mpq: huffman: tree item pool exhausted".into()));
            }
            if !sparse {
                let item = t.items_by_byte[v as usize];
                if !t.inc_weights_and_rebalance(item) {
                    return Err(Error::Corrupt("mpq: huffman: rebalance overflow".into()));
                }
            }
        }
        if out.len() as u32 >= want {
            return Ok(out);
        }
        out.push(v as u8);
        if sparse {
            let item = t.items_by_byte[v as usize];
            if !t.inc_weights_and_rebalance(item) {
                return Err(Error::Corrupt("mpq: huffman: rebalance overflow".into()));
            }
        }
    }
}

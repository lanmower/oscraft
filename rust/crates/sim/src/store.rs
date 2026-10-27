use crate::arrays::Sim;
use crate::fixed;
use crate::inventory::*;
use crate::loader::Prop;
use crate::rules::Refusal;

pub const NO_TAKE: u32 = u32::MAX;
pub const VERDICT_TAKES: usize = ITEM_COMPONENTS;
pub const NO_COMPONENT: u16 = crate::loader::NO_INDEX;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemRequest {
    Acquire { unit: usize, item: usize },
    Release { unit: usize, slot: usize },
    TakeOut { unit: usize, slot: usize },
    Carry { caster: usize, target: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemVerdict {
    Acquire { unit: usize, item: usize, allow: bool, deny: Refusal, debit: i32, takes: [u32; ITEM_COMPONENTS] },
    Release { unit: usize, slot: usize, allow: bool, deny: Refusal, credit: i32 },
    TakeOut { unit: usize, slot: usize, allow: bool, deny: Refusal },
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ItemRequestState {
    pub(crate) requests: Vec<ItemRequest>,
}

type Bank = [(u32, u32); INV_SLOTS];

impl Sim {
    pub fn buy_item(&mut self, i: usize, item: usize) -> bool {
        if !self.trade_unit_ok(i) {
            return false;
        }
        if item >= self.cat.items.len() {
            self.refuse(Refusal::NoItem);
            return false;
        }
        self.item_requests.requests.push(ItemRequest::Acquire { unit: i, item });
        true
    }

    pub fn sell_item(&mut self, i: usize, slot: usize) -> bool {
        if !self.trade_unit_ok(i) || slot >= INV_SLOTS {
            self.refuse(Refusal::NoSlot);
            return false;
        }
        if self.inv_count[i * INV_SLOTS + slot] == 0 {
            self.refuse(Refusal::NoItem);
            return false;
        }
        self.item_requests.requests.push(ItemRequest::Release { unit: i, slot });
        true
    }

    pub fn unstash(&mut self, i: usize, slot: usize) -> bool {
        if !self.trade_unit_ok(i) || slot >= STASH_SLOTS {
            self.refuse(Refusal::NoSlot);
            return false;
        }
        if self.stash_count[i * INV_SLOTS + slot] == 0 {
            self.refuse(Refusal::NoItem);
            return false;
        }
        self.item_requests.requests.push(ItemRequest::TakeOut { unit: i, slot });
        true
    }

    pub fn drain_requests(&mut self) -> Vec<ItemRequest> {
        std::mem::take(&mut self.item_requests.requests)
    }

    pub fn apply_verdict(&mut self, v: &ItemVerdict) -> bool {
        match *v {
            ItemVerdict::Acquire { unit, item, allow, deny, debit, takes } => self.apply_acquire(unit, item, allow, deny, debit, takes),
            ItemVerdict::Release { unit, slot, allow, deny, credit } => self.apply_release(unit, slot, allow, deny, credit),
            ItemVerdict::TakeOut { unit, slot, allow, deny } => self.apply_take_out(unit, slot, allow, deny),
        }
    }

    pub fn pack_stash(&mut self, i: usize) -> usize {
        if !self.trade_unit_ok(i) {
            return 0;
        }
        let mut moved = 0;
        for s in 0..STASH_SLOTS {
            if self.stash_count[i * INV_SLOTS + s] > 0 && self.unstash_slot(i, s) {
                moved += 1;
            }
        }
        moved
    }

    pub fn props_near(&self, i: usize, reach_milli: i32) -> Vec<u16> {
        if i >= self.n || self.alive[i] == 0 {
            return Vec::new();
        }
        let reach = fixed::from_milli(reach_milli) as i64;
        self.props
            .iter()
            .filter(|d: &&Prop| {
                crate::arrays::dist2(self.pos_x[i], self.pos_y[i], fixed::from_milli(d.x), fixed::from_milli(d.y)) <= reach * reach
            })
            .map(|d| d.kind)
            .collect()
    }

    fn trade_unit_ok(&mut self, i: usize) -> bool {
        if i >= self.n || self.alive[i] == 0 {
            self.refuse(Refusal::NoSlot);
            return false;
        }
        true
    }

    fn bank(&self, i: usize, stash: bool) -> Bank {
        let base = i * INV_SLOTS;
        std::array::from_fn(|s| {
            if stash {
                (self.stash_kind[base + s], self.stash_count[base + s])
            } else {
                (self.inv_kind[base + s], self.inv_count[base + s])
            }
        })
    }

    fn place_item(&self, kind: u32, count: u32, stack_max: u32, inv: &mut Bank, stash: &mut Bank) -> bool {
        for bank in [&mut inv[..MAIN_SLOTS], &mut stash[..STASH_SLOTS]] {
            let same = (stack_max > 0)
                .then(|| bank.iter().position(|&(k, n)| k == kind && n > 0 && n + count <= stack_max))
                .flatten();
            if let Some(s) = same {
                bank[s].1 += count;
                return true;
            }
            if let Some(s) = bank.iter().position(|&(_, n)| n == 0) {
                bank[s] = (kind, count);
                return true;
            }
        }
        false
    }

    fn write_banks(&mut self, i: usize, inv: &Bank, stash: &Bank) {
        let base = i * INV_SLOTS;
        for s in 0..INV_SLOTS {
            self.inv_kind[base + s] = inv[s].0;
            self.inv_count[base + s] = inv[s].1;
            self.stash_kind[base + s] = stash[s].0;
            self.stash_count[base + s] = stash[s].1;
        }
    }

    fn apply_acquire(&mut self, i: usize, item: usize, allow: bool, deny: Refusal, debit: i32, takes: [u32; ITEM_COMPONENTS]) -> bool {
        if !self.trade_unit_ok(i) {
            return false;
        }
        if !allow {
            self.refuse(deny);
            return false;
        }
        let Some(def) = self.cat.items.get(item).copied() else {
            self.refuse(Refusal::NoItem);
            return false;
        };
        if debit < 0 {
            self.refuse(Refusal::NoItem);
            return false;
        }
        let mut inv = self.bank(i, false);
        let mut stash = self.bank(i, true);
        for t in takes.iter().copied().take_while(|&t| t != NO_TAKE) {
            if t as usize >= self.cat.items.len() {
                self.refuse(Refusal::NoItem);
                return false;
            }
            let kind = item_kind(t as usize);
            let held = inv[..NEUTRAL_SLOT].iter_mut().chain(stash[..STASH_SLOTS].iter_mut()).find(|(k, n)| *k == kind && *n > 0);
            match held {
                Some(slot) => {
                    slot.1 -= 1;
                    if slot.1 == 0 {
                        slot.0 = 0;
                    }
                }
                None => {
                    self.refuse(Refusal::MissingComponent);
                    return false;
                }
            }
        }
        let p = self.player[i] as usize;
        let count = u32::from(def.charges).max(1);
        if !self.place_item(item_kind(item), count, u32::from(def.stack_max), &mut inv, &mut stash) {
            self.refuse(Refusal::InventoryFull);
            return false;
        }
        self.resource_a[p] -= debit;
        self.change_items(i, |sim| sim.write_banks(i, &inv, &stash));
        true
    }

    fn apply_release(&mut self, i: usize, slot: usize, allow: bool, deny: Refusal, credit: i32) -> bool {
        if !self.trade_unit_ok(i) || slot >= INV_SLOTS {
            self.refuse(Refusal::NoSlot);
            return false;
        }
        if !allow {
            self.refuse(deny);
            return false;
        }
        let key = i * INV_SLOTS + slot;
        if self.inv_count[key] == 0 || credit < 0 {
            self.refuse(Refusal::NoItem);
            return false;
        }
        let p = self.player[i] as usize;
        self.resource_a[p] += credit;
        self.change_items(i, |sim| {
            sim.inv_kind[key] = 0;
            sim.inv_count[key] = 0;
        });
        true
    }

    fn apply_take_out(&mut self, i: usize, slot: usize, allow: bool, deny: Refusal) -> bool {
        if !self.trade_unit_ok(i) || slot >= STASH_SLOTS {
            self.refuse(Refusal::NoSlot);
            return false;
        }
        if !allow {
            self.refuse(deny);
            return false;
        }
        if self.stash_count[i * INV_SLOTS + slot] == 0 {
            self.refuse(Refusal::NoItem);
            return false;
        }
        if !self.unstash_slot(i, slot) {
            self.refuse(Refusal::InventoryFull);
            return false;
        }
        true
    }

    pub(crate) fn unstash_slot(&mut self, i: usize, slot: usize) -> bool {
        let base = i * INV_SLOTS;
        let (kind, count) = (self.stash_kind[base + slot], self.stash_count[base + slot]);
        let Some(def) = self.item_def(kind) else {
            return false;
        };
        let stack_max = u32::from(def.stack_max);
        let target = if def.flags & ITEM_FLAG_NEUTRAL != 0 {
            (self.inv_count[base + NEUTRAL_SLOT] == 0).then_some(NEUTRAL_SLOT)
        } else {
            (0..MAIN_SLOTS)
                .find(|&s| stack_max > 0 && self.inv_kind[base + s] == kind && self.inv_count[base + s] + count <= stack_max)
                .or_else(|| (0..MAIN_SLOTS).find(|&s| self.inv_count[base + s] == 0))
        };
        let Some(s) = target else {
            return false;
        };
        self.change_items(i, |sim| {
            sim.inv_kind[base + s] = kind;
            sim.inv_count[base + s] += count;
            sim.stash_kind[base + slot] = 0;
            sim.stash_count[base + slot] = 0;
        });
        true
    }

    pub fn move_item(&mut self, i: usize, from: usize, to: usize) -> bool {
        if i >= self.n || self.alive[i] == 0 || from >= INV_SLOTS || to >= INV_SLOTS || from == to {
            self.refuse(Refusal::NoSlot);
            return false;
        }
        let base = i * INV_SLOTS;
        let fits = |sim: &Sim, kind: u32, slot: usize| match sim.item_def(kind) {
            None => true,
            Some(d) if d.flags & ITEM_FLAG_NEUTRAL != 0 => slot >= MAIN_SLOTS,
            Some(_) => slot != NEUTRAL_SLOT,
        };
        let (a, b) = ((self.inv_kind[base + from], self.inv_count[base + from]), (self.inv_kind[base + to], self.inv_count[base + to]));
        if a.1 == 0 && b.1 == 0 || !fits(self, a.0, to) || !fits(self, b.0, from) {
            self.refuse(Refusal::NoItem);
            return false;
        }
        let owners = (self.inv_owner[base + from], self.inv_owner[base + to]);
        self.change_items(i, |sim| {
            sim.inv_kind[base + from] = b.0;
            sim.inv_count[base + from] = b.1;
            sim.inv_kind[base + to] = a.0;
            sim.inv_count[base + to] = a.1;
            let cd = (sim.item_cd[base + from], sim.item_cd[base + to]);
            sim.item_cd[base + from] = cd.1;
            sim.item_cd[base + to] = cd.0;
            sim.item_lock_until.swap(base + from, base + to);
            sim.item_lock_kind.swap(base + from, base + to);
        });
        self.inv_owner[base + from] = owners.1;
        self.inv_owner[base + to] = owners.0;
        true
    }

    pub fn use_item(&mut self, i: usize, slot: usize, target: i32, x: i32, y: i32) -> bool {
        use crate::loader::NO_INDEX;
        use crate::rules::Slot;
        if i >= self.n || self.alive[i] == 0 || slot >= INV_SLOTS || !slot_active(slot) {
            self.refuse(Refusal::NoSlot);
            return false;
        }
        let key = i * INV_SLOTS + slot;
        let held = (self.inv_kind[key], self.inv_count[key]);
        let def = self.item_def(held.0);
        let ability = def.map_or(NO_INDEX, |d| d.ability);
        if ability == NO_INDEX {
            self.refuse(Refusal::NoItem);
            return false;
        }
        self.expiry.casting_slot = slot as i32;
        let cast = self.cast_index(i, ability as usize, Slot::Item(key), target, x, y);
        self.expiry.casting_slot = -1;
        let untouched = (self.inv_kind[key], self.inv_count[key]) == held;
        if cast && untouched && def.is_some_and(|d| d.flags & ITEM_FLAG_CONSUMABLE != 0) {
            self.consume_item_charge(i, slot);
        }
        cast
    }
}

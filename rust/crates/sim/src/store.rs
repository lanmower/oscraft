//! Item transfers: the sim's generic mechanics. An acquire, a release, a take-out and a carry are
//! requests the sim queues. A mod answers each acquire, release and take-out with an `ItemVerdict`
//! (its rule: the amount to debit or credit a counter, and which held copies an acquire consumes), and
//! the sim applies the verdict: it takes the listed copies, debits or credits a per-player counter and
//! places an item in a bank. Each refusal is a named `Refusal` and changes nothing. Nothing here says
//! what an item is bought or sold for, where, or what a carrier does: that is the mod's rule.

use crate::arrays::Sim;
use crate::fixed;
use crate::inventory::*;
use crate::loader::Doodad;
use crate::rules::Refusal;

/// The end of a verdict's `takes` list.
pub const NO_TAKE: u32 = u32::MAX;
/// How many catalog items one verdict may take.
pub const VERDICT_TAKES: usize = ITEM_COMPONENTS;
/// An unused component slot of an item.
pub const NO_COMPONENT: u16 = crate::loader::NO_INDEX;

/// An item request the sim queues for the mod. The mod host drains the queue each tick. The mod's
/// hook receives the kind as `acquire`, `release`, `take_out` or `carry`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemRequest {
    /// Acquire catalog item `item` for unit `unit`: the mod debits and names the copies it consumes.
    Acquire { unit: usize, item: usize },
    /// Release the item in inventory slot `slot` of unit `unit`: the mod names a credit.
    Release { unit: usize, slot: usize },
    /// Move stash slot `slot` of unit `unit` into its inventory.
    TakeOut { unit: usize, slot: usize },
    /// Carry: `caster` was sent to `target`, a living friendly unit. The mod runs the carry itself.
    Carry { caster: usize, target: usize },
}

/// A mod's answer to an item request. An allowed acquire debits `debit` and consumes the catalog
/// items in `takes` (one held copy each, inventory first, then stash; `NO_TAKE` ends the list). An
/// allowed release credits `credit`. A denial names the refusal and changes nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemVerdict {
    Acquire { unit: usize, item: usize, allow: bool, deny: Refusal, debit: i32, takes: [u32; ITEM_COMPONENTS] },
    Release { unit: usize, slot: usize, allow: bool, deny: Refusal, credit: i32 },
    TakeOut { unit: usize, slot: usize, allow: bool, deny: Refusal },
}

#[derive(Clone, Debug, Default)]
pub(crate) struct StoreState {
    pub(crate) requests: Vec<ItemRequest>,
}

type Bank = [(u32, u32); INV_SLOTS];

impl Sim {
    /// Queues an acquire of catalog item `item` for unit `i`; the mod answers it. False when the unit
    /// is dead (NoSlot), or the item is unknown or flagged no-buy (NoItem); nothing is queued then.
    pub fn buy_item(&mut self, i: usize, item: usize) -> bool {
        if !self.trade_unit_ok(i) {
            return false;
        }
        if item >= self.cat.items.len() || self.cat.items[item].flags & ITEM_FLAG_NO_BUY != 0 {
            self.refuse(Refusal::NoItem);
            return false;
        }
        self.store.requests.push(ItemRequest::Acquire { unit: i, item });
        true
    }

    /// Queues a release of the item in inventory slot `slot` of unit `i`. False (nothing queued) for a dead
    /// unit, a bad slot (NoSlot) or an empty or no-sell slot (NoItem).
    pub fn sell_item(&mut self, i: usize, slot: usize) -> bool {
        if !self.trade_unit_ok(i) || slot >= INV_SLOTS {
            self.refuse(Refusal::NoSlot);
            return false;
        }
        let kind = self.inv_kind[i * INV_SLOTS + slot];
        let sellable = self.item_def(kind).is_some_and(|d| d.flags & ITEM_FLAG_NO_SELL == 0);
        if self.inv_count[i * INV_SLOTS + slot] == 0 || !sellable {
            self.refuse(Refusal::NoItem);
            return false;
        }
        self.store.requests.push(ItemRequest::Release { unit: i, slot });
        true
    }

    /// Queues moving stash slot `slot` of unit `i` into its inventory (a take-out). False (nothing queued) for a dead
    /// unit or bad slot (NoSlot) or an empty stash slot (NoItem).
    pub fn unstash(&mut self, i: usize, slot: usize) -> bool {
        if !self.trade_unit_ok(i) || slot >= STASH_SLOTS {
            self.refuse(Refusal::NoSlot);
            return false;
        }
        if self.stash_count[i * INV_SLOTS + slot] == 0 {
            self.refuse(Refusal::NoItem);
            return false;
        }
        self.store.requests.push(ItemRequest::TakeOut { unit: i, slot });
        true
    }

    /// Puts `count` of catalog item `item` into the first empty stash slot of unit `i`. False for a dead
    /// or unknown unit, an unknown item, or a full stash. Stash items give no bonuses, so nothing else
    /// changes. A mod or a test uses it to stage items for a carry.
    pub fn stash_item(&mut self, i: usize, item: usize, count: u32) -> bool {
        if !self.trade_unit_ok(i) || item >= self.cat.items.len() || count == 0 {
            return false;
        }
        let base = i * INV_SLOTS;
        let Some(s) = (0..STASH_SLOTS).find(|&s| self.stash_count[base + s] == 0) else {
            return false;
        };
        self.stash_kind[base + s] = item_kind(item);
        self.stash_count[base + s] = count;
        true
    }

    /// Takes the queued requests, in the order they were queued.
    pub fn drain_requests(&mut self) -> Vec<ItemRequest> {
        std::mem::take(&mut self.store.requests)
    }

    /// Applies a mod's verdict on an item request. Refusals, each changing nothing: NoSlot for a dead unit or
    /// bad slot; the verdict's own `deny`; NoItem for an unknown item or a negative debit or credit;
    /// MissingComponent when a listed copy is not held; CannotAfford; InventoryFull.
    pub fn apply_verdict(&mut self, v: &ItemVerdict) -> bool {
        match *v {
            ItemVerdict::Acquire { unit, item, allow, deny, debit, takes } => self.apply_acquire(unit, item, allow, deny, debit, takes),
            ItemVerdict::Release { unit, slot, allow, deny, credit } => self.apply_release(unit, slot, allow, deny, credit),
            ItemVerdict::TakeOut { unit, slot, allow, deny } => self.apply_take_out(unit, slot, allow, deny),
        }
    }

    /// Moves every stash item of unit `i` into its free inventory slots, with no trade check. Returns
    /// how many moved. A mod calls it when a unit's stash is handed over to it.
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

    /// Ids of the doodads within `reach_milli` of unit `i`, as their kind indices (DOODAD_KINDS). The
    /// mod names the kinds it cares about. Empty for a dead or unknown unit.
    pub fn doodads_near(&self, i: usize, reach_milli: i32) -> Vec<u16> {
        if i >= self.n || self.alive[i] == 0 {
            return Vec::new();
        }
        let reach = fixed::from_milli(reach_milli) as i64;
        self.doodads
            .iter()
            .filter(|d: &&Doodad| {
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

    /// Puts `count` of `kind` into the first main slot (or stash bank) with room: a slot of the same
    /// kind that can hold the count, else an empty slot. False when nothing has room.
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
        if self.resource_a[p] < debit {
            self.refuse(Refusal::CannotAfford);
            return false;
        }
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

    /// Moves stash slot `slot` of unit `i` into its inventory: onto a stack of its kind, else into
    /// the first empty main slot (the neutral slot for a neutral item). False when nothing has room.
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

    /// Swaps the contents of inventory slots `from` and `to` of unit `i` (an empty slot takes the
    /// item). The neutral slot holds only neutral items, and a neutral item sits only in the neutral
    /// slot or the backpack. Moving between an active and an inactive slot changes the unit's
    /// bonuses. Refusals, each changing nothing: NoSlot for a dead unit or a bad slot; NoItem for two
    /// empty slots or an item the other slot refuses.
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
        self.change_items(i, |sim| {
            sim.inv_kind[base + from] = b.0;
            sim.inv_count[base + from] = b.1;
            sim.inv_kind[base + to] = a.0;
            sim.inv_count[base + to] = a.1;
            let cd = (sim.item_cd[base + from], sim.item_cd[base + to]);
            sim.item_cd[base + from] = cd.1;
            sim.item_cd[base + to] = cd.0;
        });
        true
    }

    /// Uses the active ability of the item in slot `slot` of unit `i` (a main slot or the neutral
    /// slot). A consumable loses a charge when the cast succeeds. Refusals: NoSlot for a dead unit,
    /// a bad slot or a backpack slot; NoItem for an empty slot or an item without an active
    /// ability; otherwise the cast refusals (Cooldown, Energy, targeting), each changing nothing.
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
        self.wards.casting_slot = slot as i32;
        let cast = self.cast_index(i, ability as usize, Slot::Item(key), target, x, y);
        self.wards.casting_slot = -1;
        let untouched = (self.inv_kind[key], self.inv_count[key]) == held;
        if cast && untouched && def.is_some_and(|d| d.flags & ITEM_FLAG_CONSUMABLE != 0) {
            self.consume_item_charge(i, slot);
        }
        cast
    }
}

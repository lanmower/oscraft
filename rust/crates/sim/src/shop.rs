//! Shops and item trade: buying with recipes, selling, the stash, moving items between slots and
//! using an item's active ability. Each shop doodad stocks one shop kind (home, side or secret), and
//! an item sells only at the kinds in its `shops` mask.

use crate::arrays::{dist2, Sim};
use crate::fixed;
use crate::inventory::*;
use crate::loader::NO_INDEX;
use crate::rules::{catalog_index, Refusal, Slot};

/// DOODAD_KINDS indices of the three shop kinds, and how close a unit must stand to trade.
const SHOP_DOODAD_HOME: u16 = 5;
const SHOP_DOODAD_SIDE: u16 = 9;
const SHOP_DOODAD_SECRET: u16 = 10;
pub(crate) const SHOP_REACH_MILLI: i32 = 4000;

type Bank = [(u32, u32); INV_SLOTS];

/// Teams a shop keeps stock for: one per player slot.
const STOCK_TEAMS: usize = crate::MAX_PLAYERS;

/// Shop stock of the items that have a stock maximum, per team: units in stock, and the tick the next
/// unit arrives (0 when none is due). Saved and hashed with the rest of the sim.
#[derive(Clone, Debug)]
pub(crate) struct StockState {
    pub(crate) count: Vec<u8>,
    pub(crate) next_at: Vec<u32>,
}

impl StockState {
    pub(crate) fn new(items: &[crate::loader::Item]) -> StockState {
        let mut s = StockState { count: vec![0; STOCK_TEAMS * items.len()], next_at: vec![0; STOCK_TEAMS * items.len()] };
        for (k, it) in items.iter().enumerate().filter(|(_, it)| it.stock_max > 0) {
            let first = if it.stock_start_s > 0 { it.stock_start_s } else { it.stock_time_s.max(1) };
            for team in 0..STOCK_TEAMS {
                s.count[team * items.len() + k] = it.stock_initial;
                if it.stock_initial < it.stock_max {
                    s.next_at[team * items.len() + k] = u32::from(first) * crate::TICKS_PER_SECOND as u32;
                }
            }
        }
        s
    }
}

impl Sim {
    /// Shop kinds (`SHOP_*` bits) with a shop doodad within reach of unit `i`.
    pub fn shops_in_reach(&self, i: usize) -> u8 {
        let reach = fixed::from_milli(SHOP_REACH_MILLI) as i64;
        self.doodads
            .iter()
            .filter(|d| dist2(self.pos_x[i], self.pos_y[i], fixed::from_milli(d.x), fixed::from_milli(d.y)) <= reach * reach)
            .fold(0, |mask, d| {
                mask | match d.kind {
                    SHOP_DOODAD_HOME => SHOP_HOME,
                    SHOP_DOODAD_SIDE => SHOP_SIDE,
                    SHOP_DOODAD_SECRET => SHOP_SECRET,
                    _ => 0,
                }
            })
    }

    /// Units of catalog item `item` team `team`'s shops have in stock; None for an item with no stock limit.
    pub fn item_stock(&self, team: u8, item: usize) -> Option<u32> {
        let it = self.cat.items.get(item).filter(|it| it.stock_max > 0)?;
        let _ = it;
        self.stock.count.get(team as usize * self.cat.items.len() + item).map(|&c| u32::from(c))
    }

    /// Once a second, brings each team's due restocks in: one unit, then the next after the item's
    /// restock time until the stock is full.
    pub(crate) fn tick_stock(&mut self) {
        if self.ticks % crate::TICKS_PER_SECOND as u32 != 0 {
            return;
        }
        let n = self.cat.items.len();
        for k in (0..n).filter(|&k| self.cat.items[k].stock_max > 0) {
            let (max, every) = (self.cat.items[k].stock_max, u32::from(self.cat.items[k].stock_time_s.max(1)) * crate::TICKS_PER_SECOND as u32);
            for team in 0..STOCK_TEAMS {
                let at = team * n + k;
                if self.stock.next_at[at] != 0 && self.ticks >= self.stock.next_at[at] {
                    self.stock.count[at] = (self.stock.count[at] + 1).min(max);
                    self.stock.next_at[at] = if self.stock.count[at] < max { self.ticks + every } else { 0 };
                }
            }
        }
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

    /// Claims held copies of the components of catalog item `item`, recursing into a component that
    /// is not held to claim the parts of it. Returns the gold the claimed items are worth.
    fn claim_components(&self, item: usize, inv: &mut Bank, stash: &mut Bank) -> i32 {
        let mut saved = 0;
        for &c in self.cat.items[item].components.iter().filter(|&&c| c != NO_INDEX) {
            let kind = item_kind(c as usize);
            let held = inv[..NEUTRAL_SLOT]
                .iter_mut()
                .chain(stash[..STASH_SLOTS].iter_mut())
                .find(|(k, n)| *k == kind && *n > 0);
            match held {
                Some(slot) => {
                    slot.1 -= 1;
                    if slot.1 == 0 {
                        slot.0 = 0;
                    }
                    saved += self.cat.items[c as usize].cost;
                }
                None => saved += self.claim_components(c as usize, inv, stash),
            }
        }
        saved
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

    /// Buys catalog item `item` for unit `i` from a shop in reach that stocks it. Components held in
    /// the inventory or stash are consumed, a missing component's own parts too, and the price is the
    /// cost less their cost (the cost rule). The item lands in the first main slot with room, else
    /// the stash; a stackable item joins a stack of its kind. Its bonuses apply once it is in a main
    /// slot. Refusals, each changing nothing: NoSlot for a dead unit; NoItem for an unknown item or
    /// one no shop sells or one out of stock; NoShop when no shop in reach stocks it; CannotAfford; InventoryFull.
    pub fn buy_item(&mut self, i: usize, item: usize) -> bool {
        if i >= self.n || self.alive[i] == 0 {
            self.refuse(Refusal::NoSlot);
            return false;
        }
        let Some(def) = self.cat.items.get(item).copied() else {
            self.refuse(Refusal::NoItem);
            return false;
        };
        if def.flags & ITEM_FLAG_NO_BUY != 0 || def.shops == 0 {
            self.refuse(Refusal::NoItem);
            return false;
        }
        if self.shops_in_reach(i) & def.shops == 0 {
            self.refuse(Refusal::NoShop);
            return false;
        }
        let stocked = def.stock_max > 0;
        let team = self.team[self.player[i] as usize % STOCK_TEAMS] as usize % STOCK_TEAMS;
        let stock_at = team * self.cat.items.len() + item;
        if stocked && self.stock.count[stock_at] == 0 {
            self.refuse(Refusal::NoItem);
            return false;
        }
        let mut inv = self.bank(i, false);
        let mut stash = self.bank(i, true);
        let saved = self.claim_components(item, &mut inv, &mut stash);
        let price = def.cost - saved;
        let p = self.player[i] as usize;
        if self.resource_a[p] < price {
            self.refuse(Refusal::CannotAfford);
            return false;
        }
        let count = u32::from(def.charges).max(1);
        if !self.place_item(item_kind(item), count, u32::from(def.stack_max), &mut inv, &mut stash) {
            self.refuse(Refusal::InventoryFull);
            return false;
        }
        self.resource_a[p] -= price;
        if stocked {
            if self.stock.count[stock_at] == def.stock_max {
                self.stock.next_at[stock_at] = self.ticks + u32::from(def.stock_time_s.max(1)) * crate::TICKS_PER_SECOND as u32;
            }
            self.stock.count[stock_at] -= 1;
        }
        self.change_items(i, |sim| sim.write_banks(i, &inv, &stash));
        true
    }

    /// Sells the item in inventory slot `slot` of unit `i` to a shop in reach for half its cost,
    /// rounded down, and removes its bonuses. Refusals, each changing nothing: NoSlot for a dead
    /// unit or a bad slot; NoItem for an empty slot or an item that cannot be sold; NoShop out of reach.
    pub fn sell_item(&mut self, i: usize, slot: usize) -> bool {
        if i >= self.n || self.alive[i] == 0 || slot >= INV_SLOTS {
            self.refuse(Refusal::NoSlot);
            return false;
        }
        let key = i * INV_SLOTS + slot;
        let Some(def) = self.item_def(self.inv_kind[key]).filter(|d| d.flags & ITEM_FLAG_NO_SELL == 0) else {
            self.refuse(Refusal::NoItem);
            return false;
        };
        if self.shops_in_reach(i) == 0 {
            self.refuse(Refusal::NoShop);
            return false;
        }
        let units = i64::from(def.charges).max(1);
        let refund = (i64::from(def.cost) * i64::from(self.inv_count[key]) / (2 * units)) as i32;
        let p = self.player[i] as usize;
        self.resource_a[p] += refund;
        self.change_items(i, |sim| {
            sim.inv_kind[key] = 0;
            sim.inv_count[key] = 0;
        });
        true
    }

    /// Moves the item in stash slot `slot` of unit `i` into its inventory, within reach of a shop.
    /// Refusals, each changing nothing: NoSlot for a dead unit or a bad slot; NoItem for an empty
    /// stash slot; NoShop out of reach; InventoryFull.
    pub fn unstash(&mut self, i: usize, slot: usize) -> bool {
        if i >= self.n || self.alive[i] == 0 || slot >= STASH_SLOTS {
            self.refuse(Refusal::NoSlot);
            return false;
        }
        if catalog_index(self.stash_kind[i * INV_SLOTS + slot], self.cat.items.len()).is_none() {
            self.refuse(Refusal::NoItem);
            return false;
        }
        if self.shops_in_reach(i) == 0 {
            self.refuse(Refusal::NoShop);
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

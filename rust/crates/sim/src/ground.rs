use crate::arrays::Sim;
use crate::fixed;
use crate::inventory::INV_SLOTS;
use crate::loader::CELL_MILLI;
use crate::rules::Refusal;

pub const MAX_GROUND_ITEMS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroundItem {
    pub id: u32,
    pub kind: u32,
    pub count: u32,
    pub x: i32,
    pub y: i32,
    pub owner: u8,
}

impl Sim {
    pub fn place_ground_item(&mut self, item: usize, count: u32, x: i32, y: i32) -> Result<u32, Refusal> {
        if item >= self.cat.items.len() || count == 0 {
            return Err(self.reject(Refusal::NoItem));
        }
        self.push_ground(item as u32 + 1, count, x, y, 0)
    }

    fn push_ground(&mut self, kind: u32, count: u32, x: i32, y: i32, owner: u8) -> Result<u32, Refusal> {
        let w = fixed::from_milli(self.grid.w as i32 * CELL_MILLI as i32);
        let h = fixed::from_milli(self.grid.h as i32 * CELL_MILLI as i32);
        if x < 0 || y < 0 || x >= w || y >= h {
            return Err(self.reject(Refusal::OffGrid));
        }
        if self.ground.len() >= MAX_GROUND_ITEMS {
            return Err(self.reject(Refusal::NoSpawnSlot));
        }
        let id = self.ground_next;
        self.ground_next = self.ground_next.wrapping_add(1);
        self.ground.push(GroundItem { id, kind, count, x, y, owner });
        Ok(id)
    }

    pub fn drop_item_to_ground(&mut self, unit: usize, slot: usize, x: i32, y: i32) -> Result<u32, Refusal> {
        if !self.alive(unit) || slot >= INV_SLOTS {
            return Err(self.reject(Refusal::NoItem));
        }
        let (kind, count) = self.inventory_slot(unit, slot);
        if kind == 0 || count == 0 || self.item_lock_left(unit, slot) > 0 || self.item_def(kind).is_some_and(|d| d.flags & crate::inventory::ITEM_FLAG_NO_DROP != 0) {
            return Err(self.reject(Refusal::NoItem));
        }
        let owner = self.item_owner(unit, slot).map_or(0, |p| p as u8 + 1);
        let id = self.push_ground(kind, count, x, y, owner)?;
        let at = unit * INV_SLOTS + slot;
        self.change_items(unit, |sim| {
            sim.inv_kind[at] = 0;
            sim.inv_count[at] = 0;
            sim.item_cd[at] = 0;
        });
        self.touch(unit);
        Ok(id)
    }

    pub fn pickup_ground_item(&mut self, unit: usize, id: u32) -> Result<(), Refusal> {
        let Some(k) = self.ground.iter().position(|g| g.id == id) else {
            return Err(self.reject(Refusal::NoItem));
        };
        if !self.alive(unit) {
            return Err(self.reject(Refusal::NoItem));
        }
        let g = self.ground[k];
        let before: Vec<u32> = (0..INV_SLOTS).map(|s| self.inventory_slot(unit, s).1).collect();
        if !self.inventory_add(unit, g.kind, g.count) {
            return Err(self.reject(Refusal::InventoryFull));
        }
        if g.owner != 0 {
            if let Some(slot) = (0..INV_SLOTS).find(|&s| before[s] == 0 && self.inventory_slot(unit, s).1 > 0) {
                self.inv_owner[unit * INV_SLOTS + slot] = g.owner;
            }
        }
        self.ground.remove(k);
        self.touch(unit);
        Ok(())
    }

    pub fn remove_ground_item(&mut self, id: u32) -> Result<GroundItem, Refusal> {
        match self.ground.iter().position(|g| g.id == id) {
            Some(k) => Ok(self.ground.remove(k)),
            None => Err(self.reject(Refusal::NoItem)),
        }
    }

    pub fn set_ground_item_count(&mut self, id: u32, count: u32) -> Result<(), Refusal> {
        let Some(k) = self.ground.iter().position(|g| g.id == id) else {
            return Err(self.reject(Refusal::NoItem));
        };
        if count == 0 {
            self.ground.remove(k);
        } else {
            self.ground[k].count = count;
        }
        Ok(())
    }

    pub fn ground_next_id(&self) -> u32 {
        self.ground_next
    }

    pub fn ground_items(&self) -> &[GroundItem] {
        &self.ground
    }

    pub fn set_item_charges(&mut self, unit: usize, slot: usize, count: u32) -> Result<(), Refusal> {
        if !self.alive(unit) || slot >= INV_SLOTS {
            return Err(self.reject(Refusal::NoItem));
        }
        let at = unit * INV_SLOTS + slot;
        if self.inv_kind[at] == 0 {
            return Err(self.reject(Refusal::NoItem));
        }
        self.change_items(unit, |sim| {
            sim.inv_count[at] = count;
            if count == 0 {
                sim.inv_kind[at] = 0;
                sim.item_cd[at] = 0;
            }
        });
        self.touch(unit);
        Ok(())
    }

    pub(crate) fn set_ground(&mut self, items: Vec<GroundItem>, next: u32) {
        self.ground = items;
        self.ground_next = next;
    }
}

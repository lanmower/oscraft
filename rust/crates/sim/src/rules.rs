//! Casting. An ability is refused, never queued, and every refusal is counted under
//! its reason. Instant abilities apply their effects at once; projectile abilities
//! fly and apply them on landing (see projectiles.rs).

use crate::arrays::{dist2, Sim};
use crate::fixed;
use crate::inventory::{item_kind, INV_SLOTS};
use crate::status::{EFFECT_INVISIBILITY, EFFECT_STUN};
use crate::loader::{Effect, FILTER_BUILDING, FILTER_WORKER, NO_INDEX};
use crate::*;

/// Why a cast was not applied.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum Refusal {
    #[default]
    Unset = 0,
    NoSlot,
    Cooldown,
    Energy,
    NoTarget,
    UnseenTarget,
    TargetFilter,
    NoSpawnSlot,
    NoTeleport,
    NoMissileSlot,
    EffectUnsupported,
    CalldownNested,
    NoHealerSlot,
    MoveRange,
    InvalidOrderKind,
    InvalidOrderTarget,
    CannotAfford,
    NoTrain,
    InvalidPlayer,
    InvalidTeam,
    InvalidCell,
    ActionUnsupported,
    CameraUnsupported,
    SoundUnsupported,
    UiPanelUnsupported,
    ChatUnsupported,
    UpgradeRefused,
    CellFull,
    CapacityBlocked,
    Prerequisite,
    ResearchBusy,
    TransportFull,
    Channelling,
    OffGrid,
    NoDoodad,
    DuplicateDoodad,
    TrainBusy,
    NotLearned,
    NoSkillTree,
    NoSkillPoint,
    SkillMaxed,
    SkillLevelGate,
    NoItem,
    NoShop,
    InventoryFull,
    MissingComponent,
    Stunned,
    Silenced,
    Rooted,
}

pub const REFUSAL_KINDS: usize = 49;

/// Every refusal kind, in discriminant order, so a count indexed by `refusal as usize` can be named.
pub const ALL_REFUSALS: [Refusal; REFUSAL_KINDS] = [
    Refusal::Unset,
    Refusal::NoSlot,
    Refusal::Cooldown,
    Refusal::Energy,
    Refusal::NoTarget,
    Refusal::UnseenTarget,
    Refusal::TargetFilter,
    Refusal::NoSpawnSlot,
    Refusal::NoTeleport,
    Refusal::NoMissileSlot,
    Refusal::EffectUnsupported,
    Refusal::CalldownNested,
    Refusal::NoHealerSlot,
    Refusal::MoveRange,
    Refusal::InvalidOrderKind,
    Refusal::InvalidOrderTarget,
    Refusal::CannotAfford,
    Refusal::NoTrain,
    Refusal::InvalidPlayer,
    Refusal::InvalidTeam,
    Refusal::InvalidCell,
    Refusal::ActionUnsupported,
    Refusal::CameraUnsupported,
    Refusal::SoundUnsupported,
    Refusal::UiPanelUnsupported,
    Refusal::ChatUnsupported,
    Refusal::UpgradeRefused,
    Refusal::CellFull,
    Refusal::CapacityBlocked,
    Refusal::Prerequisite,
    Refusal::ResearchBusy,
    Refusal::TransportFull,
    Refusal::Channelling,
    Refusal::OffGrid,
    Refusal::NoDoodad,
    Refusal::DuplicateDoodad,
    Refusal::TrainBusy,
    Refusal::NotLearned,
    Refusal::NoSkillTree,
    Refusal::NoSkillPoint,
    Refusal::SkillMaxed,
    Refusal::SkillLevelGate,
    Refusal::NoItem,
    Refusal::NoShop,
    Refusal::InventoryFull,
    Refusal::MissingComponent,
    Refusal::Stunned,
    Refusal::Silenced,
    Refusal::Rooted,
];


impl Refusal {
    pub fn name(self) -> &'static str {
        match self {
            Refusal::Unset => "none",
            Refusal::NoSlot => "no-slot",
            Refusal::Cooldown => "cooldown",
            Refusal::Energy => "energy",
            Refusal::NoTarget => "no-target",
            Refusal::UnseenTarget => "unseen-target",
            Refusal::TargetFilter => "target-filter",
            Refusal::NoSpawnSlot => "no-spawn-slot",
            Refusal::NoTeleport => "no-teleport",
            Refusal::NoMissileSlot => "no-missile-slot",
            Refusal::EffectUnsupported => "effect-unsupported",
            Refusal::CalldownNested => "calldown-nested",
            Refusal::NoHealerSlot => "no-healer-slot",
            Refusal::MoveRange => "move-range",
            Refusal::InvalidOrderKind => "invalid-order-kind",
            Refusal::InvalidOrderTarget => "invalid-order-target",
            Refusal::CannotAfford => "cannot-afford",
            Refusal::NoTrain => "no-train",
            Refusal::InvalidPlayer => "invalid-player",
            Refusal::InvalidTeam => "invalid-team",
            Refusal::InvalidCell => "invalid-cell",
            Refusal::ActionUnsupported => "action-unsupported",
            Refusal::CameraUnsupported => "camera-unsupported",
            Refusal::SoundUnsupported => "sound-unsupported",
            Refusal::UiPanelUnsupported => "ui-panel-unsupported",
            Refusal::ChatUnsupported => "chat-unsupported",
            Refusal::UpgradeRefused => "upgrade-refused",
            Refusal::CellFull => "cell-full",
            Refusal::CapacityBlocked => "capacity-blocked",
            Refusal::Prerequisite => "prerequisite",
            Refusal::ResearchBusy => "research-busy",
            Refusal::TransportFull => "transport-full",
            Refusal::Channelling => "channelling",
            Refusal::OffGrid => "off-grid",
            Refusal::NoDoodad => "no-doodad",
            Refusal::DuplicateDoodad => "duplicate-doodad",
            Refusal::TrainBusy => "train-busy",
            Refusal::NotLearned => "not-learned",
            Refusal::NoSkillTree => "no-skill-tree",
            Refusal::NoSkillPoint => "no-skill-point",
            Refusal::SkillMaxed => "skill-maxed",
            Refusal::SkillLevelGate => "skill-level-gate",
            Refusal::NoItem => "no-item",
            Refusal::NoShop => "no-shop",
            Refusal::InventoryFull => "inventory-full",
            Refusal::MissingComponent => "missing-component",
            Refusal::Stunned => "stunned",
            Refusal::Silenced => "silenced",
            Refusal::Rooted => "rooted",
        }
    }
}

pub(crate) const TARGET_UNIT: u8 = 1;
/// DOODAD_KINDS index of a shop (`5 shop` in FORMAT.md), and how close a unit must stand to buy.
pub(crate) const SHOP_DOODAD_KIND: u16 = 5;
pub(crate) const SHOP_REACH_MILLI: i32 = 4000;

/// Which cooldown a cast is charged to: an ability slot, by array key.
#[derive(Clone, Copy)]
enum Slot {
    Ability(usize),
    Item(usize),
}

/// The catalog item index a slot's kind names, or None for an empty slot or an id past the catalog.
fn catalog_index(kind: u32, items: usize) -> Option<usize> {
    (kind != 0 && (kind as usize) <= items).then(|| kind as usize - 1)
}
const EFFECT_DAMAGE: u16 = 0;
const EFFECT_HEAL: u16 = 1;
const EFFECT_MOVE: u16 = 2;
const EFFECT_SPAWN: u16 = 3;
const EFFECT_BUFF: u16 = 4;
const EFFECT_ISSUE_ORDER: u16 = 5;
const EFFECT_REMOVE_BEHAVIOUR: u16 = 6;
const EFFECT_CREATE_HEALER: u16 = 7;
const EFFECT_USE_CALLDOWN: u16 = 8;
const EFFECT_SHIELD_BONUS: u16 = 10;
/// Mod-only: a flat stat bonus. p0 names the stat (see `modapi::STAT_NAMES`), p1 the amount.
pub(crate) const EFFECT_STAT: u16 = 64;
const EFFECT_VITAL_FRACTION: u16 = 9;

impl Sim {
    pub(crate) fn refuse(&mut self, r: Refusal) {
        self.stats.casts_refused += 1;
        self.stats.refused_by[r as usize] += 1;
        self.stats.last_refusal = r;
    }

    /// Records a refusal and hands it back, so a caller can return it in one expression.
    pub(crate) fn reject(&mut self, r: Refusal) -> Refusal {
        self.refuse(r);
        r
    }

    /// Casts ability `slot` of unit `i` at `target` (a unit index, or -1 for the point (x, y)).
    pub fn cast_ability(&mut self, i: usize, slot: usize, target: i32, x: i32, y: i32) -> bool {
        if i >= self.n || self.alive[i] == 0 || slot >= ABILITY_SLOTS {
            self.refuse(Refusal::NoSlot);
            return false;
        }
        let idx = i * ABILITY_SLOTS + slot;
        let ai = self.abil_idx[idx];
        if ai < 0 || ai as usize >= self.cat.abilities.len() {
            self.refuse(Refusal::NoSlot);
            return false;
        }
        self.cast_index(i, ai as usize, Slot::Ability(idx), target, x, y)
    }

    /// Casts catalog ability `ai` for unit `i`, charged to cooldown `cd`. Refusals as for `cast_ability`.
    fn cast_index(&mut self, i: usize, ai: usize, cd: Slot, target: i32, x: i32, y: i32) -> bool {
        let ability = self.cat.abilities[ai];
        let rank = match cd {
            Slot::Ability(k) => Some(self.abil_rank[k] as usize),
            Slot::Item(_) => None,
        };
        let (energy_cost, cooldown_ticks) = match (ability.skill, rank) {
            (Some(skill), Some(rank)) if rank > 0 => (skill.energy[rank - 1], skill.cooldown[rank - 1]),
            (Some(_), Some(_)) => {
                self.refuse(Refusal::NotLearned);
                return false;
            }
            _ => (ability.energy, ability.cooldown),
        };
        if self.channel_left[i] > 0 {
            self.refuse(Refusal::Channelling);
            return false;
        }
        if self.stunned(i) {
            self.refuse(Refusal::Stunned);
            return false;
        }
        if matches!(cd, Slot::Ability(_)) && self.silenced(i) {
            self.refuse(Refusal::Silenced);
            return false;
        }
        if self.cooldown(cd) > 0 {
            self.refuse(Refusal::Cooldown);
            return false;
        }
        if self.energy[i] < energy_cost {
            self.refuse(Refusal::Energy);
            return false;
        }
        let (tx, ty): (i32, i32) = match ability.target {
            0 => (self.pos_x[i], self.pos_y[i]),
            TARGET_UNIT => {
                if target < 0 || target as usize >= self.n || self.alive[target as usize] == 0 {
                    self.refuse(Refusal::NoTarget);
                    return false;
                }
                let t = target as usize;
                if !self.visible(self.player[i], t) {
                    self.refuse(Refusal::UnseenTarget);
                    return false;
                }
                if !self.meets_filter(t, ability.filter as u32) {
                    self.refuse(Refusal::TargetFilter);
                    return false;
                }
                (self.pos_x[t], self.pos_y[t])
            }
            _ => (x, y),
        };

        let first = ability.effect_first as usize;
        let count = ability.effect_count as usize;
        let spawns = self.cat.effects[first..first + count].iter().any(|e| e.kind == EFFECT_SPAWN);
        if spawns && self.head >= self.cap {
            self.refuse(Refusal::NoSpawnSlot);
            return false;
        }
        let heals = self.cat.effects[first..first + count].iter().any(|e| e.kind == EFFECT_CREATE_HEALER);
        if heals && self.free_healer().is_none() {
            self.refuse(Refusal::NoHealerSlot);
            return false;
        }
        if ability.is_projectile() {
            if self.free_missile().is_none() {
                self.refuse(Refusal::NoMissileSlot);
                return false;
            }
        }

        self.stats.casts += 1;
        self.energy[i] -= energy_cost;
        self.set_cooldown(cd, cooldown_ticks);
        if ability.is_projectile() {
            self.launch_missile(i, ai, tx, ty, target);
        } else if self.cat.channels.get(ai).map_or(false, |c| c.channelled) {
            self.start_channel(i, ai, tx, ty, target);
        } else {
            self.apply_effects(i, ai, tx, ty, target);
        }
        true
    }

    fn cooldown(&self, cd: Slot) -> i32 {
        match cd {
            Slot::Ability(k) => self.abil_cd[k],
            Slot::Item(k) => self.item_cd[k],
        }
    }

    fn set_cooldown(&mut self, cd: Slot, ticks: i32) {
        match cd {
            Slot::Ability(k) => self.abil_cd[k] = ticks,
            Slot::Item(k) => self.item_cd[k] = ticks,
        }
    }

    /// Uses the active ability of the item in inventory slot `slot` of unit `i`. Refusals: NoSlot
    /// for a dead unit or a bad slot; NoItem for an empty slot or an item without an active
    /// ability; otherwise the cast refusals (Cooldown, Energy, targeting), each changing nothing.
    pub fn use_item(&mut self, i: usize, slot: usize, target: i32, x: i32, y: i32) -> bool {
        if i >= self.n || self.alive[i] == 0 || slot >= INV_SLOTS {
            self.refuse(Refusal::NoSlot);
            return false;
        }
        let key = i * INV_SLOTS + slot;
        let ability = match catalog_index(self.inv_kind[key], self.cat.items.len()) {
            Some(index) => self.cat.items[index].ability,
            None => NO_INDEX,
        };
        if ability == NO_INDEX {
            self.refuse(Refusal::NoItem);
            return false;
        }
        self.cast_index(i, ability as usize, Slot::Item(key), target, x, y)
    }

    /// Buys catalog item `item` for unit `i` from a shop in reach. A recipe first takes its
    /// components from the inventory and is charged its cost less their cost (the cost rule).
    /// The item lands in the first empty inventory slot, else in the stash; its stat bonuses
    /// apply only when it enters the inventory. Refusals, each changing nothing: NoSlot for a dead
    /// unit; NoItem for an unknown item; NoShop out of reach; MissingComponent when a component is
    /// not held; CannotAfford; InventoryFull when neither the inventory nor the stash has room.
    pub fn buy_item(&mut self, i: usize, item: usize) -> bool {
        if i >= self.n || self.alive[i] == 0 {
            self.refuse(Refusal::NoSlot);
            return false;
        }
        let Some(def) = self.cat.items.get(item).copied() else {
            self.refuse(Refusal::NoItem);
            return false;
        };
        if !self.in_shop_reach(i) {
            self.refuse(Refusal::NoShop);
            return false;
        }
        let base = i * INV_SLOTS;
        let mut held: [(u32, u32); INV_SLOTS] =
            std::array::from_fn(|s| (self.inv_kind[base + s], self.inv_count[base + s]));
        // Components consumed: cost, armour, damage and health bonus.
        let mut spent = [0i32; 4];
        for &component in def.components.iter().filter(|&&c| c != NO_INDEX) {
            let kind = item_kind(component as usize);
            let Some(s) = held.iter().position(|&(k, c)| k == kind && c > 0) else {
                self.refuse(Refusal::MissingComponent);
                return false;
            };
            held[s] = (0, 0);
            let part = self.cat.items[component as usize];
            spent[0] += part.cost;
            spent[1] += part.armor;
            spent[2] += part.damage;
            spent[3] += part.health;
        }
        let price = def.cost - spent[0];
        let p = self.player[i] as usize;
        if self.resource_a[p] < price {
            self.refuse(Refusal::CannotAfford);
            return false;
        }
        let inventory_slot = held.iter().position(|&(k, _)| k == 0);
        let stash_slot = (0..INV_SLOTS).find(|&s| self.stash_kind[base + s] == 0);
        if inventory_slot.is_none() && stash_slot.is_none() {
            self.refuse(Refusal::InventoryFull);
            return false;
        }
        self.resource_a[p] -= price;
        for (s, &(kind, count)) in held.iter().enumerate() {
            self.inv_kind[base + s] = kind;
            self.inv_count[base + s] = count;
        }
        self.apply_bonus(i, -spent[1], -spent[2], -spent[3]);
        match inventory_slot {
            Some(s) => {
                self.inv_kind[base + s] = item_kind(item);
                self.inv_count[base + s] = 1;
                self.apply_bonus(i, def.armor, def.damage, def.health);
            }
            None => {
                let s = stash_slot.unwrap_or(0);
                self.stash_kind[base + s] = item_kind(item);
                self.stash_count[base + s] = 1;
            }
        }
        true
    }

    /// Sells the item in inventory slot `slot` of unit `i` to a shop in reach for half its cost,
    /// rounded down, and removes its bonuses. Refusals, each changing nothing: NoSlot for a dead
    /// unit or a bad slot; NoItem for an empty slot; NoShop out of reach.
    pub fn sell_item(&mut self, i: usize, slot: usize) -> bool {
        if i >= self.n || self.alive[i] == 0 || slot >= INV_SLOTS {
            self.refuse(Refusal::NoSlot);
            return false;
        }
        let key = i * INV_SLOTS + slot;
        let Some(index) = catalog_index(self.inv_kind[key], self.cat.items.len()) else {
            self.refuse(Refusal::NoItem);
            return false;
        };
        if !self.in_shop_reach(i) {
            self.refuse(Refusal::NoShop);
            return false;
        }
        let def = self.cat.items[index];
        let p = self.player[i] as usize;
        self.resource_a[p] += def.cost / 2;
        self.inv_kind[key] = 0;
        self.inv_count[key] = 0;
        self.apply_bonus(i, -def.armor, -def.damage, -def.health);
        true
    }

    /// Moves the item in stash slot `slot` of unit `i` into the first empty inventory slot, within
    /// reach of a shop. Its bonuses apply then. Refusals, each changing nothing: NoSlot for a dead
    /// unit or a bad slot; NoItem for an empty stash slot; NoShop out of reach; InventoryFull.
    pub fn unstash(&mut self, i: usize, slot: usize) -> bool {
        if i >= self.n || self.alive[i] == 0 || slot >= INV_SLOTS {
            self.refuse(Refusal::NoSlot);
            return false;
        }
        let key = i * INV_SLOTS + slot;
        let Some(index) = catalog_index(self.stash_kind[key], self.cat.items.len()) else {
            self.refuse(Refusal::NoItem);
            return false;
        };
        if !self.in_shop_reach(i) {
            self.refuse(Refusal::NoShop);
            return false;
        }
        let base = i * INV_SLOTS;
        let Some(free) = (0..INV_SLOTS).find(|&s| self.inv_kind[base + s] == 0) else {
            self.refuse(Refusal::InventoryFull);
            return false;
        };
        let def = self.cat.items[index];
        self.inv_kind[base + free] = self.stash_kind[key];
        self.inv_count[base + free] = self.stash_count[key];
        self.stash_kind[key] = 0;
        self.stash_count[key] = 0;
        self.apply_bonus(i, def.armor, def.damage, def.health);
        true
    }

    /// Adds stat bonuses to unit `i` (negative to remove). Negative health clamps life to the new maximum.
    fn apply_bonus(&mut self, i: usize, armor: i32, damage: i32, health: i32) {
        self.armor[i] += fixed::from_int(armor);
        self.damage[i] += fixed::from_int(damage);
        self.hp_max[i] += fixed::from_int(health);
        self.hp[i] = if health >= 0 {
            self.hp[i] + fixed::from_int(health)
        } else {
            self.hp[i].min(self.hp_max[i])
        };
        self.touch(i);
    }

    /// True when unit `i` stands within `SHOP_REACH_MILLI` of a shop doodad.
    fn in_shop_reach(&self, i: usize) -> bool {
        let reach = fixed::from_milli(SHOP_REACH_MILLI) as i64;
        self.doodads.iter().any(|d| {
            d.kind == SHOP_DOODAD_KIND
                && dist2(self.pos_x[i], self.pos_y[i], fixed::from_milli(d.x), fixed::from_milli(d.y)) <= reach * reach
        })
    }

    /// Grants `amount` energy to unit `i`. Refusals, each changing nothing: a dead or unknown
    /// unit is NoTarget; a non-positive amount is Energy.
    pub fn grant_energy(&mut self, i: usize, amount: i32) -> Result<(), Refusal> {
        if i >= self.n || self.alive[i] == 0 {
            return Err(self.reject(Refusal::NoTarget));
        }
        if amount <= 0 {
            return Err(self.reject(Refusal::Energy));
        }
        self.energy[i] = self.energy[i].saturating_add(amount);
        self.touch(i);
        Ok(())
    }

    /// Spends `amount` energy from unit `i`. Refusals, each changing nothing: a dead or unknown
    /// unit is NoTarget; an amount above the unit's energy is Energy. A zero amount costs nothing.
    pub fn spend_energy(&mut self, i: usize, amount: i32) -> Result<(), Refusal> {
        if i >= self.n || self.alive[i] == 0 {
            return Err(self.reject(Refusal::NoTarget));
        }
        if amount > self.energy[i] {
            return Err(self.reject(Refusal::Energy));
        }
        self.energy[i] -= amount.max(0);
        self.touch(i);
        Ok(())
    }

    /// Applies one ability's effects at a point, in record order. Refusals, each changing
    /// nothing: a dead or unknown caster is NoTarget; an unknown ability index is NoSlot.
    pub fn apply_effects(&mut self, caster: usize, ai: usize, tx: i32, ty: i32, target: i32) {
        if caster >= self.n || self.alive[caster] == 0 {
            self.refuse(Refusal::NoTarget);
            return;
        }
        if ai >= self.cat.abilities.len() {
            // A caller error that changes nothing: not counted, so the state hash stays put.
            return;
        }
        let ability = self.cat.abilities[ai];
        let first = ability.effect_first as usize;
        // Format v8 receiver: the named unit target when the ability takes one, else the caster.
        let receiver = if ability.target != 0 && target >= 0 && (target as usize) < self.n {
            target
        } else {
            caster as i32
        };
        let mut amount = ability.skill.and_then(|skill| self.skill_amount(caster, ai, skill));
        for k in 0..ability.effect_count as usize {
            let mut eff: Effect = self.cat.effects[first + k];
            if let Some(scaled) = amount {
                if eff.kind == EFFECT_DAMAGE || eff.kind == EFFECT_HEAL {
                    eff.p0 = scaled;
                    amount = None;
                }
            }
            self.apply_effect(caster, ability.filter as u32, eff, tx, ty, receiver, true);
        }
    }

    /// Applies one effect. `filter` is the target filter mask of the ability that owns it.
    /// `may_call` is false inside a calldown, so a calldown never starts another one.
    /// A kind with no implementation refuses as `EffectUnsupported` and changes nothing.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply_effect(
        &mut self,
        caster: usize,
        filter: u32,
        eff: Effect,
        tx: i32,
        ty: i32,
        target: i32,
        may_call: bool,
    ) {
        let p = self.player[caster];
        let c = caster as i32;
        match eff.kind {
            EFFECT_DAMAGE => {
                if eff.p1 == 0 {
                    self.damage_target(c, p, eff.p0, target);
                } else {
                    self.damage_area(c, p, eff.p0, fixed::from_milli(eff.p1), tx, ty);
                }
            }
            EFFECT_HEAL => {
                if eff.p1 == 0 {
                    self.heal_target(p, eff.p0, target);
                } else {
                    self.heal_area(p, eff.p0, fixed::from_milli(eff.p1), tx, ty);
                }
            }
            EFFECT_MOVE => self.move_caster(caster, eff.p0, tx, ty),
            EFFECT_SPAWN => {
                // A lane offset (p1, p2) places the unit beside the caster; without one it
                // spawns at the cast point, as before.
                let (sx, sy) = if eff.p1 == 0 && eff.p2 == 0 {
                    (tx, ty)
                } else {
                    (
                        self.pos_x[caster] + fixed::from_milli(eff.p1),
                        self.pos_y[caster] + fixed::from_milli(eff.p2),
                    )
                };
                if let Some(j) = self.spawn_for(p, eff.p0 as u32, sx, sy) {
                    self.stats.summoned += 1;
                    self.march_to_enemy_base(j, p);
                }
            }
            EFFECT_BUFF => {
                if target >= 0 && (target as usize) < self.n && self.meets_filter(target as usize, filter) {
                    self.buff_unit(c, p, target as usize, eff.p0, eff.p1, false);
                }
            }
            EFFECT_STAT => self.stat_bonus(caster, target, eff.p0, eff.p1),
            EFFECT_STUN..=EFFECT_INVISIBILITY => self.status_effect(eff, target),
            EFFECT_ISSUE_ORDER => self.issue_order(caster, eff.p0, eff.p1, tx, ty, target),
            EFFECT_REMOVE_BEHAVIOUR => {
                let _ = self.remove_behaviour(caster, eff.p0, fixed::from_milli(eff.p1), tx, ty, 0);
            }
            EFFECT_SHIELD_BONUS => self.shield_bonus(caster, target, eff.p0),
            EFFECT_VITAL_FRACTION => self.vital_fraction(caster, target, eff.p0, eff.p1, eff.p2, eff.p3),
            EFFECT_CREATE_HEALER => {
                let rad = fixed::from_milli(eff.p1);
                if !self.create_healer(p, eff.p0, rad, eff.p2, eff.p3, tx, ty, target) {
                    self.refuse(Refusal::NoHealerSlot);
                }
            }
            EFFECT_USE_CALLDOWN => {
                if !may_call {
                    self.refuse(Refusal::CalldownNested);
                    return;
                }
                let called = self.cat.abilities[eff.p0 as usize];
                let first = called.effect_first as usize;
                for k in 0..called.effect_count as usize {
                    let e: Effect = self.cat.effects[first + k];
                    self.apply_effect(caster, called.filter as u32, e, tx, ty, target, false);
                }
            }
            _ => self.refuse(Refusal::EffectUnsupported),
        }
    }

    /// Sends a freshly summoned unit `j` of `player` on an attack-move toward the nearest living
    /// hostile building, along the route `approach_to` finds (A* around blocked terrain). A unit
    /// with no hostile building left stays where it spawned.
    fn march_to_enemy_base(&mut self, j: usize, player: u8) {
        let mut best: Option<(usize, i64)> = None;
        for k in 0..self.n {
            if k == j || self.alive[k] == 0 || !self.hostile(player, self.player[k]) {
                continue;
            }
            if self.filter[k] & FILTER_BUILDING == 0 {
                continue;
            }
            let d2 = dist2(self.pos_x[j], self.pos_y[j], self.pos_x[k], self.pos_y[k]);
            if best.map_or(true, |(_, b)| d2 < b) {
                best = Some((k, d2));
            }
        }
        if let Some((k, _)) = best {
            let _ = self.issue_order_kind(j, ORDER_ATTACK_MOVE, self.pos_x[k], self.pos_y[k]);
        }
    }

    /// Whether a unit may take a spell or attack. `required` is a filter mask; every
    /// bit must be present on the unit. A zero mask is always met.
    /// Ticks left in unit `i`'s channel, or 0 when it is not channelling.
    pub fn channel_left(&self, i: usize) -> i32 {
        self.channel_left.get(i).copied().unwrap_or(0)
    }

    /// The ability index unit `i` is channelling, or -1 when it is not channelling.
    pub fn channel_ai(&self, i: usize) -> i32 {
        self.channel_ai.get(i).copied().unwrap_or(-1)
    }

    pub(crate) fn meets_filter(&self, j: usize, required: u32) -> bool {
        j < self.n && self.filter[j] & required == required
    }

    /// Workers are the only units the harvester may gather with.
    pub(crate) fn is_worker(&self, j: usize) -> bool {
        j < self.n && self.filter[j] & FILTER_WORKER != 0
    }

    /// Starts channelling ability `ai` of unit `i` toward (x, y) and `target`: the caster holds
    /// still for the channel duration, then the effects apply. Cost and cooldown are paid at the cast.
    pub(crate) fn start_channel(&mut self, i: usize, ai: usize, tx: i32, ty: i32, target: i32) {
        self.channel_left[i] = self.cat.channels.get(ai).map_or(0, |c| c.duration);
        self.channel_ai[i] = ai as i32;
        self.channel_x[i] = tx;
        self.channel_y[i] = ty;
        self.channel_target[i] = target;
        self.tgt_x[i] = self.pos_x[i];
        self.tgt_y[i] = self.pos_y[i];
        self.vel_x[i] = 0;
        self.vel_y[i] = 0;
        self.touch(i);
    }

    /// Holds channelling unit `i` for one tick. When the channel runs out, the ability's effects
    /// apply at the point it was cast toward.
    pub(crate) fn tick_channel(&mut self, i: usize) {
        self.vel_x[i] = 0;
        self.vel_y[i] = 0;
        self.tgt_x[i] = self.pos_x[i];
        self.tgt_y[i] = self.pos_y[i];
        self.channel_left[i] -= 1;
        if self.channel_left[i] > 0 {
            return;
        }
        let ai = self.channel_ai[i];
        self.channel_ai[i] = -1;
        if ai >= 0 && (ai as usize) < self.cat.abilities.len() {
            let (x, y, t) = (self.channel_x[i], self.channel_y[i], self.channel_target[i]);
            self.apply_effects(i, ai as usize, x, y, t);
        }
    }

    /// Ends unit `i`'s channel early, applying no effects. Called when the unit takes a hit
    /// or is given an order.
    pub(crate) fn interrupt_channel(&mut self, i: usize) {
        if self.channel_left[i] > 0 {
            self.channel_left[i] = 0;
            self.channel_ai[i] = -1;
            self.touch(i);
        }
    }

    /// Squared distance helper kept beside the rules that use it.
    pub(crate) fn within(&self, a: usize, x: i32, y: i32, rad: i32) -> bool {
        let rad = rad as i64;
        dist2(self.pos_x[a], self.pos_y[a], x, y) <= rad * rad
    }
}

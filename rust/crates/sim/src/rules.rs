//! Casting. An ability is refused, never queued, and every refusal is counted under
//! its reason. Instant abilities apply their effects at once; projectile abilities
//! fly and apply them on landing (see projectiles.rs).

use crate::arrays::{dist2, Sim};
use crate::fixed;
use crate::status::{EFFECT_INVISIBILITY, EFFECT_STUN};
use crate::loader::{Effect, FILTER_WORKER};
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
    Passive,
    SpellImmune,
    Muted,
}

pub const REFUSAL_KINDS: usize = 52;

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
    Refusal::Passive,
    Refusal::SpellImmune,
    Refusal::Muted,
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
            Refusal::Passive => "passive",
            Refusal::SpellImmune => "spell-immune",
            Refusal::Muted => "muted",
        }
    }
}

pub(crate) const TARGET_UNIT: u8 = 1;

/// Which cooldown a cast is charged to: an ability slot, by array key.
#[derive(Clone, Copy)]
pub(crate) enum Slot {
    Ability(usize),
    Item(usize),
}

/// The catalog item index a slot's kind names, or None for an empty slot or an id past the catalog.
pub(crate) fn catalog_index(kind: u32, items: usize) -> Option<usize> {
    (kind != 0 && (kind as usize) <= items).then(|| kind as usize - 1)
}
/// Casts the cast log holds between drains.
const CAST_LOG_CAP: usize = 1024;
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
/// A flat stat bonus. p0 names the stat (see `modapi::STAT_NAMES`), p1 the amount.
pub(crate) const EFFECT_STAT: u16 = 16;
/// Places a ward unit: p0 unit type, p1 lifetime in ticks. See wards.rs.
pub(crate) const EFFECT_WARD: u16 = 17;
/// Code 18 is retired (the team glyph became a mod rule, on_cast). It is refused as unsupported.
/// Courier delivery to the receiver: p0 reach in milli-units. See courier.rs.
pub(crate) const EFFECT_DELIVER: u16 = 19;
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
    pub(crate) fn cast_index(&mut self, i: usize, ai: usize, cd: Slot, target: i32, x: i32, y: i32) -> bool {
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
        let ext = self.ext_of(ai);
        if ext.flags & crate::spell::FLAG_PASSIVE != 0 {
            self.refuse(Refusal::Passive);
            return false;
        }
        let (energy_cost, cooldown_ticks) = self.cast_costs(i, ai, energy_cost, cooldown_ticks);
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
        if matches!(cd, Slot::Item(_)) && self.muted(i) {
            self.refuse(Refusal::Muted);
            return false;
        }
        if self.cooldown(cd) > 0 {
            self.refuse(Refusal::Cooldown);
            return false;
        }
        if let Slot::Ability(k) = cd {
            if !self.has_charge(k, ai) {
                self.refuse(Refusal::Cooldown);
                return false;
            }
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
                if !self.meets_filter(t, ability.filter as u32) || self.invulnerable(t) {
                    self.refuse(Refusal::TargetFilter);
                    return false;
                }
                let friend = !self.hostile(self.player[i], self.player[t]);
                if (ext.flags & crate::spell::FLAG_ALLY_ONLY != 0 && !friend) || (ext.flags & crate::spell::FLAG_ENEMY_ONLY != 0 && friend) {
                    self.refuse(Refusal::TargetFilter);
                    return false;
                }
                if ext.flags & crate::spell::FLAG_PIERCE == 0 && self.spell_immune(t) && self.hostile(self.player[i], self.player[t]) {
                    self.refuse(Refusal::SpellImmune);
                    return false;
                }
                (self.pos_x[t], self.pos_y[t])
            }
            _ => (x, y),
        };
        if ability.target != 0 && self.out_of_cast_range(i, ai, tx, ty) {
            if let Slot::Ability(k) = cd {
                self.begin_pending_cast(i, k % ABILITY_SLOTS, target, x, y);
                return true;
            }
            self.refuse(Refusal::MoveRange);
            return false;
        }

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
        if self.cast_log.len() < CAST_LOG_CAP {
            self.cast_log.push((i as u32, ai as u16));
        }
        if let Slot::Ability(k) = cd {
            self.spend_charge(i, k, ai);
        }
        self.energy[i] -= energy_cost;
        self.set_cooldown(cd, cooldown_ticks);
        self.spell.pend_slot[i] = -1;
        let channelled = self.cat.channels.get(ai).map_or(false, |c| c.channelled);
        let point = self.cast_point_of(i, ai);
        if channelled {
            self.start_channel(i, ai, tx, ty, target);
        } else if point > 0 {
            self.start_cast_point(i, ai, tx, ty, target, point);
        } else {
            self.finish_cast(i, ai, tx, ty, target);
        }
        true
    }

    /// The moment a cast takes hold: a projectile leaves, anything else applies at once.
    pub(crate) fn finish_cast(&mut self, i: usize, ai: usize, tx: i32, ty: i32, target: i32) {
        if self.cat.abilities[ai].is_projectile() {
            self.launch_missile(i, ai, tx, ty, target);
        } else {
            self.apply_effects(i, ai, tx, ty, target);
        }
    }

    /// Holds the caster still for `ticks` before the cast takes hold (the ability's cast point).
    fn start_cast_point(&mut self, i: usize, ai: usize, tx: i32, ty: i32, target: i32, ticks: i32) {
        self.start_channel(i, ai, tx, ty, target);
        self.channel_left[i] = ticks;
    }

    pub(crate) fn cooldown(&self, cd: Slot) -> i32 {
        match cd {
            Slot::Ability(k) => self.abil_cd[k],
            Slot::Item(k) => self.item_cd[k],
        }
    }

    pub(crate) fn set_cooldown(&mut self, cd: Slot, ticks: i32) {
        match cd {
            Slot::Ability(k) => self.abil_cd[k] = ticks,
            Slot::Item(k) => self.item_cd[k] = ticks,
        }
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
        let (was_ai, was_pierce) = (self.spell.cur_ai, self.spell.pierce);
        self.spell.cur_ai = ai as i32;
        self.spell.pierce = self.ext_of(ai).flags & crate::spell::FLAG_PIERCE != 0;
        let count = ability.effect_count as usize;
        let passive_stats = self.ext_of(ai).flags & crate::spell::FLAG_PASSIVE_STATS != 0;
        let mut k = 0;
        while k < count {
            let mut eff: Effect = self.resolve_effect(caster, ai, k);
            self.spell.cur_eff = k as i32;
            if let Some(scaled) = amount {
                if eff.kind == EFFECT_DAMAGE || eff.kind == EFFECT_HEAL {
                    eff.p0 = scaled;
                    amount = None;
                }
            }
            let _ = first;
            if eff.kind == EFFECT_STAT && passive_stats {
                k += 1;
                continue;
            }
            match eff.kind {
                crate::spell::K_AOE => {
                    self.aoe_effects(caster, ai, ability.filter as u32, k, tx, ty);
                    k += 1 + eff.p2 as usize;
                    continue;
                }
                crate::spell::K_IF_KILL if !self.spell.kill_flag => {
                    k += 1 + eff.p0 as usize;
                    continue;
                }
                crate::spell::K_IF_KILL => {}
                _ => self.apply_effect(caster, ability.filter as u32, eff, tx, ty, receiver, true),
            }
            k += 1;
        }
        self.spell.cur_ai = was_ai;
        self.spell.pierce = was_pierce;
    }

    /// Runs ability `ai`'s effects for `caster` at a point even when the caster has died since (a zone
    /// or an aura pulse outlives its owner).
    pub(crate) fn run_effects(&mut self, caster: usize, ai: usize, x: i32, y: i32, target: i32) {
        if caster < self.n && self.alive[caster] == 0 {
            self.alive[caster] = 1;
            self.apply_effects(caster, ai, x, y, target);
            self.alive[caster] = 0;
        } else {
            self.apply_effects(caster, ai, x, y, target);
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
            EFFECT_DAMAGE if eff.p2 != 0 => self.typed_damage(caster, eff, tx, ty, target),
            EFFECT_DAMAGE => {
                if eff.p1 == 0 {
                    self.damage_target(c, p, eff.p0, target);
                } else {
                    self.damage_area(c, p, eff.p0, fixed::from_milli(eff.p1), tx, ty);
                }
            }
            EFFECT_HEAL => {
                let amount = match eff.p2 {
                    1 => fixed::from_int(eff.p0),
                    2 if target >= 0 && (target as usize) < self.n => (self.hp_max[target as usize] as i64 * eff.p0 as i64 / 10_000) as i32,
                    _ => eff.p0,
                };
                if eff.p1 == 0 {
                    self.heal_target(p, amount, target);
                } else {
                    self.heal_area(p, amount, fixed::from_milli(eff.p1), tx, ty);
                }
            }
            EFFECT_MOVE => self.move_caster(caster, eff.p0, tx, ty),
            EFFECT_SPAWN => {
                // An offset (p1, p2) places the unit beside the caster; without one it
                // spawns at the cast point, as before.
                let (sx, sy) = if eff.p1 == 0 && eff.p2 == 0 {
                    (tx, ty)
                } else {
                    (
                        self.pos_x[caster] + fixed::from_milli(eff.p1),
                        self.pos_y[caster] + fixed::from_milli(eff.p2),
                    )
                };
                let kind = self.wave_kind(p, eff.p0 as u32, eff.p3, sx, sy);
                if let Some(j) = self.spawn_for(p, kind, sx, sy) {
                    self.stats.summoned += 1;
                    self.request_destination(j, p);
                }
            }
            EFFECT_BUFF => {
                if target >= 0 && (target as usize) < self.n && self.meets_filter(target as usize, filter) {
                    self.buff_unit(c, p, target as usize, eff.p0, eff.p1, false);
                }
            }
            EFFECT_STAT => self.stat_bonus(caster, target, eff.p0, eff.p1),
            EFFECT_WARD => self.ward_effect(caster, eff.p0, eff.p1, tx, ty),
            EFFECT_DELIVER => self.deliver_effect(caster, target, eff.p0),
            EFFECT_STUN..=EFFECT_INVISIBILITY => self.status_effect_from(caster, eff, target),
            k if k >= crate::spell::FIRST_SPELL_KIND => self.spell_effect(caster, eff, tx, ty, target),
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

    /// Spawns a mod's unit `kind` for `player` at (x, y). A spawn that names a route (`route != 0`)
    /// asks the host for a destination (on_march); a route-0 spawn stands where it was placed, so a mod
    /// that moves its own units (a tower defense route, for one) keeps control of them.
    pub fn spawn_mod_unit(&mut self, player: u8, kind: u32, x: i32, y: i32, route: i32) -> Option<usize> {
        let unit = self.spawn_for(player, kind, x, y)?;
        if route != 0 {
            self.request_destination(unit, player);
        }
        Some(unit)
    }

    /// Takes the casts that took hold since the last call, oldest first, as (caster, ability index). Read by a
    /// host that runs an on_cast callback; a sim nobody drains only keeps the first CAST_LOG_CAP.
    pub fn take_casts(&mut self) -> Vec<(usize, usize)> {
        std::mem::take(&mut self.cast_log).into_iter().map(|(u, a)| (u as usize, a as usize)).collect()
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
        let ai = self.channel_ai[i];
        let pulsing = ai >= 0
            && (ai as usize) < self.cat.abilities.len()
            && self.cat.channels.get(ai as usize).map_or(false, |c| c.channelled)
            && self.ext_of(ai as usize).period > 0;
        if pulsing {
            self.channel_pulse(i, ai as usize);
        }
        if self.channel_left[i] > 0 {
            return;
        }
        self.channel_ai[i] = -1;
        if !pulsing && ai >= 0 && (ai as usize) < self.cat.abilities.len() {
            let (x, y, t) = (self.channel_x[i], self.channel_y[i], self.channel_target[i]);
            self.finish_cast(i, ai as usize, x, y, t);
        }
    }

    /// One pulse of a pulsing channel: its effects apply on the caster, at the cast point for a
    /// ground pulse, or at a random spot of the ability's scatter ring around the caster.
    fn channel_pulse(&mut self, i: usize, ai: usize) {
        let ext = self.ext_of(ai);
        let period = self.ability_field(i, ai, crate::spell::SCALED_PERIOD, ext.period).max(1);
        if self.channel_left[i] % period != 0 {
            return;
        }
        let target = self.channel_target[i];
        if ext.break_range > 0 && target >= 0 {
            let t = target as usize;
            let far = fixed::from_milli(ext.break_range) as i64;
            if t >= self.n || self.alive[t] == 0 || dist2(self.pos_x[i], self.pos_y[i], self.pos_x[t], self.pos_y[t]) > far * far {
                self.interrupt_channel(i);
                return;
            }
        }
        let (mut x, mut y) = if ext.flags & crate::spell::FLAG_GROUND_PULSE != 0 {
            (self.channel_x[i], self.channel_y[i])
        } else {
            (self.pos_x[i], self.pos_y[i])
        };
        if ext.scatter_max > 0 {
            let (dx, dy) = self.scatter_offset(ext.scatter_min, ext.scatter_max);
            x += dx;
            y += dy;
        }
        let t = self.channel_target[i];
        self.apply_effects(i, ai, x, y, t);
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

impl Sim {
    /// Puts `count` of catalog item `item` into living unit `unit`'s inventory, through the same slot rules as a
    /// purchase (top up a stack, else the first empty main slot). False, with nothing changed, for an unknown
    /// item, a dead or unknown unit, or full slots.
    pub fn grant_item(&mut self, unit: usize, item: usize, count: u32) -> bool {
        if item >= self.cat.items.len() || !self.alive(unit) {
            return false;
        }
        self.inventory_add(unit, item as u32 + 1, count)
    }
}

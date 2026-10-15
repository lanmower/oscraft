//! Casting. An ability is refused, never queued, and every refusal is counted under
//! its reason. Instant abilities apply their effects at once; projectile abilities
//! fly and apply them on landing (see projectiles.rs).

use crate::arrays::{dist2, Sim};
use crate::fixed;
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
}

pub const REFUSAL_KINDS: usize = 21;

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
        }
    }
}

const TARGET_UNIT: u8 = 1;
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
        let ability = self.cat.abilities[ai as usize];
        if self.abil_cd[idx] > 0 {
            self.refuse(Refusal::Cooldown);
            return false;
        }
        if self.energy[i] < ability.energy {
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
        self.energy[i] -= ability.energy;
        self.abil_cd[idx] = ability.cooldown;
        if ability.is_projectile() {
            self.launch_missile(i, ai as usize, tx, ty, target);
        } else {
            self.apply_effects(i, ai as usize, tx, ty, target);
        }
        true
    }

    /// Applies one ability's effects at a point, in record order. `caster` must be alive.
    pub(crate) fn apply_effects(&mut self, caster: usize, ai: usize, tx: i32, ty: i32, target: i32) {
        let ability = self.cat.abilities[ai];
        let first = ability.effect_first as usize;
        // Format v8 receiver: the named unit target when the ability takes one, else the caster.
        let receiver = if ability.target != 0 && target >= 0 && (target as usize) < self.n {
            target
        } else {
            caster as i32
        };
        for k in 0..ability.effect_count as usize {
            let eff: Effect = self.cat.effects[first + k];
            self.apply_effect(caster, ability.filter as u32, eff, tx, ty, receiver, true);
        }
    }

    /// Applies one effect. `filter` is the target filter mask of the ability that owns it.
    /// `may_call` is false inside a calldown, so a calldown never starts another one.
    /// A kind with no implementation refuses as `EffectUnsupported` and changes nothing.
    #[allow(clippy::too_many_arguments)]
    fn apply_effect(
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
                if self.spawn_for(p, eff.p0 as u32, tx, ty).is_some() {
                    self.stats.summoned += 1;
                }
            }
            EFFECT_BUFF => {
                if target >= 0 && (target as usize) < self.n && self.meets_filter(target as usize, filter) {
                    self.buff_unit(c, p, target as usize, eff.p0, eff.p1, false);
                }
            }
            EFFECT_ISSUE_ORDER => self.issue_order(caster, eff.p0, eff.p1, tx, ty, target),
            EFFECT_REMOVE_BEHAVIOUR => {
                self.remove_behaviour(eff.p0, fixed::from_milli(eff.p1), tx, ty);
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

    /// Whether a unit may take a spell or attack. `required` is a filter mask; every
    /// bit must be present on the unit. A zero mask is always met.
    pub(crate) fn meets_filter(&self, j: usize, required: u32) -> bool {
        j < self.n && self.filter[j] & required == required
    }

    /// Workers are the only units the harvester may gather with.
    pub(crate) fn is_worker(&self, j: usize) -> bool {
        j < self.n && self.filter[j] & FILTER_WORKER != 0
    }

    /// Squared distance helper kept beside the rules that use it.
    pub(crate) fn within(&self, a: usize, x: i32, y: i32, rad: i32) -> bool {
        let rad = rad as i64;
        dist2(self.pos_x[a], self.pos_y[a], x, y) <= rad * rad
    }
}

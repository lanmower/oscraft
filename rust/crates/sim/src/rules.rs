
use crate::arrays::{dist2, Sim};
use crate::fixed;
use crate::status::{EFFECT_INVISIBILITY, EFFECT_STUN};
use crate::loader::Effect;
use crate::*;

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
    NoProp,
    DuplicateProp,
    TrainBusy,
    NotLearned,
    NoSkillTree,
    NoSkillPoint,
    SkillMaxed,
    LevelGate,
    NoItem,
    OutOfReach,
    InventoryFull,
    MissingComponent,
    Stunned,
    Silenced,
    Rooted,
    Passive,
    SpellImmune,
    Muted,
    Unpawnable,
    Consumable,
    Locked,
}

pub const REFUSAL_KINDS: usize = 55;

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
    Refusal::NoProp,
    Refusal::DuplicateProp,
    Refusal::TrainBusy,
    Refusal::NotLearned,
    Refusal::NoSkillTree,
    Refusal::NoSkillPoint,
    Refusal::SkillMaxed,
    Refusal::LevelGate,
    Refusal::NoItem,
    Refusal::OutOfReach,
    Refusal::InventoryFull,
    Refusal::MissingComponent,
    Refusal::Stunned,
    Refusal::Silenced,
    Refusal::Rooted,
    Refusal::Passive,
    Refusal::SpellImmune,
    Refusal::Muted,
    Refusal::Unpawnable,
    Refusal::Consumable,
    Refusal::Locked,
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
            Refusal::NoProp => "no-prop",
            Refusal::DuplicateProp => "duplicate-prop",
            Refusal::TrainBusy => "train-busy",
            Refusal::NotLearned => "not-learned",
            Refusal::NoSkillTree => "no-skill-tree",
            Refusal::NoSkillPoint => "no-skill-point",
            Refusal::SkillMaxed => "skill-maxed",
            Refusal::LevelGate => "level-gate",
            Refusal::NoItem => "no-item",
            Refusal::OutOfReach => "out-of-reach",
            Refusal::InventoryFull => "inventory-full",
            Refusal::MissingComponent => "missing-component",
            Refusal::Stunned => "stunned",
            Refusal::Silenced => "silenced",
            Refusal::Rooted => "rooted",
            Refusal::Passive => "passive",
            Refusal::SpellImmune => "spell-immune",
            Refusal::Muted => "muted",
            Refusal::Unpawnable => "unpawnable",
            Refusal::Consumable => "consumable",
            Refusal::Locked => "locked",
        }
    }
}

pub(crate) const TARGET_UNIT: u8 = 1;
pub(crate) const TARGET_UNIT_OR_POINT: u8 = 3;

#[derive(Clone, Copy)]
pub(crate) enum Slot {
    Ability(usize),
    Item(usize),
}

pub(crate) fn catalog_index(kind: u32, items: usize) -> Option<usize> {
    (kind != 0 && (kind as usize) <= items).then(|| kind as usize - 1)
}
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
pub(crate) const EFFECT_STAT: u16 = 16;
pub(crate) const EFFECT_TIMED_SPAWN: u16 = 17;
pub(crate) const EFFECT_CARRY: u16 = 19;
const EFFECT_VITAL_FRACTION: u16 = 9;

impl Sim {
    pub(crate) fn refuse(&mut self, r: Refusal) {
        self.stats.casts_refused += 1;
        self.stats.refused_by[r as usize] += 1;
        self.stats.last_refusal = r;
    }

    pub(crate) fn reject(&mut self, r: Refusal) -> Refusal {
        self.refuse(r);
        r
    }

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
        let (energy_cost, cooldown_ticks) = self.cast_costs(i, ai, energy_cost, cooldown_ticks);
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
        let live_unit = target >= 0 && (target as usize) < self.n && self.alive[target as usize] != 0;
        let target = if ability.target == TARGET_UNIT_OR_POINT && !live_unit { -1 } else { target };
        let (tx, ty): (i32, i32) = match ability.target {
            0 => (self.pos_x[i], self.pos_y[i]),
            TARGET_UNIT_OR_POINT if live_unit => (self.pos_x[target as usize], self.pos_y[target as usize]),
            TARGET_UNIT => {
                if target < 0 || target as usize >= self.n || self.alive[target as usize] == 0 {
                    self.refuse(Refusal::NoTarget);
                    return false;
                }
                (self.pos_x[target as usize], self.pos_y[target as usize])
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

        let absorbed = self.absorbs_cast(i, ai, target);
        self.stats.casts += 1;
        self.last_cast[i] = (target, x, y);
        if !absorbed && self.cast_log.len() < CAST_LOG_CAP {
            self.cast_log.push((i as u32, ai as u16));
        }
        if let Slot::Ability(k) = cd {
            self.spend_charge(i, k, ai);
        }
        self.energy[i] -= energy_cost;
        self.set_cooldown(cd, cooldown_ticks);
        if absorbed {
            return true;
        }
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

    pub(crate) fn finish_cast(&mut self, i: usize, ai: usize, tx: i32, ty: i32, target: i32) {
        if self.cat.abilities[ai].is_projectile() {
            self.launch_missile(i, ai, tx, ty, target);
        } else {
            self.apply_effects(i, ai, tx, ty, target);
        }
    }

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

    pub fn apply_effects(&mut self, caster: usize, ai: usize, tx: i32, ty: i32, target: i32) {
        if caster >= self.n || self.alive[caster] == 0 {
            self.refuse(Refusal::NoTarget);
            return;
        }
        if ai >= self.cat.abilities.len() {
            return;
        }
        let ability = self.cat.abilities[ai];
        let first = ability.effect_first as usize;
        let receiver = if ability.target != 0 && target >= 0 && (target as usize) < self.n {
            target
        } else {
            caster as i32
        };
        let mut amount = ability.skill.and_then(|skill| self.rank_amount(caster, ai, skill));
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
                EFFECT_TIMED_SPAWN if ability.target == 0 => {
                    let (cx, cy) = (self.pos_x[caster], self.pos_y[caster]);
                    self.apply_effect(caster, ability.filter as u32, eff, cx, cy, receiver, true);
                }
                _ => self.apply_effect(caster, ability.filter as u32, eff, tx, ty, receiver, true),
            }
            k += 1;
        }
        self.spell.cur_ai = was_ai;
        self.spell.pierce = was_pierce;
    }

    pub(crate) fn run_effects(&mut self, caster: usize, ai: usize, x: i32, y: i32, target: i32) {
        if caster < self.n && self.alive[caster] == 0 {
            self.alive[caster] = 1;
            self.apply_effects(caster, ai, x, y, target);
            self.alive[caster] = 0;
        } else {
            self.apply_effects(caster, ai, x, y, target);
        }
    }

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
            EFFECT_MOVE => self.move_caster(caster, eff.p0, [eff.p1, eff.p2, eff.p3], tx, ty),
            EFFECT_SPAWN => {
                let (sx, sy) = if eff.p1 == 0 && eff.p2 == 0 {
                    (tx, ty)
                } else {
                    (
                        self.pos_x[caster] + fixed::from_milli(eff.p1),
                        self.pos_y[caster] + fixed::from_milli(eff.p2),
                    )
                };
                let kind = eff.p0 as u32;
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
            EFFECT_TIMED_SPAWN => {
                let by_rank = eff.p3 & crate::timed_life::SPAWN_UNIT_PER_RANK != 0;
                let kind = if by_rank { eff.p0 + self.rank_of(caster, self.spell.cur_ai.max(0) as usize).max(1) as i32 - 1 } else { eff.p0 };
                self.timed_spawn_group(caster, kind, eff.p1, eff.p2, eff.p3 & !crate::timed_life::SPAWN_UNIT_PER_RANK, tx, ty)
            }
            EFFECT_CARRY => self.friendly_request_effect(caster, target),
            EFFECT_STUN..=EFFECT_INVISIBILITY => self.status_effect_from(caster, eff, target),
            k if k >= crate::spell::FIRST_SPELL_KIND => self.spell_effect(caster, eff, tx, ty, target),
            EFFECT_ISSUE_ORDER => self.issue_order(caster, eff.p0, eff.p1, tx, ty, target),
            EFFECT_REMOVE_BEHAVIOUR => {
                let _ = self.remove_behaviour(caster, eff.p0, fixed::from_milli(eff.p1), tx, ty, 0);
            }
            EFFECT_SHIELD_BONUS => self.shield_bonus_locked(caster, target, eff.p0, eff.p1, eff.p2),
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

    pub fn spawn_mod_unit(&mut self, player: u8, kind: u32, x: i32, y: i32, route: i32) -> Option<usize> {
        let unit = self.spawn_for(player, kind, x, y)?;
        if route != 0 {
            let id = route as u8;
            if let Some((wx, wy, _)) = self.waypoint_at(player, id, 0) {
                if self.issue_order_kind(unit, ORDER_ATTACK_MOVE, wx, wy).is_ok() {
                    self.dests.id[unit] = id;
                    self.dests.step[unit] = 0;
                    return Some(unit);
                }
            }
            self.request_destination(unit, player);
        }
        Some(unit)
    }

    pub fn take_casts(&mut self) -> Vec<(usize, usize)> {
        std::mem::take(&mut self.cast_log).into_iter().map(|(u, a)| (u as usize, a as usize)).collect()
    }

    pub fn last_cast(&self, i: usize) -> (i32, i32, i32) {
        self.last_cast.get(i).copied().unwrap_or((-1, 0, 0))
    }

    pub fn channel_left(&self, i: usize) -> i32 {
        self.channel_left.get(i).copied().unwrap_or(0)
    }

    pub fn channel_ai(&self, i: usize) -> i32 {
        self.channel_ai.get(i).copied().unwrap_or(-1)
    }

    pub fn meets_filter(&self, j: usize, required: u32) -> bool {
        j < self.n && self.filter[j] & required == required
    }

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

    pub(crate) fn interrupt_channel(&mut self, i: usize) {
        if self.channel_left[i] > 0 {
            self.channel_left[i] = 0;
            self.channel_ai[i] = -1;
            self.touch(i);
        }
    }

}

impl Sim {
    pub fn grant_item(&mut self, unit: usize, item: usize, count: u32) -> bool {
        if item >= self.cat.items.len() || !self.alive(unit) {
            return false;
        }
        self.inventory_add(unit, item as u32 + 1, count)
    }
}

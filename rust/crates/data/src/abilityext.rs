//! Ability extension (AXTN): cast range, cast point, pulse period, proc chance and flags per ability, rank-scaled
//! effect parameters, and conditional modifiers (scepter, shard, talents). See FORMAT.md, section AXTN.

use anyhow::{anyhow, bail, Result};
use starlark::environment::GlobalsBuilder;
use starlark::eval::Evaluator;
use starlark::starlark_module;
use starlark::values::{list::UnpackList, none::NoneType};

use crate::format::{AXTN_FLAG_MASK, AXTN_MODS, AXTN_SCALED, MOD_CONDS, MOD_FIELD_FIRST, SKILL_RANKS};

/// Scaled-entry "effect" values naming a header field: cast range, pulse period, proc chance.
pub const SCALED_CAST_RANGE: u8 = 252;
pub const SCALED_PERIOD: u8 = 253;
pub const SCALED_CHANCE: u8 = 254;
use crate::model::{AbilityRaw, Collector, EffectRaw};

/// A passive ability: never cast, its stat effects apply while a rank is learned.
pub const FLAG_PASSIVE: u16 = 1;
/// Pulses its effects around the owner every `period` ticks while a rank is learned.
pub const FLAG_AURA: u16 = 2;
/// Rolls `chance` percent when the owner lands a weapon hit and then applies its effects to the victim.
pub const FLAG_PROC_ATTACK: u16 = 4;
/// Rolls `chance` percent when the owner is hit by a weapon and then applies its effects around the owner.
pub const FLAG_PROC_ATTACKED: u16 = 8;
/// The ability's effects reach spell-immune enemies.
pub const FLAG_PIERCE: u16 = 16;
/// Every `period`-th weapon hit fires a proc instead of a chance roll.
pub const FLAG_COUNTED: u16 = 64;
/// The ability is castable, but its `stat` effects are passive: applied by rank, skipped on a cast.
pub const FLAG_PASSIVE_STATS: u16 = 128;
/// A unit target must be a friend of the caster (or the caster).
pub const FLAG_ALLY_ONLY: u16 = 256;
/// A unit target must be an enemy of the caster.
pub const FLAG_ENEMY_ONLY: u16 = 512;
/// Channel pulses fire at the cast point of a ground target rather than on the caster.
pub const FLAG_GROUND_PULSE: u16 = 32;

#[derive(Debug)]
pub struct AbilityExtRaw {
    pub ability: String,
    pub cast_range: i32,
    pub cast_point: i32,
    pub period: i32,
    pub chance: i32,
    pub scatter_min: i32,
    pub scatter_max: i32,
    pub charges: i32,
    pub charge_restore: i32,
    pub break_range: i32,
    pub line_extra: i32,
    pub flags: u16,
}

#[derive(Debug)]
pub struct ScaledRaw {
    pub ability: String,
    pub effect: i32,
    pub param: i32,
    pub values: Vec<i32>,
}

#[derive(Debug)]
pub struct ModRaw {
    pub ability: String,
    pub cond: i32,
    pub effect: i32,
    pub param: i32,
    pub mode: i32,
    pub delta: i32,
}

#[derive(Debug, Default)]
pub struct ExtRaw {
    pub exts: Vec<AbilityExtRaw>,
    pub scaled: Vec<ScaledRaw>,
    pub mods: Vec<ModRaw>,
    pub hero_abilities: Vec<(String, Vec<String>)>,
    /// Effects whose first parameter names a unit or an ability: (effect handle, unit name, ability name).
    pub effect_refs: Vec<(usize, Option<String>, Option<String>)>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Scaled {
    pub used: bool,
    pub effect: u8,
    pub param: u8,
    pub values: [i32; SKILL_RANKS],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mod {
    pub used: bool,
    pub cond: u8,
    pub effect: u8,
    pub param: u8,
    pub mode: u8,
    pub delta: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AbilityExt {
    pub flags: u16,
    pub cast_point: u16,
    pub period: i32,
    pub cast_range: i32,
    pub chance: i32,
    pub scatter_min: i32,
    pub scatter_max: i32,
    pub charges: i32,
    pub charge_restore: i32,
    pub break_range: i32,
    pub line_extra: i32,
    pub scaled: [Scaled; AXTN_SCALED],
    pub mods: [Mod; AXTN_MODS],
}

fn ext_of<'a>(raw: &'a mut ExtRaw, ability: &str) -> &'a mut AbilityExtRaw {
    if let Some(at) = raw.exts.iter().position(|e| e.ability == ability) {
        return &mut raw.exts[at];
    }
    raw.exts.push(AbilityExtRaw { ability: ability.to_owned(), cast_range: 0, cast_point: 0, period: 0, chance: 0, scatter_min: 0, scatter_max: 0, charges: 0, charge_restore: 0, break_range: 0, line_extra: 0, flags: 0 });
    raw.exts.last_mut().expect("just pushed")
}

fn collector<'a>(eval: &'a Evaluator<'_, 'a, '_>) -> Result<&'a Collector> {
    eval.extra
        .and_then(|extra| extra.downcast_ref::<Collector>())
        .ok_or_else(|| anyhow!("builtin called without a collector"))
}

#[starlark_module]
pub fn ext_builtins(builder: &mut GlobalsBuilder) {
    /// Extension fields of an ability declared earlier. cast_range in milli-units (0 = no range check),
    /// cast_point and period in ticks, chance in percent for the proc flags.
    fn ability_ext(
        ability: &str,
        #[starlark(require = named)] cast_range: Option<i32>,
        #[starlark(require = named)] cast_point: Option<i32>,
        #[starlark(require = named)] period: Option<i32>,
        #[starlark(require = named)] chance: Option<i32>,
        #[starlark(require = named)] passive: Option<bool>,
        #[starlark(require = named)] aura: Option<bool>,
        #[starlark(require = named)] proc_attack: Option<bool>,
        #[starlark(require = named)] proc_attacked: Option<bool>,
        #[starlark(require = named)] pierce: Option<bool>,
        #[starlark(require = named)] ground_pulse: Option<bool>,
        #[starlark(require = named)] counted: Option<bool>,
        #[starlark(require = named)] scatter_min: Option<i32>,
        #[starlark(require = named)] scatter_max: Option<i32>,
        #[starlark(require = named)] charges: Option<i32>,
        #[starlark(require = named)] charge_restore: Option<i32>,
        #[starlark(require = named)] break_range: Option<i32>,
        #[starlark(require = named)] line_extra: Option<i32>,
        #[starlark(require = named)] passive_stats: Option<bool>,
        #[starlark(require = named)] ally_only: Option<bool>,
        #[starlark(require = named)] enemy_only: Option<bool>,
        eval: &mut Evaluator,
    ) -> anyhow::Result<NoneType> {
        let mut store = collector(eval)?.ext.borrow_mut();
        let e = ext_of(&mut store, ability);
        e.cast_range = cast_range.unwrap_or(e.cast_range);
        e.cast_point = cast_point.unwrap_or(e.cast_point);
        e.period = period.unwrap_or(e.period);
        e.chance = chance.unwrap_or(e.chance);
        e.scatter_min = scatter_min.unwrap_or(e.scatter_min);
        e.scatter_max = scatter_max.unwrap_or(e.scatter_max);
        e.charges = charges.unwrap_or(e.charges);
        e.charge_restore = charge_restore.unwrap_or(e.charge_restore);
        e.break_range = break_range.unwrap_or(e.break_range);
        e.line_extra = line_extra.unwrap_or(e.line_extra);
        for (on, bit) in [
            (passive, FLAG_PASSIVE),
            (aura, FLAG_AURA),
            (proc_attack, FLAG_PROC_ATTACK),
            (proc_attacked, FLAG_PROC_ATTACKED),
            (pierce, FLAG_PIERCE),
            (ground_pulse, FLAG_GROUND_PULSE),
            (counted, FLAG_COUNTED),
            (passive_stats, FLAG_PASSIVE_STATS),
            (ally_only, FLAG_ALLY_ONLY),
            (enemy_only, FLAG_ENEMY_ONLY),
        ] {
            match on {
                Some(true) => e.flags |= bit,
                Some(false) => e.flags &= !bit,
                None => {}
            }
        }
        Ok(NoneType)
    }

    /// Per-rank values of parameter `param` (0..3) of effect number `effect` of an ability: one entry per rank.
    fn ability_scaled(
        ability: &str,
        effect: i32,
        param: i32,
        values: UnpackList<i32>,
        eval: &mut Evaluator,
    ) -> anyhow::Result<NoneType> {
        collector(eval)?.ext.borrow_mut().scaled.push(ScaledRaw {
            ability: ability.to_owned(),
            effect,
            param,
            values: values.items,
        });
        Ok(NoneType)
    }

    /// A modifier active while upgrade bit `cond` is set on the unit (0 scepter, 1 shard, 2..=9 the eight talents).
    /// `effect` is an effect number, or `MOD_FIELD_FIRST` + 0 cooldown, + 1 energy, + 2 cast range, + 3 cast point.
    /// Mode 0 adds `delta`; mode 1 adds `delta` percent of the base value.
    fn ability_mod(
        ability: &str,
        cond: i32,
        effect: i32,
        param: i32,
        delta: i32,
        #[starlark(require = named)] mode: Option<i32>,
        eval: &mut Evaluator,
    ) -> anyhow::Result<NoneType> {
        collector(eval)?.ext.borrow_mut().mods.push(ModRaw {
            ability: ability.to_owned(),
            cond,
            effect,
            param,
            mode: mode.unwrap_or(0),
            delta,
        });
        Ok(NoneType)
    }

    /// Gives the unit `unit` its abilities when its `unit()` line named none.
    fn hero_abilities(unit: &str, abilities: UnpackList<String>, eval: &mut Evaluator) -> anyhow::Result<NoneType> {
        collector(eval)?.ext.borrow_mut().hero_abilities.push((unit.to_owned(), abilities.items));
        Ok(NoneType)
    }
}

/// Resolves the collected extension records against the ability list and effect pool.
pub fn resolve(raw: &ExtRaw, abilities: &[AbilityRaw], effects: &[EffectRaw]) -> Result<Vec<AbilityExt>> {
    let at = |name: &str| -> Result<usize> {
        abilities.iter().position(|a| a.name == name).ok_or_else(|| anyhow!("ability extension names unknown ability {name:?}"))
    };
    let mut out = vec![AbilityExt::default(); abilities.len()];
    for e in &raw.exts {
        let i = at(&e.ability)?;
        if e.cast_range < 0 || e.period < 0 || !(0..=100).contains(&e.chance) || e.scatter_min < 0 || e.scatter_max < e.scatter_min || e.charges < 0 || e.charge_restore < 0 || e.break_range < 0 || e.line_extra < 0 {
            bail!("ability_ext {:?}: cast_range, period or chance out of range", e.ability);
        }
        if !(0..=u16::MAX as i32).contains(&e.cast_point) {
            bail!("ability_ext {:?}: cast_point out of range", e.ability);
        }
        if e.flags & !AXTN_FLAG_MASK != 0 {
            bail!("ability_ext {:?}: unknown flag", e.ability);
        }
        let slot = &mut out[i];
        slot.flags = e.flags;
        slot.cast_point = e.cast_point as u16;
        slot.cast_range = e.cast_range;
        slot.period = e.period;
        slot.chance = e.chance;
        slot.scatter_min = e.scatter_min;
        slot.scatter_max = e.scatter_max;
        slot.charges = e.charges;
        slot.charge_restore = e.charge_restore;
        slot.break_range = e.break_range;
        slot.line_extra = e.line_extra;
    }
    for s in &raw.scaled {
        let i = at(&s.ability)?;
        let count = abilities[i].effects.len();
        let header = (SCALED_CAST_RANGE as i32..=SCALED_CHANCE as i32).contains(&s.effect);
        if !(header || (0..count as i32).contains(&s.effect)) || !(0..4).contains(&s.param) {
            bail!("ability_scaled {:?}: effect {} or param {} out of range (ability has {count} effects)", s.ability, s.effect, s.param);
        }
        if s.values.is_empty() || s.values.len() > SKILL_RANKS {
            bail!("ability_scaled {:?}: needs 1..={SKILL_RANKS} values", s.ability);
        }
        let slot = out[i].scaled.iter_mut().find(|x| !x.used || (x.effect as i32 == s.effect && x.param as i32 == s.param));
        let Some(slot) = slot else { bail!("ability {:?} has more than {AXTN_SCALED} scaled parameters", s.ability) };
        let _ = effects;
        slot.used = true;
        slot.effect = s.effect as u8;
        slot.param = s.param as u8;
        slot.values = [0; SKILL_RANKS];
        for k in 0..SKILL_RANKS {
            slot.values[k] = s.values[k.min(s.values.len() - 1)];
        }
    }
    for m in &raw.mods {
        let i = at(&m.ability)?;
        let count = abilities[i].effects.len() as i32;
        let field = m.effect >= MOD_FIELD_FIRST as i32;
        if !(0..MOD_CONDS as i32).contains(&m.cond)
            || !(0..=1).contains(&m.mode)
            || (field && m.effect >= MOD_FIELD_FIRST as i32 + 5)
            || (!field && (m.effect < 0 || m.effect >= count))
            || (!field && !(0..4).contains(&m.param))
        {
            bail!("ability_mod {:?}: cond, effect, param or mode out of range", m.ability);
        }
        let slot = out[i].mods.iter_mut().find(|x| !x.used);
        let Some(slot) = slot else { bail!("ability {:?} has more than {AXTN_MODS} modifiers", m.ability) };
        *slot = Mod { used: true, cond: m.cond as u8, effect: m.effect as u8, param: m.param as u8, mode: m.mode as u8, delta: m.delta };
    }
    Ok(out)
}

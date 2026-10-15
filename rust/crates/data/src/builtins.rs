//! Starlark builtins: `effect`, `ability`, `unit`, `behaviour`, `spawn`, `grid`, `paint`.
//! Each one validates its enum-like arguments at call time so errors carry the
//! Starlark source location.

use anyhow::{anyhow, bail, Result};
use starlark::environment::{Globals, GlobalsBuilder};
use starlark::eval::Evaluator;
use starlark::starlark_module;
use starlark::values::{list::UnpackList, none::NoneType};

use crate::format::{
    effect_params_valid, index_of, ACTIONS, ACTION_USE_ABILITY, CELL_MASK, DAMAGE_RESPONSES, EFFECT_KINDS,
    FILTERS, MAX_GRID_CELLS, MAX_PLAYERS, RESOURCE_KINDS, TARGETS, TRIGGERS,
};
use crate::model::{AbilityRaw, BehaviourRaw, Collector, EffectRaw, Grid, SpawnRaw, UnitRaw, WeaponRaw};

pub fn globals() -> Globals {
    GlobalsBuilder::standard().with(osdt_builtins).build()
}

fn collector<'a>(eval: &'a Evaluator<'_, 'a, '_>) -> Result<&'a Collector> {
    eval.extra
        .and_then(|extra| extra.downcast_ref::<Collector>())
        .ok_or_else(|| anyhow!("builtin called without a collector"))
}

fn positive(what: &str, value: i32) -> Result<u32> {
    u32::try_from(value)
        .ok()
        .filter(|v| *v >= 1)
        .ok_or_else(|| anyhow!("{what} must be >= 1, got {value}"))
}

fn cell_value(value: i32) -> Result<u8> {
    u8::try_from(value)
        .ok()
        .filter(|v| v & !CELL_MASK == 0)
        .ok_or_else(|| anyhow!("grid cell value {value} is not a walkability bitfield (0..=3)"))
}

fn filter_mask(names: Option<UnpackList<String>>) -> Result<u32> {
    let mut mask = 0u32;
    for name in names.map(|list| list.items).unwrap_or_default() {
        mask |= 1u32 << index_of(FILTERS, &name, "filter")?;
    }
    Ok(mask)
}

#[starlark_module]
fn osdt_builtins(builder: &mut GlobalsBuilder) {
    fn effect(
        kind: &str,
        #[starlark(require = named)] p0: Option<i32>,
        #[starlark(require = named)] p1: Option<i32>,
        #[starlark(require = named)] p2: Option<i32>,
        #[starlark(require = named)] p3: Option<i32>,
        eval: &mut Evaluator,
    ) -> anyhow::Result<i32> {
        let kind_code = index_of(EFFECT_KINDS, kind, "effect kind")?;
        let params = [p0.unwrap_or(0), p1.unwrap_or(0), p2.unwrap_or(0), p3.unwrap_or(0)];
        if !effect_params_valid(kind, params) {
            bail!("effect {kind:?} has out-of-range parameters {params:?}; see FORMAT.md effect table");
        }
        let pool = collector(eval)?;
        let mut effects = pool.effects.borrow_mut();
        effects.push(EffectRaw {
            kind: kind_code,
            params,
        });
        Ok(effects.len() as i32 - 1)
    }

    fn unit(
        name: &str,
        health: i32,
        speed_milli: i32,
        radius_milli: i32,
        #[starlark(require = named)] build_time: Option<i32>,
        #[starlark(require = named)] minerals: Option<i32>,
        #[starlark(require = named)] gas: Option<i32>,
        #[starlark(require = named)] supply: Option<i32>,
        #[starlark(require = named)] abilities: Option<UnpackList<String>>,
        #[starlark(require = named)] filters: Option<UnpackList<String>>,
        #[starlark(require = named)] product: Option<&str>,
        #[starlark(require = named)] weapon_damage: Option<i32>,
        #[starlark(require = named)] weapon_range_milli: Option<i32>,
        #[starlark(require = named)] weapon_period: Option<i32>,
        #[starlark(require = named)] sight_milli: Option<i32>,
        #[starlark(require = named)] shield: Option<i32>,
        #[starlark(require = named)] armor: Option<i32>,
        #[starlark(require = named)] damage_response: Option<&str>,
        #[starlark(require = named)] resource: Option<&str>,
        #[starlark(require = named)] harvest: Option<i32>,
        eval: &mut Evaluator,
    ) -> anyhow::Result<NoneType> {
        let abilities: Vec<String> = abilities.map(|list| list.items).unwrap_or_default();
        let filter_bits = filter_mask(filters)?;
        let weapon = match (weapon_damage, weapon_range_milli, weapon_period) {
            (None, None, None) => None,
            (Some(damage), Some(range_milli), Some(period)) => Some(WeaponRaw {
                damage,
                range_milli,
                period,
            }),
            _ => bail!("unit {name:?}: weapon_damage, weapon_range_milli and weapon_period go together"),
        };
        let damage_response = index_of(DAMAGE_RESPONSES, damage_response.unwrap_or("normal"), "damage response")?;
        let resource_kind = index_of(RESOURCE_KINDS, resource.unwrap_or("none"), "resource kind")?;
        collector(eval)?.units.borrow_mut().push(UnitRaw {
            name: name.to_owned(),
            health,
            speed_milli,
            radius_milli,
            build_time: build_time.unwrap_or(0),
            minerals: minerals.unwrap_or(0),
            gas: gas.unwrap_or(0),
            supply: supply.unwrap_or(0),
            abilities,
            filters: filter_bits,
            product: product.map(str::to_owned),
            weapon,
            sight_milli: sight_milli.unwrap_or(0),
            shield: shield.unwrap_or(0),
            armor: armor.unwrap_or(0),
            damage_response,
            resource_kind,
            harvest: harvest.unwrap_or(0),
        });
        Ok(NoneType)
    }

    fn ability(
        name: &str,
        cooldown: i32,
        energy: i32,
        target: &str,
        effects: Option<UnpackList<i32>>,
        #[starlark(require = named)] target_filters: Option<UnpackList<String>>,
        #[starlark(require = named)] projectile_speed_milli: Option<i32>,
        eval: &mut Evaluator,
    ) -> anyhow::Result<NoneType> {
        let target = index_of(TARGETS, target, "target")?;
        let target_filters = filter_mask(target_filters)?;
        let pool = collector(eval)?;
        let pool_len = pool.effects.borrow().len();
        let mut handles = Vec::new();
        for handle in effects.map(|list| list.items).unwrap_or_default() {
            let index = usize::try_from(handle)
                .ok()
                .filter(|index| *index < pool_len)
                .ok_or_else(|| anyhow!("ability {name:?} got effect handle {handle} that effect() did not return"))?;
            handles.push(index);
        }
        pool.abilities.borrow_mut().push(AbilityRaw {
            name: name.to_owned(),
            cooldown,
            energy,
            target,
            target_filters,
            effects: handles,
            projectile_speed: projectile_speed_milli,
        });
        Ok(NoneType)
    }

    fn behaviour(
        name: &str,
        trigger: &str,
        action: &str,
        #[starlark(require = named)] param: Option<i32>,
        #[starlark(require = named)] ability: Option<&str>,
        eval: &mut Evaluator,
    ) -> anyhow::Result<NoneType> {
        let trigger = index_of(TRIGGERS, trigger, "trigger")?;
        let action = index_of(ACTIONS, action, "action")?;
        if action == ACTION_USE_ABILITY && ability.is_none() {
            bail!("behaviour {name:?} uses use_ability and needs ability=");
        }
        collector(eval)?.behaviours.borrow_mut().push(BehaviourRaw {
            name: name.to_owned(),
            trigger,
            action,
            param: param.unwrap_or(0),
            ability: ability.map(str::to_owned),
        });
        Ok(NoneType)
    }

    fn spawn(unit: &str, player: i32, x: i32, y: i32, eval: &mut Evaluator) -> anyhow::Result<NoneType> {
        if !(0..MAX_PLAYERS).contains(&player) {
            bail!("spawn({unit:?}) player {player} is outside 0..{MAX_PLAYERS}");
        }
        if x < 0 || y < 0 {
            bail!("spawn({unit:?}) position ({x}, {y}) is negative");
        }
        collector(eval)?.spawns.borrow_mut().push(SpawnRaw {
            unit: unit.to_owned(),
            player,
            x,
            y,
        });
        Ok(NoneType)
    }

    fn grid(
        width: i32,
        height: i32,
        #[starlark(require = named)] cell_milli: Option<i32>,
        #[starlark(require = named)] fill: Option<i32>,
        eval: &mut Evaluator,
    ) -> anyhow::Result<NoneType> {
        let pool = collector(eval)?;
        let mut slot = pool.grid.borrow_mut();
        if slot.is_some() {
            bail!("grid() may be called once per map");
        }
        let width = positive("grid width", width)?;
        let height = positive("grid height", height)?;
        let cell_milli = positive("grid cell_milli", cell_milli.unwrap_or(1000))?;
        let cell_count = (width as usize)
            .checked_mul(height as usize)
            .filter(|count| *count <= MAX_GRID_CELLS)
            .ok_or_else(|| anyhow!("grid {width}x{height} exceeds {MAX_GRID_CELLS} cells"))?;
        let fill = cell_value(fill.unwrap_or(0))?;
        *slot = Some(Grid {
            width,
            height,
            cell_milli,
            cells: vec![fill; cell_count],
        });
        Ok(NoneType)
    }

    fn paint(
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        value: i32,
        eval: &mut Evaluator,
    ) -> anyhow::Result<NoneType> {
        let pool = collector(eval)?;
        let mut slot = pool.grid.borrow_mut();
        let grid = slot.as_mut().ok_or_else(|| anyhow!("paint() needs grid() first"))?;
        let (width, height) = (grid.width as i32, grid.height as i32);
        if !(0 <= x0 && x0 <= x1 && x1 <= width && 0 <= y0 && y0 <= y1 && y1 <= height) {
            bail!("paint({x0}, {y0}, {x1}, {y1}) is inverted or outside the {width}x{height} grid");
        }
        let value = cell_value(value)?;
        for y in y0..y1 {
            let row = (y * width) as usize;
            grid.cells[row + x0 as usize..row + x1 as usize].fill(value);
        }
        Ok(NoneType)
    }
}

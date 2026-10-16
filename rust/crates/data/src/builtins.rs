//! Starlark builtins. Each one validates its enum-like arguments at call time so
//! errors carry the Starlark source location.

use anyhow::{anyhow, bail, Result};
use starlark::docs::{DocItem, DocMember, DocParams};
use starlark::environment::{Globals, GlobalsBuilder};
use starlark::eval::Evaluator;
use starlark::starlark_module;
use starlark::values::{list::UnpackList, none::NoneType};

use crate::format::{
    effect_params_valid, index_of, ACTIONS, ACTION_USE_ABILITY, CELL_MASK, CLOAK_FLAGS, DAMAGE_RESPONSES,
    DAMAGE_TYPES, EFFECT_KINDS, FILTERS, MAX_GRID_CELLS, MAX_PLAYERS, RESOURCE_KINDS, REQ_KINDS, SIZE_CATEGORIES,
    DOODAD_KINDS, TARGETS, TRIGGERS, UPGRADE_APPLY, UPGRADE_STACKING, UPGRADE_TARGET_KINDS,
};
use crate::model::{
    AbilityRaw, BehaviourRaw, ChannelRaw, Collector, DamageFactorRaw, EconRaw, EffectRaw, Grid, PrereqRaw,
    DoodadRaw, ResearchRaw, SpawnRaw, UnitExtRaw, UnitRaw, UpgradeRaw, WeaponRaw,
};

pub fn globals() -> Globals {
    GlobalsBuilder::standard().with(osdt_builtins).build()
}

/// Each engine builtin as (name, signature such as `effect(kind, p0, ...)`), read from the
/// same GlobalsBuilder the validator evaluates with.
pub fn signatures() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = globals()
        .documentation()
        .members
        .iter()
        .filter_map(|(name, item)| match item {
            DocItem::Member(DocMember::Function(function)) => Some((name.clone(), signature(name, &function.params))),
            _ => None,
        })
        .collect();
    out.sort();
    out
}

fn signature(name: &str, params: &DocParams) -> String {
    let names: Vec<String> = params
        .pos_only
        .iter()
        .chain(&params.pos_or_named)
        .chain(&params.named_only)
        .map(|p| p.name.clone())
        .chain(params.args.iter().map(|p| format!("*{}", p.name)))
        .chain(params.kwargs.iter().map(|p| format!("**{}", p.name)))
        .collect();
    format!("{name}({})", names.join(", "))
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
        #[starlark(require = named)] resource_a: Option<i32>,
        #[starlark(require = named)] gas: Option<i32>,
        #[starlark(require = named)] capacity: Option<i32>,
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
        #[starlark(require = named)] shield_regen: Option<i32>,
        #[starlark(require = named)] regen_delay: Option<i32>,
        #[starlark(require = named)] health_regen: Option<i32>,
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
            resource_a: resource_a.unwrap_or(0),
            gas: gas.unwrap_or(0),
            capacity: capacity.unwrap_or(0),
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
            shield_regen: shield_regen.unwrap_or(0),
            regen_delay: regen_delay.unwrap_or(0),
            health_regen: health_regen.unwrap_or(0),
        });
        Ok(NoneType)
    }

    fn unit_extra(
        name: &str,
        #[starlark(require = named)] capacity_provided: Option<i32>,
        #[starlark(require = named)] transport_capacity: Option<i32>,
        #[starlark(require = named)] size: Option<&str>,
        #[starlark(require = named)] damage_type: Option<&str>,
        #[starlark(require = named)] cloak: Option<UnpackList<String>>,
        #[starlark(require = named)] detector_radius: Option<i32>,
        #[starlark(require = named)] ranged_miss: Option<i32>,
        #[starlark(require = named)] burrow_ticks: Option<i32>,
        #[starlark(require = named)] rally_dx: Option<i32>,
        #[starlark(require = named)] rally_dy: Option<i32>,
        eval: &mut Evaluator,
    ) -> anyhow::Result<NoneType> {
        let size = index_of(SIZE_CATEGORIES, size.unwrap_or("small"), "size category")?;
        let damage_type = index_of(DAMAGE_TYPES, damage_type.unwrap_or("normal"), "damage type")?;
        let mut cloak_bits = 0u8;
        for flag in cloak.map(|list| list.items).unwrap_or_default() {
            cloak_bits |= 1u8 << index_of(CLOAK_FLAGS, &flag, "cloak flag")?;
        }
        collector(eval)?.unit_ext.borrow_mut().push(UnitExtRaw {
            name: name.to_owned(),
            capacity_provided: capacity_provided.unwrap_or(0),
            transport_capacity: transport_capacity.unwrap_or(0),
            size,
            damage_type,
            cloak: cloak_bits,
            detector_radius: detector_radius.unwrap_or(0),
            ranged_miss: ranged_miss.unwrap_or(0),
            burrow_ticks: burrow_ticks.unwrap_or(0),
            rally_dx: rally_dx.unwrap_or(0),
            rally_dy: rally_dy.unwrap_or(0),
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

    fn channel(ability: &str, duration: i32, eval: &mut Evaluator) -> anyhow::Result<NoneType> {
        collector(eval)?.channels.borrow_mut().push(ChannelRaw {
            ability: ability.to_owned(),
            duration,
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
        if TRIGGERS[trigger] == "ability_ready" && ability.is_none() {
            bail!("behaviour {name:?} fires on ability_ready and needs ability=");
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
        if player != -1 && !(0..MAX_PLAYERS).contains(&player) {
            bail!("spawn({unit:?}) player {player} is outside -1 (neutral) or 0..{MAX_PLAYERS}");
        }
        if x < 0 || y < 0 {
            bail!("spawn({unit:?}) position ({x}, {y}) is negative");
        }
        collector(eval)?.spawns.borrow_mut().push(SpawnRaw {
            unit: Some(unit.to_owned()),
            start: false,
            player,
            x,
            y,
        });
        Ok(NoneType)
    }

    fn start(player: i32, x: i32, y: i32, eval: &mut Evaluator) -> anyhow::Result<NoneType> {
        if x < 0 || y < 0 {
            bail!("start for player {player} at ({x}, {y}) is negative");
        }
        collector(eval)?.spawns.borrow_mut().push(SpawnRaw {
            unit: None,
            player,
            x,
            y,
            start: true,
        });
        Ok(NoneType)
    }

    fn upgrade(
        name: &str,
        kind: &str,
        target: &str,
        effect: i32,
        #[starlark(require = named)] level: Option<i32>,
        #[starlark(require = named)] apply: Option<&str>,
        #[starlark(require = named)] stacking: Option<&str>,
        #[starlark(require = named)] resource_a_per_level: Option<i32>,
        #[starlark(require = named)] gas_per_level: Option<i32>,
        eval: &mut Evaluator,
    ) -> anyhow::Result<NoneType> {
        let target_kind = index_of(UPGRADE_TARGET_KINDS, kind, "upgrade target kind")?;
        let apply = index_of(UPGRADE_APPLY, apply.unwrap_or("new_only"), "upgrade apply")?;
        let stacking = index_of(UPGRADE_STACKING, stacking.unwrap_or("replace"), "upgrade stacking")?;
        let effect = usize::try_from(effect).map_err(|_| anyhow!("upgrade {name:?} has an invalid effect handle"))?;
        collector(eval)?.upgrades.borrow_mut().push(UpgradeRaw {
            name: name.to_owned(),
            target_kind,
            target: target.to_owned(),
            level: level.unwrap_or(1),
            apply,
            stacking,
            effect,
            resource_a_per_level: resource_a_per_level.unwrap_or(0),
            gas_per_level: gas_per_level.unwrap_or(0),
        });
        Ok(NoneType)
    }

    fn research(
        name: &str,
        building: &str,
        time: i32,
        resource_a: i32,
        gas: i32,
        effect: i32,
        #[starlark(require = named)] applies_to: Option<UnpackList<String>>,
        eval: &mut Evaluator,
    ) -> anyhow::Result<NoneType> {
        let effect = usize::try_from(effect).map_err(|_| anyhow!("research {name:?} has an invalid effect handle"))?;
        let applies = filter_mask(applies_to)?;
        collector(eval)?.researches.borrow_mut().push(ResearchRaw {
            name: name.to_owned(),
            building: building.to_owned(),
            time,
            resource_a,
            gas,
            effect,
            applies,
        });
        Ok(NoneType)
    }

    fn prerequisite(
        target_kind: &str,
        target: &str,
        requires_kind: &str,
        requires: &str,
        eval: &mut Evaluator,
    ) -> anyhow::Result<NoneType> {
        let target_kind = index_of(REQ_KINDS, target_kind, "prerequisite target kind")?;
        let requires_kind = index_of(REQ_KINDS, requires_kind, "prerequisite requirement kind")?;
        collector(eval)?.prerequisites.borrow_mut().push(PrereqRaw {
            target_kind,
            target: target.to_owned(),
            requires_kind,
            requires: requires.to_owned(),
        });
        Ok(NoneType)
    }

    fn damage_factor(damage_type: &str, size: &str, factor: i32, eval: &mut Evaluator) -> anyhow::Result<NoneType> {
        let damage_type = index_of(DAMAGE_TYPES, damage_type, "damage type")?;
        let size = index_of(SIZE_CATEGORIES, size, "size category")?;
        if factor < 0 {
            bail!("damage factor {factor} is negative");
        }
        collector(eval)?.damage_factors.borrow_mut().push(DamageFactorRaw {
            damage_type,
            size,
            factor,
        });
        Ok(NoneType)
    }

    fn economy(
        #[starlark(require = named)] patch_saturation: Option<i32>,
        #[starlark(require = named)] gas_saturation: Option<i32>,
        #[starlark(require = named)] patches_per_base: Option<i32>,
        #[starlark(require = named)] geysers_per_base: Option<i32>,
        #[starlark(require = named)] extractor_yield: Option<i32>,
        #[starlark(require = named)] opening_resource_a: Option<i32>,
        #[starlark(require = named)] opening_gas: Option<i32>,
        #[starlark(require = named)] opening_workers: Option<i32>,
        #[starlark(require = named)] capacity_base: Option<i32>,
        #[starlark(require = named)] capacity_max: Option<i32>,
        #[starlark(require = named)] patch_yield: Option<i32>,
        #[starlark(require = named)] geyser_yield: Option<i32>,
        eval: &mut Evaluator,
    ) -> anyhow::Result<NoneType> {
        let pool = collector(eval)?;
        let mut slot = pool.economy.borrow_mut();
        if slot.is_some() {
            bail!("economy() may be called once per map");
        }
        *slot = Some(EconRaw {
            patch_saturation: patch_saturation.unwrap_or(0),
            gas_saturation: gas_saturation.unwrap_or(0),
            patches_per_base: patches_per_base.unwrap_or(0),
            geysers_per_base: geysers_per_base.unwrap_or(0),
            extractor_yield: extractor_yield.unwrap_or(0),
            opening_resource_a: opening_resource_a.unwrap_or(0),
            opening_gas: opening_gas.unwrap_or(0),
            opening_workers: opening_workers.unwrap_or(0),
            capacity_base: capacity_base.unwrap_or(0),
            capacity_max: capacity_max.unwrap_or(0),
            patch_yield: patch_yield.unwrap_or(0),
            geyser_yield: geyser_yield.unwrap_or(0),
        });
        Ok(NoneType)
    }

    fn limit(#[starlark(require = named)] per_cell: i32, eval: &mut Evaluator) -> anyhow::Result<NoneType> {
        let pool = collector(eval)?;
        let mut slot = pool.cell_limit.borrow_mut();
        if slot.is_some() {
            bail!("limit() may be called once per map");
        }
        if per_cell < 0 {
            bail!("limit per_cell {per_cell} is negative");
        }
        *slot = Some(per_cell);
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
            height_step: 1000,
            heights: vec![0; cell_count],
            tiles: vec![0; cell_count],
        });
        Ok(NoneType)
    }

    fn paint_tile(
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        id: i32,
        eval: &mut Evaluator,
    ) -> anyhow::Result<NoneType> {
        let pool = collector(eval)?;
        let mut slot = pool.grid.borrow_mut();
        let grid = slot.as_mut().ok_or_else(|| anyhow!("paint_tile() needs grid() first"))?;
        let (width, height) = (grid.width as i32, grid.height as i32);
        if !(0 <= x0 && x0 <= x1 && x1 <= width && 0 <= y0 && y0 <= y1 && y1 <= height) {
            bail!("paint_tile({x0}, {y0}, {x1}, {y1}) is inverted or outside the {width}x{height} grid");
        }
        let id = u8::try_from(id).map_err(|_| anyhow!("tile id {id} must be 0..=255"))?;
        for y in y0..y1 {
            let row = (y * width) as usize;
            grid.tiles[row + x0 as usize..row + x1 as usize].fill(id);
        }
        Ok(NoneType)
    }

    fn doodad(
        id: i32,
        kind: &str,
        x: i32,
        y: i32,
        #[starlark(require = named)] rotation: Option<i32>,
        eval: &mut Evaluator,
    ) -> anyhow::Result<NoneType> {
        let kind = index_of(DOODAD_KINDS, kind, "doodad kind")?;
        if id < 0 {
            bail!("doodad id {id} is negative");
        }
        collector(eval)?.doodads.borrow_mut().push(DoodadRaw {
            id,
            kind,
            x,
            y,
            rotation: rotation.unwrap_or(0),
        });
        Ok(NoneType)
    }

    fn height_step(milli: i32, eval: &mut Evaluator) -> anyhow::Result<NoneType> {
        let step = positive("height step", milli)?;
        let pool = collector(eval)?;
        let mut slot = pool.grid.borrow_mut();
        let grid = slot.as_mut().ok_or_else(|| anyhow!("height_step() needs grid() first"))?;
        grid.height_step = step;
        Ok(NoneType)
    }

    fn paint_height(
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        level: i32,
        eval: &mut Evaluator,
    ) -> anyhow::Result<NoneType> {
        let pool = collector(eval)?;
        let mut slot = pool.grid.borrow_mut();
        let grid = slot.as_mut().ok_or_else(|| anyhow!("paint_height() needs grid() first"))?;
        let (width, height) = (grid.width as i32, grid.height as i32);
        if !(0 <= x0 && x0 <= x1 && x1 <= width && 0 <= y0 && y0 <= y1 && y1 <= height) {
            bail!("paint_height({x0}, {y0}, {x1}, {y1}) is inverted or outside the {width}x{height} grid");
        }
        let level = u8::try_from(level).map_err(|_| anyhow!("height level {level} must be 0..=255"))?;
        for y in y0..y1 {
            let row = (y * width) as usize;
            grid.heights[row + x0 as usize..row + x1 as usize].fill(level);
        }
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

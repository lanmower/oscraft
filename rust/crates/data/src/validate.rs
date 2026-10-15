//! Cross-record checks and name resolution. Turns the collector into a `Catalog`.

use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};

use crate::format::{
    move_cap_fits_grid, position_on_grid, ACTION_USE_ABILITY, EFFECT_KINDS, MAX_PLAYERS, NEUTRAL_PLAYER, NO_ABILITY,
    NO_UNIT, PLCE_FLAG_START,
};
use crate::model::{Ability, Behaviour, Catalog, Collector, Effect, Spawn, Unit, Upgrade, Weapon};

fn valid_name(name: &str) -> Result<()> {
    let bytes = name.as_bytes();
    let ok = !bytes.is_empty()
        && bytes.len() <= u16::MAX as usize
        && bytes[0].is_ascii_lowercase()
        && bytes.iter().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_');
    if !ok {
        bail!("name {name:?} must match [a-z][a-z0-9_]*");
    }
    Ok(())
}

fn name_index<'a>(kind: &str, names: impl Iterator<Item = &'a str>) -> Result<HashMap<String, usize>> {
    let mut index = HashMap::new();
    for (position, name) in names.enumerate() {
        valid_name(name)?;
        if index.insert(name.to_owned(), position).is_some() {
            bail!("duplicate {kind} name {name:?}");
        }
    }
    Ok(index)
}

fn unsigned(what: &str, value: i32) -> Result<u32> {
    u32::try_from(value).map_err(|_| anyhow!("{what} must be >= 0, got {value}"))
}

pub fn validate(collector: Collector) -> Result<Catalog> {
    let effects = collector.effects.into_inner();
    let units_raw = collector.units.into_inner();
    let abilities_raw = collector.abilities.into_inner();
    let behaviours_raw = collector.behaviours.into_inner();
    let spawns_raw = collector.spawns.into_inner();
    let upgrades_raw = collector.upgrades.into_inner();
    let cell_limit = match collector.cell_limit.into_inner() {
        Some(limit) => unsigned("limit per_cell", limit)?,
        None => 0,
    };
    let grid = collector
        .grid
        .into_inner()
        .ok_or_else(|| anyhow!("no grid() declared in the map"))?;

    if abilities_raw.len() >= NO_ABILITY as usize {
        bail!("too many abilities: {} (max {})", abilities_raw.len(), NO_ABILITY as usize - 1);
    }
    if units_raw.len() >= NO_UNIT as usize {
        bail!("too many units: {} (max {})", units_raw.len(), NO_UNIT as usize - 1);
    }
    let ability_index = name_index("ability", abilities_raw.iter().map(|a| a.name.as_str()))?;
    let unit_index = name_index("unit", units_raw.iter().map(|u| u.name.as_str()))?;
    name_index("behaviour", behaviours_raw.iter().map(|b| b.name.as_str()))?;
    name_index("upgrade", upgrades_raw.iter().map(|u| u.name.as_str()))?;

    let mut abilities = Vec::with_capacity(abilities_raw.len());
    for raw in &abilities_raw {
        if raw.effects.len() > u8::MAX as usize {
            bail!("ability {:?} has more than 255 effects", raw.name);
        }
        let mut resolved = Vec::with_capacity(raw.effects.len());
        for &handle in &raw.effects {
            let effect = effects[handle];
            let kind = EFFECT_KINDS
                .get(effect.kind)
                .ok_or_else(|| anyhow!("ability {:?} uses effect code {} above the registry", raw.name, effect.kind))?;
            let target = effect.params[0] as usize;
            match *kind {
                "spawn" if target >= units_raw.len() => {
                    bail!("ability {:?}: spawn names unit index {target}, only {} units exist", raw.name, units_raw.len())
                }
                "use_calldown" if target >= abilities_raw.len() => bail!(
                    "ability {:?}: use_calldown names ability index {target}, only {} abilities exist",
                    raw.name,
                    abilities_raw.len()
                ),
                "move" if !move_cap_fits_grid(effect.params[0], grid.width, grid.height, grid.cell_milli) => bail!(
                    "ability {:?}: move cap {} is outside 1..=grid diagonal for a {}x{} grid at {} milli",
                    raw.name,
                    effect.params[0],
                    grid.width,
                    grid.height,
                    grid.cell_milli
                ),
                _ => {}
            }
            resolved.push(Effect {
                kind: effect.kind as u16,
                params: effect.params,
            });
        }
        abilities.push(Ability {
            name: raw.name.clone(),
            cooldown: unsigned("cooldown", raw.cooldown)?,
            energy: unsigned("energy", raw.energy)?,
            target: raw.target as u8,
            target_filters: raw.target_filters as u16,
            effects: resolved,
            projectile_speed: raw
                .projectile_speed
                .map(|speed| unsigned("projectile_speed_milli", speed))
                .transpose()?,
        });
    }

    let mut units = Vec::with_capacity(units_raw.len());
    for raw in &units_raw {
        if raw.health <= 0 {
            bail!("unit {:?} needs health > 0", raw.name);
        }
        if raw.abilities.len() > u16::MAX as usize {
            bail!("unit {:?} has too many abilities", raw.name);
        }
        let mut ability_ids = Vec::with_capacity(raw.abilities.len());
        for name in &raw.abilities {
            let index = ability_index
                .get(name)
                .ok_or_else(|| anyhow!("unit {:?} references unknown ability {name:?}", raw.name))?;
            ability_ids.push(*index as u16);
        }
        let product = match &raw.product {
            Some(name) => unit_index
                .get(name)
                .map(|&index| index as u16)
                .ok_or_else(|| anyhow!("unit {:?} produces unknown unit {name:?}", raw.name))?,
            None => NO_UNIT,
        };
        let weapon = match &raw.weapon {
            Some(w) => Some(Weapon {
                damage: w.damage,
                range: unsigned("weapon_range_milli", w.range_milli)?,
                period: unsigned("weapon_period", w.period)?,
            }),
            None => None,
        };
        let harvest = unsigned("harvest", raw.harvest)?;
        if raw.resource_kind == 0 && harvest > 0 {
            bail!("unit {:?} sets harvest without a resource kind", raw.name);
        }
        units.push(Unit {
            name: raw.name.clone(),
            health: unsigned("health", raw.health)?,
            speed_milli: unsigned("speed_milli", raw.speed_milli)?,
            radius_milli: unsigned("radius_milli", raw.radius_milli)?,
            build_time: unsigned("build_time", raw.build_time)?,
            minerals: unsigned("minerals", raw.minerals)?,
            gas: unsigned("gas", raw.gas)?,
            supply: u16::try_from(unsigned("supply", raw.supply)?)
                .map_err(|_| anyhow!("unit {:?} supply exceeds u16", raw.name))?,
            ability_ids,
            filters: raw.filters,
            product,
            weapon,
            sight: unsigned("sight_milli", raw.sight_milli)?,
            shield: unsigned("shield", raw.shield)?,
            armor: raw.armor,
            damage_response: raw.damage_response as u8,
            resource_kind: raw.resource_kind as u8,
            harvest,
            shield_regen: unsigned("shield_regen", raw.shield_regen)?,
            regen_delay: unsigned("regen_delay", raw.regen_delay)?,
            health_regen: unsigned("health_regen", raw.health_regen)?,
        });
    }

    let mut behaviours = Vec::with_capacity(behaviours_raw.len());
    for raw in &behaviours_raw {
        let ability = match &raw.ability {
            Some(name) => {
                let index = ability_index
                    .get(name)
                    .ok_or_else(|| anyhow!("behaviour {:?} references unknown ability {name:?}", raw.name))?;
                *index as u16
            }
            None => NO_ABILITY,
        };
        if raw.action == ACTION_USE_ABILITY && ability == NO_ABILITY {
            bail!("behaviour {:?} uses use_ability without an ability", raw.name);
        }
        behaviours.push(Behaviour {
            name: raw.name.clone(),
            trigger: raw.trigger as u8,
            action: raw.action as u8,
            param: raw.param,
            ability,
        });
    }

    let mut spawns = Vec::with_capacity(spawns_raw.len());
    let mut started = std::collections::HashSet::new();
    for raw in &spawns_raw {
        let neutral = raw.player == -1;
        if !neutral && !(0..MAX_PLAYERS).contains(&raw.player) {
            bail!("placement player {} is outside -1 (neutral) or 0..{MAX_PLAYERS}", raw.player);
        }
        if neutral && raw.unit.is_none() {
            bail!("a start location cannot be neutral");
        }
        if !position_on_grid(raw.x as i64, raw.y as i64, grid.width, grid.height, grid.cell_milli) {
            bail!(
                "placement at ({}, {}) is off the {}x{} grid at {} milli",
                raw.x,
                raw.y,
                grid.width,
                grid.height,
                grid.cell_milli
            );
        }
        let (unit, flags) = match &raw.unit {
            Some(name) => {
                let index = unit_index
                    .get(name)
                    .ok_or_else(|| anyhow!("spawn names unknown unit {name:?}"))?;
                (*index as u16, 0)
            }
            None => {
                if !started.insert(raw.player) {
                    bail!("player {} has more than one start location", raw.player);
                }
                (NO_UNIT, PLCE_FLAG_START)
            }
        };
        spawns.push(Spawn {
            unit,
            player: if neutral { NEUTRAL_PLAYER } else { raw.player as u16 },
            x: raw.x as u32,
            y: raw.y as u32,
            flags,
        });
    }

    let mut upgrades = Vec::with_capacity(upgrades_raw.len());
    for raw in &upgrades_raw {
        let effect = *effects
            .get(raw.effect)
            .ok_or_else(|| anyhow!("upgrade {:?} names an unknown effect handle", raw.name))?;
        let target = match raw.target_kind {
            0 => unit_index
                .get(&raw.target)
                .ok_or_else(|| anyhow!("upgrade {:?} targets unknown unit {:?}", raw.name, raw.target))?,
            _ => ability_index
                .get(&raw.target)
                .ok_or_else(|| anyhow!("upgrade {:?} targets unknown ability {:?}", raw.name, raw.target))?,
        };
        if !(1..=255).contains(&raw.level) {
            bail!("upgrade {:?} level {} is outside 1..=255", raw.name, raw.level);
        }
        upgrades.push(Upgrade {
            name: raw.name.clone(),
            target_kind: raw.target_kind as u8,
            target: *target as u16,
            level: raw.level as u8,
            apply: raw.apply as u8,
            stacking: raw.stacking as u8,
            effect: Effect {
                kind: effect.kind as u16,
                params: effect.params,
            },
        });
    }

    Ok(Catalog {
        units,
        abilities,
        behaviours,
        spawns,
        upgrades,
        cell_limit,
        grid,
    })
}

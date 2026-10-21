//! Cross-record checks and name resolution. Turns the collector into a `Catalog`.

use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, bail, Result};

use crate::format::{
    move_cap_fits_grid, position_on_grid, CELL_BLOCKED, ACTION_USE_ABILITY, FILTER_MASK_BITS, CHANNEL_FLAG, CLOAK_MASK, EFFECT_KINDS, MAX_PLAYERS,
    NEUTRAL_PLAYER, NO_ABILITY, NO_UNIT, NORMAL_GATE, MAX_NORMAL_GATE, PLCE_FLAG_START, PLCE_FLAG_TEAM, PLCE_TEAM_SHIFT, Q16_ONE,
    ITEM_COMPONENTS, ITEM_SHOP_MASK, SKILL_RANKS, ULTIMATE_GATES, ULTIMATE_RANKS, STRUCTURE_SCAN_MAX_CELLS, TELEPORT_RING_MAX_POINTS,
    XP_CURVE_LEN,
};
use crate::model::{
    Ability, AbilityRaw, Behaviour, Catalog, Channel, Collector, DamageFactor, Econ, Effect, EffectRaw, Prereq,
    Research, Route, Skill, SkillRaw, Spawn, Unit, Doodad, Item, UnitExt, Upgrade, Weapon, Tune,
};

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

fn bounded(what: &str, value: i32, max: u32) -> Result<u32> {
    let v = unsigned(what, value)?;
    if v > max {
        bail!("{what} {value} exceeds {max}");
    }
    Ok(v)
}

/// Checks one skill tree against its ability and the effects it names, then resolves it.
fn check_skill(raw: &SkillRaw, ability: &AbilityRaw, effects: &[EffectRaw]) -> Result<Skill> {
    let ranks = raw.damage.len();
    if ranks == 0 || ranks > SKILL_RANKS {
        bail!("skill {:?} needs 1..={SKILL_RANKS} ranks, got {ranks}", raw.ability);
    }
    if raw.gates.len() != ranks || raw.cooldown.len() != ranks || raw.energy.len() != ranks {
        bail!("skill {:?}: gates, damage, cooldown and energy must each list {ranks} ranks", raw.ability);
    }
    if raw.ultimate && ranks > ULTIMATE_RANKS {
        bail!("ultimate {:?} takes at most {ULTIMATE_RANKS} ranks, got {ranks}", raw.ability);
    }
    let mut skill = Skill {
        ultimate: raw.ultimate,
        max_rank: ranks as u8,
        ..Skill::default()
    };
    for k in 0..ranks {
        let gate = raw.gates[k];
        let allowed = if raw.ultimate {
            gate == ULTIMATE_GATES[k]
        } else {
            (NORMAL_GATE..=MAX_NORMAL_GATE).contains(&gate) && (k == 0 || gate >= raw.gates[k - 1])
        };
        if !allowed {
            bail!("skill {:?}: rank {} level gate {gate} is not allowed", raw.ability, k + 1);
        }
        skill.gates[k] = gate;
        skill.damage[k] = unsigned("skill damage", raw.damage[k])? as i32;
        skill.cooldown[k] = unsigned("skill cooldown", raw.cooldown[k])? as i32;
        skill.energy[k] = unsigned("skill energy", raw.energy[k])? as i32;
    }
    let takes_amount = ability.effects.iter().any(|&h| {
        matches!(EFFECT_KINDS.get(effects[h].kind).copied(), Some("damage") | Some("heal"))
    });
    if !takes_amount && skill.damage[..ranks].iter().any(|&d| d != 0) {
        bail!("skill {:?}: damage table is set but the ability has no damage or heal effect", raw.ability);
    }
    Ok(skill)
}

fn required_index(kind: usize, name: &str, units: &HashMap<String, usize>, researches: &HashMap<String, usize>) -> Result<usize> {
    let found = match kind {
        0 => units.get(name),
        _ => researches.get(name),
    };
    found
        .copied()
        .ok_or_else(|| anyhow!("prerequisite names unknown {} {name:?}", if kind == 0 { "unit" } else { "research" }))
}

pub fn validate(collector: Collector) -> Result<Catalog> {
    let mut effects = collector.effects.into_inner();
    let mut units_raw = collector.units.into_inner();
    let abilities_raw = collector.abilities.into_inner();
    let behaviours_raw = collector.behaviours.into_inner();
    let spawns_raw = collector.spawns.into_inner();
    let upgrades_raw = collector.upgrades.into_inner();
    let doodads_raw = collector.doodads.into_inner();
    let routes_raw = collector.routes.into_inner();
    let items_raw = collector.items.into_inner();
    let skills_raw = collector.skills.into_inner();
    let unit_ext_raw = collector.unit_ext.into_inner();
    let channels_raw = collector.channels.into_inner();
    let ext_raw = collector.ext.into_inner();
    let economy_raw = collector.economy.into_inner();
    let researches_raw = collector.researches.into_inner();
    let prereqs_raw = collector.prerequisites.into_inner();
    let damage_factors_raw = collector.damage_factors.into_inner();
    let tuning_raw = collector
        .tuning
        .into_inner()
        .ok_or_else(|| anyhow!("no tuning() declared in the scenario"))?;
    let tune = {
        let t = tuning_raw;
        if t.xp_curve.len() != XP_CURVE_LEN {
            bail!("tuning xp_curve has {} thresholds, expected {XP_CURVE_LEN}", t.xp_curve.len());
        }
        let xp_curve = t
            .xp_curve
            .iter()
            .map(|&v| unsigned("tuning xp_curve threshold", v))
            .collect::<Result<Vec<u32>>>()?;
        if !xp_curve.windows(2).all(|w| w[0] <= w[1]) {
            bail!("tuning xp_curve is not nondecreasing");
        }
        Tune {
            structure_scan_cells: bounded("tuning structure_scan_cells", t.structure_scan_cells, STRUCTURE_SCAN_MAX_CELLS as u32)? as u8,
            teleport_ring_points: {
                if !(1..=TELEPORT_RING_MAX_POINTS as i32).contains(&t.teleport_ring_points) {
                    bail!("tuning teleport_ring_points {} is outside 1..={TELEPORT_RING_MAX_POINTS}", t.teleport_ring_points);
                }
                t.teleport_ring_points as u8
            },
            teleport_ring_steps: {
                if !(1..=u8::MAX as i32).contains(&t.teleport_ring_steps) {
                    bail!("tuning teleport_ring_steps {} is outside 1..=255", t.teleport_ring_steps);
                }
                t.teleport_ring_steps as u8
            },
            best_only: t.best_only,
            xp_curve,
        }
    };
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
    for (unit, list) in &ext_raw.hero_abilities {
        let raw = units_raw
            .iter_mut()
            .find(|u| &u.name == unit)
            .ok_or_else(|| anyhow!("hero_abilities names unknown unit {unit:?}"))?;
        if raw.abilities.is_empty() {
            raw.abilities = list.clone();
        }
    }
    if units_raw.len() >= NO_UNIT as usize {
        bail!("too many units: {} (max {})", units_raw.len(), NO_UNIT as usize - 1);
    }
    let ability_index = name_index("ability", abilities_raw.iter().map(|a| a.name.as_str()))?;
    let unit_index = name_index("unit", units_raw.iter().map(|u| u.name.as_str()))?;
    name_index("behaviour", behaviours_raw.iter().map(|b| b.name.as_str()))?;
    name_index("upgrade", upgrades_raw.iter().map(|u| u.name.as_str()))?;
    let research_index = name_index("research", researches_raw.iter().map(|r| r.name.as_str()))?;
    for (handle, unit, ability) in &ext_raw.effect_refs {
        if let Some(name) = unit {
            effects[*handle].params[0] = *unit_index.get(name).ok_or_else(|| anyhow!("effect names unknown unit {name:?}"))? as i32;
        }
        if let Some(name) = ability {
            effects[*handle].params[0] = *ability_index.get(name).ok_or_else(|| anyhow!("effect names unknown ability {name:?}"))? as i32;
        }
    }

    let mut skills_by_ability: HashMap<usize, Skill> = HashMap::new();
    for raw in &skills_raw {
        let index = *ability_index
            .get(&raw.ability)
            .ok_or_else(|| anyhow!("skill names unknown ability {:?}", raw.ability))?;
        if skills_by_ability.contains_key(&index) {
            bail!("ability {:?} has more than one skill tree", raw.ability);
        }
        let skill = check_skill(raw, &abilities_raw[index], &effects)?;
        skills_by_ability.insert(index, skill);
    }

    let mut abilities = Vec::with_capacity(abilities_raw.len());
    for (ability_no, raw) in abilities_raw.iter().enumerate() {
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
                "spawn" | "ward" if target >= units_raw.len() => {
                    bail!("ability {:?}: spawn names unit index {target}, only {} units exist", raw.name, units_raw.len())
                }
                "spawn" if effect.params[3] & 0x3FFF_FFFF != 0 && !wave_spec_fits(effect.params[3] & 0x3FFF_FFFF, units_raw.len()) => {
                    bail!("ability {:?}: spawn p3 {} names a unit or tag out of range", raw.name, effect.params[3])
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
            initial_cooldown: unsigned("initial_cooldown", raw.initial_cooldown)?,
            skill: skills_by_ability.get(&ability_no).copied(),
        });
    }

    let mut channels = vec![Channel::default(); abilities_raw.len()];
    let mut channel_seen = HashSet::new();
    for raw in &channels_raw {
        let index = *ability_index
            .get(&raw.ability)
            .ok_or_else(|| anyhow!("channel names unknown ability {:?}", raw.ability))?;
        if !channel_seen.insert(index) {
            bail!("ability {:?} has more than one channel", raw.ability);
        }
        if raw.duration < 1 {
            bail!("channel on {:?} needs a duration of at least 1 tick", raw.ability);
        }
        channels[index] = Channel {
            flags: CHANNEL_FLAG,
            duration: unsigned("channel duration", raw.duration)?,
        };
    }

    let ext = crate::abilityext::resolve(&ext_raw, &abilities_raw, &effects)?;

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
            resource_a: unsigned("resource_a", raw.resource_a)?,
            gas: unsigned("gas", raw.gas)?,
            capacity: u16::try_from(unsigned("capacity", raw.capacity)?)
                .map_err(|_| anyhow!("unit {:?} capacity exceeds u16", raw.name))?,
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
            model: raw.model.clone(),
        });
    }

    let mut unit_ext = vec![UnitExt::default(); units_raw.len()];
    let mut ext_seen = HashSet::new();
    for raw in &unit_ext_raw {
        let index = *unit_index
            .get(&raw.name)
            .ok_or_else(|| anyhow!("unit_extra names unknown unit {:?}", raw.name))?;
        if !ext_seen.insert(index) {
            bail!("unit {:?} has more than one unit_extra", raw.name);
        }
        if raw.cloak as u8 & !CLOAK_MASK != 0 {
            bail!("unit {:?} cloak flags exceed the cloak registry", raw.name);
        }
        unit_ext[index] = UnitExt {
            capacity_provided: u16::try_from(unsigned("capacity_provided", raw.capacity_provided)?)
                .map_err(|_| anyhow!("unit {:?} capacity_provided exceeds u16", raw.name))?,
            transport_capacity: u16::try_from(unsigned("transport_capacity", raw.transport_capacity)?)
                .map_err(|_| anyhow!("unit {:?} transport_capacity exceeds u16", raw.name))?,
            size: raw.size as u8,
            damage_type: raw.damage_type as u8,
            cloak: raw.cloak,
            detector_radius: unsigned("detector_radius", raw.detector_radius)?,
            ranged_miss: bounded("ranged_miss", raw.ranged_miss, Q16_ONE as u32)?,
            burrow_ticks: unsigned("burrow_ticks", raw.burrow_ticks)?,
            rally_dx: raw.rally_dx,
            rally_dy: raw.rally_dy,
            bounty: unsigned("bounty", raw.bounty)?,
            deny_pct: 0,
            deny_xp_pct: bounded("deny_xp_pct", raw.deny_xp_pct, 100)? as u8,
            player_controlled: raw.player_controlled,
            xp_bounty: unsigned("xp_bounty", raw.xp_bounty)?,
            assist_radius: unsigned("assist_radius_milli", raw.assist_radius_milli)?,
            acquire: unsigned("acquire", raw.acquire)?,
        };
    }
    for (index, raw) in units_raw.iter().enumerate() {
        unit_ext[index].deny_pct = bounded("deny_pct", raw.deny_pct, 100)? as u8;
    }

    let econ = match economy_raw {
        None => Econ::default(),
        Some(e) => {
            if e.patch_saturation < 1 || e.gas_saturation < 1 {
                bail!("economy saturation counts must be at least 1");
            }
            Econ {
                patch_saturation: bounded("patch_saturation", e.patch_saturation, u8::MAX as u32)? as u8,
                gas_saturation: bounded("gas_saturation", e.gas_saturation, u8::MAX as u32)? as u8,
                patches_per_base: bounded("patches_per_base", e.patches_per_base, u8::MAX as u32)? as u8,
                geysers_per_base: bounded("geysers_per_base", e.geysers_per_base, u8::MAX as u32)? as u8,
                extractor_yield: unsigned("extractor_yield", e.extractor_yield)?,
                opening_resource_a: unsigned("opening_resource_a", e.opening_resource_a)?,
                opening_gas: unsigned("opening_gas", e.opening_gas)?,
                opening_workers: bounded("opening_workers", e.opening_workers, u16::MAX as u32)? as u16,
                capacity_base: bounded("capacity_base", e.capacity_base, u16::MAX as u32)? as u16,
                capacity_max: bounded("capacity_max", e.capacity_max, u16::MAX as u32)? as u16,
                patch_yield: unsigned("patch_yield", e.patch_yield)?,
                geyser_yield: unsigned("geyser_yield", e.geyser_yield)?,
            }
        }
    };

    let mut researches = Vec::with_capacity(researches_raw.len());
    for raw in &researches_raw {
        let building = *unit_index
            .get(&raw.building)
            .ok_or_else(|| anyhow!("research {:?} runs at unknown unit {:?}", raw.name, raw.building))?;
        let effect = *effects
            .get(raw.effect)
            .ok_or_else(|| anyhow!("research {:?} names an unknown effect handle", raw.name))?;
        researches.push(Research {
            name: raw.name.clone(),
            building: building as u16,
            time: unsigned("research time", raw.time)?,
            resource_a: unsigned("research resource_a", raw.resource_a)?,
            gas: unsigned("research gas", raw.gas)?,
            effect: Effect {
                kind: effect.kind as u16,
                params: effect.params,
            },
            applies: raw.applies,
        });
    }

    let mut prereqs = Vec::with_capacity(prereqs_raw.len());
    for raw in &prereqs_raw {
        let target = required_index(raw.target_kind, &raw.target, &unit_index, &research_index)?;
        let requires = required_index(raw.requires_kind, &raw.requires, &unit_index, &research_index)?;
        prereqs.push(Prereq {
            target_kind: raw.target_kind as u8,
            target: target as u16,
            requires_kind: raw.requires_kind as u8,
            requires: requires as u16,
        });
    }

    let mut damage_factors = Vec::with_capacity(damage_factors_raw.len());
    let mut factor_seen = HashSet::new();
    for raw in &damage_factors_raw {
        if !factor_seen.insert((raw.damage_type, raw.size)) {
            bail!("more than one damage factor for type {} and size {}", raw.damage_type, raw.size);
        }
        if raw.factor < 0 {
            bail!("damage factor {} is negative", raw.factor);
        }
        damage_factors.push(DamageFactor {
            damage_type: raw.damage_type as u8,
            size: raw.size as u8,
            factor: raw.factor as u32,
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
    let mut started = HashSet::new();
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
                let team_flags = match raw.team {
                    Some(team) => PLCE_FLAG_TEAM | ((team as u32) << PLCE_TEAM_SHIFT),
                    None => 0,
                };
                (NO_UNIT, PLCE_FLAG_START | team_flags)
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
            resource_a_per_level: unsigned("upgrade resource_a per level", raw.resource_a_per_level)?,
            gas_per_level: unsigned("upgrade gas per level", raw.gas_per_level)?,
            effect: Effect {
                kind: effect.kind as u16,
                params: effect.params,
            },
        });
    }

    let mut doodads = Vec::with_capacity(doodads_raw.len());
    let mut doodad_ids = HashSet::new();
    for raw in &doodads_raw {
        if raw.id < 0 || !doodad_ids.insert(raw.id) {
            bail!("doodad id {} is negative or repeats", raw.id);
        }
        if !position_on_grid(raw.x as i64, raw.y as i64, grid.width, grid.height, grid.cell_milli) {
            bail!("doodad {} at ({}, {}) is off the grid", raw.id, raw.x, raw.y);
        }
        if !(0..1000).contains(&raw.rotation) {
            bail!("doodad {} rotation {} is outside 0..1000", raw.id, raw.rotation);
        }
        doodads.push(Doodad {
            id: raw.id as u32,
            kind: raw.kind as u16,
            x: raw.x as u32,
            y: raw.y as u32,
            rotation: raw.rotation as u32,
        });
    }

    let mut routes = Vec::with_capacity(routes_raw.len());
    let mut route_keys = HashSet::new();
    for raw in &routes_raw {
        if !route_keys.insert((raw.player, raw.lane)) {
            bail!("route for player {} lane {} is declared twice", raw.player, raw.lane);
        }
        let mut points = Vec::with_capacity(raw.points.len());
        for (k, &(x, y)) in raw.points.iter().enumerate() {
            if !position_on_grid(x as i64, y as i64, grid.width, grid.height, grid.cell_milli) {
                bail!("route player {} lane {} point {k} ({x}, {y}) is off the grid", raw.player, raw.lane);
            }
            let cell = (y as u32 / grid.cell_milli) as usize * grid.width as usize + (x as u32 / grid.cell_milli) as usize;
            if grid.cells[cell] & CELL_BLOCKED != 0 {
                bail!("route player {} lane {} point {k} ({x}, {y}) is on a blocked cell", raw.player, raw.lane);
            }
            points.push((x as u32, y as u32));
        }
        routes.push(Route { player: raw.player as u16, lane: raw.lane as u16, points });
    }

    name_index("item", items_raw.iter().map(|i| i.name.as_str()))?;
    let mut items = Vec::with_capacity(items_raw.len());
    for (at, raw) in items_raw.iter().enumerate() {
        let ability = match &raw.ability {
            Some(name) => match ability_index.get(name) {
                Some(&index) => index as u16,
                None => bail!("item {:?} names unknown ability {name:?}", raw.name),
            },
            None => NO_ABILITY,
        };
        let mut components = [NO_ABILITY; ITEM_COMPONENTS];
        let mut component_cost: i64 = 0;
        for (slot, name) in raw.components.iter().enumerate() {
            match items_raw[..at].iter().position(|earlier| earlier.name == *name) {
                Some(index) => {
                    components[slot] = index as u16;
                    component_cost += i64::from(items_raw[index].cost);
                }
                None => bail!("item {:?} names component {name:?} not declared before it", raw.name),
            }
        }
        if component_cost > i64::from(raw.cost) {
            bail!("item {:?}: its components cost {component_cost}, more than its cost {}", raw.name, raw.cost);
        }
        if [raw.charges, raw.stack_max, raw.stock_max, raw.stock_initial, raw.stock_time_s, raw.stock_start_s]
            .iter()
            .any(|&v| v < 0 || v > i32::from(u16::MAX))
            || raw.stock_max > i32::from(u8::MAX)
            || raw.stock_initial > raw.stock_max
            || (raw.stack_max > 0 && raw.charges > raw.stack_max)
            || !(0..=i32::from(ITEM_SHOP_MASK)).contains(&raw.shops)
        {
            bail!("item {:?}: charges, stack or stock outside their ranges", raw.name);
        }
        items.push(Item {
            name: raw.name.clone(),
            cost: raw.cost as u32,
            stats: raw.stats,
            ability,
            components,
            flags: raw.flags,
            tier: raw.tier as u8,
            shops: raw.shops as u8,
            charges: raw.charges as u16,
            stack_max: raw.stack_max as u16,
            stock_max: raw.stock_max as u8,
            stock_initial: raw.stock_initial as u8,
            stock_time_s: raw.stock_time_s as u16,
            stock_start_s: raw.stock_start_s as u16,
        });
    }

    Ok(Catalog {
        items,
        doodads,
        units,
        unit_ext,
        abilities,
        channels,
        ext,
        behaviours,
        spawns,
        upgrades,
        cell_limit,
        econ,
        tune,
        researches,
        prereqs,
        damage_factors,
        grid,
        routes,
    })
}

/// Whether a spawn p3 wave spec (see sim waves.rs: two kind fields of 10 bits, two tag fields of 5 bits, the route lane in the top two bits)
/// names only existing units and filter bits.
fn wave_spec_fits(spec: i32, units: usize) -> bool {
    let spec = (spec as u32 & 0x3FFF_FFFF) as i32;
    let kind = |shift: u32| ((spec >> shift) & 0x3FF) as usize;
    let tag = |shift: u32| ((spec >> shift) & 0x1F) as u32;
    kind(0) <= units && kind(10) <= units && tag(20) <= FILTER_MASK_BITS && tag(25) <= FILTER_MASK_BITS
}
